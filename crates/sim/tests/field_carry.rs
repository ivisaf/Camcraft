//! #637 step A: a field carried through a setup change's lattice turn is
//! the same field seen through the turn - the carried distance at T(p) is
//! the original distance at p, exactly, for every one of the eight turns
//! and their compositions, cut layers included.
//!
//! Gate map (#637): covers sim::stock_sim::field_carry (CarriedField),
//! carry::LatticeTurn (unapply, then), field_turn (SdfField::turned,
//! #779), field_carry_cut (CarriedField::simulate against the bake and
//! the shifted moves by hand, #781).
//! Provenance: rigid-motion invariance of the Euclidean distance; our own
//! fixtures.

use camcraft_sim::cl::{ArcPlane, Move, MoveKind};
use camcraft_sim::geo2d::Point2;
use camcraft_sim::geometry::{cube, Point3};
use camcraft_sim::stock_sim::{Backend, CarriedField, Lattice, LatticeTurn, SimOp, Stock, FIELD_BAND};

/// The eight lattice turns (Z on +-Z, no mirror), each with a shift.
fn turns() -> Vec<LatticeTurn> {
    let mut out = Vec::new();
    for (r, t) in [
        ([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [3.0, -2.0, 1.0]),
        ([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], [10.0, 0.0, 0.0]),
        ([[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]], [20.0, 12.0, 0.0]),
        ([[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], [0.0, 20.0, 0.0]),
        ([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]], [0.0, 12.0, 5.0]),
        ([[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]], [20.0, 0.0, 5.0]),
        ([[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]], [0.0, 0.0, 5.0]),
        ([[0.0, -1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]], [12.0, 20.0, 5.0]),
    ] {
        out.push(LatticeTurn::from_matrix(r, t).unwrap_or_else(|e| panic!("turn {r:?}: {e}")));
    }
    out
}

/// A 20 x 12 x 5 block with a slot cut across it - a field with a cut layer.
fn cut_field() -> camcraft_sim::stock_sim::SdfField {
    let mut s = Stock::block(Backend::Field { band: FIELD_BAND }, 0.0, 0.0, 20.0, 12.0, 5.0, 0.5).expect("block");
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let moves = [
        Move::new(MoveKind::Rapid, p(-4.0, 6.0, 8.0), p(-4.0, 6.0, 3.0), None),
        Move::new(MoveKind::Feed, p(-4.0, 6.0, 3.0), p(24.0, 6.0, 3.0), Some(500.0)),
    ];
    s.simulate(&[SimOp { moves: &moves, radius: 2.0, ball: false, cone: None, corner: None }]).expect("cut");
    s.field().expect("a field").clone()
}

fn samples() -> Vec<Point3> {
    let mut v = Vec::new();
    for i in -2..=22 {
        for j in -2..=14 {
            for k in -2..=7 {
                v.push(Point3::new(f64::from(i) * 1.0 + 0.3, f64::from(j) * 1.0 + 0.2, f64::from(k) * 1.0 + 0.1));
            }
        }
    }
    v
}

#[test]
fn unapply_inverts_apply_and_then_composes() {
    let pts = samples();
    for a in turns() {
        for p in &pts {
            let q = a.unapply(a.apply(*p));
            assert!((q.x - p.x).abs() < 1e-12 && (q.y - p.y).abs() < 1e-12 && (q.z - p.z).abs() < 1e-12, "{a:?}: {p:?} -> {q:?}");
        }
        for b in turns() {
            let ab = a.then(b);
            for p in pts.iter().step_by(37) {
                let (x, y) = (ab.apply(*p), b.apply(a.apply(*p)));
                assert!((x.x - y.x).abs() < 1e-12 && (x.y - y.y).abs() < 1e-12 && (x.z - y.z).abs() < 1e-12, "{a:?} then {b:?}");
            }
        }
    }
}

#[test]
fn a_carried_field_reads_the_same_solid_through_every_turn() {
    let f = cut_field();
    let pts = samples();
    for t in turns() {
        let c = CarriedField::new(f.clone(), t);
        for p in &pts {
            let q = t.apply(*p);
            assert_eq!(c.distance(q).to_bits(), f.distance(*p).to_bits(), "{t:?} at {p:?}");
            assert_eq!(c.mesh_distance(q).to_bits(), f.mesh_distance(*p).to_bits(), "{t:?} mesh at {p:?}");
            assert_eq!(c.inside(q), f.inside(*p), "{t:?} parity at {p:?}");
        }
    }
}

#[test]
fn carrying_twice_is_carrying_through_the_composed_turn() {
    let f = cut_field();
    let ts = turns();
    let (a, b) = (ts[1], ts[5]);
    let twice = CarriedField::new(f.clone(), a).then(b);
    let once = CarriedField::new(f, a.then(b));
    for p in samples().iter().step_by(11) {
        assert_eq!(twice.distance(*p).to_bits(), once.distance(*p).to_bits(), "{p:?}");
    }
}

#[test]
fn a_clip_intersects_the_carried_solid_with_the_next_block() {
    // #637 step C: the next setup's block clips the carried body -
    // max(d_body, d_block); inside only where both hold material.
    use camcraft_sim::stock_sim::box_signed;
    let f = cut_field();
    let t = turns()[0];
    let (lo, hi) = (Point3::new(5.0, 0.0, 0.0), Point3::new(15.0, 14.0, 9.0));
    let c = CarriedField::new(f.clone(), t).clipped(lo, hi).expect("a finite block");
    for p in samples() {
        let body = f.distance(t.unapply(p));
        let block = box_signed(lo, hi, p);
        assert_eq!(c.distance(p).to_bits(), body.max(block).to_bits(), "{p:?}");
        assert_eq!(c.inside(p), f.inside(t.unapply(p)) && block < 0.0, "{p:?}");
    }
    // A second clip narrows, never widens.
    let narrow = c.clone().clipped(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 14.0, 9.0)).expect("a finite block");
    let p = Point3::new(12.3, 4.2, 2.1);
    assert!(narrow.distance(p) >= c.distance(p) && narrow.distance(p) > 0.0, "x 12.3 is outside the narrowed block");
}

