//! The index-swing clause of the program gate (#804): before a 4th axis
//! that turns the work about X indexes, the tool is retracted to the
//! machine's safe Z - and the tool tip there must clear the radius the
//! stock, the parts and the workholding that turns with them sweep about
//! the A centerline. Too short a clearance refuses the post by name.
//!
//! Provenance: our own design (issue #804). The geometry is plain: every
//! point of a vertical tool at height h above a line along X is at least
//! h from it, so the tip is the tool's nearest point to the A axis; the
//! swept radius is the farthest stated vertex from that line. Machine
//! numbers are the `.mch` file's (Tormach 770MX spec page, Tormach 4th-
//! axis comparison sheet 02/24) - no vendor implementation inspected.
//!
//! The laws:
//! - **Who is judged.** Every posted op whose machine states a rotary
//!   that turns the work about X ([`work_a`]): a G30 or a section start
//!   can turn A before any op, so every op's retract is an index pose.
//!   A rotary with no `axis` vector takes its RS-274 letter's line (an
//!   A is about X, #813); one with neither refuses by name - it may be
//!   an A under the tool. Machines without one are untouched.
//! - **Where the tip is.** WCS Y0 Z0 sits on the A centerline (the
//!   indexed post's setup law), so at the safe Z the tip is at program
//!   Z `nose_to_axis - tool length` (cutter, shank and holder: the same
//!   protrusion the magazine law measures). An unstated `nose_to_axis`
//!   refuses: the gate cannot show the turn clears.
//! - **What sweeps.** The farthest vertex, from the line through the
//!   WCS origin along X, of the setup's stock, its parts and its tagged
//!   fixtures that turn with the work (a chuck); fixed steel does not
//!   turn. Over the whole X range: the tool's X at the index is the G30
//!   position, which the program does not know.
//! - **The margin.** [`MARGIN_MM`] between the tip and the swept radius.

use crate::{fixture, model, setup, setup_align};
use flecs_ecs::prelude::World;

/// The clearance the tip must keep over the swept radius, mm (5 mm,
/// about 0.2 in): covers the nominal spec numbers behind `nose_to_axis`,
/// a tool-length measurement and the chuck's centring.
pub const MARGIN_MM: f64 = 5.0;

/// Which line a rotary turns about, as far as the swing clause can
/// tell (#813).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotaryLine {
    /// About X: a stated axis vector along X, or an unstated one on a
    /// rotary named A (RS-274 / ISO 6983: A turns about X - the post's
    /// own reading of the tool axis as A, `a = atan2(-y, z)`).
    AboutX,
    /// Off X: a stated vector off X, or an unstated one on a rotary
    /// named B (about Y) or C (about Z).
    OffX,
    /// No vector and no RS-274 rotary letter: the line is unknown.
    Unknown,
}

/// The line rotary `a` turns about (#813): its stated `axis` vector,
/// else the RS-274 letter's default line, else [`RotaryLine::Unknown`].
#[must_use]
pub fn rotary_line(a: &crate::machine::AxisLimits) -> RotaryLine {
    match a.axis {
        Some(d) if d[0] * d[0] >= 0.99 * (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) => RotaryLine::AboutX,
        Some(_) => RotaryLine::OffX,
        None => match a.name.trim().to_ascii_uppercase().as_str() {
            "A" => RotaryLine::AboutX,
            "B" | "C" => RotaryLine::OffX,
            _ => RotaryLine::Unknown,
        },
    }
}

/// What the clause makes of a machine's work-turning rotaries.
enum WorkA {
    /// A rotary turning the work about X: its letter and stated
    /// `nose_to_axis` (None = unstated).
    Judged(String, Option<f64>),
    /// A rotary turning the work about a line nobody stated (#813):
    /// its name. Refused - it may be an A about X under the tool.
    Unlined(String),
}

/// The machine's work-turning rotary the clause judges; an unknown
/// line wins (it refuses), then the first about X. None when there is
/// none.
fn work_a(m: &crate::machine::Machine) -> Option<WorkA> {
    let turning = || m.axes.iter().filter(|a| a.rotary && a.turns_part == Some(true));
    turning()
        .find(|a| rotary_line(a) == RotaryLine::Unknown)
        .map(|a| WorkA::Unlined(a.name.clone()))
        .or_else(|| {
            turning()
                .find(|a| rotary_line(a) == RotaryLine::AboutX)
                .map(|a| WorkA::Judged(a.name.clone(), a.nose_to_axis))
        })
}

