//! Sinker EDM rough and finish with undersize (#543): which pass a burn
//! is, the size law that ties an electrode to its condition and to the
//! finished cavity, and the order law between a rough and its finish.
//!
//! Provenance: general die-sink EDM practice from public machine-tool
//! handbooks - an electrode is made smaller than the finished cavity by
//! a per-side undersize; the cavity it burns is the electrode grown by
//! its condition's spark gap on every side; a rough electrode carries
//! the larger undersize and a finish electrode the smaller, so the
//! finish takes off what the rough left. The refusals and the warning
//! are our own design (#290 D1-D10). No vendor material.
//!
//! The law, per axis, for an electrode `e` wide burning a cavity `c`
//! wide under a condition of undersize `u` and spark gap `g`:
//! - the electrode is the one the condition calls for: `e = c - 2u`;
//! - it burns `e + 2g`, which never exceeds `c` (more would burn past
//!   the finished wall: a gouge);
//! - a finish burns exactly `c` - less leaves the finished cavity
//!   under size, refused by name;
//! - the electrode stands `u` inside the finished surface on every
//!   side, its face included: the tip stops `u` above the finished
//!   floor, so a rough leaves `u - g` on the floor as on the walls.

use crate::machining2d::GenError;

/// Which pass of a cavity a burn is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SinkerPass {
    /// One electrode, no undersize law (#541): the tip stops one spark
    /// gap above the stated floor.
    #[default]
    Single,
    /// The rough electrode: undersized for its condition, it leaves
    /// stock for the finish on the walls and floor.
    Rough,
    /// The finish electrode: it burns the cavity to its finished size.
    Finish,
}

impl SinkerPass {
    /// The pass in the words a refusal or a page names.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            SinkerPass::Single => "single",
            SinkerPass::Rough => "rough",
            SinkerPass::Finish => "finish",
        }
    }
}

/// Slack under which two sizes count as equal, mm: a micron, below any
/// electrode's stated tolerance.
pub const SIZE_SLACK: f64 = 1e-3;

/// One undersized burn's numbers, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Undersize {
    pub pass: SinkerPass,
    /// The finished cavity's X and Y size.
    pub cavity: (f64, f64),
    /// The electrode face's X and Y size.
    pub electrode: (f64, f64),
    /// The condition's spark gap per side.
    pub gap: f64,
    /// The condition's electrode undersize per side.
    pub undersize: f64,
}

/// Judge one undersized burn, by name: the cavity it burns against the
/// finished size, and the electrode against its condition. On success,
/// the stand-off the tip keeps above the finished floor (the undersize).
/// A `Single` pass is not judged here and stands one gap off.
///
/// # Errors
/// A garbled number; a burn wider than the finished cavity (it would
/// burn past the wall); a finish that leaves the cavity under size; an
/// electrode that is not the size its condition calls for.
pub fn judge(u: &Undersize, condition: &str) -> Result<f64, GenError> {
    if u.pass == SinkerPass::Single {
        return Ok(u.gap);
    }
    let pass = u.pass.label();
    let nums = [u.cavity.0, u.cavity.1, u.electrode.0, u.electrode.1, u.gap, u.undersize];
    if !nums.iter().all(|v| v.is_finite()) {
        return Err(GenError::Invalid(format!("sinker {pass}: a size, gap or undersize is not a finite number")));
    }
    if !(u.cavity.0 > 0.0 && u.cavity.1 > 0.0) {
        return Err(GenError::Invalid(format!(
            "sinker {pass}: the finished cavity size {} x {} is not stated - a {pass} burn is judged against it",
            u.cavity.0, u.cavity.1
        )));
    }
    if u.undersize < 0.0 || u.gap < 0.0 {
        return Err(GenError::Invalid(format!(
            "sinker {pass}: condition {condition:?} states a negative gap or undersize"
        )));
    }
    for (axis, c, e) in [("X", u.cavity.0, u.electrode.0), ("Y", u.cavity.1, u.electrode.1)] {
        let burned = e + 2.0 * u.gap;
        if burned > c + SIZE_SLACK {
            return Err(GenError::Invalid(format!(
                "sinker {pass}: the electrode's {e:.3} in {axis} plus 2 x the {:.3} spark gap of condition {condition:?} burns {burned:.3}, wider than the {c:.3} cavity - the wall would be burned {:.3} past size per side",
                u.gap,
                (burned - c) * 0.5
            )));
        }
        if u.pass == SinkerPass::Finish && burned < c - SIZE_SLACK {
            return Err(GenError::Invalid(format!(
                "sinker finish leaves the cavity under size: the electrode's {e:.3} in {axis} plus 2 x the {:.3} spark gap of condition {condition:?} burns {burned:.3}, short of the {c:.3} cavity by {:.3} per side",
                u.gap,
                (c - burned) * 0.5
            )));
        }
        let wanted = c - 2.0 * u.undersize;
        if (e - wanted).abs() > SIZE_SLACK {
            return Err(GenError::Invalid(format!(
                "sinker {pass}: condition {condition:?} undersizes its electrode {:.3} per side, so under the {c:.3} cavity it is {wanted:.3} in {axis} - the electrode is {e:.3}",
                u.undersize
            )));
        }
    }
    Ok(u.undersize)
}

/// The order law between a rough and the finish after it: the finish
/// electrode's undersize must be smaller than the rough's, or the finish
/// takes nothing off the walls the rough left. `true` when the pair is
/// inverted (or equal) - a warning, not a refusal: the finish's own size
/// law already holds the finished cavity to size.
#[must_use]
pub fn inverted(finish_undersize: f64, rough_undersize: f64) -> bool {
    finish_undersize >= rough_undersize - SIZE_SLACK
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(pass: SinkerPass, e: f64, gap: f64, undersize: f64) -> Undersize {
        Undersize { pass, cavity: (20.0, 12.0), electrode: (e, e - 8.0), gap, undersize }
    }

    #[test]
    fn sinker_undersize_judges_rough_and_finish() {
        // Rough: 0.3 under per side, 0.1 gap - burns 19.6, leaves 0.2.
        assert_eq!(judge(&pair(SinkerPass::Rough, 19.4, 0.1, 0.3), "R").expect("rough"), 0.3);
        // Finish: undersize equals the gap - burns exactly 20.
        assert_eq!(judge(&pair(SinkerPass::Finish, 19.96, 0.02, 0.02), "F").expect("finish"), 0.02);
        // A finish whose condition undersizes more than it gaps: under size.
        let e = judge(&pair(SinkerPass::Finish, 19.9, 0.02, 0.05), "F").expect_err("under size");
        assert!(e.to_string().contains("leaves the cavity under size"), "{e}");
        // A rough whose gap outburns its undersize: past the wall.
        let e = judge(&pair(SinkerPass::Rough, 19.8, 0.2, 0.1), "R").expect_err("overburn");
        assert!(e.to_string().contains("past size"), "{e}");
        // The wrong electrode for the condition.
        let e = judge(&pair(SinkerPass::Rough, 19.0, 0.1, 0.3), "R").expect_err("wrong electrode");
        assert!(e.to_string().contains("undersizes its electrode"), "{e}");
        // No cavity stated.
        let none = Undersize { cavity: (0.0, 12.0), ..pair(SinkerPass::Rough, 19.4, 0.1, 0.3) };
        assert!(judge(&none, "R").is_err());
        assert!(inverted(0.3, 0.3) && inverted(0.4, 0.3) && !inverted(0.02, 0.3));
    }
}
