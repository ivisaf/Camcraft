//! Built-in 5-axis self-centering vises (#787): the four rows build closed,
//! outward shells of the stated dimensions (with and without the dovetail
//! lip, closed, half and fully open); nonsense parameters and stock outside
//! the clamping range are refused by name; a fitted vise sits centred on
//! the stock with both jaws against it and re-closes when the stock
//! changes; placing and editing are one undo each and survive a save.
//!
//! Gate map (M4.4): covers app::vise, app::vise::self_centering,
//! app::commands::builtin_vise, app::project (fixture vise key).

use std::collections::HashMap;

use camcraft_app::commands::{History, InsertVise, SetStock, SetVise, VisePlacement};
use camcraft_app::fixture;
use camcraft_app::geometry::{IDENTITY_Q, Point3, Triangle};
use camcraft_app::model;
use camcraft_app::project;
use camcraft_app::schema::Location;
use camcraft_app::stock::StockShape;
use camcraft_app::vise::{self, SelfCentering, ViseParams, ViseStyle};
use camcraft_app::world::new_world;
use camcraft_sim::fixture_mesh::FixtureMesh;
use flecs_ecs::prelude::World;

const LABELS: [&str; 4] = ["52 mm self-centering", "75 mm self-centering", "80 mm self-centering", "125 mm self-centering"];

fn row(label: &str) -> ViseParams {
    vise::size(label).map(vise::ViseSize::half_open).expect(label)
}

fn sc_of(p: &ViseParams) -> SelfCentering {
    match p.style {
        ViseStyle::SelfCentering(sc) => sc,
        ViseStyle::Standard => panic!("{} is not self-centering", p.size),
    }
}

fn bounds(tris: &[Triangle]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in tris.iter().flat_map(|t| [t.a, t.b, t.c]) {
        for (k, v) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[k] = lo[k].min(v);
            hi[k] = hi[k].max(v);
        }
    }
    (lo, hi)
}

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
        assert_eq!(edges.get(&(*b, *a)), Some(&1), "{what}: an edge with no reverse (open shell)");
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

/// Body, two jaw bodies, two lips - all prisms across the overall width.
fn expected_volume(p: &ViseParams) -> f64 {
    let sc = sc_of(p);
    (p.length * p.base_height + 2.0 * p.jaw_body_depth * p.jaw_height + 2.0 * sc.lip_depth * sc.lip_height) * p.width
}

fn with_lip(p: &ViseParams, h: f64, d: f64) -> ViseParams {
    let mut q = p.clone();
    if let ViseStyle::SelfCentering(sc) = &mut q.style {
        sc.lip_height = h;
        sc.lip_depth = d;
    }
    q
}

fn assert_shell(p: &ViseParams, what: &str) {
    p.check().unwrap_or_else(|e| panic!("{what}: {e}"));
    let tris = p.mesh().expect("meshes");
    assert_watertight(&tris, what);
    let (v, want) = (volume(&tris), expected_volume(p));
    assert!((v - want).abs() < 1e-6 * want, "{what}: outward volume {v} vs {want}");
    let (lo, hi) = bounds(&tris);
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(close(lo[0], -0.5 * p.width) && close(hi[0], 0.5 * p.width), "{what}: width");
    assert!(close(lo[1], -0.5 * p.length) && close(hi[1], 0.5 * p.length), "{what}: length, centred");
    let top = p.jaw_height + sc_of(p).lip_height;
    assert!(close(lo[2], -p.base_height) && close(hi[2], top), "{what}: height");
    let m = FixtureMesh::build(&tris).unwrap_or_else(|e| panic!("{what}: the mesh law refuses it: {e:?}"));
    // Both jaws are steel at their mid-depth, the gap between them air.
    let z = 0.5 * p.jaw_height;
    let y = 0.5 * p.opening + 0.5 * p.jaw_body_depth;
    assert!(m.contains(Point3::new(0.0, y, z)) && m.contains(Point3::new(0.0, -y, z)), "{what}: both jaws solid");
    if p.opening > 1.0 {
        assert!(!m.contains(Point3::new(0.0, 0.0, z)), "{what}: the gap is open");
    }
}

