//! #637 step D: a carried stock reads the same on both engines. The same
//! block, cut by the same slots, carried through every lattice turn: the
//! column carry (`Body::carry`, re-indexed) and the field carry
//! (`CarriedField::column_body`, read back) agree column for column on
//! the tops, the floors and which columns are empty.
//!
//! Gate map (#637): covers sim::stock_sim::field_body (CarriedField::
//! column_body), carry::Body (from_parts) against carry::Body::carry,
//! field_turn (SdfField::turned against the view, #779), march_up and
//! field_floor's two-sided read (#780), field_carry_cut (CarriedField::
//! simulate: cuts in the new frame after a carry, #781).
//! Provenance: our own fixtures.

use camcraft_sim::cl::{Move, MoveKind};
use camcraft_sim::geo2d::Point2;
use camcraft_sim::geometry::Point3;
use camcraft_sim::stock_sim::{simulate, Backend, Body, CarriedField, Lattice, LatticeTurn, SimOp, Stock, FIELD_BAND};

const TOP: f64 = 5.0;
const CELL: f64 = 0.5;
/// The two engines read a top at a cell centre each: equal to rounding
/// on the mesh planes and the tool's flat floor.
const EPS: f64 = 1e-6;

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

/// A flat-end slot across the 20 x 12 block at depth `z`.
fn cut(b: Backend, z: f64) -> Stock {
    cut_with(b, 6.0, z, 2.0, false)
}

/// A slot across the block along y = `y`, depth `z`, radius `r`.
fn cut_with(b: Backend, y: f64, z: f64, r: f64, ball: bool) -> Stock {
    cut_block(b, TOP, y, z, r, ball)
}

/// A slot across a block `top` tall along y = `y`, depth `z`, radius `r`.
fn cut_block(b: Backend, top: f64, y: f64, z: f64, r: f64, ball: bool) -> Stock {
    let mut s = Stock::block(b, 0.0, 0.0, 20.0, 12.0, top, CELL).expect("block");
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let moves = [
        Move::new(MoveKind::Rapid, p(-4.0, y, 8.0), p(-4.0, y, z), None),
        Move::new(MoveKind::Feed, p(-4.0, y, z), p(24.0, y, z), Some(500.0)),
    ];
    s.simulate(&[SimOp { moves: &moves, radius: r, ball, cone: None, corner: None }]).expect("cut");
    s
}

fn agree(slot_z: f64) {
    agree_with(cut(Backend::Column, slot_z), cut(Backend::Field { band: FIELD_BAND }, slot_z), EPS);
}

fn agree_with(col: Stock, field: Stock, eps: f64) {
    let col = match col {
        Stock::Column(d) => d,
        Stock::Field(_) => panic!("a column stock"),
    };
    let field = field.field().expect("a field").clone();
    // The block stock's floor as the field built it (an uncut corner).
    let floor = match field.floor_at(0.25, 0.25) {
        camcraft_sim::stock_sim::FloorRead::Floor(z) => z,
        r => panic!("the block's floor: {r:?}"),
    };
    let body = Body::on_floor(col, floor);
    for t in turns() {
        let a = body.carry(&t).unwrap_or_else(|e| panic!("{t:?}: column carry: {e}"));
        let d = &a.dex;
        let lat = Lattice { x0: d.x0, y0: d.y0, cell: d.cell, nx: d.nx, ny: d.ny };
        let b = CarriedField::new(field.clone(), t).column_body(lat, false).unwrap_or_else(|e| panic!("{t:?}: field carry: {e}"));
        assert_eq!(a.floor_judged(), b.floor_judged(), "{t:?}");
        for k in 0..d.nx * d.ny {
            assert_eq!(a.empty_at(k), b.empty_at(k), "{t:?} column {k}: empty");
            if a.empty_at(k) {
                continue;
            }
            let (ta, tb) = (a.dex.top_at(k), b.dex.top_at(k));
            let (fa, fb) = (a.floor_at(k), b.floor_at(k));
            assert!((ta - tb).abs() <= eps, "{t:?} column {k}: top {ta} vs {tb}");
            assert!((fa - fb).abs() <= eps, "{t:?} column {k}: floor {fa} vs {fb}");
        }
        let cells = (d.nx * d.ny) as f64 * d.cell * d.cell;
        assert!((a.volume() - b.volume()).abs() <= eps * cells, "{t:?}: volume {} vs {}", a.volume(), b.volume());
    }
}

#[test]
fn a_slot_from_the_top_carries_the_same_on_both_engines() {
    agree(3.0);
}

#[test]
fn a_deep_slot_carries_the_same_on_both_engines() {
    // A slot 4 deep: the cut columns stand on the block's floor on both.
    agree(1.0);
}

