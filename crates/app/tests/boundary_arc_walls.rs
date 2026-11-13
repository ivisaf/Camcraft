//! #905: outside `Mode::Area` law (b) read an arc move only at its chord
//! joints, so a sharp wall corner between two joints was never measured
//! against the cutter. In Boundary mode an XY arc now answers on its true
//! circle to the walls (`sim::verify::arc_wall`), so the verdict no longer
//! depends on where the joints land. Area mode reads its chords whole and
//! is unchanged.
//!
//! Gate map: covers app::verify (sim::verify::section, sim::verify::arc_wall, law (b)).
//!
//! Provenance: point-to-circle distance along the radius, from elementary
//! plane geometry; the tangency law is our own (sim::verify::section).

use camcraft_app::cl::{Move, MoveKind, Toolpath};
use camcraft_app::geo2d::{Loop, Point2, Region};
use camcraft_app::geometry::Point3;
use camcraft_app::verify::{Mode, ToolGeom, VerifyContext, Warning, verify};

/// The gouge verdict tolerance (`GOUGE_TOL`).
const TOL: f64 = 0.01;
/// The cutter radius.
const R: f64 = 4.0;
/// The arc of the issue's evidence: r 200 about the origin, 0.048 rad.
const RAD: f64 = 200.0;
const SWEEP: f64 = 0.048;
/// Midway between the joints near it: the fine set (5 chords) stands at
/// 0.0192 and 0.0288 rad, the whole-tolerance set (3 chords) at 0.016
/// and 0.032 - each about 1 mm along the arc from here.
const MID: f64 = 0.024;

fn p2(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

fn floor() -> f64 {
    R - TOL
}

fn tool() -> ToolGeom {
    ToolGeom {
        radius: R,
        nose_radius: 0.0,
        flute_len: 30.0,
        shank_radius: R,
        shank_len: 40.0,
        holder_radius: 20.0,
        holder_len: 60.0,
    }
}

fn ctx<'a>(region: &'a Region, tool: &'a ToolGeom, mode: Mode) -> VerifyContext<'a> {
    VerifyContext {
        region,
        stock: None,
        model: None,
        tool,
        cutter: None,
        mode,
        stock_top_z: 0.0,
        clearance_z: 5.0,
        stock_to_leave: 0.0,
        gouge_tol: TOL,
        z_tol: 1e-6,
        finish_tol: None,
        fixtures: &[],
        spindle: None,
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop {
    Loop::new(vec![p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1)]).expect("rect")
}

/// The arc as one feed at z -1, ccw from 0 to `SWEEP`, or the same arc
/// cut clockwise from `SWEEP` back to 0.
fn arc(ccw: bool) -> Toolpath {
    let at = |a: f64| Point3::new(RAD * a.cos(), RAD * a.sin(), -1.0);
    let (a0, a1) = if ccw { (0.0, SWEEP) } else { (SWEEP, 0.0) };
    Toolpath {
        moves: vec![Move::new(MoveKind::Feed, at(a0), at(a1), Some(500.0)).with_arc(p2(0.0, 0.0), ccw)],
        ..Toolpath::default()
    }
}

/// A sharp island vertex `d` off the arc at `MID`: outside the circle
/// (`outward`) or inside it, its sides opening away from the arc
/// (half-angle ~14 deg), in a pocket that stands far from the arc.
fn region_at(d: f64, outward: bool) -> (Region, Point2) {
    let s = if outward { 1.0 } else { -1.0 };
    let (ux, uy) = (s * MID.cos(), s * MID.sin()); // away from the arc
    let (tx, ty) = (-MID.sin(), MID.cos()); // along it
    let v = p2(RAD * MID.cos() + ux * d, RAD * MID.sin() + uy * d);
    let side = |k: f64| p2(v.x + ux * 20.0 + tx * 5.0 * k, v.y + uy * 20.0 + ty * 5.0 * k);
    let island = Loop::new(vec![v, side(1.0), side(-1.0)]).expect("island");
    (Region::new(vec![rect(150.0, -50.0, 260.0, 60.0), island]).expect("region"), v)
}

/// The nearest any joint of the arc comes to `v`, at the whole tolerance
/// and at the quarter: what the joint reading alone measured.
fn joint_clearance(m: &Move, v: Point2) -> f64 {
    [TOL, TOL * 0.25]
        .into_iter()
        .flat_map(|t| m.chords(t))
        .flat_map(|(a, b)| [a, b])
        .map(|p| (p.x - v.x).hypot(p.y - v.y))
        .fold(f64::INFINITY, f64::min)
}

fn boundary(t: &Toolpath, region: &Region) -> Vec<Warning> {
    let g = tool();
    verify(t, &ctx(region, &g, Mode::Boundary))
}

#[test]
fn a_corner_between_an_arcs_joints_past_the_floor_is_a_gouge() {
    for outward in [true, false] {
        for ccw in [true, false] {
            let t = arc(ccw);
            // 90 um past the floor: the joints stand ~1 mm along the arc
            // and read the vertex clear, so the joint reading passed it.
            let (region, v) = region_at(3.90, outward);
            let seen = joint_clearance(&t.moves[0], v);
            assert!(seen >= floor(), "the joints read it clear: {seen} (outward {outward})");
            assert_eq!(boundary(&t, &region), vec![Warning::Gouge { at: 0 }], "outward {outward}, ccw {ccw}");
            // Deeper and shallower past the floor: convicted too.
            for d in [3.5, 3.8, 3.98] {
                let (region, _) = region_at(d, outward);
                assert_eq!(boundary(&t, &region), vec![Warning::Gouge { at: 0 }], "d {d}, outward {outward}, ccw {ccw}");
            }
        }
    }
}

#[test]
fn the_same_corner_just_inside_the_floor_stays_clean() {
    for outward in [true, false] {
        for ccw in [true, false] {
            let t = arc(ccw);
            for d in [3.995, 4.0, 4.5] {
                assert!(d >= floor());
                let (region, _) = region_at(d, outward);
                assert_eq!(boundary(&t, &region), Vec::new(), "d {d}, outward {outward}, ccw {ccw}");
            }
        }
    }
}

#[test]
fn area_mode_verdicts_on_the_corner_are_unchanged() {
    // Area mode reads the arc's chords along their whole length: it
    // convicted this corner before #905 and still does, and passes it
    // clear of the floor.
    let g = tool();
    let t = arc(true);
    let (past, _) = region_at(3.90, true);
    assert_eq!(verify(&t, &ctx(&past, &g, Mode::Area)), vec![Warning::Gouge { at: 0 }]);
    let (clear, _) = region_at(4.0, true);
    assert_eq!(verify(&t, &ctx(&clear, &g, Mode::Area)), Vec::new());
}
