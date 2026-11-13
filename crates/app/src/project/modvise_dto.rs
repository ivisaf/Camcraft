//! A mod-vise jaw block as files hold it (#820): one optional `mod_jaw`
//! key on the fixture marker, the block's parameters (its place on the
//! plate rides `on_plate`, #819). Absent - every other fixture and every
//! older file - reads as none, and an older build ignores the key and
//! keeps the saved facets, so no version bump. Read leniently (#812, the
//! #819 drop-and-log way): kept as raw JSON, parsed on apply, and a key
//! this build can't read or one that breaks the block law is dropped and
//! logged - the fixture and its saved steel stay - never a failed load.
//! Its drops reach the user's load notes (#826), like #812's keys.
//!
//! Provenance: our own wire shape over `modvise::JawParams`'s serde.

use serde_json::Value;

use crate::modvise::JawParams;

/// The `mod_jaw` key of a block.
#[must_use]
pub fn value(jaw: Option<&JawParams>) -> Option<Value> {
    jaw.and_then(|j| serde_json::to_value(j).ok())
}

/// The block parameters of fixture `model` from its key, or none (logged).
#[must_use]
pub fn params_from(v: Option<&Value>, model: &str, notes: &mut Vec<String>) -> Option<JawParams> {
    let v = v?;
    match serde_json::from_value::<JawParams>(v.clone()) {
        Ok(p) => match p.check() {
            Ok(()) => Some(p),
            Err(why) => {
                tracing::warn!(model, "mod vise jaw block parameters dropped on load, kept as plain steel: {why}");
                notes.push(format!("fixture '{model}': mod vise jaw block parameters dropped on load, kept as plain steel: {why}"));
                None
            }
        },
        Err(why) => {
            tracing::warn!(model, "mod vise jaw block key not understood, dropped on load: {why}");
            notes.push(format!("fixture '{model}': mod_jaw key not understood, dropped on load, kept as plain steel: {why}"));
            None
        }
    }
}
