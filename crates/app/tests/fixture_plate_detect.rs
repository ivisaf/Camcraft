//! Fixture plates (#819): an imported plate's grid derived from its own
//! holes - a regular lattice with a gap and a dowel lattice, a staggered
//! (checkerboard) lattice, and the refusals for holes off any pitch and
//! a plate with no holes; fixtures snap to the derived grid.
//!
//! Gate map (M4.4): covers app::plate::detect, app::plate::grid,
//! app::commands::fixture_plate (SetPlateGrid, PlaceOnPlate).

use camcraft_app::commands::{History, ImportModel, PlaceOnPlate, SetPlateGrid, TagFixture};
use camcraft_app::fixture;
use camcraft_app::geometry::{Mesh, Point3, Triangle, quat_from_euler};
use camcraft_app::model;
use camcraft_app::plate::{self, MountAt};
use camcraft_app::schema::Location;
use camcraft_app::world::new_world;
use flecs_ecs::prelude::World;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn pose(world: &World, name: &str) -> Location {
    model::find_by_name(world, name).and_then(|e| model::transform_of(world, &e)).expect(name)
}

fn same_turn(a: Location, rot: [f64; 4]) -> bool {
    let q = (0..4).map(|k| (a.rot[k] - rot[k]).abs()).sum::<f64>().min((0..4).map(|k| (a.rot[k] + rot[k]).abs()).sum());
    q < 1e-9
}

/// An imported plate's walls: a `w` x `h` outline and round holes
/// (inward walls, 32 sides), z in [-20, 0].
fn holed_plate(w: f64, h: f64, holes: &[(f64, f64, f64)]) -> Vec<Triangle> {
    let ring = |poly: &[(f64, f64)]| {
        let n = poly.len();
        (0..n).flat_map(move |i| {
            let ((x0, y0), (x1, y1)) = (poly[i], poly[(i + 1) % n]);
            let (a, b) = (Point3::new(x0, y0, -20.0), Point3::new(x1, y1, -20.0));
            let (c, d) = (Point3::new(x1, y1, 0.0), Point3::new(x0, y0, 0.0));
            [Triangle::new(a, b, c), Triangle::new(a, c, d)]
        }).collect::<Vec<_>>()
    };
    let mut t = ring(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]);
    for &(x, y, r) in holes {
        let mut c: Vec<(f64, f64)> = (0..32).map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / 32.0;
            (x + r * a.cos(), y + r * a.sin())
        }).collect();
        c.reverse();
        t.extend(ring(&c));
    }
    t
}

#[test]
fn an_imported_plates_grid_comes_from_its_holes() {
    let mut holes = Vec::new();
    for r in 0..5 {
        for c in 0..7 {
            if (r, c) != (2, 3) {
                holes.push((20.0 + 25.0 * f64::from(c), 20.0 + 25.0 * f64::from(r), 4.0));
            }
        }
    }
    for r in 0..2 {
        for c in 0..3 {
            holes.push((32.5 + 50.0 * f64::from(c), 32.5 + 50.0 * f64::from(r), 5.0));
        }
    }
    let world = new_world();
    let mut history = History::default();
    history.execute(&world, ImportModel::from_mesh("Shop plate", Mesh::from_facets(holed_plate(200.0, 150.0, &holes))));
    history.execute(&world, TagFixture::new(&world, "Shop plate", None).expect("tag"));
    history.execute(&world, SetPlateGrid::detect(&world, "Shop plate").expect("a regular grid"));
    let def = model::find_by_name(&world, "Shop plate").and_then(|e| fixture::def_of(&world, &e)).and_then(|d| d.plate.plate).expect("a plate");
    let g = &def.holes;
    assert_eq!((g.rows, g.cols), (5, 7));
    assert!(close(g.x0, 20.0) && close(g.y0, 20.0) && close(g.pitch_x, 25.0) && close(g.pitch_y, 25.0), "{g:?}");
    assert!((g.diameter - 8.0).abs() < 0.05, "{g:?}");
    assert_eq!(g.missing, ["C4"], "the gap is named");
    assert!(close(def.top_z, 0.0));
    let d = def.dowels.as_ref().expect("the dowel lattice");
    assert_eq!((d.rows, d.cols), (2, 3));
    assert!(close(d.pitch_x, 50.0) && close(d.x0, 32.5));

    // Fixtures snap to it.
    let cube = plate::slab(10.0, 10.0, 10.0);
    history.execute(&world, ImportModel::from_mesh("Block", Mesh::from_facets(cube)));
    history.execute(&world, TagFixture::new(&world, "Block", None).expect("tag"));
    history.execute(&world, PlaceOnPlate::new(&world, "Block", "Shop plate", MountAt::Hole("B2".into()), 0.0).expect("B2"));
    let at = pose(&world, "Block");
    assert!(close(at.pos.x, 45.0) && close(at.pos.y, 45.0) && close(at.pos.z, 10.0), "{at:?}");
    let e = PlaceOnPlate::new(&world, "Block", "Shop plate", MountAt::Hole("C4".into()), 0.0).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("C4 has no hole"), "{e}");
    history.execute(&world, PlaceOnPlate::new(&world, "Block", "Shop plate", MountAt::Dowels(["A1".into(), "B1".into()]), 0.0).expect("dowels"));
    let at = pose(&world, "Block");
    assert!(close(at.pos.x, 32.5) && close(at.pos.y, 32.5), "{at:?}");
    assert!(same_turn(at, quat_from_euler(0.0, 0.0, 90f64.to_radians())), "+X toward B1");
    let e = SetPlateGrid::clear(&world, "Shop plate").err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("still carries Block"), "{e}");

    // Holes off any regular pitch are refused by name.
    let off: Vec<_> = [(20.0, 20.0), (45.0, 20.0), (80.0, 20.0), (20.0, 45.0), (45.0, 45.0), (80.0, 45.0)].iter().map(|&(x, y)| (x, y, 4.0)).collect();
    history.execute(&world, ImportModel::from_mesh("Odd", Mesh::from_facets(holed_plate(120.0, 80.0, &off))));
    history.execute(&world, TagFixture::new(&world, "Odd", None).expect("tag"));
    let e = SetPlateGrid::detect(&world, "Odd").err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("don't form a regular grid") && e.contains("off the"), "{e}");
    history.execute(&world, ImportModel::from_mesh("Blank", Mesh::from_facets(plate::slab(50.0, 50.0, 5.0))));
    history.execute(&world, TagFixture::new(&world, "Blank", None).expect("tag"));
    let e = SetPlateGrid::detect(&world, "Blank").err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("no holes found"), "{e}");
}

