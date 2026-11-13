//! The MCP mod-vise tool (#820): place a modular vise - a fixed and an
//! adjustable jaw block - on a fixture plate's grid, fitted to the
//! setup's stock. It is the very undoable command the Fixtures page
//! queues (`PlaceModVisePair`), one History step, so `undo` takes both
//! blocks back.
//!
//! Provenance: our own tool surface over this repo's mod-vise command.

use serde_json::{Map, Value, json};

use super::setup_tools::{cmd_err, loc_view, obj};
use super::tools::{Tool, req_str};
use crate::commands::PlaceModVisePair;
use crate::modvise::{self, JawInsert, JawParams};
use crate::script::{Env, fnum};

pub(super) const TOOLS: &[Tool] = &[Tool {
    name: "add_mod_vise_pair",
    description: "Place a modular plate-mounted vise on a fixture plate: two jaw blocks, each bolted to grid holes. The FIXED block goes on `hole` (its anchor bolt), turned `yaw` (0, 90, 180 or 270 degrees; at 0 its jaw faces the plate's +Y). The ADJUSTABLE block, turned half a turn, goes on the same grid line at the hole nearest the setup's stock that leaves it clear, then slides along its slots until its jaw touches the stock. Refused by name: stock out of the slots' reach, a block off the plate, a plate with no grid, no stock, stock in the fixed jaw or beside the jaws. Generic preset sizes with optional overrides (mm); both blocks ride the plate. One undoable step.",
    schema: s_add,
    run: add_mod_vise_pair,
}];

const DIMS: [&str; 9] =
    ["body_length", "body_width", "body_height", "jaw_width", "jaw_height", "jaw_thickness", "face_offset", "bolt_diameter", "slot_travel"];

fn points(v: &Value, k: &str) -> Result<Vec<[f64; 2]>, String> {
    let bad = || format!("\"{k}\" must be a list of [x, y] mm positions");
    v.as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|p| match p.as_array().map(Vec::as_slice) {
            Some([x, y]) => Ok([x.as_f64().ok_or_else(bad)?, y.as_f64().ok_or_else(bad)?]),
            _ => Err(bad()),
        })
        .collect()
}

/// `p` with the overrides of object `key` in `a` applied.
fn with_overrides(mut p: JawParams, a: &Map<String, Value>, key: &str) -> Result<JawParams, String> {
    let Some(o) = a.get(key) else { return Ok(p) };
    let o = o.as_object().ok_or_else(|| format!("\"{key}\" must be an object of overrides"))?;
    for (k, v) in o {
        match k.as_str() {
            "bolts" => p.bolts = points(v, k)?,
            "pins" => p.pins = points(v, k)?,
            _ => {
                let slot = match k.as_str() {
                    "body_length" => &mut p.body_length,
                    "body_width" => &mut p.body_width,
                    "body_height" => &mut p.body_height,
                    "jaw_width" => &mut p.jaw_width,
                    "jaw_height" => &mut p.jaw_height,
                    "jaw_thickness" => &mut p.jaw_thickness,
                    "face_offset" => &mut p.face_offset,
                    "bolt_diameter" => &mut p.bolt_diameter,
                    "slot_travel" => &mut p.slot_travel,
                    other => return Err(format!("unknown jaw block figure \"{other}\" (one of {}, bolts, pins)", DIMS.join(", "))),
                };
                *slot = v.as_f64().ok_or_else(|| format!("\"{k}\" must be a number of mm"))?;
            }
        }
    }
    Ok(p)
}

fn view(p: &JawParams) -> Value {
    json!({
        "size": p.size, "side": p.side.word(), "insert": if p.insert == JawInsert::Soft { "soft" } else { "hard" },
        "body_length": fnum(p.body_length), "body_width": fnum(p.body_width), "body_height": fnum(p.body_height),
        "jaw_width": fnum(p.jaw_width), "jaw_height": fnum(p.jaw_height), "jaw_thickness": fnum(p.jaw_thickness),
        "face_offset": fnum(p.face_offset), "slot_travel": fnum(p.slot_travel), "slide": fnum(p.slide),
        "bolts": p.bolts, "pins": p.pins,
    })
}

