//! #511: the Model page's Detect pockets / slots. A part with a through
//! slot and a closed pocket lists both; each row's Create adds the
//! operation machining it (a Slot along the slot, a Pocket over the
//! floor face) through the command queue, opens its page, and one Undo
//! removes it. A document change marks the list stale instead of
//! offering coordinates the part no longer has.
//!
//! Gate map (#511): covers ui::state (state_pockets), ui::egui_app
//! (pockets_page row text), app::recognize (plan_operation).
//! Provenance: our own fixtures (#35's pocket_support height field).

#[path = "../../app/tests/pocket_support/mod.rs"]
mod pocket_support;

use camcraft_app::commands::{AddTool, ImportModel, SetStock};
use camcraft_app::stock::StockShape;
use camcraft_app::model;
use camcraft_app::operation::{OpParams, SrcRef};
use camcraft_app::recognize::PocketKind;
use camcraft_app::tools::catalog;
use camcraft_app::units::Unit;

use crate::egui_app::pockets_row_text;
use crate::state::{CommandKind, State, UiTarget};

const TOOL: &str = "EM-0400-2FL";

/// 60 x 40 x 20 block: a 10 wide slot through along X to z 10 (row
/// j = 1), a 2 wide wall, and a closed 20 x 18 pocket to z 12 in the
/// cell (1, 3).
fn part_state() -> State {
    let mut state = State::new();
    let mesh = pocket_support::height_field(
        &[-30.0, -10.0, 10.0, 30.0],
        &[-20.0, -12.0, -2.0, 0.0, 18.0, 20.0],
        |i, j| match (i, j) {
            (_, 1) => 10.0,
            (1, 3) => 12.0,
            _ => 20.0,
        },
    );
    state.history.execute(&state.world, ImportModel::from_mesh("part", mesh));
    let stock = SetStock::new(&state.world, StockShape::Box { dx: 60.0, dy: 40.0, dz: 20.0 }, Default::default());
    state.history.execute(&state.world, stock.expect("stock"));
    let em = catalog().into_iter().find(|(n, _)| *n == TOOL).map(|(_, t)| t).expect("tool");
    state.history.execute(&state.world, AddTool::new(&state.world, TOOL, em).expect("add tool"));
    state.doc_version += 1;
    state
}

fn ops(state: &State) -> Vec<String> {
    model::named_entities(&state.world)
        .into_iter()
        .filter(|(_, e)| model::is_operation(&state.world, e))
        .map(|(n, _)| n)
        .collect()
}

#[test]
fn detect_lists_the_slot_and_the_pocket() {
    let mut state = part_state();
    assert!(state.pocket_scan_for("part").is_none(), "nothing listed before a click");
    state.detect_pockets("part");
    let found = state.pocket_scan_for("part").expect("listed").to_vec();
    let kinds: Vec<PocketKind> = found.iter().map(|p| p.kind).collect();
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(kinds.contains(&PocketKind::Slot) && kinds.contains(&PocketKind::Closed), "{kinds:?}");
    let rows: Vec<String> = found.iter().enumerate().map(|(i, p)| pockets_row_text(i, p, Unit::Millimeters)).collect();
    assert!(rows.iter().any(|r| r.contains("slot 10.00") && r.contains("ends open/open")), "{rows:?}");
    assert!(rows.iter().any(|r| r.contains("closed pocket") && r.contains("floor Z12.00") && r.contains("depth 8.00")), "{rows:?}");
    assert!(state.status.contains("2 pocket(s) / slot(s)"), "{}", state.status);
}

#[test]
fn create_adds_each_operation_as_one_undo_step() {
    let mut state = part_state();
    state.detect_pockets("part");
    let found = state.pocket_scan_for("part").expect("listed").to_vec();
    for (i, p) in found.iter().enumerate() {
        // The list is held across creates within one scan of an unchanged part.
        state.detect_pockets("part");
        let before = ops(&state);
        state.create_from_pocket("part", i);
        state.drain_actions();
        let after = ops(&state);
        assert_eq!(after.len(), before.len() + 1, "feature {i}: {after:?} ({})", state.status);
        let name = after.iter().find(|n| !before.contains(n)).expect("new op").clone();
        let e = model::find_by_name(&state.world, &name).expect("op entity");
        let op = model::operation_of(&state.world, &e).expect("operation");
        match p.kind {
            PocketKind::Slot => {
                assert!(matches!(op.params, OpParams::Slot { .. }), "{name}: {:?}", op.params);
                assert_eq!(op.src, SrcRef::Stock, "a slot cuts into the stock");
            }
            _ => {
                assert!(matches!(op.params, OpParams::Pocket { .. }), "{name}: {:?}", op.params);
                assert!(
                    matches!(&op.src, SrcRef::Selection(sel) if sel.len() == 1 && sel[0].entity == "part"),
                    "a pocket picks the part's floor face: {:?}",
                    op.src
                );
            }
        }
        assert_eq!(op.tool, TOOL);
        assert!(matches!(&state.ui_target, UiTarget::Operation(n) if *n == name), "its page opens");
        state.pending.push_back(CommandKind::Undo);
        state.drain_actions();
        assert_eq!(ops(&state), before, "feature {i}: one Undo removes it");
    }
}

#[test]
fn a_changed_document_makes_the_list_stale() {
    let mut state = part_state();
    state.detect_pockets("part");
    assert!(state.pocket_scan_for("part").is_some());
    state.doc_version += 1; // any command after the scan (a move, an undo...)
    assert!(state.pocket_scan_for("part").is_none(), "a stale list is not offered");
    let before = ops(&state);
    state.create_from_pocket("part", 0);
    state.drain_actions();
    assert_eq!(ops(&state), before, "nothing created from a stale list");
    assert!(state.status.contains("detect pockets again"), "{}", state.status);
}

#[test]
fn a_model_without_geometry_is_refused_by_name() {
    let mut state = State::new();
    state.detect_pockets("ghost");
    assert!(state.status.contains("pockets not detected") && state.status.contains("ghost"), "{}", state.status);
    assert!(state.pocket_scan_for("ghost").is_none());
}
