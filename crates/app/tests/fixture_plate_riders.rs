//! Fixture plates (#819, review of PR #823): a plate that goes away -
//! deleted (from the Fixtures page or the tree), untagged, or
//! re-imported without its grid - lets go of its riders in the same
//! command, so a later plate given the same name never carries a fixture
//! that was never placed on it; one undo puts plate and places back. A
//! file naming a plate it does not hold loads with the place dropped. A
//! moved plate's riders are clash-checked where they land, not only the
//! plate - and so are an edited plate's (`SetPlate`, #820 review). The
//! grid law refuses holes closer than a hole apart.
//!
//! Gate map (M4.4): covers app::plate::mount (Unmounts, drop_dangling),
//! app::plate::grid (Grid::check), app::commands::fixture (DeleteFixture,
//! UntagFixture, MoveFixture), app::commands::fixture_plate (SetPlate
//! warnings), app::fixture_clash, app::commands::delete_entity,
//! app::commands::import_fixture, app::project (load).

use camcraft_app::commands::{
    DeleteEntity, DeleteFixture, History, ImportFixture, ImportModel, InsertPlate, MoveFixture, PlaceOnPlate,
    PlatePlacement, SetPlate, SetStock, TagFixture, UntagFixture,
};
use camcraft_app::fixture;
use camcraft_app::geometry::{IDENTITY_Q, Mesh, Point3};
use camcraft_app::model;
use camcraft_app::plate::{self, MountAt, PlateMount};
use camcraft_app::project;
use camcraft_app::schema::Location;
use camcraft_app::stock::StockShape;
use camcraft_app::world::new_world;
use flecs_ecs::prelude::World;

fn close(a: Point3, b: Point3) -> bool {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() < 1e-6
}

fn pose(world: &World, name: &str) -> Point3 {
    model::find_by_name(world, name).and_then(|e| model::transform_of(world, &e)).expect(name).pos
}

fn mount_of(world: &World, name: &str) -> Option<PlateMount> {
    model::find_by_name(world, name).and_then(|e| fixture::def_of(world, &e)).and_then(|d| d.plate.mount)
}

fn at(x: f64, y: f64, z: f64) -> Location {
    Location { pos: Point3::new(x, y, z), rot: IDENTITY_Q }
}

/// A 300 mm plate at `x`, its name.
fn insert_plate(world: &World, history: &mut History, x: f64) -> String {
    let p = plate::preset("300 x 300 mm").map(plate::PlatePreset::params).expect("preset");
    let cmd = InsertPlate::new(world, &p, None, PlatePlacement::At(at(x, 0.0, 0.0))).expect("plate");
    let name = cmd.name().to_owned();
    history.execute(world, cmd);
    name
}

/// A 10 mm block fixture placed at B2 of `plate`.
fn block_on(world: &World, history: &mut History, plate: &str) {
    history.execute(world, ImportModel::from_mesh("Block", Mesh::from_facets(plate::slab(10.0, 10.0, 10.0))));
    history.execute(world, TagFixture::new(world, "Block", None).expect("tag"));
    history.execute(world, PlaceOnPlate::new(world, "Block", plate, MountAt::Hole("B2".into()), 0.0).expect("B2"));
}

const B2: Point3 = Point3 { x: 37.5, y: 37.5, z: 10.0 };

/// The reviewer's scenario: place, delete the plate, insert a new one
/// of the same name, move it - the block stays put and unmounted; undo
/// of the delete puts it back on the plate at B2.
#[test]
fn a_deleted_plates_name_never_carries_its_old_riders() {
    let world = new_world();
    let mut history = History::default();
    let p = insert_plate(&world, &mut history, 0.0);
    block_on(&world, &mut history, &p);
    assert!(close(pose(&world, "Block"), B2), "{:?}", pose(&world, "Block"));

    let del = DeleteFixture::new(&world, &p).expect("delete");
    history.execute(&world, del);
    assert!(mount_of(&world, "Block").is_none(), "the delete takes the block off the gone plate");
    assert!(close(pose(&world, "Block"), B2), "it stays where it stands");

    let again = insert_plate(&world, &mut history, 500.0);
    assert_eq!(again, p, "the name is free again");
    let mv = MoveFixture::new(&world, &again, at(500.0, 200.0, 0.0)).expect("move");
    assert_eq!(mv.carried().count(), 0, "the new plate carries nothing");
    history.execute(&world, mv);
    assert!(close(pose(&world, "Block"), B2) && mount_of(&world, "Block").is_none());

    assert!(history.undo(&world) && history.undo(&world), "undo the move and the insert");
    assert!(history.undo(&world), "undo the delete");
    let m = mount_of(&world, "Block").expect("back on the plate");
    assert_eq!((m.plate.as_str(), m.at), (p.as_str(), MountAt::Hole("B2".into())));
    assert!(close(pose(&world, "Block"), B2));
    assert!(history.redo(&world));
    assert!(mount_of(&world, "Block").is_none(), "redo lets go again");
}

