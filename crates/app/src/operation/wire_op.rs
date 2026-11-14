//! The Wire Contour operation's seed, command door and generation
//! (#532, part of #290): the op's condition id read against the
//! machine its setup mounts (#531's door - the wire diameter and the
//! spark gap have no other source), the picked profile or the model's
//! through-silhouette as the geometry, and `wire_edm::wire_contour`
//! run over it.
//!
//! Provenance: our own glue over the motion crate's wire generator
//! (whose module head cites the public sources) and the machine file's
//! condition table (#531); the umbrella's decisions D2, D4 and D8.

use flecs_ecs::prelude::World;

use crate::cl::Toolpath;
use crate::geo2d::{Loop, Point2, Region};
use crate::machine::{ConditionField, Kind};
use crate::machining2d::GenError;
use crate::verify::Warning;
use crate::wire_edm::{clearance_door, wire_burnout, wire_contour_tapered, TabParams, WireParams, WireTaper};

use super::kinds::Carried;
use super::types::{OpParams, WireContourOp, WireSide};
use super::ResolvedHeights;

/// The wire draft: the carried workpiece planes, the die side, and
/// neither a condition nor a start hole - both are the user's to state
/// (a draft that guessed either would be an inference).
pub(super) fn seed_wire_contour(c: &Carried) -> OpParams {
    OpParams::WireContour(WireContourOp {
        top_z: c.top,
        bottom_z: c.bottom,
        condition: String::new(),
        side: WireSide::Die,
        start: None,
        tabs: None,
        capture: false,
        taper_deg: 0.0,
        burnout: None,
        clearance: super::types::DEFAULT_CLEARANCE,
    })
}

/// What the machine's condition states for this cut.
#[derive(Clone, Debug, PartialEq)]
pub struct WireCondition {
    pub id: String,
    /// The wire's diameter, mm.
    pub wire_diameter: f64,
    /// The spark gap per side, mm.
    pub gap: f64,
    /// The machine's name and its stated maximum taper (#536), for the
    /// taper's door.
    pub machine: String,
    pub max_taper_deg: Option<f64>,
}

/// The condition door (#531, #532): the op's condition on the machine
/// its setup mounts, with the wire diameter and the gap both stated -
/// or the refusal that names what is missing. Called at command
/// creation, so a missing wire diameter, gap or condition never
/// reaches a generator.
///
/// # Errors
/// No wire EDM mounted, no condition chosen, no such condition on the
/// machine, or a condition that states no gap or no wire diameter.
pub fn condition_of(world: &World, setup: Option<&str>, op: &WireContourOp) -> Result<WireCondition, String> {
    let (_, mounted) = crate::machine::of_setup(world, setup);
    let Some(machine) = mounted.filter(|m| crate::machine::kind(Some(m)) == Kind::WireEdm) else {
        return Err(
            "a Wire Contour needs a wire EDM machine mounted on its setup - the machine's EDM conditions state the wire diameter and the spark gap".into(),
        );
    };
    let stated = || match machine.edm.as_ref().map(|e| e.conditions.as_slice()) {
        None | Some([]) => "it states none - add edm.conditions to the machine file".to_owned(),
        Some(cs) => format!("it states {}", cs.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", ")),
    };
    if op.condition.trim().is_empty() {
        return Err(format!(
            "a Wire Contour names no EDM condition - choose one of machine {}'s conditions ({})",
            machine.model,
            stated()
        ));
    }
    let Some(edm) = machine.edm.as_ref() else {
        return Err(format!("machine {} states no EDM condition {:?} ({})", machine.model, op.condition, stated()));
    };
    let known = edm.conditions.iter().any(|c| c.id == op.condition);
    let c = edm
        .condition(&machine.model, &op.condition, &[ConditionField::Gap, ConditionField::WireDiameter])
        .map_err(|e| if known { e } else { format!("{e} ({})", stated()) })?;
    // The door above promised both; a `None` here is its law broken,
    // named rather than read as zero.
    let (Some(gap), Some(wire_diameter)) = (c.gap, c.wire_diameter) else {
        return Err(format!("machine {}: EDM condition {:?} states no gap or no wire diameter", machine.model, c.id));
    };
    let cond = WireCondition { id: c.id.clone(), wire_diameter, gap, machine: machine.model.clone(), max_taper_deg: edm.max_taper_deg };
    // The burnout's clearance against the condition's kerf (#540).
    if let Some(clear) = op.burnout {
        clearance_door(clear, cond.wire_diameter * 0.5 + cond.gap)?;
    }
    Ok(cond)
}

