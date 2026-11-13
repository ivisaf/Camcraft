//! #793, #792: every library holder's outline matches the recorded
//! dimension tables (docs/decisions/793-taper-holders.md,
//! docs/decisions/792-er-collets.md), validates as a spun profile, and
//! refuses nonsense by name.

use super::*;
use crate::holder_profile::HolderBand;
use crate::spun::SpunProfile;

fn entry(name: &str) -> HolderEntry {
    find(name).expect("a library entry")
}

/// The dimension table, as recorded in the decision file (mm).
#[test]
fn the_tapers_and_nuts_match_the_recorded_table() {
    assert_eq!((BT30.gauge_d, BT30.flange_d, BT30.standoff, BT30.flange_t, BT30.front_face()), (31.75, 46.0, 2.0, 20.0, 22.0));
    assert_eq!((BT40.gauge_d, BT40.flange_d, BT40.standoff, BT40.flange_t, BT40.front_face()), (44.45, 63.0, 2.0, 25.0, 27.0));
    assert_eq!((CAT40.gauge_d, CAT40.flange_d, CAT40.standoff, CAT40.flange_t), (44.45, 63.5, 3.175, 15.875));
    assert!((CAT40.front_face() - 19.05).abs() < 1e-9, "0.750 in from the gauge line");
    let nuts: Vec<_> = ER_SIZES.iter().map(|e| (e.name, e.min_bore, e.max_bore, e.nut_d, e.nut_len)).collect();
    assert_eq!(
        nuts,
        [
            ("ER11", 0.5, 7.0, 22.0, 11.5),
            ("ER16", 0.5, 10.0, 32.0, 17.5),
            ("ER20", 1.0, 13.0, 35.0, 19.5),
            ("ER25", 1.0, 16.0, 42.0, 20.0),
            ("ER32", 2.0, 20.0, 50.0, 22.5),
            ("ER40", 3.0, 26.0, 63.0, 29.0),
        ]
    );
    assert_eq!((WELDON_SCREW, FLARE), (1.0, 1.0), "1 mm screw allowance, 45-degree flare");
}

/// The user's own holder (770MX, BT30 spindle, ER20 collets): every
/// point of the outline, read off the table.
#[test]
fn the_bt30_er20_outline_is_the_table() {
    let e = entry("BT30 ER20");
    assert_eq!(e.gauge_len, 60.0);
    let nose = 52.0;
    let pts = e.points(nose, 60.0).expect("draws");
    let zr: Vec<(f64, f64)> = pts.iter().map(|p| (p.z, p.r)).collect();
    let sr = (31.75 + 2.0 * 7.0 / 24.0) * 0.5;
    assert_eq!(
        zr,
        [
            (52.0, 0.0),
            (52.0, 17.5),         // nut face, Ø35
            (71.5, 17.5),         // nut top, 19.5 long
            (84.5, 17.5),         // body up to the flare: 5.5 below the flange
            (90.0, 23.0),         // 45-degree flare to the flange front (gauge - 22), Ø46
            (110.0, 23.0),        // 20 thick
            (110.0, sr),          // standoff, 2 to the gauge line
            (112.0, sr),
            (112.0, 0.0),         // the gauge line
        ]
    );
    assert!(pts.iter().all(|p| p.arc == 0.0));
}