#[test]
fn box_signed_is_negative_inside_and_euclidean_outside() {
    use camcraft_sim::stock_sim::box_signed;
    let (lo, hi) = (Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 2.0, 2.0));
    assert_eq!(box_signed(lo, hi, Point3::new(1.0, 1.0, 1.0)), -1.0, "1 from the nearest face (y and z)");
    assert_eq!(box_signed(lo, hi, Point3::new(7.0, 1.0, 1.0)), 3.0);
    assert!((box_signed(lo, hi, Point3::new(7.0, 6.0, 1.0)) - 5.0).abs() < 1e-12, "a 3-4-5 corner");
    assert_eq!(box_signed(lo, hi, Point3::new(4.0, 1.0, 1.0)), 0.0, "on a face");
}

#[test]
fn a_nan_point_stays_nan_through_the_clip() {
    // Review of #651: f64::max dropped the NaN, so a NaN point read deep
    // inside (-inf) once a clip was set.
    use camcraft_sim::stock_sim::box_signed;
    let (lo, hi) = (Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 2.0, 2.0));
    assert!(box_signed(lo, hi, Point3::new(f64::NAN, 1.0, 1.0)).is_nan());
    assert!(box_signed(lo, hi, Point3::new(f64::NAN, f64::NAN, f64::NAN)).is_nan());
    let c = CarriedField::new(cut_field(), turns()[0]).clipped(lo, hi).expect("a finite block");
    for p in [Point3::new(f64::NAN, 1.0, 1.0), Point3::new(f64::NAN, f64::NAN, f64::NAN)] {
        assert!(c.distance(p).is_nan(), "{p:?} -> {}", c.distance(p));
        assert!(!c.inside(p));
    }
}

#[test]
fn two_clips_that_miss_stay_empty_through_a_turn() {
    // Review of #651: an empty intersection re-sorted by `then` became a
    // real box, so material reappeared after a turn.
    use camcraft_sim::stock_sim::Clip;
    let f = cut_field();
    let c = CarriedField::new(f, turns()[0])
        .clipped(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 14.0, 9.0))
        .and_then(|c| c.clipped(Point3::new(10.0, 0.0, 0.0), Point3::new(20.0, 14.0, 9.0)))
        .expect("finite blocks");
    assert_eq!(c.clip, Some(Clip::Empty));
    for t in turns() {
        let d = c.clone().then(t);
        assert_eq!(d.clip, Some(Clip::Empty), "{t:?}");
        for p in samples() {
            assert!(!d.inside(p) && d.distance(p) > 0.0, "{t:?} {p:?}");
        }
    }
}

#[test]
fn a_non_finite_or_inverted_clip_is_refused_by_name() {
    use camcraft_sim::stock_sim::ClipRefusal;
    let c = || CarriedField::new(cut_field(), turns()[0]);
    let ok = Point3::new(20.0, 14.0, 9.0);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(c().clipped(Point3::new(bad, 0.0, 0.0), ok).err(), Some(ClipRefusal::NonFinite));
        assert_eq!(c().clipped(Point3::new(0.0, 0.0, 0.0), Point3::new(20.0, bad, 9.0)).err(), Some(ClipRefusal::NonFinite));
    }
    assert_eq!(c().clipped(ok, Point3::new(0.0, 0.0, 0.0)).err(), Some(ClipRefusal::Inverted));
}

