//! The MCP drawn-cutter tools (#93 slice 2): read a milling tool's
//! cutter, neck and shank outline (drawn, or the one its numbers draw)
//! and set or clear the drawn one - the very `SetToolCutter` the Tool
//! editor's Profile tab queues, one History step, so `undo` takes it back.
//!
//! Provenance: our own tool surface over this repo's tool commands.

use super::setup_tools::{cmd_err, obj};
use super::tools::{Tool, req_str};
use crate::commands::SetToolCutter;
use crate::model;
use crate::script::{Env, fnum};
use crate::spun::{SpunPoint, SpunProfile};
use crate::tool_cutter;

use serde_json::{Map, Value, json};

pub(super) const TOOLS: &[Tool] = &[
    Tool { name: "get_tool_cutter_profile", description: "A milling tool's cutter, neck and shank as a spun half-outline: points [z, r, arc] from the tip (z = 0, on the axis) up to the shank top, arc = 0 for a straight segment else the signed radius of an arc to the next point (positive bulges away from the axis). \"drawn\" says whether it is a hand-drawn outline or the one the tool's numbers draw. Turning tools have none (they are not spun profiles).", schema: s_get, run: get_cutter },
    Tool { name: "set_tool_cutter_profile", description: "Set a milling tool's drawn cutter outline (points [z, r, arc] tip-up, closed to the axis at both ends), or clear it with points null (back to the outline its numbers draw). It must agree with the tool's numbers: start at the tip, reach exactly the stated diameter within the flute length, end at flute + shank length, never be wider than the cutter or shank above the flutes, not fold over itself - else refused by name. Display only: toolpaths and checks keep reading the numbers. One undoable step.", schema: s_set, run: set_cutter },
];

fn tool_of(env: &Env, name: &str) -> Result<(crate::tools::Tool, Option<SpunProfile>), String> {
    let e = model::find_by_name(&env.world, name).ok_or_else(|| format!("no tool named '{name}'"))?;
    let t = model::tool_of(&env.world, &e).ok_or_else(|| format!("'{name}' is not a tool"))?;
    Ok((t, tool_cutter::cutter_of(&env.world, &e)))
}

fn view(name: &str, tool: &crate::tools::Tool, drawn: Option<&SpunProfile>) -> Value {
    let shown = tool_cutter::shown(tool, drawn);
    let points: Option<Vec<Value>> =
        shown.as_ref().map(|s| s.points().iter().map(|p| json!([fnum(p.z), fnum(p.r), fnum(p.arc)])).collect());
    json!({ "tool": name, "drawn": drawn.is_some(), "points": points })
}

fn get_cutter(env: &mut Env, a: &Map<String, Value>) -> Result<Value, String> {
    let name = req_str(a, "tool")?;
    let (t, drawn) = tool_of(env, name)?;
    Ok(view(name, &t, drawn.as_ref()))
}

fn set_cutter(env: &mut Env, a: &Map<String, Value>) -> Result<Value, String> {
    let name = req_str(a, "tool")?;
    let points = match a.get("points") {
        None => return Err("\"points\" is required (null clears the drawn outline)".to_owned()),
        Some(Value::Null) => None,
        Some(v) => Some(read_points(v)?),
    };
    let cmd = SetToolCutter::new(&env.world, name, points).map_err(cmd_err)?;
    env.history.execute(&env.world, cmd);
    let (t, drawn) = tool_of(env, name)?;
    Ok(view(name, &t, drawn.as_ref()))
}

/// `[[z, r, arc?], ...]` - arc defaults to 0 (a straight segment).
fn read_points(v: &Value) -> Result<Vec<SpunPoint>, String> {
    let rows = v.as_array().ok_or("\"points\" must be an array of [z, r, arc] rows")?;
    if rows.len() > crate::spun::MAX_POINTS {
        return Err(format!("\"points\" holds more than {} rows", crate::spun::MAX_POINTS));
    }
    rows.iter()
        .enumerate()
        .map(|(i, row)| {
            let n: Option<Vec<f64>> = row.as_array().map(|r| r.iter().filter_map(Value::as_f64).collect());
            match n.as_deref() {
                Some([z, r]) if row.as_array().is_some_and(|a| a.len() == 2) => Ok(SpunPoint::line(*z, *r)),
                Some([z, r, arc]) if row.as_array().is_some_and(|a| a.len() == 3) => Ok(SpunPoint { z: *z, r: *r, arc: *arc }),
                _ => Err(format!("point {} must be [z, r] or [z, r, arc] numbers", i + 1)),
            }
        })
        .collect()
}

fn s_get() -> Value {
    obj(json!({ "tool": { "type": "string", "description": "The document tool's name." } }), &["tool"])
}

fn s_set() -> Value {
    obj(
        json!({
            "tool": { "type": "string", "description": "The document tool's name (a milling tool)." },
            "points": { "type": ["array", "null"], "items": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 3 },
                "description": "The outline tip-up, mm: [z, r] or [z, r, arc] per point; null clears the drawn outline." }
        }),
        &["tool", "points"],
    )
}

