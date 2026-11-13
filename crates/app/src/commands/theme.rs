//! The theme in force (M6.1 C), a user preference since #877.
//!
//! The theme belongs to the person at the screen, not to the part: no
//! project file carries it, opening or starting a document never
//! changes it, and a pick is not an undoable edit of the document. The
//! world singleton is what the shell reads every frame; `app::prefs`
//! keeps the choice per user across restarts. What the choice MEANS on
//! screen is the ui's resolution - the token tables and the JSON parser
//! live there (layering: app cannot name an egui type) - so this seam
//! owns the reference and its existence law.
//! Provenance: our own design, specced from ROADMAP M6.1 and issue #877.

use flecs_ecs::core::WorldGet; // not in prelude (0.2.2)
use flecs_ecs::prelude::World;

use crate::schema::ThemeRef;

/// The built-in themes, addressed by name. The ui seam maps each name
/// to its token table; this list is what a `Named` reference and a
/// preferences file's `{"named": ...}` are checked against.
pub const BUILTINS: [&str; 2] = ["dark", "light"];

/// The theme in force. Absent singleton = the default (a fresh world has
/// never been themed).
#[must_use]
pub fn theme_of(world: &World) -> ThemeRef {
    world.try_get::<&ThemeRef>(|t| t.clone()).unwrap_or_default()
}

/// Write the singleton: a pick that passed [`check_theme`], or the
/// stored preference at startup. Every reader goes through `theme_of`.
pub fn theme_set(world: &World, next: &ThemeRef) {
    world.set(next.clone());
}

/// The cheap existence law for a pick. A `File` reference must exist as
/// a file right now - the full parse is the surface's pre-step (it holds
/// the parser), so a corrupt theme is refused before it is ever set. A
/// `Named` reference must be a real built-in.
pub fn check_theme(next: &ThemeRef) -> Result<(), String> {
    match next {
        ThemeRef::File(path) => {
            let meta = std::fs::metadata(path).map_err(|e| format!("theme file {path}: {e}"))?;
            if !meta.is_file() {
                return Err(format!("theme file {path}: not a file"));
            }
        }
        ThemeRef::Named(name) => {
            if !BUILTINS.contains(&name.as_str()) {
                return Err(format!("built-in theme {name} is not one of {}", BUILTINS.join(", ")));
            }
        }
        ThemeRef::Dark => {}
    }
    Ok(())
}
