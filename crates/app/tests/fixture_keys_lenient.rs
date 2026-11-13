//! #812: the fixture marker's `vise` and `accessory` keys are read
//! leniently. A key this build can't read (an unknown kind, a bad shape)
//! is dropped on its own with a load note naming the fixture and the
//! key; the fixture keeps its saved steel as plain steel and the rest of
//! the file loads. And a strap clamp's stud wider than its strap is
//! refused by name when the command is built.
//!
//! Gate map (M4.4): covers app::project (fixture_keys, vise_dto, the
//! fixture marker's load), app::accessory (the strap clamp law),
//! app::commands::accessory.
//! Provenance: our own tests over our own file format.

use camcraft_app::accessory::Accessory;
use camcraft_app::commands::{AccessoryPlacement, History, InsertAccessory, InsertVise, SetAccessory, VisePlacement};
use camcraft_app::fixture;
use camcraft_app::geometry::Point3;
use camcraft_app::model;
use camcraft_app::project;
use camcraft_app::schema::Location;
use camcraft_app::vise;
use camcraft_app::world::new_world;
use flecs_ecs::prelude::World;
use serde_json::{Value, json};

fn at(x: f64) -> Location {
    Location { pos: Point3::new(x, 0.0, 0.0), ..Location::default() }
}

fn vise_at(world: &World, history: &mut History, x: f64) -> String {
    let six = vise::size("6 in").expect("6 in").half_open();
    let cmd = InsertVise::new(world, &six, None, VisePlacement::At(at(x))).expect("vise");
    let name = cmd.name().to_owned();
    history.execute(world, cmd);
    name
}

fn accessory_at(world: &World, history: &mut History, kind: &str, x: f64) -> String {
    let a = Accessory::default_of(kind).expect(kind);
    let cmd = InsertAccessory::new(world, &a, None, AccessoryPlacement::At(at(x))).expect("accessory");
    let name = cmd.name().to_owned();
    history.execute(world, cmd);
    name
}

/// The `fixture` object of the entity named `name` in a saved project.
fn marker<'a>(j: &'a mut Value, name: &str) -> &'a mut serde_json::Map<String, Value> {
    let entities = j.get_mut("entities").and_then(Value::as_array_mut).expect("entities");
    let e = entities.iter_mut().find(|e| e.get("name").and_then(Value::as_str) == Some(name)).expect(name);
    e.get_mut("fixture").and_then(Value::as_object_mut).expect("a fixture marker")
}

#[test]
fn unknown_or_malformed_fixture_keys_are_dropped_with_a_note_and_the_load_goes_on() {
    let world = new_world();
    let mut history = History::default();
    let odd_vise = vise_at(&world, &mut history, 0.0);
    let good_vise = vise_at(&world, &mut history, 400.0);
    let odd_kind = accessory_at(&world, &mut history, "block", 800.0);
    let bad_shape = accessory_at(&world, &mut history, "soft_jaw", 1000.0);
    let good = accessory_at(&world, &mut history, "strap_clamp", 1200.0);

    let text = serde_json::to_string(&project::collect(&world)).expect("save");
    let mut j: Value = serde_json::from_str(&text).expect("json");
    // A vise kind a later release might add, in a shape this build can't read.
    marker(&mut j, &odd_vise).insert("vise".into(), json!({ "kind": "hydraulic", "jaw_width": 150.0 }));
    // An accessory kind this build does not know.
    marker(&mut j, &odd_kind).insert("accessory".into(), json!({ "kind": "vacuum_pod", "diameter": 100.0 }));
    // A known kind with a malformed figure.
    if let Some(a) = marker(&mut j, &bad_shape).get_mut("accessory").and_then(Value::as_object_mut) {
        a.insert("height".into(), json!("tall"));
    }

    let back = new_world();
    let notes = project::open_noted(&back, &j.to_string()).expect("the file still loads");
    assert_eq!(notes.len(), 3, "one note per dropped key: {notes:?}");
    for (name, key) in [(&odd_vise, "vise"), (&odd_kind, "accessory"), (&bad_shape, "accessory")] {
        assert!(
            notes.iter().any(|n| n.contains(&format!("fixture '{name}'")) && n.contains(&format!("{key} key not understood"))),
            "a note names {name}'s {key} key: {notes:?}"
        );
        let e = model::find_by_name(&back, name).expect("the fixture is kept");
        let def = fixture::def_of(&back, &e).expect("still a fixture");
        assert!(def.vise.is_none() && def.accessory.is_none(), "{name} is plain steel now");
        assert!(model::facets_of(&back, &e).is_some_and(|f| !f.is_empty()), "{name}'s saved steel stays");
    }
    assert!(notes.iter().any(|n| n.contains("vacuum_pod")), "the unknown kind is named: {notes:?}");
    // The keys it could read are untouched.
    let def = |n: &str| model::find_by_name(&back, n).and_then(|e| fixture::def_of(&back, &e));
    assert!(def(&good_vise).is_some_and(|d| d.vise.is_some()), "the other vise stays a vise");
    assert_eq!(def(&good).and_then(|d| d.accessory), Accessory::default_of("strap_clamp"), "the clamp stays a clamp");

    // A clean file has no notes, and a saved-then-loaded file says the same.
    let again = new_world();
    assert_eq!(project::open_noted(&again, &text).expect("loads"), Vec::<String>::new());
}

#[test]
fn a_law_breaking_accessory_key_is_noted_too() {
    let world = new_world();
    let mut history = History::default();
    let block = accessory_at(&world, &mut history, "block", 0.0);
    let mut j: Value = serde_json::from_str(&serde_json::to_string(&project::collect(&world)).expect("save")).expect("json");
    if let Some(a) = marker(&mut j, &block).get_mut("accessory").and_then(Value::as_object_mut) {
        a.insert("height".into(), json!(-1.0));
    }
    let back = new_world();
    let notes = project::open_noted(&back, &j.to_string()).expect("loads");
    assert!(
        notes.len() == 1 && notes[0].contains(&format!("fixture '{block}'")) && notes[0].contains("accessory parameters dropped on load"),
        "{notes:?}"
    );
}

#[test]
fn a_stud_wider_than_its_strap_is_refused_by_name() {
    let world = new_world();
    let Some(Accessory::StrapClamp { strap_width, .. }) = Accessory::default_of("strap_clamp") else { panic!("clamp") };
    let wide = Accessory::default_of("strap_clamp").expect("clamp").with_dim("stud_diameter", strap_width).expect("dim");
    let e = InsertAccessory::new(&world, &wide, None, AccessoryPlacement::At(at(0.0))).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("stud diameter") && e.contains("enclosing") && e.contains("must be less than the strap width"), "{e}");

    let mut history = History::default();
    let clamp = accessory_at(&world, &mut history, "strap_clamp", 0.0);
    let e = SetAccessory::new(&world, &clamp, wide).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("must be less than the strap width"), "a resize is refused the same way: {e}");
    // Just under the strap still passes.
    let snug = Accessory::default_of("strap_clamp").expect("clamp").with_dim("stud_diameter", 0.9 * strap_width).expect("dim");
    assert!(snug.check().is_ok(), "{:?}", snug.check());
}
