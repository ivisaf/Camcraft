//! Review of #817, round 2: a floor skin left under a cut that holds no
//! cell centre is a VERDICT, never NaN. The marches alone used to answer
//! `NoMaterial` for such a column, which `Stock::top`, `deviate`, the
//! sheet and the column masks all read as "nothing stands here" - the
//! rest under a slot vanished from the report, and the rest mask would
//! have treated that steel as air. Only `column_span` read the skin,
//! through `field_floor`'s sliver law; now `march::walk` reads it the
//! same way before it ever answers "no material".
//!
//! Gate map (#780): covers sim::stock_sim::march (walk, centreless,
//! column_top), march_verdicts (deviate_field), columns (Columns::top_at,
//! Columns::above), field_floor (sliver, shell_extent).
//! Provenance: the reviewer's scenario and sweep on PR #817; our own
//! fixtures.

use camcraft_sim::cl::{Move, MoveKind};
use camcraft_sim::geo2d::Point2;
use camcraft_sim::geometry::{cube, Point3};
use camcraft_sim::stock_sim::{ColumnSpan, LatticeTurn, SdfField, SimOp, Stock, FIELD_BAND};

/// An 8 x 6 plate `floor..top` at cell 0.5 with a flat r 2 slot along
/// y 3 at `z` (the reviewer's fixture).
fn plate_with_slot(floor: f64, top: f64, z: f64) -> Stock {
    let f = SdfField::from_mesh(&cube(Point3::new(0.0, 0.0, floor), Point3::new(8.0, 6.0, top)).facets, 0.5, FIELD_BAND)
        .unwrap_or_else(|e| panic!("plate {floor}..{top}: {e:?}"));
    let mut s = Stock::Field(f);
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let moves = [
        Move::new(MoveKind::Rapid, p(-4.0, 3.0, 5.0), p(-4.0, 3.0, z), None),
        Move::new(MoveKind::Feed, p(-4.0, 3.0, z), p(12.0, 3.0, z), Some(500.0)),
    ];
    if let Err(e) = s.simulate(&[SimOp { moves: &moves, radius: 2.0, ball: false, cone: None, corner: None }]) {
        panic!("cut: {e:?}");
    }
    s
}

/// The stock upside down, baked (#779): y -> -y, z -> -z.
fn flipped(s: &Stock) -> Stock {
    let flip = LatticeTurn::from_matrix([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]], [0.0; 3]).expect("a flip");
    Stock::Field(s.field().expect("a field").turned(&flip).0)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9
}

