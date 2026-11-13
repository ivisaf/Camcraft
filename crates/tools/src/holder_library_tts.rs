//! The Tormach TTS holders of the built-in holder library (#794): the
//! Tormach Tooling System's 3/4 in straight shank, held in an R8 (or
//! 3MT) spindle collet, locates on a ground shoulder that seats on the
//! spindle face. Specced from Tormach's public TTS operation manual and
//! product pages (sources and the numbers taken in
//! docs/decisions/794-tts-holders.md); the outlines are our own
//! conservative bodies, not Tormach's models.
//!
//! What is drawn, tip-up: the nose (an ER nut, or a set-screw body),
//! the body up to the collar, the collar, and nothing above it. The
//! collar's top face is the gauge line: it is the TTS shoulder that
//! seats on the spindle face, and the surface Tormach's manual says
//! every tool length is measured from - so gauge length plus projection
//! is the collar-to-tip length the tool setter reads and the H offset
//! holds. The 3/4 in shank above it is inside the spindle collet, out
//! of reach of the work, the way a steep taper is.
//!
//! Tormach publishes the ER holders' outside diameter and installed
//! height but not the collar's diameter or thickness, nor any
//! set-screw holder's body: those are enveloped, larger never smaller
//! ([`TTS`], [`TTS_BODY_D`]).

use super::{Clamp, HolderEntry, Taper, ER16, ER20};

/// The TTS collar as an envelope: Ø1.5 in (38.1 mm, the diameter of a
/// TTS blank holder's machinable end) by 0.5 in (12.7 mm) thick,
/// over a Ø3/4 in shank; no standoff - its top face is the gauge line.
/// Neither the collar's diameter nor its thickness is published.
pub const TTS: Taper = Taper { name: "TTS", ring: "collar", gauge_d: 19.05, flange_d: 38.1, standoff: 0.0, flange_t: 12.7 };

/// The set-screw holders' body, enveloped at Ø1.5 in (38.1 mm) like the
/// collar: Tormach does not publish their body diameters. The clamp
/// screw allowance ([`super::WELDON_SCREW`]) rides on top.
pub const TTS_BODY_D: f64 = 38.1;

/// The default gauge length (collar face to nose face) of a set-screw
/// holder, mm: 2.36 in, the machinable length of a TTS blank - no
/// set-screw holder's installed height is published.
const SET_SCREW_GAUGE: f64 = 60.0;

/// Tormach's TTS set-screw holder bores (PN 31818-31824): 1/4, 5/16,
/// 3/8, 1/2 in and 6, 10, 12 mm.
const BORES: [(&str, f64); 7] = [
    ("1/4 in", 6.35),
    ("5/16 in", 7.9375),
    ("3/8 in", 9.525),
    ("1/2 in", 12.7),
    ("6 mm", 6.0),
    ("10 mm", 10.0),
    ("12 mm", 12.0),
];

/// The TTS entries, in the editor's order: the ER holders at their
/// published installed heights (ER20 2.05 in / 52 mm, ER16 1.93 in /
/// 49 mm), then the set-screw holders.
pub(super) fn entries() -> Vec<HolderEntry> {
    let er = [(ER20, 52.0), (ER16, 49.0)]
        .into_iter()
        .map(|(e, gauge_len)| HolderEntry { taper: Some(TTS), clamp: Clamp::Er(e), gauge_len, label: "" });
    let weldon = BORES.iter().map(|&(label, bore)| HolderEntry {
        taper: Some(TTS),
        clamp: Clamp::Weldon { bore, body_d: TTS_BODY_D },
        gauge_len: SET_SCREW_GAUGE,
        label,
    });
    er.chain(weldon).collect()
}

#[cfg(test)]
#[path = "holder_library_tts_tests.rs"]
mod tests;