#[test]
fn a_slot_through_the_block_carries_the_same_on_both_engines() {
    // #637 E1: a slot below the block's floor - cut through on both
    // engines, its columns empty through every turn, the columns beside
    // it (within a cell of the wall) whole.
    agree(-1.0);
}

#[test]
fn a_sliver_on_the_floor_carries_the_same_on_both_engines() {
    // #637 E1: 0.2 of stock left under the slot, thinner than the 0.5
    // cell and holding no cell centre: the field reads it from the cut
    // layer's ray entry, the column engine from its lowered top.
    agree(0.2);
}

#[test]
fn an_off_grid_ball_slot_carries_the_same_on_both_engines() {
    // Review of #656: walls off the cell boundaries and a curved floor -
    // both engines read the ball's surface at the same cell centres; the
    // field's march brackets its top within MARCH_EPS.
    const MARCH_EPS: f64 = 1e-3;
    let (y, z, r) = (6.3, 3.2, 1.7);
    agree_with(
        cut_with(Backend::Column, y, z, r, true),
        cut_with(Backend::Field { band: FIELD_BAND }, y, z, r, true),
        MARCH_EPS,
    );
}

/// #637 E2a (#779): a field with the turn baked into its bricks, seen
/// through the translation alone, is the view. Through all eight turns the
/// distance at every column centre of the carried lattice, at every cell
/// centre height, is bit-identical, and so is the whole column read-back -
/// a flipped bake's floor under a cut read by the up-march from the same
/// record the view's down-march reads (#780, E2b).
fn baked_agrees(field: Stock) {
    let f = field.field().expect("a field").clone();
    let col = match cut(Backend::Column, 3.0) {
        Stock::Column(d) => d,
        Stock::Field(_) => panic!("a column stock"),
    };
    let body = Body::on_floor(col, 0.0);
    for t in turns() {
        let d = body.carry(&t).unwrap_or_else(|e| panic!("{t:?}: column carry: {e}")).dex;
        let lat = Lattice { x0: d.x0, y0: d.y0, cell: d.cell, nx: d.nx, ny: d.ny };
        let (g, shift) = f.turned(&t);
        let view = CarriedField::new(f.clone(), t);
        let only = LatticeTurn::from_matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [shift.x, shift.y, shift.z])
            .unwrap_or_else(|e| panic!("a translation: {e}"));
        let baked = CarriedField::new(g, only);
        for jy in 0..d.ny {
            for jx in 0..d.nx {
                let (x, y) = (d.x0 + (jx as f64 + 0.5) * d.cell, d.y0 + (jy as f64 + 0.5) * d.cell);
                for k in -4..16 {
                    let q = Point3::new(x, y, (f64::from(k) + 0.5) * CELL);
                    assert_eq!(baked.distance(q).to_bits(), view.distance(q).to_bits(), "{t:?} at {q:?}");
                }
            }
        }
        let a = view.column_body(lat, false).unwrap_or_else(|e| panic!("{t:?}: view: {e}"));
        // The bake keeps the flip's floor flag with its caller (E2c): the
        // read-back through the translation alone is told it.
        let b = baked.column_body(lat, t.flips()).unwrap_or_else(|e| panic!("{t:?}: baked: {e}"));
        assert_eq!(a.floor_judged(), b.floor_judged(), "{t:?}");
        for k in 0..d.nx * d.ny {
            assert_eq!(a.empty_at(k), b.empty_at(k), "{t:?} column {k}: empty");
            assert_eq!(a.dex.top_at(k).to_bits(), b.dex.top_at(k).to_bits(), "{t:?} column {k}: top");
            assert_eq!(a.floor_at(k).to_bits(), b.floor_at(k).to_bits(), "{t:?} column {k}: floor");
        }
    }
}

#[test]
fn a_baked_turn_reads_what_the_view_reads_through_every_turn() {
    let field = Backend::Field { band: FIELD_BAND };
    baked_agrees(cut(field, 3.0));
    baked_agrees(cut(field, -1.0));
    baked_agrees(cut(field, 0.2));
    baked_agrees(cut_with(field, 6.3, 3.2, 1.7, true));
}

#[test]
fn an_unaligned_block_top_carries_the_same_on_both_engines() {
    // #780: a top of 5.1 is off the 0.5 lattice; flipped, so is the floor.
    // Both ends are read from the shell's own crossing, not a cell
    // boundary, so they agree with the column engine's exact heights.
    let field = Backend::Field { band: FIELD_BAND };
    for z in [3.0, 0.2, -1.0] {
        agree_with(cut_block(Backend::Column, 5.1, 6.0, z, 2.0, false), cut_block(field, 5.1, 6.0, z, 2.0, false), EPS);
    }
}