/// The same law on the other ways a plate stops being one.
#[test]
fn tree_delete_untag_and_a_gridless_reimport_let_go_of_riders() {
    let world = new_world();
    let mut history = History::default();
    let p = insert_plate(&world, &mut history, 0.0);
    block_on(&world, &mut history, &p);

    history.execute(&world, DeleteEntity::new(&world, &p).expect("tree delete"));
    assert!(mount_of(&world, "Block").is_none());
    assert!(history.undo(&world));
    assert!(mount_of(&world, "Block").is_some());

    history.execute(&world, UntagFixture::new(&world, &p).expect("untag"));
    assert!(mount_of(&world, "Block").is_none());
    assert!(history.undo(&world));
    assert!(mount_of(&world, "Block").is_some());

    let re = ImportModel::from_mesh(&p, Mesh::from_facets(plate::slab(300.0, 300.0, 25.0)));
    history.execute(&world, ImportFixture::new(&world, re, None).expect("re-import"));
    assert!(mount_of(&world, "Block").is_none(), "the grid went with the old geometry");
    assert!(history.undo(&world));
    assert!(mount_of(&world, "Block").is_some_and(|m| m.plate == p));
    assert!(close(pose(&world, "Block"), B2));
}

/// A file whose place names a plate it does not hold loads, the place
/// dropped.
#[test]
fn a_place_on_a_missing_plate_is_dropped_on_load() {
    let world = new_world();
    let mut history = History::default();
    let p = insert_plate(&world, &mut history, 0.0);
    block_on(&world, &mut history, &p);
    let path = std::env::temp_dir().join(format!("camcraft-plate-riders-{}.camproj", std::process::id()));
    project::save(&world, &path).expect("save");
    let text = std::fs::read_to_string(&path).expect("read");
    let mut j: serde_json::Value = serde_json::from_str(&text).expect("json");
    let mut renamed = 0;
    if let Some(es) = j.get_mut("entities").and_then(|e| e.as_array_mut()) {
        for e in es {
            if let Some(m) = e.pointer_mut("/fixture/on_plate/plate") {
                *m = serde_json::json!("Nowhere");
                renamed += 1;
            }
        }
    }
    assert_eq!(renamed, 1, "the place names its plate");
    std::fs::write(&path, j.to_string()).expect("write");
    let back = new_world();
    let loaded = project::load(&back, &path);
    let _ = std::fs::remove_file(&path);
    loaded.expect("still loads");
    assert!(mount_of(&back, "Block").is_none());
    assert!(close(pose(&back, "Block"), B2), "the block stays where it was saved");
}

/// A moved plate's riders are judged where they land: the block lands in
/// a part the plate itself misses.
#[test]
fn a_moved_plates_riders_are_clash_checked() {
    let world = new_world();
    let mut history = History::default();
    let p = insert_plate(&world, &mut history, 0.0);
    block_on(&world, &mut history, &p);
    // A 20 mm cube over the plate (z 5..25), where B2 lands after +200 Y.
    history.execute(&world, ImportModel::from_mesh("Part", Mesh::from_facets(plate::slab(20.0, 20.0, 20.0))));
    let part = model::find_by_name(&world, "Part").expect("part");
    model::set_transform(&world, &part, at(32.5, 232.5, 25.0));
    let still = MoveFixture::new(&world, &p, at(1.0, 0.0, 0.0)).expect("move");
    assert_eq!(still.warning(), None, "nowhere near the part");
    let mv = MoveFixture::new(&world, &p, at(0.0, 200.0, 0.0)).expect("move");
    let w = mv.warning().expect("the carried block sinks into the part");
    assert!(w.contains("Block") && !w.contains(&p), "{w}");
}

