//! The Model page's "Pockets and slots" section (#511): a Detect button,
//! the recognised features as rows (kind, floor Z, depth, tightest inside
//! corner or slot width, open sides) and a Create button per row that
//! adds the operation machining it - Pocket over the floor, or Slot along
//! a round- or open-ended slot's centreline - one undo step. Every action
//! is a `State` method over `app::recognize`; nothing here recognises.
//!
//! Provenance: our own UI over this repo's recognize module.

use egui::Ui;

use camcraft_app::recognize::{self, Pocket, PocketKind, SlotEnd};
use camcraft_app::units::{self, Unit};

use super::layout::hint;
use crate::state::State;

fn end_word(e: SlotEnd) -> &'static str {
    match e {
        SlotEnd::Open => "open",
        SlotEnd::Round => "round",
        SlotEnd::Square => "square",
    }
}

/// One row's description: what it is and the numbers a machinist picks
/// a tool by.
pub(crate) fn row_text(i: usize, p: &Pocket, u: Unit) -> String {
    let len = |v: f64| units::len(v, u, 2);
    let what = match (p.kind, &p.slot) {
        (PocketKind::Slot, Some(s)) => format!(
            "slot {} wide x {} long, ends {}/{}",
            len(s.width),
            len(s.length),
            end_word(s.ends[0]),
            end_word(s.ends[1])
        ),
        (PocketKind::Open, _) => format!("open pocket ({} open side(s))", p.open_spans.len()),
        _ => String::from("closed pocket"),
    };
    let corner = match p.min_corner_radius {
        Some(r) if r <= 0.0 => String::from(", sharp corners"),
        Some(r) => format!(", corner R{}", len(r)),
        None => String::new(),
    };
    let islands = if p.islands > 0 { format!(", {} island(s)", p.islands) } else { String::new() };
    format!("{}  {what}  floor Z{}  depth {}{corner}{islands}", i + 1, len(p.floor_z), len(p.depth))
}

pub(super) fn section(ui: &mut Ui, state: &mut State, model: &str) {
    let u = state.units();
    ui.add_space(6.0);
    ui.label("Pockets and slots");
    let rows: Option<Vec<(String, String, Option<String>)>> = state.pocket_scan_for(model).map(|found| {
        found
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let kind = recognize::op_kind_of(p);
                let label = camcraft_app::operation::KindInfo::ALL
                    .get(kind as usize)
                    .map_or("Operation", |k| k.label);
                (row_text(i, p, u), label.to_owned(), state.pocket_tool(p))
            })
            .collect()
    });
    match &rows {
        None => hint(ui, "Reads the model's flat floors held by walls: closed and open pockets, and straight slots."),
        Some(r) if r.is_empty() => hint(ui, "No pockets or slots on this model."),
        Some(_) => {}
    }
    let mut create = None;
    for (i, (text, label, tool)) in rows.iter().flatten().enumerate() {
        ui.horizontal(|ui| {
            ui.label(text);
            let tip = match tool {
                Some(t) => format!("Adds a {label} with {t} over this feature (one undo step)"),
                None => String::from("Add a tool to the document first"),
            };
            if ui.add_enabled(tool.is_some(), egui::Button::new(format!("Create {label}"))).on_hover_text(tip).clicked() {
                create = Some(i);
            }
        });
    }
    if ui.button("Detect pockets / slots").clicked() {
        state.detect_pockets(model);
    }
    if let Some(i) = create {
        state.create_from_pocket(model, i);
    }
}
