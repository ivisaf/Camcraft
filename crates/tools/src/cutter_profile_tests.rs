//! Unit tests of the drawn cutter law (#93 slice 2): every catalog
//! milling tool's own numbers pass it, turning tools never do, and a
//! drawn outline that disagrees with the numbers is refused by name.

use super::*;
use crate::tools::catalog;

fn p(z: f64, r: f64) -> SpunPoint {
    SpunPoint::line(z, r)
}

fn em10() -> Tool {
    catalog().into_iter().find(|(n, _)| *n == "EM-1000-4FL").map(|(_, t)| t).unwrap()
}

/// EM-1000-4FL drawn by hand with a neck: Ø10 x 22 flutes, a Ø8 neck
/// from 22 to 30, the Ø10 shank to 62 (22 + 40).
fn necked() -> Vec<SpunPoint> {
    vec![p(0.0, 0.0), p(0.0, 5.0), p(22.0, 5.0), p(22.0, 4.0), p(30.0, 4.0), p(30.0, 5.0), p(62.0, 5.0), p(62.0, 0.0)]
}

#[test]
fn every_milling_catalog_tool_passes_its_own_cutter_law() {
    for (name, t) in catalog() {
        let pts = parametric(&t);
        if is_lathe(t.kind) {
            assert!(pts.is_empty(), "{name}: a turning tool has no spun cutter");
            assert_eq!(check(&t, necked()), Err(CutterError::Lathe), "{name}");
            continue;
        }
        let s = check(&t, pts).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!((s.max_radius() - (t.diameter * 0.5).max(t.shank_d * 0.5)).abs() < 1e-9, "{name}");
    }
}

#[test]
fn a_hand_drawn_neck_that_keeps_the_numbers_is_accepted() {
    let s = check(&em10(), necked()).unwrap();
    assert_eq!(s.points().len(), 8);
}

#[test]
fn a_cutter_that_disagrees_with_the_numbers_is_refused_by_name() {
    let t = em10();
    let refuse = |pts: Vec<SpunPoint>| check(&t, pts).unwrap_err();
    // Ø 12 within the flutes, against a stated Ø 10.
    let mut wide = necked();
    wide[1].r = 6.0;
    wide[2].r = 6.0;
    assert_eq!(refuse(wide), CutterError::Diameter { drawn: 12.0, stated: 10.0 });
    // Never reaching the stated diameter is as wrong as passing it.
    let mut thin = necked();
    thin[1].r = 4.5;
    thin[2].r = 4.5;
    assert!(matches!(refuse(thin), CutterError::Diameter { .. }));
    // Too short / too long against flute + shank = 62.
    let mut short = necked();
    short[6].z = 50.0;
    short[7].z = 50.0;
    assert_eq!(refuse(short), CutterError::Length { drawn: 50.0, stated: 62.0 });
    // Starts above the tip.
    let mut lifted = necked();
    lifted[0].z = 1.0;
    lifted[1].z = 1.0;
    assert_eq!(refuse(lifted), CutterError::OffTip(1.0));
    // A collar wider than shank and cutter above the flutes.
    let mut collar = necked();
    collar[5].r = 7.0;
    collar[6].r = 7.0;
    assert_eq!(refuse(collar), CutterError::TooWide { point: 6, limit: 5.0 });
    // Out and back at one height: the outline crosses itself.
    let fold = vec![p(0.0, 0.0), p(0.0, 5.0), p(22.0, 5.0), p(22.0, 4.0), p(22.0, 4.5), p(62.0, 4.5), p(62.0, 0.0)];
    assert_eq!(refuse(fold), CutterError::Folds(4));
    // Every spun refusal rides through by name.
    let mut neg = necked();
    neg[3].r = -1.0;
    assert_eq!(refuse(neg), CutterError::Shape(SpunError::Negative(4)));
    let mut nan = necked();
    nan[2].z = f64::NAN;
    assert_eq!(refuse(nan), CutterError::Shape(SpunError::NonFinite(3)));
    assert_eq!(refuse(Vec::new()), CutterError::Shape(SpunError::TooFew));
    assert!(refuse(necked()[..2].to_vec()).to_string().contains("at least 3 points"));
}

#[test]
fn a_number_edit_is_re_judged_against_the_stored_cutter() {
    let s = check(&em10(), necked()).unwrap();
    let t = Tool { diameter: 12.0, ..em10() };
    assert!(matches!(agrees(&t, &s), Err(CutterError::Diameter { .. })));
    let t = Tool { shank_len: 50.0, ..em10() };
    assert!(matches!(agrees(&t, &s), Err(CutterError::Length { .. })));
    assert_eq!(agrees(&em10(), &s), Ok(()));
}