#[test]
fn every_entry_draws_a_valid_profile_with_its_flange_and_nose() {
    let lib = library();
    assert!(lib.len() >= 30, "{} entries", lib.len());
    let mut names: Vec<String> = lib.iter().map(HolderEntry::name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), lib.len(), "names are unique");
    for e in &lib {
        let nose = 40.0;
        let pts = e.points(nose, e.gauge_len).unwrap_or_else(|err| panic!("{}: {err}", e.name()));
        let s = SpunProfile::new(pts.clone()).unwrap_or_else(|err| panic!("{}: {err}", e.name()));
        let gauge = nose + e.gauge_len;
        assert_eq!(pts.last().map(|p| p.z), Some(gauge), "{}: tops out at the gauge line", e.name());
        let bands = s.bands();
        let reach = |z: f64| bands.iter().filter(|b| b.above <= z && z <= b.above + b.len).fold(0.0f64, |m, b| m.max(b.r));
        let (nose_r, takes) = match e.clamp {
            Clamp::Er(er) => (er.nut_d * 0.5, er.max_bore),
            Clamp::Weldon { bore, body_d } => (body_d * 0.5 + WELDON_SCREW, bore),
        };
        let Some(t) = e.taper else {
            assert!((reach(gauge - 0.5) - nose_r).abs() < 1e-9, "{}: a standalone body runs at nut width", e.name());
            assert!(e.fits_shank(takes).is_ok(), "{}", e.name());
            continue;
        };
        let mid_flange = gauge - t.standoff - t.flange_t * 0.5;
        assert!(reach(mid_flange) >= t.flange_d * 0.5, "{}: the flange is judged whole", e.name());
        assert!(s.max_radius() >= t.flange_d * 0.5);
        assert!((reach(nose + 0.5) - nose_r).abs() < 1e-9, "{}: the nose reads its table diameter", e.name());
        assert!(nose_r > takes * 0.5, "{}: the body is wider than what it holds", e.name());
        assert!(e.fits_shank(takes).is_ok(), "{}", e.name());
    }
    for want in ["ER11 chuck", "ER16 chuck", "ER20 chuck", "ER25 chuck", "ER32 chuck", "ER40 chuck", "BT40 ER40", "BT30 ER16", "BT30 ER20", "BT30 ER25", "BT30 ER32", "BT40 ER16", "BT40 ER20", "BT40 ER25", "BT40 ER32", "CAT40 ER32"] {
        assert!(find(want).is_ok(), "{want} is in the library");
    }
    for want in ["BT30 Weldon 6 mm", "BT30 Weldon 1/2 in", "BT40 Weldon 20 mm", "BT40 Weldon 3/4 in", "CAT40 Weldon 1/2 in", "CAT40 Weldon 12 mm"] {
        assert!(find(want).is_ok(), "{want} is in the library");
    }
}

#[test]
fn a_longer_gauge_raises_the_flange_and_keeps_the_nose() {
    // The first band at flange width is the flare's: its 45-degree
    // envelope starts 31.5 - (21 + 1) = 9.5 below the flange front.
    let e = entry("BT40 Weldon 12 mm");
    let flange_foot = |g: f64| {
        let s = SpunProfile::new(e.points(30.0, g).expect("draws")).expect("valid");
        s.bands().iter().find(|b| b.r >= 31.5).map(|b: &HolderBand| b.above)
    };
    assert_eq!(flange_foot(63.0), Some(30.0 + 63.0 - 27.0 - 9.5));
    assert_eq!(flange_foot(120.0), Some(30.0 + 120.0 - 27.0 - 9.5));
}

#[test]
fn nonsense_is_refused_by_name() {
    let e = entry("bt30   er20");
    assert_eq!(e.name(), "BT30 ER20", "lookup ignores case and spacing");
    assert!(matches!(find("BT35 ER20"), Err(LibraryError::Unknown(_))));
    assert!(find("BT35 ER20").err().is_some_and(|m| m.to_string().contains("no holder 'BT35 ER20'")));
    assert_eq!(e.points(f64::NAN, 60.0), Err(LibraryError::NonFinite));
    assert_eq!(e.points(10.0, f64::INFINITY), Err(LibraryError::NonFinite));
    assert_eq!(e.points(-1.0, 60.0), Err(LibraryError::BelowTip));
    assert!(matches!(e.points(10.0, 30.0), Err(LibraryError::TooShort { .. })), "22 flange + 19.5 nut need more than 30");
    assert!(e.points(10.0, e.min_gauge()).is_ok());
    assert_eq!(e.points(10.0, 600.0), Err(LibraryError::TooLong));
    // ER20 takes 1-13 mm; a Weldon its own bore.
    assert!(e.fits_shank(6.0).is_ok() && e.fits_shank(13.0).is_ok());
    assert!(e.fits_shank(16.0).err().is_some_and(|m| m.to_string().contains("ER20 collets")));
    assert!(e.fits_shank(f64::NAN).is_err());
    let w = entry("BT30 Weldon 1/2 in");
    assert!(w.fits_shank(12.7).is_ok());
    assert!(w.fits_shank(12.0).is_err(), "a 12 mm shank is loose in a 1/2 in bore");
}