#[test]
fn every_self_centering_row_builds_a_closed_shell_of_its_dimensions() {
    let labels: Vec<&str> = vise::presets().filter(|r| matches!(r.style, ViseStyle::SelfCentering(_))).map(|r| r.label).collect();
    assert_eq!(labels, LABELS, "the issue's four sizes, after the standard table");
    assert_eq!(vise::SIZES.len() + LABELS.len(), vise::presets().count());
    for label in LABELS {
        let p = row(label);
        let sc = sc_of(&p);
        for opening in [sc.min_opening, 0.5 * (sc.min_opening + p.max_opening), p.max_opening] {
            let q = p.with_opening(opening);
            assert_shell(&q, &format!("{label} @ {opening}"));
            assert_shell(&with_lip(&q, 3.0, 0.5 * q.jaw_body_depth), &format!("{label} @ {opening} lipped"));
            assert_shell(&with_lip(&q, 2.0, q.jaw_body_depth), &format!("{label} @ {opening} full-depth lip"));
        }
    }
    // Jaws at the body's ends at the max opening: flush, still one shell.
    let p = row("80 mm self-centering");
    let flush = ViseParams { length: p.max_opening + 2.0 * p.jaw_body_depth, ..p.with_opening(p.max_opening) };
    assert_shell(&flush, "80 mm, jaws at the body's ends");
}

#[test]
fn the_table_keeps_the_issues_starting_figures() {
    for (label, min, max) in [
        ("52 mm self-centering", 3.5, 38.5),
        ("75 mm self-centering", 0.0, 40.0),
        ("80 mm self-centering", 0.0, 127.0),
        ("125 mm self-centering", 0.0, 208.0),
    ] {
        let p = row(label);
        assert_eq!((sc_of(&p).min_opening, p.max_opening), (min, max), "{label}: clamping range");
        let jaw: f64 = label.split(' ').next().and_then(|w| w.parse().ok()).expect("a width");
        assert_eq!(p.jaw_width, jaw, "{label}: the jaw width is the label");
        assert_eq!(p.width, p.jaw_width, "{label}: narrow - the body no wider than the jaws");
        assert_eq!((p.housing_length, p.housing_height), (0.0, 0.0), "{label}: no screw housing");
        assert!(p.jaw_body_depth >= p.jaw_thickness, "{label}: the jaw body includes its plate");
    }
    assert!(vise::size("80MM self-centering").is_some(), "the lookup is case- and space-blind");
    assert_eq!(vise::size("125 mm").map(|r| r.style), Some(ViseStyle::Standard), "the standard 125 mm keeps its label");
}

#[test]
fn nonsense_parameters_are_refused_by_name() {
    let p = row("52 mm self-centering");
    let refused = |q: ViseParams, word: &str| {
        let e = q.mesh().expect_err(word);
        assert!(e.contains(word), "'{e}' names {word}");
    };
    refused(p.with_opening(3.0), "outside its clamping range 3.500-38.500");
    refused(p.with_opening(39.0), "outside its clamping range");
    refused(p.with_opening(f64::NAN), "jaw opening must be a finite length");
    refused(ViseParams { jaw_height: f64::INFINITY, ..p.clone() }, "jaw height");
    refused(ViseParams { base_height: 0.0, ..p.clone() }, "base height");
    refused(ViseParams { housing_length: 10.0, ..p.clone() }, "no screw housing");
    refused(ViseParams { width: 40.0, ..p.clone() }, "narrower than its jaw width");
    refused(ViseParams { jaw_body_depth: 5.0, ..p.clone() }, "less than its jaw thickness");
    refused(ViseParams { length: 60.0, ..p.clone() }, "overall length");
    refused(with_lip(&p, 3.0, 0.0), "both a height and a depth");
    refused(with_lip(&p, 3.0, 20.0), "lip depth");
    refused(with_lip(&p, f64::NAN, 2.0), "lip height must be a finite length");
    refused(with_lip(&p, -1.0, 2.0), "lip height must not be negative");
    let mut q = p.clone();
    if let ViseStyle::SelfCentering(sc) = &mut q.style {
        sc.min_opening = 40.0;
    }
    refused(q, "min opening");

    // Stock outside the clamping range, at placement.
    for (dy, word) in [(50.0, "wider than its clamping range"), (2.0, "narrower than its clamping range")] {
        let world = new_world();
        History::default().execute(&world, SetStock::new(&world, StockShape::Box { dx: 30.0, dy, dz: 10.0 }, Location::default()).expect("stock"));
        let e = InsertVise::new(&world, &p, None, VisePlacement::FitStock).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(e.contains(word), "{dy} mm stock: '{e}' names {word}");
    }
}

/// Stock `dx` x `dy` x 20 centred at (`cx`, `cy`), floor at z 0.
fn stock(world: &World, history: &mut History, dx: f64, dy: f64, cx: f64, cy: f64) {
    let loc = Location { pos: Point3::new(cx, cy, 0.0), rot: IDENTITY_Q };
    history.execute(world, SetStock::new(world, StockShape::Box { dx, dy, dz: 20.0 }, loc).expect("stock"));
}

fn params_of(world: &World, name: &str) -> Option<ViseParams> {
    model::find_by_name(world, name).and_then(|e| fixture::def_of(world, &e)).and_then(|d| d.vise)
}

