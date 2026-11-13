//! The simulated tool as the collision laws read it (#743 part 1): the
//! cutter and shank from the judged tool record, and above them the very
//! band stack the fixture laws sweep (`tool_body::stack` - the holder
//! cylinder, a modelled body's revolved envelope (#368) or a drawn spun
//! holder's bands (#93)), spun about the tool axis into a display mesh.
//! What is drawn is what is checked: a modelled body's own facets (flats,
//! slots, its turn about the axis) are not what any law reads, so the
//! simulation draws the envelope instead. A solid of revolution from its
//! meridian is elementary geometry; our own design, no vendor file or
//! source consulted.
//!
//! Display only: nothing here writes the world or feeds a verdict. A
//! degenerate outline (non-finite, negative, running back down the axis,
//! no radius, past the body caps) draws nothing and is logged by name,
//! never a crash. The mesh is cached for the one tool at the playhead
//! and dropped when that tool or operation is gone, its outline changes,
//! or the transport closes ([`SimToolCache`]).

use std::sync::Arc;

use flecs_ecs::prelude::World;

use camcraft_tools::holder_profile::{HolderBand, HolderStack, MAX_BODY_HEIGHT, MAX_BODY_RADIUS};

use crate::geometry::Triangle;
use crate::tools::Tool;
use crate::{model, tool_body, tool_body_view};

/// Float dust on the outline checks (mm).
const EPS: f64 = 1e-9;

/// Why an outline draws no body. Each refusal names itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineError {
    NonFinite,
    Negative,
    RunsBack,
    NoRadius,
    TooLarge,
}

impl core::fmt::Display for OutlineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "the tool outline has non-finite numbers",
            Self::Negative => "the tool outline has a negative radius or length",
            Self::RunsBack => "the tool outline runs back down the axis (it would cross itself)",
            Self::NoRadius => "the tool outline has no radius or no length - nothing to draw",
            Self::TooLarge => "the tool outline is larger than any tool a spindle carries",
        })
    }
}

impl std::error::Error for OutlineError {}

/// The tool's judged shape, tool frame (tip at the origin, +Z to the
/// spindle): the cutter-and-shank meridian and the holder bands.
#[derive(Clone, Debug, PartialEq)]
pub struct JudgedOutline {
    /// `(z, r)` from the tip centre up the cutter and the shank, closed
    /// to the axis at the shank's top - `Tool::profile` of the judged
    /// record, whose breakpoints are the bands the shank laws sweep.
    pub cutter: Vec<(f64, f64)>,
    /// What stands above the shank, bottom-up: exactly the bands
    /// `tool_body::stack` hands the fixture laws (empty bands dropped).
    pub holder: Vec<HolderBand>,
}

impl JudgedOutline {
    /// The outline of `tool` (the record as the laws read it,
    /// `tool_body::judged`) under `stack` (`tool_body::stack` of its
    /// geometry). Refuses a degenerate outline by name.
    pub fn new(tool: &Tool, stack: &HolderStack) -> Result<Self, OutlineError> {
        let cutter = Tool { holder_len: 0.0, ..tool.clone() }.profile();
        if !cutter.iter().all(|p| p.0.is_finite() && p.1.is_finite())
            || !stack.bands.iter().all(|b| b.above.is_finite() && b.len.is_finite() && b.r.is_finite())
        {
            return Err(OutlineError::NonFinite);
        }
        if cutter.iter().any(|p| p.0 < -EPS || p.1 < 0.0) || stack.bands.iter().any(|b| b.above < -EPS || b.len < 0.0 || b.r < 0.0) {
            return Err(OutlineError::Negative);
        }
        let holder: Vec<HolderBand> = stack.bands.iter().copied().filter(|b| b.len > EPS && b.r > EPS).collect();
        if cutter.windows(2).any(|w| w[1].0 < w[0].0 - EPS) || holder.windows(2).any(|w| w[1].above < w[0].above + w[0].len - EPS) {
            return Err(OutlineError::RunsBack);
        }
        let out = Self { cutter, holder };
        let (top, wide) = (out.top(), out.max_radius());
        if wide <= EPS || top <= EPS {
            return Err(OutlineError::NoRadius);
        }
        if wide > MAX_BODY_RADIUS || top > MAX_BODY_HEIGHT {
            return Err(OutlineError::TooLarge);
        }
        Ok(out)
    }

    /// The highest point above the tip.
    #[must_use]
    pub fn top(&self) -> f64 {
        let c = self.cutter.iter().fold(0.0_f64, |m, p| m.max(p.0));
        self.holder.iter().fold(c, |m, b| m.max(b.above + b.len))
    }

    /// The farthest point from the axis.
    #[must_use]
    pub fn max_radius(&self) -> f64 {
        let c = self.cutter.iter().fold(0.0_f64, |m, p| m.max(p.1));
        self.holder.iter().fold(c, |m, b| m.max(b.r))
    }

