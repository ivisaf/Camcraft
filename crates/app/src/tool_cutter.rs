//! A milling tool's stored drawn cutter (#93 slice 2): the one reader
//! and the one writer of the `ToolCutter` component, and the outline the
//! editor and the side view show - the drawn one while it still agrees
//! with the tool's numbers, else the one the numbers draw. Display only:
//! no generator or judging law reads it (the numbers stay the truth).
//! Our own design over `camcraft_tools::cutter_profile`.

use flecs_ecs::core::EntityViewGet; // try_get is not re-exported in prelude (0.2.2)
use flecs_ecs::prelude::{Entity, World};

use camcraft_tools::cutter_profile;
use camcraft_tools::spun::SpunProfile;
use camcraft_tools::tools::Tool;

use crate::schema::ToolCutter;

/// The drawn cutter on a tool entity, if it has one.
#[must_use]
pub fn cutter_of(world: &World, entity: &Entity) -> Option<SpunProfile> {
    entity.entity_view(world).try_get::<&ToolCutter>(|c| c.spun.clone())
}

/// Land (`Some`) or clear (`None`) a tool's drawn cutter - the one
/// writer, called by `SetToolCutter`, the tool snapshots and the loader.
pub fn set_cutter(world: &World, entity: &Entity, spun: Option<&SpunProfile>) {
    let view = entity.entity_view(world);
    match spun {
        Some(s) => {
            view.set(ToolCutter { spun: s.clone() });
        }
        None => {
            view.remove(world.component::<ToolCutter>());
        }
    }
}

/// What the side view draws for `tool`: its drawn cutter when there is
/// one that agrees with these numbers, else the outline the numbers
/// draw. `None` for a turning tool.
#[must_use]
pub fn shown(tool: &Tool, drawn: Option<&SpunProfile>) -> Option<SpunProfile> {
    drawn
        .filter(|s| cutter_profile::agrees(tool, s).is_ok())
        .cloned()
        .or_else(|| SpunProfile::cutter_of(tool))
}
