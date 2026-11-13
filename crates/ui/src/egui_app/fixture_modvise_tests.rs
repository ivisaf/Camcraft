//! #820 in the shell, structurally: a plate's row on the Fixtures page
//! offers "Add mod vise pair…" (presets, the fixed jaw's row and column,
//! the way its jaw faces, soft jaws); "Place pair" queues one
//! `PlaceModVisePair` that lands both jaw blocks on the plate (one undo
//! takes both), and each block's row says which side it is and how far
//! it slid; a refusal is named in the status. Driven through the real
//! page code in a kittest harness - no pixels (the snapshot lane is release).
//! Provenance: our own tests over our own shell.

use egui::Vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use camcraft_app::commands::{SetStock, SetUnits};
use camcraft_app::fixture;
use camcraft_app::model;
use camcraft_app::plate;
use camcraft_app::schema::Location;
use camcraft_app::stock::StockShape;
use camcraft_app::units::Unit;

use crate::state::{CommandKind, State};

fn page(state: State) -> Harness<'static, State> {
    Harness::builder().with_size(Vec2::new(700.0, 1600.0)).build_ui_state(
        |ui, st: &mut State| {
            st.drain_actions();
            egui::ScrollArea::vertical().show(ui, |ui| super::pages::fixtures_section(ui, st, None));
        },
        state,
    )
}

/// A 12 in plate laid so a 1 in preset fixed jaw at C3 has its face on
/// the -Y side of a 60 mm stock; with or without the stock.
fn plated(stock: bool) -> State {
    let mut state = State::new();
    if stock {
        let cmd = SetStock::new(&state.world, StockShape::Box { dx: 60.0, dy: 60.0, dz: 20.0 }, Location::default()).expect("stock");
        state.history.execute(&state.world, cmd);
    }
    let p = plate::preset("12 x 12 in").map(plate::PlatePreset::params).expect("preset");
    let pos = [-25.4 - 2.5 * 25.4, -30.0 - 1.75 * 25.4 - 2.5 * 25.4, 0.0];
    state.pending.push_back(CommandKind::InsertFixturePlate { setup: None, params: p, on_table: false, pos, yaw_deg: 0.0 });
    state.drain_actions();
    let d = state.mod_vise_draft("Plate 12 x 12 in");
    d.row = 2; // C
    d.col = 2; // 3
    state
}

fn names(state: &State) -> Vec<String> {
    model::named_entities(&state.world).into_iter().map(|(n, _)| n).filter(|n| n.starts_with("Mod vise")).collect()
}

#[test]
fn a_plate_row_places_a_mod_vise_pair_in_one_undoable_step() {
    let mut h = page(plated(true));
    h.run();
    h.get_by_label("Add mod vise pair…").click();
    h.run();
    for label in ["1 in grid 3/8-16", "2 in grid 1/2-13", "50 mm grid M12", "Fixed jaw at row", "+Y", "-X", "Soft jaws", "Place pair"] {
        assert!(h.query_by_label(label).is_some(), "the section shows {label}");
    }
    h.get_by_label("1 in grid 3/8-16").click();
    h.run();
    h.get_by_label("Soft jaws").click();
    h.run();
    h.get_by_label("Place pair").click();
    h.run();
    h.run();
    let st = h.state();
    assert!(st.status.contains("fixed at C3, adjustable at I5"), "{}", st.status);
    assert!(st.status.ends_with(" mm to the stock"), "the slide in document units: {}", st.status);
    let mut placed = names(st);
    placed.sort();
    assert_eq!(placed, ["Mod vise adjustable 1 in grid 3/8-16", "Mod vise fixed 1 in grid 3/8-16"]);
    let def = model::find_by_name(&st.world, &placed[0]).and_then(|e| fixture::def_of(&st.world, &e)).expect("block");
    assert_eq!(def.mod_jaw.map(|j| j.insert), Some(camcraft_app::modvise::JawInsert::Soft));
    assert!(h.query_by_label_contains("Mod vise adjustable jaw block (1 in grid 3/8-16), soft jaws, slid").is_some());
    assert!(h.query_by_label_contains("Mod vise fixed jaw block").is_some());
    assert!(h.query_by_label_contains("Carries: Mod vise").is_some(), "the plate carries them");
    let st = h.state_mut();
    assert!(st.history.undo(&st.world));
    assert!(names(st).is_empty(), "one undo takes both blocks");
}

#[test]
fn a_refused_pair_is_named_in_the_status() {
    let mut h = page(plated(false));
    h.run();
    h.get_by_label("Add mod vise pair…").click();
    h.run();
    h.get_by_label("Place pair").click();
    h.run();
    h.run();
    assert!(h.state().status.contains("mod vise not placed") && h.state().status.contains("no stock"), "{}", h.state().status);
    assert!(names(h.state()).is_empty());
}

#[test]
fn the_placed_status_states_the_slide_in_document_units() {
    let mut state = plated(true);
    let cmd = SetUnits::new(&state.world, Unit::Inches).expect("units");
    state.history.execute(&state.world, cmd);
    let mut h = page(state);
    h.run();
    h.get_by_label("Add mod vise pair…").click();
    h.run();
    h.get_by_label("1 in grid 3/8-16").click();
    h.run();
    h.get_by_label("Place pair").click();
    h.run();
    h.run();
    let st = h.state();
    assert!(st.status.contains("fixed at C3, adjustable at I5"), "{}", st.status);
    assert!(st.status.ends_with(" in to the stock") && !st.status.contains("mm"), "an inch document reads inches: {}", st.status);
}
