//! #672 step 2: a plasma program read from a path's plasma record - the
//! torch spans become cuts (arcs chorded), the torch-off travel between
//! them the rapids - and judged: a generated-shape program is clean, the
//! same cuts in the wrong order are refused, a path without the record
//! (or with one off its moves) is refused by name.
//!
//! Gate map (#672): covers sim::verify::plasma (program).
//! Provenance: our own fixtures.

use camcraft_sim::cl::{ContourRole, MoveKind, PathBuilder, PlasmaCut as Record, Toolpath, TorchSpan};
use camcraft_sim::geo2d::Point2;
use camcraft_sim::geometry::Point3;
use camcraft_sim::verify::plasma::{judge, CutRole, PlasmaFinding, PlasmaProgram};

const Z: f64 = 0.0;
const UP: f64 = 10.0;

fn at(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// One lit contour: a rapid to the pierce, the descent, the lit points
/// (straight), and the span that says so.
fn lit(b: &mut PathBuilder, contour: u32, role: ContourRole, parent: Option<u32>, pts: &[(f64, f64)]) -> TorchSpan {
    let (px, py) = pts[0];
    b.push(MoveKind::Rapid, at(b.at().x, b.at().y, UP), None);
    b.push(MoveKind::Rapid, at(px, py, UP), None);
    b.push(MoveKind::Rapid, at(px, py, Z), None);
    let on = b.moves_len();
    for &(x, y) in &pts[1..] {
        b.push(MoveKind::Feed, at(x, y, Z), Some(2500.0));
    }
    TorchSpan { contour, role, parent, on, off: b.moves_len() - 1, small: None }
}

/// The square hole (40..60) with a lead-in from the centre and no
/// lead-out (it ends on its own wall at (50, 40)), and the outline
/// (0..100) with a lead-in and lead-out on the outside, pierced below the
/// hole's end so the rapid between them runs straight outward from the
/// hole's wall - never back over its slug (#814 review).
const HOLE: &[(f64, f64)] = &[(50.0, 50.0), (50.0, 40.0), (60.0, 40.0), (60.0, 60.0), (40.0, 60.0), (40.0, 40.0), (50.0, 40.0)];
const OUTLINE: &[(f64, f64)] =
    &[(50.0, -5.0), (50.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0), (0.0, 0.0), (50.0, 0.0), (45.0, -5.0)];

fn plate(hole_first: bool) -> Toolpath {
    let mut b = PathBuilder::new(at(50.0, 50.0, UP));
    let mut torches = Vec::new();
    if hole_first {
        torches.push(lit(&mut b, 0, ContourRole::Inner, Some(1), HOLE));
        torches.push(lit(&mut b, 1, ContourRole::Outer, None, OUTLINE));
    } else {
        torches.push(lit(&mut b, 1, ContourRole::Outer, None, OUTLINE));
        torches.push(lit(&mut b, 0, ContourRole::Inner, Some(1), HOLE));
    }
    b.push(MoveKind::Rapid, at(b.at().x, b.at().y, UP), None);
    let mut t = b.finish();
    let rec = Record { kerf: 1.5, torches, ..Record::default() };
    assert!(rec.is_sane(&t.moves), "fixture: the spans sit on the moves");
    t.plasma = Some(rec);
    t
}

#[test]
fn the_spans_become_cuts_and_the_travel_between_them_the_rapids() {
    let t = plate(true);
    let p = PlasmaProgram::from_toolpath(&t).expect("read");
    assert_eq!(p.kerf, 1.5);
    assert_eq!(p.cuts.len(), 2);
    assert_eq!((p.cuts[0].contour, p.cuts[0].role, p.cuts[0].parent, p.cuts[0].small), (0, CutRole::Inner, Some(1), false));
    assert_eq!(p.cuts[0].path.len(), HOLE.len(), "lead-in and ring, point for point");
    assert_eq!(p.cuts[0].path[0], Point2::new(50.0, 50.0));
    // The ring between the two visits of the start (mid-edge, so five points).
    assert_eq!(p.cuts[0].ring().map(<[Point2]>::len), Some(5));
    assert_eq!(p.cuts[1].ring().map(<[Point2]>::len), Some(5));
    assert_eq!(p.rapids, vec![None, Some((Point2::new(50.0, 40.0), Point2::new(50.0, -5.0)))]);
    assert_eq!(judge(&p), vec![], "hole first, outline after: clean");
}

#[test]
fn the_outline_cut_before_its_hole_is_refused_from_the_record() {
    let p = PlasmaProgram::from_toolpath(&plate(false)).expect("read");
    let f = judge(&p);
    // The order refusal, and - since the pierce lands inside the freed
    // outline - the tip-up warning on the rapid into the hole.
    assert_eq!(f[0], PlasmaFinding::CutOrder { contour: 0, enclosing: 1 });
    assert!(f[0].refuses());
    assert!(matches!(f.get(1), Some(PlasmaFinding::TipUp { contour: 0, freed: 1, .. })), "{f:?}");
    assert_eq!(f.len(), 2);
}

#[test]
fn lit_arcs_are_chorded_into_the_cut() {
    let mut b = PathBuilder::new(at(50.0, 50.0, UP));
    b.push(MoveKind::Rapid, at(50.0, 50.0, Z), None);
    let on = b.moves_len();
    b.push(MoveKind::Feed, at(50.0, 40.0, Z), Some(2500.0));
    b.push_arc(MoveKind::Feed, at(50.0, 60.0, Z), Some(2500.0), Point2::new(50.0, 50.0), true);
    b.push_arc(MoveKind::Feed, at(50.0, 40.0, Z), Some(2500.0), Point2::new(50.0, 50.0), true);
    let hole = TorchSpan { contour: 0, role: ContourRole::Inner, parent: Some(1), on, off: b.moves_len() - 1, small: Some(0.65) };
    let outline = lit(&mut b, 1, ContourRole::Outer, None, OUTLINE);
    let mut t = b.finish();
    t.plasma = Some(Record { kerf: 1.5, torches: vec![hole, outline], ..Record::default() });
    let p = PlasmaProgram::from_toolpath(&t).expect("read");
    let ring = p.cuts[0].ring().expect("a chorded circle closes");
    assert!(ring.len() > 8, "{} chord points", ring.len());
    assert!(ring.iter().all(|q| ((q.x - 50.0).hypot(q.y - 50.0) - 10.0).abs() < 0.02), "on the Ø20 circle");
    assert!(p.cuts[0].small);
    assert_eq!(judge(&p), vec![]);
}

#[test]
fn a_rapid_over_a_part_already_freed_is_read_and_warned() {
    // Part A (0..50) ends its lead-out at the far left; the rapid to
    // part B at x 120 then crosses A's whole width.
    let a: &[(f64, f64)] = &[(-5.0, 25.0), (0.0, 25.0), (0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0), (0.0, 25.0), (-5.0, 30.0)];
    let b: &[(f64, f64)] = &[(115.0, 10.0), (120.0, 10.0), (120.0, 0.0), (150.0, 0.0), (150.0, 30.0), (120.0, 30.0), (120.0, 10.0)];
    let mut pb = PathBuilder::new(at(-5.0, 25.0, UP));
    let torches = vec![lit(&mut pb, 0, ContourRole::Outer, None, a), lit(&mut pb, 1, ContourRole::Outer, None, b)];
    let mut t = pb.finish();
    t.plasma = Some(Record { kerf: 1.5, torches, ..Record::default() });
    let p = PlasmaProgram::from_toolpath(&t).expect("read");
    let f = judge(&p);
    assert!(matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 1, freed: 0, .. }]), "{f:?}");
    assert!(!f[0].refuses());
}