/// `p - shift`, the residual view's own arithmetic (`unapply`'s `x - t`).
fn minus(p: Point3, s: Point3) -> Point3 {
    Point3::new(p.x - s.x, p.y - s.y, p.z - s.z)
}

/// #637 E2a (#779): the turn baked into the bricks reads bit for bit what
/// the view reads, through all eight turns, for the block (closed-form box,
/// cut layer) and a mesh-seeded field (band bricks, cut layer). The samples
/// sit off the lattice planes and the shell's crossings, the two places a
/// negated coordinate may fall into the other of two equal claimants.
#[test]
fn a_baked_turn_reads_bit_for_bit_what_the_view_reads() {
    let mut mesh = camcraft_sim::stock_sim::SdfField::from_mesh(cut_field().facets(), 0.5, FIELD_BAND).expect("mesh field");
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let slot = Move::new(MoveKind::Feed, p(-4.0, 6.3, 3.2), p(24.0, 6.3, 3.2), Some(500.0));
    mesh.subtract(&slot, &camcraft_sim::stock_sim::SdfTool::Shanked { r: 1.7, shank: 20.0 }, 0.01).expect("cut");
    let pts = samples();
    for f in [cut_field(), mesh] {
        assert!(f.cut_brick_count() > 0, "the fixture carries a cut layer");
        for t in turns() {
            let (g, shift) = f.turned(&t);
            assert_eq!(shift, t.apply(Point3::new(0.0, 0.0, 0.0)), "{t:?}: the residual is the translation");
            let view = CarriedField::new(f.clone(), t);
            for s in &pts {
                let q = t.apply(*s);
                let b = minus(q, shift);
                assert_eq!(g.distance(b).to_bits(), view.distance(q).to_bits(), "{t:?} at {q:?}");
                assert_eq!(g.mesh_distance(b).to_bits(), view.mesh_distance(q).to_bits(), "{t:?} mesh at {q:?}");
                assert_eq!(g.inside(b), view.inside(q), "{t:?} parity at {q:?}");
            }
        }
    }
}

/// The translation alone, as a turn.
fn only(s: Point3) -> LatticeTurn {
    LatticeTurn::from_matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [s.x, s.y, s.z]).unwrap_or_else(|e| panic!("a translation: {e}"))
}

/// An XY arc (half a circle about an off-lattice centre) and an XZ arc
/// (a G18 plunge across the block), both in the new frame.
fn arcs(z: f64) -> Vec<Move> {
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    vec![
        Move::new(MoveKind::Rapid, p(6.4, 6.05, 8.0), p(6.4, 6.05, z), None),
        Move::new(MoveKind::Feed, p(6.4, 6.05, z), p(13.8, 6.05, z), Some(500.0)).with_arc(Point2::new(10.1, 6.05), false),
        Move::new(MoveKind::Feed, p(13.8, 6.05, z), p(17.8, 6.05, z), Some(500.0))
            .with_arc_in(Point2::new(15.8, z + 0.9), true, ArcPlane::Xz),
    ]
}

/// A flat r 1.3 op on `m`.
fn op(m: &[Move]) -> SimOp<'_> {
    SimOp { moves: m, radius: 1.3, ball: false, cone: None, corner: None }
}

