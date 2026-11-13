//! Mod vises (#820): every preset's jaw blocks are watertight solids of
//! their exact figures; a pair placed on a plate's grid lands its fixed
//! block on the named hole and its adjustable block on the nearest hole
//! clear of the stock, slid until its face touches the stock (to 1e-6),
//! at every quarter turn; one undo takes both; and every refusal is
//! named - out of the slots' reach, off the plate, no grid, no stock,
//! a stock in the fixed jaw, an off-quarter turn, nonsense figures.
//!
//! Gate map (M4.4): covers app::modvise (JawParams, mesh, table, fit),
//! app::commands::mod_vise (PlaceModVisePair).

use std::collections::HashMap;

use camcraft_app::commands::{History, ImportModel, InsertPlate, PlaceModVisePair, PlatePlacement, SetStock, TagFixture};
use camcraft_app::fixture;
use camcraft_app::geometry::{IDENTITY_Q, Mesh, Point3, Triangle, transform_point};
use camcraft_app::model;
use camcraft_app::modvise::{self, JawParams, JawSide};
use camcraft_app::plate::{self, MountAt};
use camcraft_app::schema::Location;
use camcraft_app::stock::StockShape;
use camcraft_app::world::new_world;
use flecs_ecs::prelude::World;

const IN: f64 = 25.4;

fn key(p: Point3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}

fn assert_watertight(tris: &[Triangle], what: &str) {
    let mut edges: HashMap<([u64; 3], [u64; 3]), i32> = HashMap::new();
    for t in tris {
        for (a, b) in [(t.a, t.b), (t.b, t.c), (t.c, t.a)] {
            *edges.entry((key(a), key(b))).or_default() += 1;
        }
    }
    for ((a, b), n) in &edges {
        assert_eq!(*n, 1, "{what}: a directed edge used {n} times");
        assert_eq!(edges.get(&(*b, *a)), Some(&1), "{what}: an open edge");
    }
}

fn volume(tris: &[Triangle]) -> f64 {
    tris.iter()
        .map(|t| {
            let (a, b, c) = (t.a, t.b, t.c);
            a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x) + a.z * (b.x * c.y - b.y * c.x)
        })
        .sum::<f64>()
        / 6.0
}

fn bounds(pts: impl Iterator<Item = Point3>) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in pts {
        for (k, v) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[k] = lo[k].min(v);
            hi[k] = hi[k].max(v);
        }
    }
    (lo, hi)
}

#[test]
fn every_preset_block_is_a_watertight_solid_of_its_exact_figures() {
    let labels: Vec<&str> = modvise::PRESETS.iter().map(|p| p.label).collect();
    assert_eq!(labels, ["1 in grid 3/8-16", "2 in grid 1/2-13", "50 mm grid M12"]);
    for row in modvise::PRESETS {
        for (b, slide) in [(row.fixed(), 0.0), (row.adjustable(), 0.0), (row.adjustable(), row.pitch)] {
            let b = b.with_slide(slide);
            let what = format!("{} {} slid {slide}", row.label, b.side.word());
            b.check().unwrap_or_else(|e| panic!("{what}: {e}"));
            let tris = b.mesh().expect("meshes");
            assert_watertight(&tris, &what);
            let face = b.face_offset + slide;
            let rear = face - b.jaw_thickness - b.body_length;
            let w = b.body_width.max(b.jaw_width);
            let xc = b.centre_x();
            let (lo, hi) = bounds(tris.iter().flat_map(|t| [t.a, t.b, t.c]));
            assert_eq!(lo, [xc - 0.5 * w, rear, 0.0], "{what}: exact low corner");
            assert_eq!(hi, [xc + 0.5 * w, face, b.body_height.max(b.jaw_height)], "{what}: exact high corner");
            // The body plus the insert slab (jaw shorter than the body).
            let want = w * (b.body_length * b.body_height + b.jaw_thickness * b.jaw_height);
            assert!((volume(&tris) - want).abs() < 1e-6 * want, "{what}: outward volume {} vs {want}", volume(&tris));
        }
    }
    // A jaw as tall as the body: one plain box, still closed.
    let mut b = modvise::preset("50 mm grid M12").map(modvise::ModVisePreset::fixed).expect("preset");
    b.jaw_height = b.body_height;
    let tris = b.mesh().expect("meshes");
    assert_watertight(&tris, "flush jaw");
    assert_eq!(tris.len(), 12, "a flush jaw is a plain box");
}

