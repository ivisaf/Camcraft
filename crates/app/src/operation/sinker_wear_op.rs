//! The Sinker Burn's electrode wear compensation (#546): which axial
//! wear rate the burn uses - a wear measured on the op, else the named
//! condition's stated `wear` (#531's table) - and the as-burned reading
//! the stock sim judges. With neither, the burn is not compensated and
//! says so on its page and its path.
//!
//! Provenance: our own glue over the motion crate's `sinker_wear`, whose
//! module head cites its public sources (general die-sink EDM practice);
//! the rates are the user's data (#290 D4), never a shipped table.

use flecs_ecs::prelude::World;

use crate::cl::Move;
use crate::sinker_wear::{summary, worn_tip, SinkerWear, WearComp};

use super::types::{OpParams, SinkerBurnOp, SrcRef};

/// The named condition's stated axial wear rate, read from the op's
/// setup's machine; `None` when it states none - or when the condition
/// cannot be read at all (the gap door, `sinker_op::condition_gap`,
/// refuses that by name before any burn exists).
#[must_use]
pub fn condition_wear(world: &World, setup: Option<&str>, op: &SinkerBurnOp) -> Option<f64> {
    let mounted = crate::machine::of_setup(world, setup);
    let id = op.condition.trim();
    mounted.1.as_ref()?.edm.as_ref()?.conditions.iter().find(|c| c.id == id)?.wear
}

/// The burn's compensation: the wear measured on the op first (it is
/// this electrode's own reading), else the condition's stated rate, else
/// none - no fixed ratio is ever assumed.
#[must_use]
pub fn wear_comp(op: &SinkerBurnOp, cond_wear: Option<f64>) -> Option<WearComp> {
    if let (Some(rate), Some(l), Some(o)) = (op.measured_rate(), op.wear_lost, op.wear_over) {
        return Some(WearComp { rate, from: format!("measured: {l} lost over {o} burned") });
    }
    cond_wear.map(|rate| WearComp { rate, from: format!("condition {}", op.condition.trim()) })
}

/// The motion crate's wear and layering for this op.
#[must_use]
pub fn sinker_wear(op: &SinkerBurnOp, cond_wear: Option<f64>) -> SinkerWear {
    SinkerWear { comp: wear_comp(op, cond_wear), layer: op.layer }
}

/// The page's line: "not compensated", or the rate and its source.
#[must_use]
pub fn wear_status(world: &World, setup: Option<&str>, op: &SinkerBurnOp) -> String {
    if let Some(why) = op.wear_refusal() {
        return why;
    }
    summary(wear_comp(op, condition_wear(world, setup, op)).as_ref())
}

/// The moves the stock sim judges for `op`: a sinker burn's
/// [`as_burned`] (with its electrode atom's `gap`), every other op's own.
///
/// # Errors
/// [`as_burned`]'s.
pub fn judged(world: &World, op: &crate::schema::Operation, gap: Option<f64>, moves: Vec<Move>) -> Result<Vec<Move>, String> {
    match (&op.params, gap) {
        (OpParams::SinkerBurn(sb), Some(gap)) => as_burned(world, op.setup.as_deref(), &op.src, sb, gap, moves),
        _ => Ok(moves),
    }
}

/// The burn's moves as the worn electrode burns them (#546): each tip
/// below the work raised by the wear the rate states by then, so the
/// stock sim judges the cavity the burn leaves, not the deeper Z the
/// unworn electrode is programmed to. Unchanged when not compensated.
///
/// # Errors
/// The op's heights do not resolve (the work top is the wear's datum).
pub fn as_burned(
    world: &World,
    setup: Option<&str>,
    src: &SrcRef,
    op: &SinkerBurnOp,
    gap: f64,
    moves: Vec<Move>,
) -> Result<Vec<Move>, String> {
    let Some(c) = wear_comp(op, condition_wear(world, setup, op)) else {
        return Ok(moves);
    };
    let params = OpParams::SinkerBurn(op.clone());
    let hs = super::heights::resolve_params(world, setup, Some(src), &params)?;
    let top = hs.top.ok_or_else(|| "sinker burn: the work top does not resolve".to_owned())?;
    Ok(moves
        .into_iter()
        .map(|mut m| {
            // Both ends: the box atom sweeps `from` -> `to`, so an
            // orbit's level move (#544) burns at its unmapped start Z
            // unless the start is read as the worn electrode too.
            m.from.z = worn_tip(m.from.z, top, gap, c.rate);
            m.to.z = worn_tip(m.to.z, top, gap, c.rate);
            m
        })
        .collect())
}