#[test]
fn a_path_without_the_record_or_with_one_off_its_moves_is_refused() {
    let mut t = plate(true);
    let rec = t.plasma.take().expect("fixture");
    let e = PlasmaProgram::from_toolpath(&t).expect_err("no record");
    assert!(e.contains("no plasma record"), "{e}");
    let mut off = rec;
    off.torches[0].off = t.moves.len() + 5;
    t.plasma = Some(off);
    let e = PlasmaProgram::from_toolpath(&t).expect_err("off the moves");
    assert!(e.contains("does not sit on the path's moves"), "{e}");
}

/// #801: a hole's slug read from the record is a freed area for tip-up.
/// Two holes (20..40 and 60..80, lead-in from the centre, no lead-out)
/// cut right, then left, then the outline. With no lead-out each hole
/// ends ON its bottom wall, and a rapid from there is judged in full
/// (#814 review): the rapid to the left hole runs back over the right
/// slug, and the outline's crosses the left slug it leaves and then the
/// right one - three tip-ups, each naming its hole.
#[test]
fn a_rapid_over_a_freed_hole_slug_is_read_and_warned() {
    let right: &[(f64, f64)] = &[(70.0, 50.0), (70.0, 40.0), (80.0, 40.0), (80.0, 60.0), (60.0, 60.0), (60.0, 40.0), (70.0, 40.0)];
    let left: &[(f64, f64)] = &[(30.0, 50.0), (30.0, 40.0), (40.0, 40.0), (40.0, 60.0), (20.0, 60.0), (20.0, 40.0), (30.0, 40.0)];
    let outline: &[(f64, f64)] = &[(105.0, 50.0), (100.0, 50.0), (100.0, 100.0), (0.0, 100.0), (0.0, 0.0), (100.0, 0.0), (100.0, 50.0)];
    let mut b = PathBuilder::new(at(70.0, 50.0, UP));
    let torches = vec![
        lit(&mut b, 1, ContourRole::Inner, Some(3), right),
        lit(&mut b, 2, ContourRole::Inner, Some(3), left),
        lit(&mut b, 3, ContourRole::Outer, None, outline),
    ];
    let mut t = b.finish();
    t.plasma = Some(Record { kerf: 1.5, torches, ..Record::default() });
    let p = PlasmaProgram::from_toolpath(&t).expect("read");
    // The rapid into the outline: (30, 40) on the left hole's edge to
    // (105, 50) - across the material, then over the right hole's slug.
    assert_eq!(p.rapids[2], Some((Point2::new(30.0, 40.0), Point2::new(105.0, 50.0))));
    let f = judge(&p);
    assert!(
        matches!(f.as_slice(), [
            PlasmaFinding::TipUp { contour: 2, freed: 1, slug: true, at: a },
            PlasmaFinding::TipUp { contour: 3, freed: 1, slug: true, at: b },
            PlasmaFinding::TipUp { contour: 3, freed: 2, slug: true, at: c },
        ] if a.x > 60.0 && a.x < 80.0 && b.x > 60.0 && b.x < 80.0 && c.x > 20.0 && c.x < 40.0),
        "{f:?}"
    );
    assert!(f.iter().all(|x| !x.refuses()), "warnings, as the part's tip-up is");
    assert!(f[1].to_string().contains("slug of hole C1"), "{}", f[1]);
    assert!(f[2].to_string().contains("slug of hole C2"), "{}", f[2]);
}
