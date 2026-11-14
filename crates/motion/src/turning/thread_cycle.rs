//! The thread-cycle record over a thread's laid passes (issue #34,
//! filled from the infeed plan since #416): the numbers a lathe
//! control's multi-pass threading cycle needs, read off the built path
//! and the plan that laid it.
//!
//! Provenance: the cycle quantities (first-pass and minimum depth,
//! finish allowance, infeed angle, spring passes) as public lathe
//! programming references list them for a multi-pass threading cycle
//! (GSK988TA manual G76, Fanuc-compatible; LinuxCNC G76) - behaviour
//! only; our own mapping from our pass plan.

use crate::cl::{MoveKind, ThreadCycle, Toolpath};

use super::infeed::{DepthSchedule, InfeedMethod, Pass, ThreadInfeed};
use super::thread::ThreadParams;

/// Attach the record over `moves[first..=last]`. The first depth is
/// the first pass's; the minimum depth is the rung for a constant-depth
/// ladder (every step is the rung) and the plan's own for a
/// constant-area one; the infeed angle is the INCLUDED angle of the
/// compound infeed (twice the flank angle; 0 radial). Alternating flank
/// infeed has no single compound angle, so it gets no record and says
/// so; the passes still post. A span the builder did not lay as
/// planned gets no record either.
#[allow(clippy::too_many_arguments)]
pub(super) fn attach_cycle(
    path: &mut Toolpath,
    first: usize,
    last: usize,
    ref_top: f64,
    root: f64,
    p: &ThreadParams,
    plan: &ThreadInfeed,
    passes: &[Pass],
) {
    const NONE: &str = "threading: the passes did not lay as planned - no thread cycle record";
    let infeed_angle = match plan.method {
        InfeedMethod::Radial => 0.0,
        InfeedMethod::Flank => 2.0 * plan.flank_angle,
        InfeedMethod::Alternating => {
            path.notes.push(
                "threading: alternating flank infeed has no single compound angle - no thread cycle \
                 record; the passes post as themselves"
                    .into(),
            );
            return;
        }
    };
    let (Some(m0), Some(end)) = (path.moves.get(first), last.checked_sub(1).and_then(|i| path.moves.get(i)))
    else {
        path.notes.push(NONE.into());
        return;
    };
    let height = ref_top - root;
    let first_depth = passes.first().map_or(p.depth, |q| ref_top - q.tip);
    let min_depth = match plan.schedule {
        DepthSchedule::ConstantDepth => p.infeed.min(height),
        DepthSchedule::ConstantArea => plan.min_depth,
    };
    let rec = ThreadCycle {
        start: m0.to,
        end: end.to,
        major_diameter: 2.0 * ref_top,
        minor_diameter: 2.0 * root,
        lead: p.pitch,
        height,
        first_depth,
        min_depth,
        finish_allowance: plan.finish_allowance,
        infeed_angle,
        spring_passes: plan.spring_passes,
        pull_out: 0.0,
        first,
        last,
        hand: p.hand,
    };
    if m0.kind == MoveKind::Rapid && rec.is_sane(&path.moves) {
        path.threads.push(rec);
    } else {
        path.notes.push(NONE.into());
    }
}