/// The A an op indexes to: the first leaning move's tool axis read as a
/// turn about X (the post's own reading, a = atan2(-y, z)), else 0.
fn index_angle(path: &crate::cl::Toolpath) -> f64 {
    path.moves
        .iter()
        .find_map(|mv| mv.axis.filter(|a| !crate::cl::is_vertical(a)))
        .map_or(0.0, |a| (-a[1]).atan2(a[2]).to_degrees())
}

/// The farthest point of the setup's turning bodies from the A line
/// through `origin` along X, mm (0 with nothing stated).
fn swept_radius(world: &World, declared: Option<&str>, origin: [f64; 3]) -> f64 {
    let mut bodies: Vec<flecs_ecs::prelude::Entity> = Vec::new();
    bodies.extend(setup::stock_entity(world, declared));
    bodies.extend(
        model::model_entities(world)
            .into_iter()
            .filter(|(_, e)| setup_align::is_part(world, e))
            .map(|(_, e)| e),
    );
    bodies.extend(
        fixture::tagged(world, declared)
            .into_iter()
            .filter(|(_, e)| !fixture::def_of(world, e).is_some_and(|d| d.fixed))
            .map(|(_, e)| e),
    );
    let mut r = 0.0_f64;
    for e in bodies {
        for t in setup_align::facets_in(world, &e, declared).unwrap_or_default() {
            for p in [t.a, t.b, t.c] {
                let d = (p.y - origin[1]).hypot(p.z - origin[2]);
                if d.is_finite() {
                    r = r.max(d);
                }
            }
        }
    }
    r
}

/// The clause: one problem per op whose index pose cannot be shown to
/// clear, in program order.
pub(super) fn index_swing_problems(world: &World) -> Vec<String> {
    let Ok(ops) = super::program_ops(world) else {
        return Vec::new(); // the post refuses the program by name first
    };
    let mut out = Vec::new();
    for (name, e) in ops {
        let Some(op) = model::operation_of(world, &e) else { continue };
        let Some(m) = crate::machine::of_setup(world, op.setup.as_deref()).1 else { continue };
        let (letter, nose) = match work_a(&m) {
            None => continue,
            Some(WorkA::Unlined(r)) => {
                out.push(format!(
                    "{name}: {} turns the work on rotary {r} but states no axis vector for it, and {r} is no \
                     RS-274 rotary letter - whether its index swings the work under the tool cannot be judged \
                     (state its \"axis\" in the machine file)",
                    m.model
                ));
                continue;
            }
            Some(WorkA::Judged(letter, nose)) => (letter, nose),
        };
        let a = model::toolpath_of(world, &e).map_or(0.0, |c| index_angle(&c.path));
        let Some(nose) = nose else {
            out.push(format!(
                "{name}: {} turns the work on {letter} but states no nose_to_axis - the tool's clearance \
                 over the part swept by the {letter}{a:.3} index cannot be judged",
                m.model
            ));
            continue;
        };
        // A lost tool is the machine clause's refusal, by name, already.
        let Some(tool_e) = model::find_by_name(world, &op.tool) else { continue };
        let Some(tool) = model::tool_of(world, &tool_e) else { continue };
        let tool = match crate::tool_body::judged(world, &tool_e, tool) {
            Ok((t, _)) => t,
            Err(why) => {
                out.push(format!("{name}: tool {}: {why} - the {letter} index clearance cannot be judged", op.tool));
                continue;
            }
        };
        let len = tool.flute_len + tool.shank_len + tool.holder_len;
        let (_, origin) = super::wcs_context(world, op.setup.as_deref());
        let swept = swept_radius(world, op.setup.as_deref(), origin);
        let tip = nose - len;
        let clearance = tip - swept;
        if !(clearance.is_finite() && clearance >= MARGIN_MM) {
            out.push(format!(
                "{name}: the {letter}{a:.3} index turns the work under the tool - at the safe Z the tip of \
                 {} ({len:.3} mm long) is {tip:.3} mm above the {letter} centerline, the stock, part and \
                 turning fixtures sweep {swept:.3} mm, a clearance of {clearance:.3} mm against the \
                 {MARGIN_MM:.1} mm needed",
                op.tool
            ));
        }
    }
    out
}
