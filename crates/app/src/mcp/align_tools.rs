//! The MCP part-alignment tools: align the part from axis picks - a
//! straight edge, or two points, for +Y and/or +X - through the same
//! undoable `AlignPartToSetup::align_from_axes` the Setups page's "Part
//! axes" rows queue: one axis turns it in the work plane (#557), both
//! define the full 3D frame (#769, #1329); and lay it on a plane through
//! three picked points (#555), through `AlignPartToSetup::lay_on_points`
//! like the "Lay on a plane - pick three points" row; or on one picked flat
//! face (#729), through `AlignPartToSetup::lay_on_plane`. Every refusal is
//! the command's own text, returned as a tool error.
//!
//! Provenance: our own tool surface over this repo's setup alignment
//! command; no vendor material consulted.

use super::pick::{pick_list_schema, picks_of};
use super::setup_tools::{cmd_err, loc_view, obj, setup_or_active};
use super::tools::{Tool, opt_str};
use crate::commands::AlignPartToSetup;
use crate::cl::ArcPlane;
use crate::pick::{AxisPick, InPlaneAxes, InPlaneAxis, PlaneLay, PointsLay, in_plane_turn_deg, plane_side, points_side};
use crate::script::{Env, fnum};
use crate::setup_align;

use serde_json::{Map, Value, json};

pub(super) const TOOLS: &[Tool] = &[
    Tool {
        name: "set_part_axes",
        description: "Align the part from picked axes, as the Setups page's \"Part axes\" rows do. Picks are list_features ids: one edge chain, or two vertices (first to second); flip_y / flip_x reverse a pick. One axis (+Y or +X): the part turns in the setup's work plane (its XY) so the pick, projected into the plane, runs along that axis (turn_deg reports the turn). Both axes: their 3D directions define the full frame (frame \"full_3d\") - primary (default \"y\") lands exactly, the other is made square to it, and +Z lands on the right-hand cross of the picks, so a left-handed pair is honoured literally (the part lands on its head), not refused. The part turns about its bounds' centre, composed onto the setup's current part alignment; the work offset is untouched; one undo step. Refused by name: a face, a curved edge, coincident points, a single pick along Z, X and Y picks running (nearly) along each other.",
        schema: s_set_part_axes,
        run: set_part_axes,
    },
    Tool {
        name: "lay_part_on_points",
        description: "Lay the part on a setup's G17 (XY), G18 (ZX) or G19 (YZ) plane through three picked points (list_features vertex ids). The plane through them is turned so its outside (the side away from the part's material) faces the plane's axis (+Z for G17), or the opposite way with flip (G17 + flip: the points rest on the table, part above), and point 1 is set onto the plane's level (place, default true). frame (3-2-1) also turns the part so point 1 -> 2 runs along +X (+Y for G19) and moves point 1 to the setup origin. Absolute like the face pick; one undo step; the work offset is untouched. Refused by name: not three vertices, points closer than 0.001 mm, (nearly) collinear points (under 1 degree of spread), a pick off the part, no model.",
        schema: s_lay_part_on_points,
        run: lay_part_on_points,
    },
    Tool {
        name: "lay_part_on_face",
        description: "Lay the part on a setup's G17 (XY), G18 (ZX) or G19 (YZ) plane by one picked flat face (a list_features face id, as {model, id}). The face's outward normal is turned to face the plane's axis (+Z for G17), or the opposite way with flip (G17 + flip: the face rests on the table, part above), and the face's centroid is set onto the plane's level (place, default true). Absolute like the face pick on the Setups page; one undo step; the work offset is untouched. Returns the setup, the face's outside, the stored alignment and its description. Refused by name: an edge, a vertex, a curved or degenerate face, more than one face, a pick off the part, no model, and a lay the setup already holds.",
        schema: s_lay_part_on_face,
        run: lay_part_on_face,
    },
];

fn flag(a: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match a.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(format!("\"{key}\" must be true or false, got {other}")),
    }
}

fn axis_of(env: &Env, a: &Map<String, Value>, key: &str, flip: &str) -> Result<Option<AxisPick>, String> {
    match a.get(key) {
        None | Some(Value::Null) => Ok(None),
        v => Ok(Some(AxisPick { picks: picks_of(env, v, key)?, flip: flag(a, flip)? })),
    }
}

fn set_part_axes(env: &mut Env, a: &Map<String, Value>) -> Result<Value, String> {
    let setup = setup_or_active(env, a)?.ok_or("no setup is declared - add_setup first")?;
    let primary = match opt_str(a, "primary")? {
        None | Some("y") => InPlaneAxis::Y,
        Some("x") => InPlaneAxis::X,
        Some(other) => return Err(format!("\"primary\" must be \"y\" or \"x\", got \"{other}\"")),
    };
    let axes = InPlaneAxes { y: axis_of(env, a, "y", "flip_y")?, x: axis_of(env, a, "x", "flip_x")?, primary };
    // Both axes: the full 3D frame (#769) - no in-plane turn angle, and
    // a left-handed pair is honoured, as the GUI does. One axis: the
    // in-plane turn (#557), whose angle is reported.
    let full_3d = axes.y.as_ref().is_some_and(|p| !p.picks.is_empty())
        && axes.x.as_ref().is_some_and(|p| !p.picks.is_empty());
    let turn = if full_3d { None } else { Some(in_plane_turn_deg(&env.world, &setup, &axes)?) };
    let cmd = AlignPartToSetup::align_from_axes(&env.world, &setup, &axes).map_err(cmd_err)?;
    env.history.execute(&env.world, cmd);
    let held = setup_align::stored(&env.world, &setup);
    Ok(json!({
        "setup": setup,
        "frame": if full_3d { "full_3d" } else { "in_plane" },
        "turn_deg": turn.map(fnum),
        "alignment": held.as_ref().map(loc_view),
        "description": setup_align::describe(held),
    }))
}

