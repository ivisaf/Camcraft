//! SetToolCutter (#93 slice 2): set or clear a milling tool's drawn
//! cutter, neck and shank outline as one undoable command. The outline
//! is validated against the tool's numbers at construction
//! (`cutter_profile::check`) and refused by name; `apply`/`invert` only
//! land the snapshot. Our own command model, the snapshot-and-restore
//! inverse every sibling uses.

use flecs_ecs::prelude::World;

use camcraft_tools::cutter_profile;
use camcraft_tools::spun::{SpunPoint, SpunProfile};

use crate::commands::{Command, CommandError};
use crate::model;
use crate::tool_cutter;

/// Set (`Some`) or clear (`None`, back to the outline the numbers draw)
/// a tool's drawn cutter.
#[derive(Debug)]
pub struct SetToolCutter {
    name: String,
    prev: Option<SpunProfile>,
    next: Option<SpunProfile>,
}

impl SetToolCutter {
    /// Validate `points` against tool `name` now. Refused by name: no
    /// such tool, not a tool, a turning tool, and every
    /// `cutter_profile::check` refusal ("tool 'T1': cutter profile: ...").
    pub fn new(world: &World, name: &str, points: Option<Vec<SpunPoint>>) -> Result<Self, CommandError> {
        let Some(entity) = model::find_by_name(world, name) else {
            return Err(CommandError::NotFound(name.to_string()));
        };
        let tool = model::tool_of(world, &entity)
            .ok_or_else(|| CommandError::Invalid(format!("'{name}' is not a tool")))?;
        let next = match points {
            None => None,
            Some(p) => Some(
                cutter_profile::check(&tool, p)
                    .map_err(|e| CommandError::Invalid(format!("tool '{name}': cutter profile: {e}")))?,
            ),
        };
        Ok(SetToolCutter { name: name.to_string(), prev: tool_cutter::cutter_of(world, &entity), next })
    }

    /// The outline this command lands (`None`: a clear).
    #[must_use]
    pub fn next(&self) -> Option<&SpunProfile> {
        self.next.as_ref()
    }
}

impl Command for SetToolCutter {
    fn apply(&self, world: &World) {
        if let Some(entity) = model::find_by_name(world, &self.name) {
            tool_cutter::set_cutter(world, &entity, self.next.as_ref());
        }
    }

    fn invert(&self, world: &World) {
        if let Some(entity) = model::find_by_name(world, &self.name) {
            tool_cutter::set_cutter(world, &entity, self.prev.as_ref());
        }
    }
}
