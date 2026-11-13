//! #794: the TTS holders match the recorded table
//! (docs/decisions/794-tts-holders.md), draw closed, revolvable
//! profiles, and top out at the collar face - the gauge line the tool
//! setter measures from.

use super::super::{find, library, Clamp, HolderEntry, WELDON_SCREW};
use super::*;
use crate::spun::SpunProfile;

fn entry(name: &str) -> HolderEntry {
    find(name).expect("a library entry")
}

fn close(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p.0 - q.0).abs() < 1e-9 && (p.1 - q.1).abs() < 1e-9)
}

fn tts() -> Vec<HolderEntry> {
    library().into_iter().filter(|e| e.taper.is_some_and(|t| t.name == "TTS")).collect()
}

/// The published and enveloped numbers, as recorded.
#[test]
fn the_tts_table_is_the_recorded_one() {
    assert_eq!((TTS.gauge_d, TTS.flange_d, TTS.standoff, TTS.flange_t, TTS.ring), (19.05, 38.1, 0.0, 12.7, "collar"));
    assert_eq!(TTS_BODY_D, 38.1);
    let names: Vec<(String, f64)> = tts().iter().map(|e| (e.name(), e.gauge_len)).collect();
    let want = [
        ("TTS ER20", 52.0),
        ("TTS ER16", 49.0),
        ("TTS Weldon 1/4 in", 60.0),
        ("TTS Weldon 5/16 in", 60.0),
        ("TTS Weldon 3/8 in", 60.0),
        ("TTS Weldon 1/2 in", 60.0),
        ("TTS Weldon 6 mm", 60.0),
        ("TTS Weldon 10 mm", 60.0),
        ("TTS Weldon 12 mm", 60.0),
    ];
    assert_eq!(names, want.map(|(n, g)| (n.to_owned(), g)).to_vec());
    // ER20 and ER16 nuts are drawn from the one ER table at Ø35 / Ø32,
    // no smaller than Tormach's published Ø34 / Ø27.
    assert!(entry("TTS ER20").body_r() * 2.0 >= 34.0 && entry("TTS ER16").body_r() * 2.0 >= 27.0);
}

/// The 770M / 1100M ER20 holder, point by point: nut, body, 45-degree
/// flare into the Ø38.1 collar, collar up to the gauge line.
#[test]
fn the_tts_er20_outline_is_the_table() {
    let e = entry("TTS ER20");
    let zr: Vec<(f64, f64)> = e.points(40.0, 52.0).expect("draws").iter().map(|p| (p.z, p.r)).collect();
    let front = 92.0 - 12.7;
    let want = [
        (40.0, 0.0),
        (40.0, 17.5),                  // nut face, Ø35
        (59.5, 17.5),                  // nut top, 19.5 long
        (front - 1.55, 17.5),          // body up to the flare foot
        (front, 19.05),                // the collar's front face, Ø38.1
        (92.0, 19.05),                 // the collar's top face: the gauge line
        (92.0, 0.0),
    ];
    assert!(close(&zr, &want), "{zr:?}");
    assert!((e.min_gauge() - (12.7 + 19.5 + 1.55)).abs() < 1e-9);
}

/// A set-screw body (Ø38.1 + the screw allowance) stands wider than
/// the collar: the outline steps in to the collar, never under it.
#[test]
fn a_tts_set_screw_holder_steps_in_to_its_collar() {
    let e = entry("TTS Weldon 1/2 in");
    let r = TTS_BODY_D * 0.5 + WELDON_SCREW;
    assert!(matches!(e.clamp, Clamp::Weldon { bore, .. } if bore == 12.7));
    let zr: Vec<(f64, f64)> = e.points(30.0, 60.0).expect("draws").iter().map(|p| (p.z, p.r)).collect();
    let front = 90.0 - 12.7;
    assert!(close(&zr, &[(30.0, 0.0), (30.0, r), (front, r), (front, 19.05), (90.0, 19.05), (90.0, 0.0)]), "{zr:?}");
    assert!(e.fits_shank(12.7).is_ok() && e.fits_shank(12.0).is_err());
    assert!(entry("TTS Weldon 5/16 in").fits_shank(7.9375).is_ok());
}

/// Every TTS entry is a closed, revolvable profile (on the axis at both
/// ends, valid as a spun profile) whose highest point is the collar
/// face: tip to gauge line = projection + gauge length, the TTS tool
/// length the setter measures and the H offset holds.
#[test]
fn every_tts_profile_is_closed_and_tops_out_at_the_collar_face() {
    for e in tts() {
        for (nose, gauge) in [(25.0, e.gauge_len), (60.0, e.gauge_len + 30.0), (0.0, e.min_gauge())] {
            let pts = e.points(nose, gauge).unwrap_or_else(|err| panic!("{}: {err}", e.name()));
            let s = SpunProfile::new(pts.clone()).unwrap_or_else(|err| panic!("{}: {err}", e.name()));
            let (first, last) = (pts.first().expect("points"), pts.last().expect("points"));
            assert_eq!((first.z, first.r, last.r), (nose, 0.0, 0.0), "{}: closed to the axis", e.name());
            let top = pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
            assert!((top - (nose + gauge)).abs() < 1e-9, "{}: the gauge line is the top", e.name());
            let collar = pts.iter().rev().nth(1).map(|p| (p.z, p.r));
            assert_eq!(collar, Some((nose + gauge, 19.05)), "{}: the collar face reaches the gauge line", e.name());
            assert!(s.max_radius() >= 19.05 && pts.iter().all(|p| p.arc == 0.0));
        }
    }
}