#[test]
fn simulate_cuts_what_the_bake_and_the_shifted_moves_cut_by_hand() {
    // #637 E2c (#781): the real `simulate` is #780's by-hand pin, bit for
    // bit, through every turn: bake, shift `from`, `to` and the arc centre
    // (in its plane's word order) by the residual translation, cut with
    // `Stock::simulate`, view through the translation alone. A mesh plate
    // 5 thick (band bricks, not the deep closed-form block), so a cut from
    // the flipped top is cheap.
    let mut f = camcraft_sim::stock_sim::SdfField::from_mesh(&cube(Point3::new(0.0, 0.0, 0.0), Point3::new(20.0, 12.0, 5.0)).facets, 0.5, FIELD_BAND)
        .expect("plate");
    let slot = Move::new(MoveKind::Feed, Point3::new(-4.0, 6.0, 3.0), Point3::new(24.0, 6.0, 3.0), Some(500.0));
    f.subtract(&slot, &camcraft_sim::stock_sim::SdfTool::Flat { r: 2.0, flank: 40.0 }, 0.01).expect("slot");
    let moves = arcs(2.2);
    let pts = samples();
    for t in turns() {
        let mut real = CarriedField::new(f.clone(), t);
        real.simulate(&[op(&moves)]).unwrap_or_else(|e| panic!("{t:?}: cut: {e}"));
        let (g, shift) = f.turned(&t);
        let q = |v: Point3| Point3::new(v.x - shift.x, v.y - shift.y, v.z - shift.z);
        let by_hand: Vec<Move> = moves
            .iter()
            .map(|m| {
                let mut n = *m;
                (n.from, n.to) = (q(m.from), q(m.to));
                if let Some(a) = n.arc.as_mut() {
                    a.center = match a.plane {
                        ArcPlane::Xy => Point2::new(a.center.x - shift.x, a.center.y - shift.y),
                        ArcPlane::Xz => Point2::new(a.center.x - shift.x, a.center.y - shift.z),
                        ArcPlane::Yz => Point2::new(a.center.x - shift.y, a.center.y - shift.z),
                    };
                }
                n
            })
            .collect();
        let mut s = Stock::Field(g);
        s.simulate(&[op(&by_hand)]).unwrap_or_else(|e| panic!("{t:?}: by hand: {e}"));
        let hand = CarriedField::new(s.field().expect("a field").clone(), only(shift));
        assert_eq!(real.turn, only(shift), "{t:?}: the view keeps the translation alone");
        assert_eq!(real.flipped(), t.flips(), "{t:?}: a baked flip is remembered");
        for v in pts.iter().step_by(5) {
            let v = t.apply(*v);
            assert_eq!(real.distance(v).to_bits(), hand.distance(v).to_bits(), "{t:?} at {v:?}");
        }
        // The block's image and a margin, read every other field cell
        // (a 1.0 lattice whose centres are the field's own) for the cost.
        let (a, b) = (t.apply(Point3::new(0.0, 0.0, 0.0)), t.apply(Point3::new(20.0, 12.0, 0.0)));
        let (nx, ny) = if t.swaps_xy() { (14, 22) } else { (22, 14) };
        let lat = Lattice { x0: a.x.min(b.x) - 0.75, y0: a.y.min(b.y) - 0.75, cell: 1.0, nx, ny };
        let a = real.column_body(lat, false).unwrap_or_else(|e| panic!("{t:?}: real read-back: {e}"));
        let b = hand.column_body(lat, t.flips()).unwrap_or_else(|e| panic!("{t:?}: by-hand read-back: {e}"));
        assert_eq!(a.floor_judged(), b.floor_judged(), "{t:?}");
        for k in 0..nx * ny {
            assert_eq!(a.dex.top_at(k).to_bits(), b.dex.top_at(k).to_bits(), "{t:?} column {k}: top");
            assert_eq!(a.floor_at(k).to_bits(), b.floor_at(k).to_bits(), "{t:?} column {k}: floor");
        }
    }
}

#[test]
fn a_refused_program_leaves_the_carried_stock_as_it_was() {
    // `check_ops` runs on the caller's moves before the bake: nothing moves.
    let t = turns()[4];
    let mut c = CarriedField::new(cut_field(), t);
    let p = Point3::new(1.0, 1.0, 1.0);
    let bad = [Move::new(MoveKind::Feed, p, Point3::new(f64::NAN, 1.0, 1.0), Some(500.0))];
    assert!(c.simulate(&[SimOp { moves: &bad, radius: 1.0, ball: false, cone: None, corner: None }]).is_err());
    let good = arcs(4.0);
    assert!(c.simulate(&[SimOp { moves: &good, radius: -1.0, ball: false, cone: None, corner: None }]).is_err());
    assert_eq!(c.turn, t, "no bake behind a refusal");
    let view = CarriedField::new(cut_field(), t);
    for v in samples().iter().step_by(7) {
        assert_eq!(c.distance(t.apply(*v)).to_bits(), view.distance(t.apply(*v)).to_bits());
    }
}

#[test]
fn a_flip_is_remembered_through_two_flips_and_a_bake() {
    // Two flips compose into an upright turn; the floor has still faced the
    // spindle, as `Body::carry`'s sticky flag says.
    let flip = turns()[4];
    let lat = Lattice { x0: 0.0, y0: 0.0, cell: 0.5, nx: 40, ny: 24 };
    let twice = CarriedField::new(cut_field(), flip).then(flip);
    assert!(!twice.turn.flips() && twice.flipped());
    assert!(twice.column_body(lat, false).expect("read-back").floor_judged());
    assert!(!CarriedField::new(cut_field(), turns()[2]).column_body(lat, false).expect("read-back").floor_judged());
    let mut cut = CarriedField::new(cut_field(), flip);
    cut.simulate(&[op(&arcs(4.0))]).expect("cut");
    assert!(!cut.turn.flips() && cut.flipped(), "the baked flip leaves a translation and the flag");
    assert!(cut.column_body(lat, false).expect("read-back").floor_judged());
}