#[test]
fn a_floor_skin_under_a_slot_is_a_verdict_on_every_path() {
    // The reviewer's scenario: an 8 x 6 plate z -6.05..0 at cell 0.5, a
    // flat r 2 slot at -5.8 leaves [-6.05, -5.8] standing with no cell
    // centre inside it (-5.75 is in the tool, -6.25 under the floor).
    // The part is the plate's bottom 0.05 and below, so 0.2 of the skin
    // is rest: every column reads, every column is rest, none is NaN.
    let s = plate_with_slot(-6.05, 0.0, -5.8);
    let top = s.top(4.25, 3.25);
    assert!(near(top, -5.8), "Stock::top under the slot: {top} (was NaN)");
    assert!(near(s.top(4.25, 0.75), 0.0), "beside the slot");
    let part = cube(Point3::new(0.0, 0.0, -7.0), Point3::new(8.0, 6.0, -6.0)).facets;
    let d = s.deviate(&part, 0.01).expect("a verdict");
    let nan = d.dev.iter().filter(|v| v.is_nan()).count();
    assert_eq!(nan, 0, "{nan} of {} columns read no verdict", d.dev.len());
    assert_eq!(d.dev.len(), 192);
    assert_eq!(d.rest_cells, 192, "every column holds rest: 0.2 under the slot, 6 beside it");
    assert_eq!(d.gouge_cells, 0);
    let k = |x: f64, y: f64| ((x / 0.5) as usize) + ((y / 0.5) as usize) * d.cells_x;
    assert!(near(d.dev[k(4.25, 3.25)], 0.2), "dev under the slot {} (was NaN)", d.dev[k(4.25, 3.25)]);
    assert!(near(d.dev[k(4.25, 0.75)], 6.0), "dev beside the slot {}", d.dev[k(4.25, 0.75)]);
    // The column masks (the rest and program masks read `above`): the
    // skin stands above -6.0, and no column of the plate is NaN.
    let cols = s.columns().expect("columns");
    for iy in 0..cols.ny() {
        for ix in 0..cols.nx() {
            let t = cols.top_at(ix, iy);
            assert!(t.is_finite(), "column ({ix}, {iy}) reads {t}");
        }
    }
    let above = cols.above(-6.0).expect("a mask");
    assert!(above.contains(Point2::new(4.25, 3.25)), "the skin under the slot is in the rest mask above -6.0");
    assert!(above.contains(Point2::new(4.25, 0.75)));
    assert!(!cols.above(-5.8).expect("a mask").contains(Point2::new(4.25, 3.25)), "the slot's floor is exact");
    assert!(!cols.above(-5.0).expect("a mask").contains(Point2::new(4.25, 3.25)));
    // Flipped, the skin is [5.8, 6.05] under the new top: `Stock::top`
    // reads 6.05 through the same sliver (no material centre either way).
    let h = flipped(&s);
    assert!(near(h.top(4.25, -3.25), 6.05), "flipped top under the slot: {}", h.top(4.25, -3.25));
    match h.field().expect("a field").column_span(4.25, -3.25) {
        ColumnSpan::Material { top, floor } => assert!(near(top, 6.05) && near(floor, 5.8), "flipped: [{floor}, {top}]"),
        other => panic!("flipped: {other:?}"),
    }
    // The floor skin older than #817: a floor on a cell boundary (-6.0)
    // cut to -5.8 was NaN on `main` too; it reads the same way.
    let o = plate_with_slot(-6.0, 0.0, -5.8);
    assert!(near(o.top(4.25, 3.25), -5.8), "floor -6.0, slot -5.8: {}", o.top(4.25, 3.25));
    // Cut through, the column is air on every path - and the plate's
    // other columns keep their verdict.
    let t = plate_with_slot(-6.05, 0.0, -6.5);
    assert!(t.top(4.25, 3.25).is_nan(), "cut through: {}", t.top(4.25, 3.25));
    assert!(near(t.top(4.25, 0.75), 0.0));
    assert!(!t.columns().expect("columns").above(-7.0).expect("a mask").contains(Point2::new(4.25, 3.25)));
}

/// What the column under the slot holds: `[floor, min(top, slot)]`,
/// `None` when the slot went through.
fn truth(floor: f64, top: f64, slot: f64) -> Option<(f64, f64)> {
    (slot >= floor).then_some((floor, top.min(slot)))
}

/// Is `read` a safe `Stock::top` for `truth`? Where material stands the
/// read is finite and at or above its top (to rounding) - never NaN,
/// never under; where nothing stands any read is safe. Returns whether
/// the read was exact.
fn safe_top(read: f64, truth: Option<(f64, f64)>, what: &str) -> bool {
    match truth {
        Some((_, t)) => {
            assert!(read.is_finite(), "{what}: the shell proves material to {t}, the top read {read}");
            assert!(read >= t - 1e-9, "{what}: top read {read} under {t}");
            near(read, t)
        }
        None => read.is_nan(),
    }
}

