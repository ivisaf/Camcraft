//! A plate row's mod vise lines (#820): "Add mod vise pair…" - pick a
//! preset pair, the fixed block's hole (row letter, column number), its
//! turn (a quarter turn: the jaw faces along a grid row or column) and
//! hard or soft jaws, then "Place pair" queues one `PlaceModVisePair`:
//! the adjustable block lands on the nearest hole clear of the stock and
//! slides to touch it. A jaw block's own row says which side it is and
//! how far it slid.
//! Provenance: our own layout over our own mod-vise command.

use egui::Ui;

use camcraft_app::modvise::{self, JawInsert, JawSide};
use camcraft_app::plate::{self, PlateDef};
use camcraft_app::{fixture, model, units};

use crate::state::{CommandKind, State};

/// The "Add mod vise pair…" section of plate `name` (grid `def`).
pub(super) fn pair_rows(ui: &mut Ui, state: &mut State, name: &str, def: &PlateDef) {
    let mut draft = state.mod_vise_draft(name).clone();
    let g = &def.holes;
    draft.row = draft.row.min(g.rows.saturating_sub(1));
    draft.col = draft.col.min(g.cols.saturating_sub(1));
    let mut go = false;
    egui::CollapsingHeader::new("Add mod vise pair…").id_salt(("mod-vise", name)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (k, row) in modvise::PRESETS.iter().enumerate() {
                if ui.selectable_label(draft.preset == k, row.label).clicked() {
                    draft.preset = k;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Fixed jaw at row");
            egui::ComboBox::from_id_salt(("mod-vise-row", name)).selected_text(plate::row_letters(draft.row)).show_ui(ui, |ui| {
                for r in 0..g.rows {
                    ui.selectable_value(&mut draft.row, r, plate::row_letters(r));
                }
            });
            ui.label("column");
            egui::ComboBox::from_id_salt(("mod-vise-col", name)).selected_text((draft.col + 1).to_string()).show_ui(ui, |ui| {
                for c in 0..g.cols {
                    ui.selectable_value(&mut draft.col, c, (c + 1).to_string());
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Jaw faces");
            for (q, label) in ["+Y", "-X", "-Y", "+X"].iter().enumerate() {
                ui.radio_value(&mut draft.quarter, q as u8, *label);
            }
        });
        ui.checkbox(&mut draft.soft, "Soft jaws");
        ui.label(
            egui::RichText::new("The adjustable jaw goes on the nearest hole clear of the stock and slides to touch it.").small(),
        );
        go = ui.button("Place pair").clicked();
    });
    if go && let Some((fixed, adjustable)) = draft.blocks() {
        state.pending.push_back(CommandKind::PlaceModVisePair {
            plate: name.to_owned(),
            fixed,
            adjustable,
            at: draft.at(),
            yaw_deg: 90.0 * f64::from(draft.quarter),
        });
    }
    *state.mod_vise_draft(name) = draft;
}

/// A jaw block's line: its side, jaws and slide.
pub(super) fn block_line(ui: &mut Ui, state: &State, name: &str) {
    let Some(j) = model::find_by_name(&state.world, name).and_then(|e| fixture::def_of(&state.world, &e)).and_then(|d| d.mod_jaw) else {
        return;
    };
    let jaws = if j.insert == JawInsert::Soft { "soft jaws" } else { "hard jaws" };
    let mut line = format!("Mod vise {} jaw block ({}), {jaws}", j.side.word(), j.size);
    if j.side == JawSide::Adjustable {
        let u = state.units();
        line.push_str(&format!(", slid {} of {}", units::len(j.slide, u, 3), units::len(j.slot_travel, u, 3)));
    }
    ui.label(line);
}