/// One step of a carry chain: the turn onto the next setup, then a program
/// stated in that setup's frame (a flat tool of radius `r`).
struct Step {
    turn: LatticeTurn,
    moves: Vec<Move>,
    r: f64,
}

fn flip() -> LatticeTurn {
    LatticeTurn::from_matrix([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]], [0.0, 12.0, 5.0]).unwrap_or_else(|e| panic!("a flip: {e}"))
}

/// A flat slot across the new frame along x = 10 at depth `z`.
fn slot_x(z: f64) -> Vec<Move> {
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    vec![
        Move::new(MoveKind::Rapid, p(10.0, -4.0, 8.0), p(10.0, -4.0, z), None),
        Move::new(MoveKind::Feed, p(10.0, -4.0, z), p(10.0, 16.0, z), Some(500.0)),
    ]
}

/// #637 E2c (#781): the block, a slot from the top, then each step - the
/// carry, then a program in the new frame. The column engine carries its
/// body and lowers its tops; the field is carried (`CarriedField::new`,
/// `then`) and cut through `CarriedField::simulate` (the bake, the residual
/// translation on the moves, `Stock::simulate`'s field arm). Read back on
/// the column body's lattice, every column agrees on `empty_at`, top and
/// floor to 1e-6, and the two agree on `floor_judged`. Returns the field's
/// carried stock and the counts (two-sided columns, empty columns).
fn chain_agrees(first_z: f64, steps: &[Step]) -> (CarriedField, usize, usize) {
    let f = cut(Backend::Field { band: FIELD_BAND }, first_z).field().expect("a field").clone();
    let floor = match f.floor_at(0.25, 0.25) {
        camcraft_sim::stock_sim::FloorRead::Floor(z) => z,
        r => panic!("the block's floor: {r:?}"),
    };
    let col = match cut(Backend::Column, first_z) {
        Stock::Column(d) => d,
        Stock::Field(_) => panic!("a column stock"),
    };
    let mut a = Body::on_floor(col, floor);
    let mut c: Option<CarriedField> = None;
    for (n, st) in steps.iter().enumerate() {
        a = a.carry(&st.turn).unwrap_or_else(|e| panic!("step {n}: column carry: {e}"));
        let mut cf = match c.take() {
            None => CarriedField::new(f.clone(), st.turn),
            Some(cf) => cf.then(st.turn),
        };
        let op = [SimOp { moves: &st.moves, radius: st.r, ball: false, cone: None, corner: None }];
        simulate(&mut a.dex, &op).unwrap_or_else(|e| panic!("step {n}: column cut: {e}"));
        cf.simulate(&op).unwrap_or_else(|e| panic!("step {n}: field cut: {e}"));
        c = Some(cf);
    }
    let c = c.unwrap_or_else(|| panic!("a chain of at least one step"));
    let d = &a.dex;
    let lat = Lattice { x0: d.x0, y0: d.y0, cell: d.cell, nx: d.nx, ny: d.ny };
    let b = c.column_body(lat, false).unwrap_or_else(|e| panic!("({first_z}) field read-back: {e}"));
    assert_eq!(a.floor_judged(), b.floor_judged(), "({first_z}) floor_judged");
    let (mut two_sided, mut empties) = (0, 0);
    let (block_top, block_floor) = (5.0 - floor, 0.0);
    for k in 0..d.nx * d.ny {
        let spans = ([a.floor_at(k), a.dex.top_at(k)], [b.floor_at(k), b.dex.top_at(k)]);
        assert_eq!(a.empty_at(k), b.empty_at(k), "({first_z}) column {k}: empty - column {:?}, field {:?}", spans.0, spans.1);
        if a.empty_at(k) {
            empties += 1;
            continue;
        }
        let (ta, tb) = (a.dex.top_at(k), b.dex.top_at(k));
        let (fa, fb) = (a.floor_at(k), b.floor_at(k));
        assert!((ta - tb).abs() <= EPS, "({first_z}) column {k}: top {ta} vs {tb}");
        assert!((fa - fb).abs() <= EPS, "({first_z}) column {k}: floor {fa} vs {fb}");
        two_sided += usize::from(ta < block_top - EPS && fa > block_floor + EPS);
    }
    let cells = (d.nx * d.ny) as f64 * d.cell * d.cell;
    assert!((a.volume() - b.volume()).abs() <= EPS * cells, "volume {} vs {}", a.volume(), b.volume());
    (c, two_sided, empties)
}