/// `safe_top`'s twin for `column_span` on the flipped stock.
fn safe_span(read: ColumnSpan, truth: Option<(f64, f64)>, what: &str) -> bool {
    match (read, truth) {
        (ColumnSpan::Material { top, floor }, Some((f, t))) => {
            assert!(top >= t - 1e-9 && floor <= f + 1e-9, "{what}: read [{floor}, {top}] under [{f}, {t}]");
            near(top, t) && near(floor, f)
        }
        (ColumnSpan::Empty, Some((f, t))) => {
            assert!(t - f <= 1e-9, "{what}: read empty under [{f}, {t}]");
            true
        }
        (ColumnSpan::Empty, None) => true,
        (ColumnSpan::Material { .. }, None) => false,
        (ColumnSpan::CutFromBelow | ColumnSpan::Unreadable, Some(_)) => {
            panic!("{what}: the shell proves material and the column refused {read:?}")
        }
        (ColumnSpan::CutFromBelow | ColumnSpan::Unreadable, None) => false,
    }
}

#[test]
fn no_column_the_shell_proves_reads_nan_or_air_through_stock_top() {
    // The reviewer's sweep through the VERDICT path: floors on and off
    // cell centres and boundaries, tops likewise, slots from half a cell
    // under the floor to well above it - skins of 0.01 to 0.3 among them.
    // Under the slot and beside it, upright and flipped, `Stock::top` is
    // finite and at or above the truth wherever the shell proves material
    // (never NaN, never under), and `deviate` against the plate's own
    // bottom counts every column. The exact count is pinned so a drift
    // toward refusal shows.
    let (mut exact, mut total) = (0usize, 0usize);
    for floor in [-6.0, -6.05, -6.1, -6.2, -6.25, -6.3] {
        for top in [0.0, 0.25, -0.2] {
            for d in [-0.5, 0.0, 0.01, 0.05, 0.1, 0.2, 0.25, 0.3, 0.45, 0.5, 0.75, 1.5] {
                let slot_z = floor + d;
                let s = plate_with_slot(floor, top, slot_z);
                let h = flipped(&s);
                let under = truth(floor, top, slot_z);
                let beside = Some((floor, top));
                let what = format!("plate {floor}..{top}, slot {slot_z}");
                let flip = |t: Option<(f64, f64)>| t.map(|(f, t)| (-t, -f));
                for (read, t, side) in [
                    (s.top(4.25, 3.25), under, "upright, under the slot"),
                    (s.top(4.25, 0.75), beside, "upright, beside the slot"),
                    (h.top(4.25, -3.25), flip(under), "flipped, under the slot"),
                    (h.top(4.25, -0.75), flip(beside), "flipped, beside the slot"),
                ] {
                    total += 1;
                    if safe_top(read, t, &format!("{what}, {side}")) {
                        exact += 1;
                    }
                }
                let hf = h.field().expect("a field");
                for (read, t, side) in [
                    (hf.column_span(4.25, -3.25), flip(under), "flipped span, under the slot"),
                    (hf.column_span(4.25, -0.75), flip(beside), "flipped span, beside the slot"),
                ] {
                    total += 1;
                    if safe_span(read, t, &format!("{what}, {side}")) {
                        exact += 1;
                    }
                }
                // The verdict against the plate's own bottom: every
                // column has material (the slot went through or not),
                // so no column is NaN, and where the skin is thicker
                // than the tolerance every column of the plate is rest.
                let part = cube(Point3::new(0.0, 0.0, floor - 1.0), Point3::new(8.0, 6.0, floor)).facets;
                let dv = s.deviate(&part, 0.001).expect("a verdict");
                if d >= 0.0 {
                    let nan = dv.dev.iter().filter(|v| v.is_nan()).count();
                    assert_eq!(nan, 0, "{what}: {nan} columns of {} read no verdict", dv.dev.len());
                    assert_eq!(dv.gouge_cells, 0, "{what}");
                    if d > 0.001 {
                        assert_eq!(dv.rest_cells, dv.dev.len(), "{what}: rest cells");
                    }
                }
            }
        }
    }
    eprintln!("exact {exact} of {total}");
    assert_eq!(total, 1296);
    assert!(exact * 10 >= total * 9, "exact reads {exact} of {total}: the sweep is refusing where it read");
}