fn add_mod_vise_pair(env: &mut Env, a: &Map<String, Value>) -> Result<Value, String> {
    let plate = req_str(a, "plate")?;
    let hole = req_str(a, "hole")?;
    let label = req_str(a, "size")?;
    let row = modvise::preset(label).ok_or_else(|| {
        let all: Vec<&str> = modvise::PRESETS.iter().map(|s| s.label).collect();
        format!("no mod vise preset '{label}' (one of {})", all.join(", "))
    })?;
    let yaw = match a.get("yaw") {
        None | Some(Value::Null) => 0.0,
        Some(v) => v.as_f64().ok_or("\"yaw\" must be degrees")?,
    };
    let insert = match a.get("insert").and_then(Value::as_str) {
        None | Some("hard") => JawInsert::Hard,
        Some("soft") => JawInsert::Soft,
        Some(other) => return Err(format!("\"insert\" is hard or soft (got \"{other}\")")),
    };
    let mut fixed = with_overrides(row.fixed(), a, "fixed")?;
    let mut adjustable = with_overrides(row.adjustable(), a, "adjustable")?;
    fixed.insert = insert;
    adjustable.insert = insert;
    let cmd = PlaceModVisePair::new(&env.world, plate, &fixed, &adjustable, hole, yaw).map_err(cmd_err)?;
    let (f, j) = cmd.names();
    let (f, j) = (f.to_owned(), j.to_owned());
    let fit = cmd.fit().clone();
    let (lf, la) = cmd.locations();
    env.history.execute(&env.world, cmd);
    let adj_view = view(&adjustable.with_slide(fit.slide));
    Ok(json!({
        "plate": plate,
        "fixed": { "fixture": f, "at": fit.fixed_at, "yaw": fnum(fit.yaw_deg), "location": loc_view(&lf), "block": view(&fixed) },
        "adjustable": { "fixture": j, "at": fit.adjustable_at, "yaw": fnum(fit.yaw_deg + 180.0), "location": loc_view(&la), "block": adj_view },
        "slide": fnum(fit.slide),
        "opening": fnum(fit.opening),
        "fixed_gap": fnum(fit.fixed_gap),
    }))
}

fn s_add() -> Value {
    let sizes: Vec<&str> = modvise::PRESETS.iter().map(|s| s.label).collect();
    let block = |what: &str| {
        let mut props: Map<String, Value> = DIMS.iter().map(|d| ((*d).to_owned(), json!({ "type": "number", "description": "mm" }))).collect();
        let pts = json!({ "type": "array", "items": { "type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2 },
            "description": "[x, y] mm in the block frame (origin = the anchor bolt, +Y toward the jaw face); one bolt is [0, 0]." });
        props.insert("bolts".into(), pts.clone());
        props.insert("pins".into(), pts);
        json!({ "type": "object", "properties": props, "additionalProperties": false, "description": format!("Override the {what} block's figures (mm).") })
    };
    obj(
        json!({
            "plate": { "type": "string", "description": "The fixture plate's name (a built-in plate or one with a detected grid)." },
            "hole": { "type": "string", "description": "The fixed block's anchor hole, e.g. C3." },
            "size": { "type": "string", "enum": sizes, "description": "The preset pair (generic figures for a grid)." },
            "yaw": { "type": "number", "description": "The fixed block's turn about the plate's Z: 0, 90, 180 or 270 degrees (default 0: its jaw faces +Y)." },
            "insert": { "type": "string", "enum": ["hard", "soft"], "description": "The jaw inserts (the same slab to the collision check)." },
            "fixed": block("fixed"),
            "adjustable": block("adjustable")
        }),
        &["plate", "hole", "size"],
    )
}