#[test]
fn a_fitted_vise_is_centred_on_the_stock_and_closes_on_it() {
    let world = new_world();
    let mut history = History::default();
    stock(&world, &mut history, 60.0, 30.0, 12.0, -7.0);
    let cmd = InsertVise::new(&world, &row("80 mm self-centering"), None, VisePlacement::FitStock).expect("fits");
    let name = cmd.name().to_owned();
    assert_eq!(name, "Vise 80 mm self-centering");
    history.execute(&world, cmd);
    let e = model::find_by_name(&world, &name).expect("placed");
    let loc = model::transform_of(&world, &e).expect("pose");
    assert_eq!([loc.pos.x, loc.pos.y, loc.pos.z], [12.0, -7.0, 0.0], "centred on the stock, floor under it");
    let p = params_of(&world, &name).expect("a built-in vise");
    assert!((p.opening - 30.0).abs() < 1e-9, "closed to the stock's 30 mm: {}", p.opening);

    // Both jaw faces touch the stock's sides: steel just outside each,
    // air just inside.
    let tris = model::world_facets(&world, &e).expect("facets");
    let m = FixtureMesh::build(&tris).expect("mesh");
    let s = model::stock_world_aabb(&world).expect("stock box");
    for (y, out) in [(s.max.y, 1.0), (s.min.y, -1.0)] {
        assert!(m.contains(Point3::new(12.0, y + 0.5 * out, 5.0)), "a jaw against the stock at y {y}");
        assert!(!m.contains(Point3::new(12.0, y - 0.5 * out, 5.0)), "no steel inside the stock at y {y}");
    }
    assert!(m.contains(Point3::new(12.0, -7.0, -1.0)) && !m.contains(Point3::new(12.0, -7.0, 1.0)), "the floor under the stock");

    // The stock grows, still centred: the jaws close on it again.
    stock(&world, &mut history, 60.0, 90.0, 12.0, -7.0);
    history.execute(&world, SetVise::opening_from_stock(&world, &name).expect("re-close"));
    assert_eq!(params_of(&world, &name).map(|p| p.opening), Some(90.0));
    // Off the vise's centre, or past the clamping range: refused by name.
    stock(&world, &mut history, 60.0, 30.0, 12.0, 3.0);
    let e = SetVise::opening_from_stock(&world, &name).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("off the self-centering vise's centre"), "{e}");
    stock(&world, &mut history, 60.0, 130.0, 12.0, -7.0);
    let e = SetVise::opening_from_stock(&world, &name).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("wider than its clamping range"), "{e}");
}

#[test]
fn placing_and_editing_are_one_undo_each_and_survive_a_save() {
    let world = new_world();
    let mut history = History::default();
    stock(&world, &mut history, 40.0, 30.0, 0.0, 0.0);
    let cmd = InsertVise::new(&world, &row("75 mm self-centering"), None, VisePlacement::FitStock).expect("fits");
    let name = cmd.name().to_owned();
    history.execute(&world, cmd);
    let e = model::find_by_name(&world, &name).expect("placed");
    let before = model::facets_of(&world, &e).expect("facets").to_vec();
    let lipped = with_lip(&params_of(&world, &name).expect("vise").with_opening(36.0), 4.0, 6.0);
    history.execute(&world, SetVise::new(&world, &name, lipped.clone()).expect("edit"));
    assert_eq!(params_of(&world, &name), Some(lipped.clone()));
    let after = model::facets_of(&world, &e).expect("facets").to_vec();
    assert_ne!(after, before, "the edit regenerates the mesh");

    let path = std::env::temp_dir().join(format!("camcraft-sc-vise-{}.camproj", std::process::id()));
    project::save(&world, &path).expect("save");
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("\"self_centering\"") && text.contains("\"min_opening\""), "the marker carries the style");
    let back = new_world();
    project::load(&back, &path).expect("load");
    let _ = std::fs::remove_file(&path);
    assert_eq!(params_of(&back, &name), Some(lipped), "the parameters reload, style and lip included");
    let refused = SetVise::opening(&back, &name, 41.0).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(refused.contains("clamping range"), "and stay judged by its own law: {refused}");

    assert!(history.undo(&world));
    assert_eq!(model::facets_of(&world, &e).expect("facets").to_vec(), before, "one undo takes the edit");
    assert_eq!(params_of(&world, &name).map(|p| p.opening), Some(30.0));
    assert!(history.undo(&world));
    assert!(model::find_by_name(&world, &name).is_none(), "one undo removes the vise");
    assert!(history.redo(&world));
    assert!(history.redo(&world));
    assert_eq!(model::find_by_name(&world, &name).and_then(|e| model::facets_of(&world, &e)).map(|f| f.to_vec()), Some(after), "redo both");
}