/// #792: the standalone ER20 chuck (the user's collet size) is its nut
/// and a body at nut width up to its back face - no flange, no taper.
#[test]
fn the_er20_chuck_outline_is_the_table() {
    let e = entry("ER20 chuck");
    assert_eq!((e.taper, e.gauge_len, e.min_gauge()), (None, 45.0, 20.5));
    let zr: Vec<(f64, f64)> = e.points(52.0, 45.0).expect("draws").iter().map(|p| (p.z, p.r)).collect();
    assert_eq!(zr, [(52.0, 0.0), (52.0, 17.5), (71.5, 17.5), (97.0, 17.5), (97.0, 0.0)]);
    for (n, d, len, g) in [("ER11", 22.0, 11.5, 30.0), ("ER40", 63.0, 29.0, 65.0)] {
        let e = entry(&format!("{n} chuck"));
        let zr: Vec<(f64, f64)> = e.points(10.0, g).expect("draws").iter().map(|p| (p.z, p.r)).collect();
        assert_eq!(zr, [(10.0, 0.0), (10.0, d / 2.0), (10.0 + len, d / 2.0), (10.0 + g, d / 2.0), (10.0 + g, 0.0)], "{n}");
    }
    assert!(e.fits_shank(13.0).is_ok() && e.fits_shank(14.0).is_err());
}

/// #800 review: the body-to-flange flare is enveloped by a 45-degree
/// cone ending at the flange front, and the gauge length must hold it.
#[test]
fn the_flare_envelope_runs_45_degrees_into_the_flange_front() {
    for e in library().iter().filter(|e| e.taper.is_some()) {
        let t = e.taper.expect("filtered");
        let pts = e.points(20.0, e.gauge_len).expect("draws");
        let front = 20.0 + e.gauge_len - t.front_face();
        let rise = (t.flange_d * 0.5 - e.body_r()).max(0.0);
        assert!((e.flare_len() - rise).abs() < 1e-9, "{}", e.name());
        if rise > 0.0 {
            let foot = pts.iter().position(|p| (p.z - (front - rise)).abs() < 1e-9 && (p.r - e.body_r()).abs() < 1e-9);
            let next = foot.and_then(|i| pts.get(i + 1)).map(|p| (p.z, p.r));
            assert_eq!(next, Some((front, t.flange_d * 0.5)), "{}: the cone ends at the flange front", e.name());
        }
        assert!(e.points(20.0, e.min_gauge()).is_ok(), "{}", e.name());
    }
    // BT30 Weldon 6 mm: body 25 + 2 → r 13.5; flange r 23: 9.5 of flare.
    let w = entry("BT30 Weldon 6 mm");
    assert_eq!((w.body_r(), w.flare_len(), w.min_gauge()), (13.5, 9.5, 31.5));
    assert!(matches!(w.points(0.0, 31.0), Err(LibraryError::TooShort { .. })), "the flare needs its room");
}

/// #800 review: a Weldon body is drawn 1 mm wider (radially) than its
/// catalog diameter, for the clamp screw head.
#[test]
fn a_weldon_body_carries_its_clamp_screw_allowance() {
    let w = entry("BT40 Weldon 12 mm");
    let pts = w.points(0.0, 63.0).expect("draws");
    assert_eq!(pts.get(1).map(|p| (p.z, p.r)), Some((0.0, 42.0 / 2.0 + 1.0)));
    assert_eq!(entry("BT30 ER20").body_r(), 17.5, "an ER nut takes no screw allowance");
}