#[test]
fn nonsense_block_figures_are_refused_by_name() {
    let fixed = modvise::preset("1 in grid 3/8-16").map(modvise::ModVisePreset::fixed).expect("preset");
    let adj = modvise::preset("1 in grid 3/8-16").map(modvise::ModVisePreset::adjustable).expect("preset");
    let bad = |p: JawParams, want: &str| {
        let e = p.check().err().unwrap_or_default();
        assert!(e.contains(want), "want '{want}' in: {e}");
    };
    bad(JawParams { body_length: 0.0, ..fixed.clone() }, "body length must be more than zero");
    bad(JawParams { jaw_height: f64::NAN, ..fixed.clone() }, "jaw height must be a finite length");
    bad(JawParams { slot_travel: 5.0, ..fixed.clone() }, "has no slot travel");
    bad(adj.with_slide(adj.slot_travel + 1.0), "past its slot travel");
    bad(JawParams { bolts: vec![[IN, 0.0]], ..fixed.clone() }, "anchor bolt at [0, 0]");
    bad(JawParams { face_offset: 0.0, ..fixed.clone() }, "bolt at [0.000, 0.000] mm is not inside the body");
    bad(JawParams { body_length: 40.0, ..adj.clone() }, "inside the body at every slide");
    bad(JawParams { bolts: vec![[0.0, 0.0]; 17], ..fixed }, "1 to 16 bolts");
}

/// A 60 x 60 x 20 stock (centred in XY, z 0..20) and a 770MX table plate
/// laid so the 1 in preset's fixed block at `hole` turned `yaw` has its
/// jaw face on the stock's near side, the bodies centred on it.
fn rig(yaw: f64, hole: &str) -> (World, History, String) {
    let world = new_world();
    let mut history = History::default();
    history.execute(&world, SetStock::new(&world, StockShape::Box { dx: 60.0, dy: 60.0, dz: 20.0 }, Location::default()).expect("stock"));
    let fixed = modvise::preset("1 in grid 3/8-16").map(modvise::ModVisePreset::fixed).expect("preset");
    let (s, c) = yaw.to_radians().sin_cos();
    let (d, e) = ((-s.round(), c.round()), (c.round(), s.round()));
    let near = (-30.0 * d.0, -30.0 * d.1);
    let anchor = (near.0 - fixed.face_offset * d.0 - fixed.centre_x() * e.0, near.1 - fixed.face_offset * d.1 - fixed.centre_x() * e.1);
    let p = plate::preset("770MX table 28 x 15 in").map(plate::PlatePreset::params).expect("preset");
    let (hx, hy) = p.grid().locate(hole).expect("hole");
    let cmd = InsertPlate::new(&world, &p, None, PlatePlacement::At(Location { pos: Point3::new(anchor.0 - hx, anchor.1 - hy, 0.0), rot: IDENTITY_Q }))
        .expect("plate");
    let name = cmd.name().to_owned();
    history.execute(&world, cmd);
    (world, history, name)
}

fn pair(label: &str) -> (JawParams, JawParams) {
    let row = modvise::preset(label).expect("preset");
    (row.fixed(), row.adjustable())
}

/// Fixture `name`'s mesh in the world.
fn world_mesh(world: &World, name: &str) -> Vec<Point3> {
    let e = model::find_by_name(world, name).expect(name);
    let loc = model::transform_of(world, &e).unwrap_or_default();
    model::facets_of(world, &e).map(|f| f.iter().flat_map(|t| [t.a, t.b, t.c]).map(|p| transform_point(p, loc.pos, loc.rot)).collect()).unwrap_or_default()
}

#[test]
fn a_pair_lands_on_the_grid_and_the_adjustable_jaw_touches_the_stock_at_every_quarter_turn() {
    for (yaw, hole, want_adj) in [(0.0, "C14", "I16"), (90.0, "H20", "J14"), (180.0, "M14", "G12"), (270.0, "H8", "F14")] {
        let (world, mut history, plate) = rig(yaw, hole);
        let (f, a) = pair("1 in grid 3/8-16");
        let cmd = PlaceModVisePair::new(&world, &plate, &f, &a, hole, yaw).unwrap_or_else(|e| panic!("{yaw}: {e}"));
        let fit = cmd.fit().clone();
        assert_eq!((fit.fixed_at.as_str(), fit.adjustable_at.as_str()), (hole, want_adj), "{yaw}: the holes");
        // Stock far side 60 + 44.45 from the fixed anchor; the nearest
        // clear hole is 6 pitches on (its face 1 in back), so it slides
        // 6 in - 1 in - 104.45 mm.
        assert!((fit.slide - (6.0 * IN - IN - 104.45)).abs() < 1e-9, "{yaw}: slide {}", fit.slide);
        assert!(fit.fixed_gap.abs() < 1e-9 && (fit.opening - 60.0).abs() < 1e-9, "{yaw}: {fit:?}");
        let (fixed_name, adj_name) = (cmd.names().0.to_owned(), cmd.names().1.to_owned());
        history.execute(&world, cmd);

        let (s, c) = yaw.to_radians().sin_cos();
        let d = (-s.round(), c.round());
        let along = |p: &Point3| p.x * d.0 + p.y * d.1;
        let fixed_front = world_mesh(&world, &fixed_name).iter().map(along).fold(f64::MIN, f64::max);
        let adj_front = world_mesh(&world, &adj_name).iter().map(along).fold(f64::MAX, f64::min);
        assert!((fixed_front + 30.0).abs() < 1e-6, "{yaw}: the fixed jaw on the stock's near side: {fixed_front}");
        assert!((adj_front - 30.0).abs() < 1e-6, "{yaw}: the adjustable jaw touches the far side: {adj_front}");
        let z = world_mesh(&world, &adj_name).iter().map(|p| p.z).fold(f64::MAX, f64::min);
        assert!(z.abs() < 1e-9, "{yaw}: standing on the plate top");

        let def = |n: &str| model::find_by_name(&world, n).and_then(|e| fixture::def_of(&world, &e)).expect(n);
        assert_eq!(def(&fixed_name).plate.mount.map(|m| (m.plate, m.at)), Some((plate.clone(), MountAt::Hole(hole.into()))));
        let adj = def(&adj_name);
        assert_eq!(adj.plate.mount.map(|m| m.at), Some(MountAt::Hole(want_adj.into())));
        assert_eq!(adj.mod_jaw.as_ref().map(|j| (j.side, (j.slide - fit.slide).abs() < 1e-12)), Some((JawSide::Adjustable, true)));

        assert!(history.undo(&world), "one undo");
        assert!(model::find_by_name(&world, &fixed_name).is_none() && model::find_by_name(&world, &adj_name).is_none(), "{yaw}: takes both");
        assert!(history.redo(&world));
        assert!(model::find_by_name(&world, &adj_name).is_some(), "redo brings them back");
    }
}