/// A checkerboard plate (every other place, alternating by row - a
/// staggered grid) reads as a staggered grid: names from the full
/// lattice, the off places refused by name.
#[test]
fn a_staggered_plates_grid_is_read_as_staggered() {
    let mut holes = Vec::new();
    for r in 0..6u32 {
        for c in 0..8u32 {
            if (r + c) % 2 == 0 {
                holes.push((10.0 + 15.0 * f64::from(c), 10.0 + 15.0 * f64::from(r), 3.0));
            }
        }
    }
    let world = new_world();
    let mut history = History::default();
    history.execute(&world, ImportModel::from_mesh("Stagger", Mesh::from_facets(holed_plate(125.0, 95.0, &holes))));
    history.execute(&world, TagFixture::new(&world, "Stagger", None).expect("tag"));
    let cmd = SetPlateGrid::detect(&world, "Stagger").expect("a staggered grid");
    let g = cmd.plate().map(|d| d.holes.clone()).expect("grid");
    assert_eq!((g.rows, g.cols, g.stagger), (6, 8, Some(0)));
    assert!(g.missing.is_empty(), "{g:?}");
    assert_eq!(g.holes().len(), 24);
    assert!(g.locate("A1").is_ok() && g.locate("B2").is_ok());
    assert!(g.locate("A2").err().is_some_and(|e| e.contains("staggered")));
}

/// A square plate whose square grid is turned exactly 45 degrees in its
/// model reads, hole for hole, as a checkerboard - but its edges don't
/// run along X/Y, so it is refused as turned (the decision's law), while
/// the same checkerboard on a square-edged plate stays a staggered grid.
/// A grid turned a few degrees is refused naming the likely cause.
#[test]
fn a_turned_plate_is_refused_naming_the_turn() {
    let turn = |deg: f64, w: f64| {
        let (s, c) = deg.to_radians().sin_cos();
        let rot = move |x: f64, y: f64| (c * (x - w / 2.0) - s * (y - w / 2.0), s * (x - w / 2.0) + c * (y - w / 2.0));
        let mut holes = Vec::new();
        for r in 0..9 {
            for k in 0..9 {
                let (x, y) = rot(25.0 + 25.0 * f64::from(k), 25.0 + 25.0 * f64::from(r));
                holes.push((x, y, 4.0));
            }
        }
        let corners: Vec<(f64, f64)> = [(0.0, 0.0), (w, 0.0), (w, w), (0.0, w)].iter().map(|&(x, y)| rot(x, y)).collect();
        (corners, holes)
    };
    let world = new_world();
    let mut history = History::default();
    for (name, deg) in [("Diamond", 45.0), ("Askew", 3.0)] {
        let (outline, holes) = turn(deg, 250.0);
        let mut t = holed_plate(1.0, 1.0, &holes);
        // Swap the unit outline for the turned one.
        t.drain(0..8);
        let n = outline.len();
        for i in 0..n {
            let ((x0, y0), (x1, y1)) = (outline[i], outline[(i + 1) % n]);
            let (a, b) = (Point3::new(x0, y0, -20.0), Point3::new(x1, y1, -20.0));
            let (c, d) = (Point3::new(x1, y1, 0.0), Point3::new(x0, y0, 0.0));
            t.push(Triangle::new(a, b, c));
            t.push(Triangle::new(a, c, d));
        }
        history.execute(&world, ImportModel::from_mesh(name, Mesh::from_facets(t)));
        history.execute(&world, TagFixture::new(&world, name, None).expect("tag"));
        let e = SetPlateGrid::detect(&world, name).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(e.contains("don't line up with the model's X/Y axes") && e.contains("turned"), "{name}: {e}");
    }
}