/// #780's pins, now through the real `simulate` (#781): a slot from the
/// top, flipped, then a slot from the NEW top across it. Each column's span
/// (a floor from the first cut, a top from the second, a sliver between
/// them, or nothing) equals the column body's to 1e-6.
fn cut_after_flip_agrees(first_z: f64, second_z: f64) {
    let (_, two_sided, empties) = chain_agrees(first_z, &[Step { turn: flip(), moves: slot_x(second_z), r: 1.5 }]);
    // The fixture exercised what it claims: where the slots cross, a column
    // bounded by both cuts, or one cut through.
    assert!(two_sided > 0 || empties > 0, "({first_z}, {second_z}): the slots never met");
}

#[test]
fn a_slot_cut_after_the_flip_carries_the_same_on_both_engines() {
    // The first slot at 3 leaves [2, 5] in the new frame; the second at 4
    // sits above it (two-sided columns with a centre each).
    cut_after_flip_agrees(3.0, 4.0);
}

#[test]
fn a_sliver_between_a_cut_from_below_and_one_from_above_carries_the_same() {
    // [2, 2.2] where the slots cross: no centre (2.25 is inside the second
    // cut), the floor from the first cut's record, the top from the
    // second's - and [2.3, 2.6], a sliver floating between the two cuts.
    cut_after_flip_agrees(3.0, 2.2);
    cut_after_flip_agrees(2.7, 2.6);
}

#[test]
fn a_second_slot_through_the_first_carries_the_same_on_both_engines() {
    // The second cut passes the first's floor: the crossing is cut through.
    cut_after_flip_agrees(3.0, 1.5);
    cut_after_flip_agrees(-1.0, 2.5);
}

/// A true XY arc in the new frame (#413 on the field, the exact arc on the
/// column engine): half a circle about an off-lattice centre, at depth `z`.
fn arc_at(z: f64) -> Vec<Move> {
    let (cx, cy, rr) = (10.1, 6.05, 3.7);
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    vec![
        Move::new(MoveKind::Rapid, p(cx - rr, cy, 8.0), p(cx - rr, cy, z), None),
        Move::new(MoveKind::Feed, p(cx - rr, cy, z), p(cx + rr, cy, z), Some(500.0)).with_arc(Point2::new(cx, cy), false),
    ]
}

#[test]
fn an_arc_cut_after_the_flip_carries_the_same_on_both_engines() {
    // The arc's ends cross the first slot (y' 4..8 in the new frame): above
    // its floor at 2 (two-sided columns), into a sliver, and through it.
    for (first_z, arc_z) in [(3.0, 4.0), (3.0, 2.2), (3.0, 1.5)] {
        let (_, two_sided, empties) = chain_agrees(first_z, &[Step { turn: flip(), moves: arc_at(arc_z), r: 1.3 }]);
        assert!(two_sided > 0 || empties > 0, "({first_z}, {arc_z}): the arc never met the slot");
    }
}

#[test]
fn a_second_carry_after_a_cut_carries_the_same_on_both_engines() {
    // Flip, cut from the new top, then carry again - a quarter turn about Z,
    // and a flip back (the bake moves the cut_downs records up again, the
    // second cut's cut_ups down) - and cut once more in that frame.
    let quarter = LatticeTurn::from_matrix([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], [12.0, 0.0, 0.0])
        .unwrap_or_else(|e| panic!("a quarter turn: {e}"));
    let back = flip();
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    // A slot along x' = 6 of the quarter-turned frame (12 wide in x).
    let across = |z: f64| {
        vec![
            Move::new(MoveKind::Rapid, p(-4.0, 9.7, 8.0), p(-4.0, 9.7, z), None),
            Move::new(MoveKind::Feed, p(-4.0, 9.7, z), p(16.0, 9.7, z), Some(500.0)),
        ]
    };
    // (Depths off each other's mirror: a third cut at exactly 5 - second
    // leaves a zero-thick column, which the column engine's strict
    // `top < floor` law keeps and the field reads as no material - a tie
    // of measure zero, not a verdict.)
    for (second, third) in [(4.0, 4.4), (2.2, 2.6), (1.5, 3.7)] {
        for steps in [
            [Step { turn: flip(), moves: slot_x(second), r: 1.5 }, Step { turn: quarter, moves: across(third), r: 1.2 }],
            [Step { turn: flip(), moves: arc_at(second), r: 1.3 }, Step { turn: back, moves: slot_x(third), r: 1.1 }],
        ] {
            let (c, two_sided, empties) = chain_agrees(3.0, &steps);
            assert!(c.flipped() && two_sided + empties > 0, "({second}, {third}): the cuts never met");
        }
    }
}
