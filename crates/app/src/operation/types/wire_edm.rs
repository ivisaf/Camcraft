//! The wire EDM contour's stored parameters (#532, part of #290): the
//! two planes of the workpiece, the machine condition the cut burns
//! with (by the control's own id), the stated punch / die side and the
//! start hole - resolved into the motion crate's `wire_edm::WireParams`
//! at generation. The wire's diameter and the spark gap are NOT stored
//! here: the machine's condition table is their one source (#531).
//!
//! Provenance: our own parameter set over the umbrella's decisions
//! (#290 D2, D4, D8) and general wire EDM practice (see
//! `motion::wire_edm`); no vendor material consulted.

use super::height::Height;

pub use crate::wire_edm::WireSide;

/// The run-out clearance (mm) a new wire cut is seeded with (#1109): an
/// open run is threaded and run out this far clear of the stock's edge,
/// past the kerf. A closed profile never reads it.
pub const DEFAULT_CLEARANCE: f64 = 1.0;

/// The longest condition id an operation stores (a control's selector
/// is a short word; this only bounds a hand-edited file).
pub const CONDITION_ID_MAX: usize = 64;

/// The wire contour's parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct WireContourOp {
    /// The workpiece's top: the plane the profile is read on (the upper
    /// programmed plane a taper will pose toward, #536).
    pub top_z: Height,
    /// The workpiece's bottom: the lower programmed plane the moves
    /// ride (#290 D2).
    pub bottom_z: Height,
    /// The machine's EDM condition this cut burns with, by its id; the
    /// empty word is "none chosen", refused at the command door.
    pub condition: String,
    /// Which side of the profile is the product - stated by the user,
    /// never inferred from the geometry (#290 D8): die / punch for a
    /// closed profile, the kerf's left / right of travel for an open
    /// run (#1109).
    pub side: WireSide,
    /// The start hole / threading point, world XY mm; `None` is not
    /// stated, and a closed profile is refused at generation without it.
    pub start: Option<[f64; 2]>,
    /// The tabs (#539); `None` is off, the default (#290 D6).
    pub tabs: Option<WireTabs>,
    /// The operator states the freed piece is caught (a slug catcher, a
    /// magnet, the machine's slug retention), so it may drop when the
    /// cut closes. `false` - the default - refuses a cut whose slug
    /// would drop unheld (#534's slug law, #539).
    pub capture: bool,
    /// The constant taper (#536), degrees from vertical: 0 is a
    /// vertical cut; positive widens the profile downward from the top
    /// plane (a die's relief, a drafted punch's base), negative narrows
    /// it. The machine's `edm.max_taper_deg` bounds it at generation.
    pub taper_deg: f64,
    /// No-core burnout (#540): `Some(clearance)` burns the pocket's
    /// whole interior out by offset rings, the wire centre at least
    /// `clearance` mm from the walls (never under the kerf offset);
    /// `None` cuts the profile alone.
    pub burnout: Option<f64>,
    /// An open run's threading and run-out clearance past the kerf, mm
    /// (#1109); unread for a closed profile.
    pub clearance: f64,
}

/// The tabs as stated (#539). Every field is required when tabs are
/// on; a missing one is refused by name at generation, never defaulted.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WireTabs {
    pub count: Option<u32>,
    /// The bridge's width along the profile, mm.
    pub width: Option<f64>,
    /// One position per tab, world XY mm, on the profile.
    pub at: Vec<[f64; 2]>,
    /// The machine condition the tab cut-off burns with, by its id; the
    /// empty word is none named.
    pub cutoff_condition: String,
}

impl WireTabs {
    #[must_use]
    pub fn is_sane(&self) -> bool {
        self.width.is_none_or(|w| w.is_finite() && w.abs() < 1e9)
            && self.at.len() <= 1024
            && self.at.iter().all(|p| p.iter().all(|v| v.is_finite() && v.abs() < 1e9))
            && self.cutoff_condition.len() <= CONDITION_ID_MAX
            && !self.cutoff_condition.chars().any(char::is_control)
    }
}

impl WireContourOp {
    /// Structural sanity (no world): the heights, a finite start point
    /// and a condition id that is one short word. Whether the machine
    /// states that condition is the command door's question
    /// (`operation::wire_op::condition_of`).
    #[must_use]
    pub fn is_sane(&self) -> bool {
        self.top_z.is_sane()
            && self.bottom_z.is_sane()
            && self.start.is_none_or(|s| s.iter().all(|v| v.is_finite() && v.abs() < 1e9))
            && self.taper_deg.is_finite()
            && self.taper_deg.abs() < 90.0
            && self.burnout.is_none_or(|c| c.is_finite() && c > 0.0 && c < 1e6)
            && self.clearance.is_finite()
            && (0.0..=crate::wire_edm::CLEARANCE_MAX).contains(&self.clearance)
            && self.condition.len() <= CONDITION_ID_MAX
            && !self.condition.chars().any(char::is_control)
            && self.tabs.as_ref().is_none_or(WireTabs::is_sane)
    }
}