fn s_set_part_axes() -> Value {
    obj(
        json!({
            "setup": { "type": "string", "description": "A declared setup; defaults to the active one." },
            "y": pick_list_schema("+Y: one straight edge chain, or two vertices (first to second)."),
            "x": pick_list_schema("+X: one straight edge chain, or two vertices (first to second)."),
            "flip_y": { "type": "boolean", "description": "Take the +Y pick the other way." },
            "flip_x": { "type": "boolean", "description": "Take the +X pick the other way." },
            "primary": { "type": "string", "enum": ["y", "x"], "description": "With both axes given, the one landed exactly; the other is made square to it (default y)." }
        }),
        &[],
    )
}

/// The `plane` option shared by the lay tools: g17 (default), g18, g19.
fn plane_of(a: &Map<String, Value>) -> Result<ArcPlane, String> {
    match opt_str(a, "plane")? {
        None | Some("g17") => Ok(ArcPlane::Xy),
        Some("g18") => Ok(ArcPlane::Xz),
        Some("g19") => Ok(ArcPlane::Yz),
        Some(other) => Err(format!("\"plane\" must be \"g17\", \"g18\" or \"g19\", got \"{other}\"")),
    }
}

fn lay_part_on_face(env: &mut Env, a: &Map<String, Value>) -> Result<Value, String> {
    let setup = setup_or_active(env, a)?.ok_or("no setup is declared - add_setup first")?;
    let plane = plane_of(a)?;
    let place = match a.get("place") {
        None | Some(Value::Null) => true,
        _ => flag(a, "place")?,
    };
    // One pick object; an array is passed on so the command refuses more than one by name.
    let face = match a.get("face") {
        Some(v @ Value::Object(_)) => Value::Array(vec![v.clone()]),
        Some(v) => v.clone(),
        None => Value::Null,
    };
    let picks = picks_of(env, Some(&face), "face")?;
    let lay = PlaneLay { plane, flip: flag(a, "flip")?, place };
    let side = plane_side(&env.world, &picks)?;
    let cmd = AlignPartToSetup::lay_on_plane(&env.world, &setup, &picks, lay).map_err(cmd_err)?;
    env.history.execute(&env.world, cmd);
    let held = setup_align::stored(&env.world, &setup);
    Ok(json!({
        "setup": setup,
        "outside": side,
        "alignment": held.as_ref().map(loc_view),
        "description": setup_align::describe(held),
    }))
}

fn s_lay_part_on_face() -> Value {
    obj(
        json!({
            "setup": { "type": "string", "description": "A declared setup; defaults to the active one." },
            "face": {
                "type": "object",
                "description": "One flat face of the part: {\"model\", \"id\"} with a list_features face id.",
                "properties": { "model": { "type": "string" }, "id": { "type": "string" } },
                "required": ["model", "id"]
            },
            "plane": { "type": "string", "enum": ["g17", "g18", "g19"], "description": "The plane to lay on (default g17)." },
            "flip": { "type": "boolean", "description": "Face the outside the opposite way (-Z for G17: rest the face on the table)." },
            "place": { "type": "boolean", "description": "Set the face onto the plane's level (default true); false keeps the part where it is." }
        }),
        &["face"],
    )
}

fn lay_part_on_points(env: &mut Env, a: &Map<String, Value>) -> Result<Value, String> {
    let setup = setup_or_active(env, a)?.ok_or("no setup is declared - add_setup first")?;
    let plane = match opt_str(a, "plane")? {
        None | Some("g17") => ArcPlane::Xy,
        Some("g18") => ArcPlane::Xz,
        Some("g19") => ArcPlane::Yz,
        Some(other) => return Err(format!("\"plane\" must be \"g17\", \"g18\" or \"g19\", got \"{other}\"")),
    };
    let place = match a.get("place") {
        None | Some(Value::Null) => true,
        _ => flag(a, "place")?,
    };
    let picks = picks_of(env, a.get("points"), "points")?;
    let pl = PointsLay { lay: PlaneLay { plane, flip: flag(a, "flip")?, place }, frame: flag(a, "frame")? };
    let side = points_side(&env.world, &picks)?;
    let cmd = AlignPartToSetup::lay_on_points(&env.world, &setup, &picks, pl).map_err(cmd_err)?;
    env.history.execute(&env.world, cmd);
    let held = setup_align::stored(&env.world, &setup);
    Ok(json!({
        "setup": setup,
        "outside": side,
        "alignment": held.as_ref().map(loc_view),
        "description": setup_align::describe(held),
    }))
}

fn s_lay_part_on_points() -> Value {
    obj(
        json!({
            "setup": { "type": "string", "description": "A declared setup; defaults to the active one." },
            "points": pick_list_schema("Exactly three vertices (points 1, 2, 3)."),
            "plane": { "type": "string", "enum": ["g17", "g18", "g19"], "description": "The plane to lay on (default g17)." },
            "flip": { "type": "boolean", "description": "Face the points' outside the opposite way (-Z for G17: rest the points on the table)." },
            "place": { "type": "boolean", "description": "Set point 1 onto the plane's level (default true); false keeps the part where it is." },
            "frame": { "type": "boolean", "description": "3-2-1: point 1 to the origin, point 1 -> 2 along +X (+Y for G19)." }
        }),
        &["points"],
    )
}
