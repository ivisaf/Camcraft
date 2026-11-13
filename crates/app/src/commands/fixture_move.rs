//! `MoveFixture`: the rigid placement a fixture receives (M3.2), split out
//! of `fixture.rs` along the move seam (#826, the 400-line law).
//!
//! Provenance: our own design; same construct-then-pure-write command shape
//! as its siblings in `fixture.rs`.

use flecs_ecs::prelude::World;

use super::fixture::{check_location, require_writable};
use crate::commands::{Command, CommandError};
use crate::fixture;
use crate::model;
use crate::schema::{FixtureDef, Location};

/// Move a fixture: the same rigid placement any model receives, named
/// for the page that offers it. A fixture that is not a tagged model is
/// refused - the alternative is silently moving a part someone else
/// calls workholding.
pub struct MoveFixture {
    name: String,
    prev: Location,
    next: Location,
    warning: Option<String>,
    /// #819: the fixtures standing on a moved plate, carried with it.
    carried: Vec<crate::plate::RiderMove>,
    /// #819: a fixture placed on a plate and then moved by hand leaves
    /// its grid location: the marker before and after.
    unmount: Option<(FixtureDef, FixtureDef)>,
}

impl MoveFixture {
    /// A Fixtures-page move. Since #582 it carries a by-name warning when
    /// the moved fixture would sit in a part or the setup's stock - the
    /// same sampled check a locate runs, never a refusal.
    pub fn new(world: &World, name: &str, location: Location) -> Result<Self, CommandError> {
        let mut command = Self::locating(world, name, location)?;
        // #819: a moved plate's riders are judged where they land too.
        let mut warnings: Vec<String> = crate::fixture_clash::after_fixture_move(world, name, command.next).into_iter().collect();
        warnings.extend(command.rider_warnings(world));
        command.warning = crate::fixture_clash::join(warnings);
        Ok(command)
    }

    /// The clash warnings of the riders this move carries, judged where
    /// they land (#826: a locate checks them like a move).
    #[must_use]
    pub(crate) fn rider_warnings(&self, world: &World) -> Vec<String> {
        self.carried.iter().filter_map(|m| crate::fixture_clash::after_fixture_move(world, &m.name, m.after)).collect()
    }

    /// The move a `LocateFixture` makes: no check here, the locate runs
    /// its own (mate-worded) one.
    pub fn locating(world: &World, name: &str, location: Location) -> Result<Self, CommandError> {
        let entity = require_writable(world, name)?;
        if !model::is_fixture(world, &entity) {
            return Err(CommandError::Invalid(format!("'{name}' is not a fixture")));
        }
        let next = check_location(location, &format!("'{name}'"))?;
        let prev = model::transform_of(world, &entity).unwrap_or_default();
        let carried = crate::plate::mount::carry(world, name, crate::plate::mount::delta(prev, next), &[]);
        let unmount = fixture::def_of(world, &entity).filter(|d| d.plate.mount.is_some()).map(|d| {
            let mut off = d.clone();
            off.plate.mount = None;
            (d, off)
        });
        Ok(MoveFixture {
            name: name.to_string(),
            prev,
            next,
            warning: None,
            carried,
            unmount,
        })
    }

    /// The interpenetration warning (#582), if any.
    #[must_use]
    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    /// The fixtures a moved plate carries with it (#819), by name.
    pub fn carried(&self) -> impl Iterator<Item = &str> {
        self.carried.iter().map(|m| m.name.as_str())
    }
}

impl Command for MoveFixture {
    fn apply(&self, world: &World) {
        if let Some(entity) = model::find_by_name(world, &self.name) {
            model::set_transform(world, &entity, self.next);
            if let Some((_, off)) = &self.unmount {
                entity.entity_view(world).set(off.clone());
            }
        }
        crate::plate::mount::write(world, &self.carried, true);
    }

    fn invert(&self, world: &World) {
        if let Some(entity) = model::find_by_name(world, &self.name) {
            model::set_transform(world, &entity, self.prev);
            if let Some((on, _)) = &self.unmount {
                entity.entity_view(world).set(on.clone());
            }
        }
        crate::plate::mount::write(world, &self.carried, false);
    }
}