fn refused(world: &World, plate: &str, f: &JawParams, a: &JawParams, hole: &str, yaw: f64) -> String {
    PlaceModVisePair::new(world, plate, f, a, hole, yaw).err().map(|e| e.to_string()).unwrap_or_default()
}

#[test]
fn every_refusal_is_named() {
    let (world, mut history, plate) = rig(0.0, "C14");
    let (f, a) = pair("1 in grid 3/8-16");
    // Out of reach: the slots travel 10 mm, the stock is 22.55 mm away.
    let short = JawParams { slot_travel: 10.0, ..a.clone() };
    let e = refused(&world, &plate, &f, &short, "C14", 0.0);
    assert!(e.contains("out of reach") && e.contains("22.550") && e.contains("I16"), "{e}");
    // The stock in the fixed jaw, and behind it.
    let e = refused(&world, &plate, &f, &a, "D14", 0.0);
    assert!(e.contains("into the fixed jaw block at D14"), "{e}");
    // An off-quarter turn, an unknown hole, a hole beside the stock.
    assert!(refused(&world, &plate, &f, &a, "C14", 45.0).contains("0, 90, 180 or 270"));
    assert!(refused(&world, &plate, &f, &a, "Z99", 0.0).contains("no location Z99"));
    assert!(refused(&world, &plate, &f, &a, "C20", 0.0).contains("not between the jaws"));
    // Two blocks of one side.
    assert!(refused(&world, &plate, &f, &f, "C14", 0.0).contains("a fixed and an adjustable"));
    // A plate with no grid.
    history.execute(&world, ImportModel::from_mesh("Slab", Mesh::from_facets(plate::slab(300.0, 300.0, 10.0))));
    history.execute(&world, TagFixture::new(&world, "Slab", None).expect("tag"));
    assert!(refused(&world, "Slab", &f, &a, "A1", 0.0).contains("is not a fixture plate"));
    assert!(refused(&world, "Nope", &f, &a, "A1", 0.0).contains("Nope"));
    assert!(model::named_entities(&world).iter().all(|(n, _)| !n.starts_with("Mod vise")), "nothing placed");

    // Off the plate: the plate laid so A1 is the fixed anchor - its body
    // reaches 1.5 in past the anchor, the hole only 0.5 in from the edge.
    let (world, _h, plate) = rig(0.0, "A1");
    let e = refused(&world, &plate, &f, &a, "A1", 0.0);
    assert!(e.contains("off the plate's -X edge") && e.contains("25.400"), "{e}");
    // No stock to fit to.
    let world = new_world();
    let mut history = History::default();
    let p = plate::preset("12 x 12 in").map(plate::PlatePreset::params).expect("preset");
    let cmd = InsertPlate::new(&world, &p, None, PlatePlacement::At(Location::default())).expect("plate");
    let name = cmd.name().to_owned();
    history.execute(&world, cmd);
    assert!(refused(&world, &name, &f, &a, "C3", 0.0).contains("no stock"));
}

#[test]
fn bolts_and_pins_must_land_on_holes() {
    let (world, _h, plate) = rig(0.0, "C14");
    let (f, a) = pair("1 in grid 3/8-16");
    let between = JawParams { bolts: vec![[0.0, 0.0], [1.5 * IN, 0.0]], ..f.clone() };
    let e = refused(&world, &plate, &between, &a, "C14", 0.0);
    assert!(e.contains("fixed jaw block bolt at [38.100, 0.000]") && e.contains("between holes"), "{e}");
    let pinned = JawParams { pins: vec![[IN, 0.0]], ..f.clone() };
    let e = refused(&world, &plate, &pinned, &a, "C14", 0.0);
    assert!(e.contains("locating pins") && e.contains("no dowel bores"), "{e}");
}