/// The condition a wire EDM kind's generation cuts with, read where
/// `collect_job` reads every other kind's tool: `None` for a kind that
/// mounts a library tool.
///
/// # Errors
/// [`condition_of`]'s refusals, as a generation refusal.
pub(super) fn job_wire(world: &World, op: &crate::schema::Operation) -> Result<Option<(WireCondition, Option<WireCondition>)>, GenError> {
    match &op.params {
        OpParams::WireContour(w) => {
            let setup = op.setup.as_deref();
            let main = condition_of(world, setup, w).map_err(GenError::Invalid)?;
            let cutoff = cutoff_of(world, setup, w).map_err(GenError::Invalid)?;
            Ok(Some((main, cutoff)))
        }
        _ => Ok(None),
    }
}

/// The tab cut-off's condition (#539): `None` when tabs are off or no
/// cut-off condition is named (the generator refuses the latter by
/// name); a named one the machine does not state is refused here.
///
/// # Errors
/// The condition lookup's refusal, by name.
pub fn cutoff_of(world: &World, setup: Option<&str>, op: &WireContourOp) -> Result<Option<WireCondition>, String> {
    let Some(t) = op.tabs.as_ref().filter(|t| !t.cutoff_condition.trim().is_empty()) else {
        return Ok(None);
    };
    let probe = WireContourOp { condition: t.cutoff_condition.clone(), ..op.clone() };
    condition_of(world, setup, &probe).map(Some).map_err(|e| format!("the tab cut-off's condition: {e}"))
}

/// What stands in the job's tool slot for a wire cut: a spindle-less
/// cylinder of the wire's diameter (the torch's shape - the one kind
/// no cutting law, spindle word or stock engine reads as a cutter).
/// No library entity backs it and the wire arm never reads it; it only
/// keeps the job's shared head (the carried-stock note's radius) whole.
pub(super) fn job_tool(cond: &WireCondition) -> crate::tools::Tool {
    crate::tools::torch_tool(cond.wire_diameter)
}

/// The condition's share of the operation's stamp (#532): its id, wire
/// diameter and gap as the machine states them now - or one byte for
/// "not stated", which no generated path's stamp carries.
pub(super) fn stamp_condition(world: &World, setup: Option<&str>, op: &WireContourOp, h: &mut std::hash::DefaultHasher) {
    use std::hash::Hasher;
    h.write(b"wire-condition");
    match condition_of(world, setup, op) {
        Ok(c) => {
            h.write_u8(1);
            h.write(c.id.as_bytes());
            h.write_u8(0xff);
            h.write(&c.wire_diameter.to_bits().to_le_bytes());
            h.write(&c.gap.to_bits().to_le_bytes());
            // #536: a taper also reads the machine's maximum; a vertical
            // cut's stamp stays what it was.
            if op.taper_deg != 0.0 {
                h.write(&c.max_taper_deg.map_or(u64::MAX, f64::to_bits).to_le_bytes());
            }
        }
        Err(_) => h.write_u8(0),
    }
    // #539: the cut-off's condition, only when tabs are on.
    if op.tabs.is_some() {
        h.write(b"wire-cutoff");
        match cutoff_of(world, setup, op) {
            Ok(Some(c)) => {
                h.write_u8(1);
                h.write(&c.wire_diameter.to_bits().to_le_bytes());
                h.write(&c.gap.to_bits().to_le_bytes());
            }
            Ok(None) | Err(_) => h.write_u8(0),
        }
    }
}

/// The condition ids the setup's wire EDM states, in the file's order -
/// what a page offers to choose from. Empty when no wire EDM is mounted
/// or its table is empty; whether a condition is complete is
/// [`condition_of`]'s question, asked when one is chosen.
#[must_use]
pub fn conditions_of(world: &World, setup: Option<&str>) -> Vec<String> {
    crate::machine::of_setup(world, setup)
        .1
        .filter(|m| crate::machine::kind(Some(m)) == Kind::WireEdm)
        .and_then(|m| m.edm)
        .map(|e| e.conditions.into_iter().map(|c| c.id).collect())
        .unwrap_or_default()
}

