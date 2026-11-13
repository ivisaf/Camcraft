//! Mod vise state (#820): each plate row's "Add mod vise pair…" draft
//! (by plate name, like the place-on-plate drafts) and the handler its
//! command lands through - `PlaceModVisePair`, one undo unit for both
//! jaw blocks. Drafts are view state, never history.
//! Provenance: our own dialog state over our own mod-vise command.

use camcraft_app::commands::PlaceModVisePair;
use camcraft_app::model;
use camcraft_app::modvise::{self, JawInsert, JawParams};
use camcraft_app::plate;

use super::{ErrorScope, State};

/// One plate row's "Add mod vise pair…" draft.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModViseDraft {
    /// Index into `modvise::PRESETS`.
    pub preset: usize,
    /// The fixed block's hole, zero-based.
    pub row: u32,
    pub col: u32,
    /// Quarter turns of the fixed block (0..4): 0 - its jaw faces +Y.
    pub quarter: u8,
    pub soft: bool,
}

impl ModViseDraft {
    /// The fixed block's hole name.
    #[must_use]
    pub fn at(&self) -> String {
        plate::location_name(self.row, self.col)
    }

    /// The pair the draft places: (fixed, adjustable).
    #[must_use]
    pub fn blocks(&self) -> Option<(JawParams, JawParams)> {
        let row = modvise::PRESETS.get(self.preset)?;
        let insert = if self.soft { JawInsert::Soft } else { JawInsert::Hard };
        let (mut f, mut a) = (row.fixed(), row.adjustable());
        f.insert = insert;
        a.insert = insert;
        Some((f, a))
    }
}

impl State {
    /// Plate `name`'s mod-vise draft, made on first use (the inch preset
    /// for an inch document).
    pub fn mod_vise_draft(&mut self, name: &str) -> &mut ModViseDraft {
        let inch = self.units() == camcraft_app::units::Unit::Inches;
        self.mod_vise.entry(name.to_owned()).or_insert_with(|| {
            let want = if inch { "1 in grid 3/8-16" } else { "50 mm grid M12" };
            ModViseDraft { preset: modvise::PRESETS.iter().position(|p| p.label == want).unwrap_or(0), ..ModViseDraft::default() }
        })
    }

    /// Drop the mod-vise drafts of plates that are gone.
    pub fn retire_mod_vise_drafts(&mut self) {
        let world = &self.world;
        self.mod_vise.retain(|name, _| model::find_by_name(world, name).is_some());
    }

    /// `CommandKind::PlaceModVisePair`: one `PlaceModVisePair`.
    pub(super) fn place_mod_vise_pair(&mut self, plate: &str, fixed: &JawParams, adjustable: &JawParams, at: &str, yaw_deg: f64) {
        match PlaceModVisePair::new(&self.world, plate, fixed, adjustable, at, yaw_deg) {
            Ok(command) => {
                let fit = command.fit().clone();
                self.history.execute(&self.world, command);
                self.doc_version += 1;
                let u = self.units();
                self.status = format!(
                    "mod vise on {plate}: fixed at {}, adjustable at {} slid {} {} to the stock",
                    fit.fixed_at,
                    fit.adjustable_at,
                    camcraft_app::units::len(fit.slide, u, 3),
                    u.suffix()
                );
            }
            Err(err) => self.refuse(ErrorScope::Setup, format!("mod vise not placed: {err}")),
        }
    }
}