/// The grid law: holes no closer than a hole apart.
#[test]
fn a_grid_whose_holes_overlap_is_refused() {
    let p = plate::preset("300 x 300 mm").map(plate::PlatePreset::params).expect("preset");
    let mut def = p.def().expect("def");
    def.params = None;
    def.holes.diameter = def.holes.pitch_x + 1.0;
    let e = def.check().expect_err("overlapping holes");
    assert!(e.contains("run into each other"), "{e}");
    def.holes.diameter = 0.01;
    def.holes.pitch_x = 0.08;
    let e = def.check().expect_err("a pitch inside the snap tolerance");
    assert!(e.contains("pitch must be more than 0.10"), "{e}");
}

/// #820 review: `SetPlate` re-seats a plain rider on its named hole but
/// the stock stays put - a rider pushed into the stock is warned about by
/// name, as a `MoveFixture` is; an edit that keeps it clear is quiet; undo
/// and redo put the poses back to the bit.
#[test]
fn an_edited_plates_riders_are_clash_checked_where_they_land() {
    let world = new_world();
    let mut history = History::default();
    let p = insert_plate(&world, &mut history, 0.0);
    block_on(&world, &mut history, &p);
    let steel = model::find_by_name(&world, "Block").and_then(|e| model::world_facets(&world, &e)).expect("block");
    let (x_max, z_min) = steel.iter().flat_map(|t| [t.a, t.b, t.c]).fold((f64::MIN, f64::MAX), |(x, z), q| (x.max(q.x), z.min(q.z)));
    // A 20 mm stock 1 mm clear of the block's +X side, level with it.
    let stock_at = at(x_max + 1.0 + 10.0, B2.y, z_min);
    history.execute(&world, SetStock::new(&world, StockShape::Box { dx: 20.0, dy: 40.0, dz: 20.0 }, stock_at).expect("stock"));
    let base = plate::preset("300 x 300 mm").map(plate::PlatePreset::params).expect("preset");

    // Pitch X 25 -> 25.4: B2 moves 0.4 mm toward the stock, still clear.
    let clear = SetPlate::new(&world, &p, plate::PlateParams { pitch_x: 25.4, dowels: None, ..base.clone() }).expect("clear edit");
    assert_eq!(clear.warning(), None, "an edit that keeps clear is quiet");

    // Pitch X 25 -> 30: B2 moves 5 mm, 4 mm into the stock - warned by name.
    let into = SetPlate::new(&world, &p, plate::PlateParams { pitch_x: 30.0, dowels: None, ..base }).expect("a warning, not a refusal");
    let w = into.warning().unwrap_or_default().to_owned();
    assert!(w.contains("'Block'") && w.contains("stock"), "the re-seated rider is clash-checked: {w}");
    let before = model::find_by_name(&world, "Block").and_then(|e| model::transform_of(&world, &e)).expect("pose");
    history.execute(&world, into);
    let after = model::find_by_name(&world, "Block").and_then(|e| model::transform_of(&world, &e)).expect("pose");
    assert!(close(after.pos, Point3::new(B2.x + 5.0, B2.y, B2.z)), "{:?}", after.pos);
    assert!(history.undo(&world));
    assert_eq!(model::find_by_name(&world, "Block").and_then(|e| model::transform_of(&world, &e)), Some(before), "undo is exact");
    assert!(history.redo(&world));
    assert_eq!(model::find_by_name(&world, "Block").and_then(|e| model::transform_of(&world, &e)), Some(after), "redo is exact");
}

/// #826 review: `SetPlate` re-seats the riders itself, so they stay
/// mounted - the plate still carries them - and undo/redo are exact.
#[test]
fn an_edited_plate_keeps_its_riders_mounted() {
    let world = new_world();
    let mut history = History::default();
    let p = insert_plate(&world, &mut history, 0.0);
    block_on(&world, &mut history, &p);
    let base = plate::preset("300 x 300 mm").map(plate::PlatePreset::params).expect("preset");
    let mount = mount_of(&world, "Block").expect("mounted");
    let before = pose(&world, "Block");
    history.execute(&world, SetPlate::new(&world, &p, plate::PlateParams { pitch_x: 25.4, dowels: None, ..base }).expect("edit"));
    assert!(mount_of(&world, "Block").is_some_and(|m| m.plate == p), "still mounted after the edit");
    let mv = MoveFixture::new(&world, &p, at(1.0, 0.0, 0.0)).expect("move the plate");
    assert_eq!(mv.carried().count(), 1, "the plate still carries its block");
    let after = pose(&world, "Block");
    assert!(history.undo(&world));
    assert!(close(pose(&world, "Block"), before) && mount_of(&world, "Block") == Some(mount.clone()));
    assert!(history.redo(&world));
    assert!(close(pose(&world, "Block"), after) && mount_of(&world, "Block").is_some());
}