/// The generator's numbers from the op's, the condition's and the
/// world's. `stock` is the setup's stock outline in XY - the edge an
/// open run is entered from (#1109).
///
/// # Errors
/// A workpiece top or bottom that does not resolve, or a bottom that
/// does not stand below the top.
pub fn wire_params(
    op: &WireContourOp,
    cond: &WireCondition,
    cutoff: Option<&WireCondition>,
    hs: &ResolvedHeights,
    stock: Option<Region>,
) -> Result<WireParams, GenError> {
    let (Some(top), Some(bottom)) = (hs.top, hs.bottom) else {
        return Err(GenError::Invalid("wire contour: the workpiece's top and bottom must both resolve".into()));
    };
    if !(top.is_finite() && bottom.is_finite()) || bottom >= top {
        return Err(GenError::Invalid(format!(
            "wire contour: the workpiece's bottom ({bottom:.3}) does not stand below its top ({top:.3})"
        )));
    }
    Ok(WireParams {
        wire_diameter: cond.wire_diameter,
        gap: cond.gap,
        condition: cond.id.clone(),
        side: op.side,
        start: op.start.map(|[x, y]| Point2::new(x, y)),
        plane_z: bottom,
        tabs: op.tabs.as_ref().map(|t| TabParams {
            count: t.count,
            width: t.width,
            at: t.at.iter().map(|[x, y]| Point2::new(*x, *y)).collect(),
            cutoff: cutoff.map(|c| (c.id.clone(), c.wire_diameter * 0.5 + c.gap)),
        }),
        stock,
        clearance: op.clearance,
    })
}

/// Tabs with a taper (#539 x #536), refused by name at the command
/// door and at generation: the tapered builder reads the top-plane path
/// as one closed ring, and a tabbed path is not one. `None` when the
/// op states no tabs or no taper.
#[must_use]
pub fn tabs_taper_refusal(op: &WireContourOp) -> Option<String> {
    (op.tabs.is_some() && op.taper_deg != 0.0).then(|| {
        format!(
            "wire contour: tabs can't be combined with a taper yet (a {:.3} degree taper is asked) - remove the tabs or set the taper to 0",
            op.taper_deg
        )
    })
}

/// Generate: `picks` is the picked profile (closed rings, open runs)
/// when the source is a selection; otherwise `base` - the model's
/// through-silhouette - is the profile. `stock` is read only for an
/// open run (#1109). No verification runs yet (the
/// wire sweep against the part is #534), and the path's notes say so.
pub(super) fn generate_wire(
    op: &WireContourOp,
    cond: Result<(WireCondition, Option<WireCondition>), GenError>,
    hs: &ResolvedHeights,
    picks: Option<(Vec<Loop>, Vec<Vec<Point2>>)>,
    base: impl FnOnce() -> Result<Region, GenError>,
    stock: impl FnOnce() -> Option<Region>,
) -> Result<(Toolpath, Vec<Warning>), GenError> {
    let (cond, cutoff) = cond?;
    let (loops, chains) = match picks {
        Some(geom) => geom,
        None => (base()?.loops().to_vec(), Vec::new()),
    };
    let stock = if chains.is_empty() { None } else { stock() };
    let p = wire_params(op, &cond, cutoff.as_ref(), hs, stock)?;
    let taper = WireTaper {
        angle_deg: op.taper_deg,
        top_z: hs.top.unwrap_or(p.plane_z),
        max_deg: cond.max_taper_deg,
        machine: cond.machine.clone(),
    };
    let path = match op.burnout {
        // #536 x #540: the burnout's rings are cut vertical; a taper
        // with them is refused by name, never cut upright silently.
        Some(_) if op.taper_deg != 0.0 => {
            return Err(GenError::Invalid(format!(
                "wire contour: a {:.3} degree taper is asked with a no-core burnout - the burnout is cut vertical only; set the taper to 0 or turn the burnout off",
                op.taper_deg
            )))
        }
        Some(clearance) => wire_burnout(&loops, &chains, &p, clearance)?,
        // #539 x #536: the tapered builder reads one closed ring; a
        // tabbed path is not one, so the pair is refused by name.
        None => {
            if let Some(why) = tabs_taper_refusal(op) {
                return Err(GenError::Invalid(why));
            }
            wire_contour_tapered(&loops, &chains, &p, &taper)?
        }
    };
    Ok((path, Vec::new()))
}
