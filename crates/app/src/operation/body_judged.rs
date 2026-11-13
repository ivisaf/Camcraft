//! Which shape of a modelled tool body each law judges (#835). The
//! verdict laws - reach (c), the mid-turn sweep (c3), the solid (b2),
//! containment and rotary laws, and the fixture laws - read the body's
//! band stack, the shape the simulation draws. The laws that read one
//! cylinder by construction keep the cylinder enclosing the body and say
//! so by name: the drill-reach and cross-drill refusals, and the touch
//! probe's stylus reach (its stem and holder are one band). The stepped
//! reading of a solid of revolution is machine-tool handbook practice;
//! our own design, no vendor source consulted.

use camcraft_tools::holder_profile::HolderProfile;

use crate::machining2d::GenError;
use crate::tools::Tool;

/// The words every kept-cylinder law adds for a tool with a body.
#[must_use]
pub fn enclosure_words(tool: &Tool) -> String {
    format!(
        "the tool body is judged here by the cylinder enclosing it, Ø{:.3} mm to {:.3} mm above the tip",
        tool.holder_d,
        tool.flute_len + tool.shank_len + tool.holder_len
    )
}

/// A generator refusal that names the holder, for a tool carrying a
/// body (`tool` is the judged record, its holder the enclosing
/// cylinder): the refusal says the body was judged by that cylinder.
/// Every other refusal, and every refusal of a tool with no body, is
/// returned unchanged.
#[must_use]
pub fn name_enclosure(e: GenError, tool: &Tool, body: Option<&HolderProfile>) -> GenError {
    if body.is_none() {
        return e;
    }
    let say = |m: String| if m.contains("holder") { format!("{m} ({})", enclosure_words(tool)) } else { m };
    match e {
        GenError::Invalid(m) => GenError::Invalid(say(m)),
        GenError::At { msg, at } => GenError::At { msg: say(msg), at },
        other => other,
    }
}

/// The touch probe's note for a tool with a body: its stylus reach
/// reads stem and holder as one enclosing band.
#[must_use]
pub fn probe_note(tool: &Tool, body: Option<&HolderProfile>) -> Option<String> {
    body.map(|_| format!("probe reach: {}", enclosure_words(tool)))
}
