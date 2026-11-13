//! Fixture plates as files hold them (#819): two optional keys on the
//! fixture marker - `plate` (the fixture IS a plate: its grid, and a
//! built-in plate's figures) and `on_plate` (it stands on one: the
//! plate's name, the location, the turn). Absent - every older file -
//! reads as neither, and an older build ignores both keys, so no version
//! bump. Read leniently (#812): a key this build can't parse, or one
//! that breaks the plate law, is dropped and logged - the fixture and
//! its saved steel stay - never a failed load.
//!
//! Provenance: our own wire shape over `plate`'s serde types.

use serde_json::Value;

use crate::plate::{PlateDef, PlateLink, PlateMount};

/// The `plate` key of `link`.
#[must_use]
pub fn plate_value(link: &PlateLink) -> Option<Value> {
    link.plate.as_ref().and_then(|p| serde_json::to_value(p).ok())
}

/// The `on_plate` key of `link`.
#[must_use]
pub fn on_plate_value(link: &PlateLink) -> Option<Value> {
    link.mount.as_ref().and_then(|m| serde_json::to_value(m).ok())
}

/// The plate roles of fixture `model` from its two keys, each dropped
/// (and logged) on its own when it does not read or breaks the law.
#[must_use]
pub fn link_from(plate: Option<&Value>, on_plate: Option<&Value>, model: &str, notes: &mut Vec<String>) -> PlateLink {
    let plate = plate.and_then(|v| match serde_json::from_value::<PlateDef>(v.clone()) {
        Ok(p) => match p.check() {
            Ok(()) => Some(p),
            Err(why) => {
                tracing::warn!(model, "fixture plate grid dropped on load, kept as plain steel: {why}");
            notes.push(format!("fixture '{model}': plate grid dropped on load, kept as plain steel: {why}"));
                None
            }
        },
        Err(why) => {
            tracing::warn!(model, "fixture plate key not understood, dropped on load: {why}");
            notes.push(format!("fixture '{model}': plate key not understood, dropped on load: {why}"));
            None
        }
    });
    let mount = on_plate.and_then(|v| match serde_json::from_value::<PlateMount>(v.clone()) {
        Ok(m) if m.yaw_deg.is_finite() && !m.plate.is_empty() => Some(m),
        Ok(_) => {
            tracing::warn!(model, "fixture plate location dropped on load: no plate or a non-finite turn");
            notes.push(format!("fixture '{model}': plate location dropped on load: no plate or a non-finite turn"));
            None
        }
        Err(why) => {
            tracing::warn!(model, "fixture plate location not understood, dropped on load: {why}");
            notes.push(format!("fixture '{model}': plate location not understood, dropped on load: {why}"));
            None
        }
    });
    PlateLink { plate, mount }
}
