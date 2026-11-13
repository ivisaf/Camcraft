//! The viewer's colour-vision remap (M13.4), a user preference since
//! #877, like the theme.
//!
//! A deficiency belongs to the person at the screen, not to the part, so
//! no project file carries the mode, opening a document leaves it as it
//! was, and a pick is not an undoable edit of the document. The world
//! singleton is what the shell reads every frame; `app::prefs` keeps the
//! choice per user across restarts. What a mode MEANS on screen (the
//! remapped verdict, toolpath and selection colours) is the ui's
//! resolution - app cannot name a colour type - so this seam owns only
//! the reference.
//! Provenance: our own design, specced from ROADMAP M13.4 and issue #877.

use flecs_ecs::core::WorldGet; // not in prelude (0.2.2)
use flecs_ecs::prelude::World;

use crate::schema::ColorVision;

/// The remap in force. Absent singleton = `Off` (a fresh world has
/// never been remapped), exactly like `theme_of`.
#[must_use]
pub fn color_vision_of(world: &World) -> ColorVision {
    world.try_get::<&ColorVision>(|c| *c).unwrap_or_default()
}

/// Write the singleton: a pick that passed the ui's colour-vision gate,
/// or the stored preference at startup. Every reader goes through
/// `color_vision_of`.
pub fn color_vision_set(world: &World, next: ColorVision) {
    world.set(next);
}