    /// The holder bands as one `(z, r)` meridian: each band a cylinder,
    /// flat steps between them, closed to the axis at the foot, the top
    /// and across any gap.
    #[must_use]
    pub fn holder_meridian(&self) -> Vec<(f64, f64)> {
        let mut m: Vec<(f64, f64)> = Vec::with_capacity(self.holder.len() * 2 + 2);
        for b in &self.holder {
            match m.last().copied() {
                None => m.push((b.above, 0.0)),
                Some((z, _)) if b.above > z + EPS => m.extend([(z, 0.0), (b.above, 0.0)]),
                Some(_) => {}
            }
            m.extend([(b.above, b.r), (b.above + b.len, b.r)]);
        }
        if let Some((z, _)) = m.last().copied() {
            m.push((z, 0.0));
        }
        m
    }

    /// The outline spun about +Z, tool frame.
    #[must_use]
    pub fn mesh(&self) -> Vec<Triangle> {
        let mut tris = tool_body_view::revolve(&self.cutter);
        tris.extend(tool_body_view::revolve(&self.holder_meridian()));
        tris
    }
}

/// The judged outline of the tool operation `op` runs. `Ok(None)` when
/// there is nothing to spin: the operation or its tool is gone, or the
/// tool is a lathe insert kind (its truth lives in the work's meridian;
/// it keeps the axis marker only). `Err` names why a tool that should
/// draw does not - its body no longer fits it, or its outline is
/// degenerate.
pub fn for_operation(world: &World, op: &str) -> Result<Option<JudgedOutline>, String> {
    let Some(e) = model::find_by_name(world, op) else { return Ok(None) };
    let Some(name) = model::operation_of(world, &e).map(|o| o.tool) else { return Ok(None) };
    let Some(te) = model::find_by_name(world, &name) else { return Ok(None) };
    let Some(tool) = model::tool_of(world, &te) else { return Ok(None) };
    if tool_body::LATHE_HELD.contains(&tool.kind) {
        return Ok(None);
    }
    // The same two reads every fixture law makes (`fixture_struck`,
    // `operation::generate`): the judged record, then its stack.
    let (judged, body) = tool_body::judged(world, &te, tool).map_err(|e| format!("tool {name}: {e}"))?;
    let stack = tool_body::stack(&judged.geom(), body.as_ref());
    JudgedOutline::new(&judged, &stack).map(Some).map_err(|e| format!("tool {name}: {e}"))
}

/// The revolved mesh of the one tool at the playhead. Keyed by the
/// outline itself: an edit to the tool or its body that moves a judged
/// number rebuilds it, anything else reuses it. Holds nothing once the
/// tool or operation is gone or refused.
#[derive(Debug, Default)]
pub struct SimToolCache {
    built: Option<(JudgedOutline, Arc<[Triangle]>)>,
    refused: Option<String>,
}

impl SimToolCache {
    /// The tool-frame mesh for `op`'s tool, rebuilt only when its judged
    /// outline changed. `None` (and the cache emptied) when nothing is
    /// to be drawn; a refusal is logged once per distinct reason.
    pub fn tool_frame(&mut self, world: &World, op: &str) -> Option<Arc<[Triangle]>> {
        match for_operation(world, op) {
            Ok(Some(outline)) => {
                self.refused = None;
                if let Some((held, tris)) = &self.built
                    && *held == outline
                {
                    return Some(Arc::clone(tris));
                }
                let tris: Arc<[Triangle]> = Arc::from(outline.mesh());
                self.built = Some((outline, Arc::clone(&tris)));
                Some(tris)
            }
            Ok(None) => {
                self.clear();
                None
            }
            Err(why) => {
                if self.refused.as_deref() != Some(why.as_str()) {
                    tracing::warn!(op, "simulated tool not drawn: {why}");
                }
                self.built = None;
                self.refused = Some(why);
                None
            }
        }
    }

    /// [`Self::tool_frame`] posed at a tool position: `tip` in world,
    /// `axis` the unit tip-to-spindle direction (`sim::Frame::axis`) -
    /// the body follows a tilted axis, not only Z.
    pub fn posed(&mut self, world: &World, op: &str, tip: [f64; 3], axis: [f64; 3]) -> Option<Vec<Triangle>> {
        let tris = self.tool_frame(world, op)?;
        Some(tool_body_view::posed(&tris, tip, axis))
    }

    /// Drop the mesh and any remembered refusal (the transport closed).
    pub fn clear(&mut self) {
        self.built = None;
        self.refused = None;
    }

    /// Does the cache hold no mesh?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.built.is_none()
    }

    /// Why the last tool asked for was not drawn, if it was refused.
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refused.as_deref()
    }
}
