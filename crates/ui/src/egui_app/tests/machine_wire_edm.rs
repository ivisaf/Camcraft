//! #535: the wire EDM through the real shell - the machine editor's
//! wire section (keys, codes, condition table, "not stated" where
//! absent, a refusal by name, one undo step) and the Wire Contour's
//! page (no Tool row, quick-add naming the machine's first condition,
//! a side edit that is one undo step). Structural only: no golden PNG.
//! Provenance: our own tests over the synthetic fixture (invented
//! codes, no vendor manual); specced from the issue's "Done means".
//!
//! Gate map (M4.4): covers ui::egui_app::machine_edm, ui::egui_app::machine_editor, ui::egui_app::op_page, ui::actions, app::machine::holders.

use egui::accesskit::Role;
use egui_kittest::kittest::NodeT;

use super::*;
use crate::egui_app::machine_editor::MachineEditor;

const FIXTURE: &str = include_str!("../../../../app/tests/edm/wire-iso-fixture.mch");

fn wire_machine() -> camcraft_app::machine::Machine {
    camcraft_app::machine::parse_mch(FIXTURE).expect("the fixture parses")
}

fn mount(h: &mut Harness<'static, ShellUi>, m: camcraft_app::machine::Machine) {
    h.state_mut().state.pending.push_back(CommandKind::EditMachineApply { path: "wire-test.mch".to_owned(), machine: Box::new(m) });
    h.run();
}

fn type_into(h: &mut Harness<'static, ShellUi>, field: &str, text: &str) {
    h.get_by_role_and_label(Role::TextInput, field).scroll_to_me();
    h.run_steps(4);
    h.get_by_role_and_label(Role::TextInput, field).click();
    h.run();
    h.input_mut().events.push(Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    h.run();
    h.get_by_role_and_label(Role::TextInput, field).type_text(text);
    h.run_steps(3);
}

fn press(h: &mut Harness<'static, ShellUi>, label: &str) {
    h.get_by_label(label).scroll_to_me();
    h.run_steps(4);
    h.get_by_label(label).click();
    h.run_steps(3);
}

/// Click the enabled button of that label (the op pane's own Apply
/// stands dark beside the machine window's).
fn press_live(h: &mut Harness<'static, ShellUi>, label: &str) {
    h.query_all_by_label(label).find(|n| !n.accesskit_node().is_disabled()).expect("a live button").scroll_to_me();
    h.run_steps(4);
    h.query_all_by_label(label).find(|n| !n.accesskit_node().is_disabled()).expect("a live button").click();
    h.run_steps(3);
}

fn mounted(h: &Harness<'static, ShellUi>) -> camcraft_app::machine::Machine {
    camcraft_app::machine::of(&h.state().state.world).1.expect("a machine is mounted")
}

#[test]
fn the_machine_editor_states_the_wire_section_refuses_by_name_and_undoes_in_one_step() {
    let _g = snapshot_guard();
    let mut h = shell_harness();
    h.run();
    let mut bare = wire_machine();
    bare.edm = None;
    bare.edm_codes.clear();
    mount(&mut h, bare.clone());
    h.state_mut().state.machine_editor = Some(MachineEditor { path: "wire-test.mch".to_owned(), machine: bare.clone() });
    h.run_steps(3);
    // Nothing is stated: every wire value wears "not stated"; no spindle.
    assert!(h.query_all_by_label("not stated").count() >= 8, "absent values say so");
    assert!(h.query_by_label("Spindle (rpm)").is_none(), "a wire EDM has no spindle");
    press(&mut h, "add condition");
    type_into(&mut h, "Condition 1 id", "C110");
    // A condition code without {id}: refused by name, nothing mounts.
    press(&mut h, "code condition");
    type_into(&mut h, "Code condition", "FX-COND");
    press_live(&mut h, "Apply");
    let status = h.state().state.status.clone();
    assert!(status.contains("edm.codes.condition") && status.contains("{id}"), "refused by name: {status}");
    assert_eq!(mounted(&h).edm, None, "a refused edit mounts nothing");
    assert!(h.state().state.machine_editor.is_some(), "the draft stays for the user to fix");
    // With {id} it applies, as one command.
    type_into(&mut h, "Code condition", "FX-COND {id}");
    press_live(&mut h, "Apply");
    let m = mounted(&h);
    assert_eq!(m.edm_codes.get("condition").map(String::as_str), Some("FX-COND {id}"));
    assert_eq!(m.edm.as_ref().map(|e| e.conditions.len()), Some(1));
    assert_eq!(m.edm.as_ref().map(|e| e.conditions[0].id.as_str()), Some("C110"));
    h.state_mut().state.pending.push_back(CommandKind::Undo);
    h.run();
    let m = mounted(&h);
    assert_eq!((m.edm, m.edm_codes.is_empty()), (None, true), "one undo takes the whole edit back");
}

#[test]
fn a_wire_contour_quick_adds_with_the_first_condition_and_its_page_has_no_tool_row() {
    let _g = snapshot_guard();
    let mut h = shell_harness();
    h.run();
    mount(&mut h, wire_machine());
    let kind = camcraft_app::operation::KindInfo::ALL.iter().position(|k| k.label == "Wire Contour").expect("the kind exists");
    crate::actions::add_operation(&mut h.state_mut().state, kind);
    h.run_steps(3);
    let name = h.state().state.selected_operation().expect("the op is created and selected");
    let op = camcraft_app::model::find_by_name(&h.state().state.world, &name)
        .and_then(|e| camcraft_app::model::operation_of(&h.state().state.world, &e))
        .expect("the operation exists");
    let camcraft_app::operation::OpParams::WireContour(w) = &op.params else { panic!("a wire contour") };
    assert_eq!(w.condition, "C110", "quick-add names the machine's first condition");
    assert!(op.tool.is_empty(), "the op mounts no library tool");
    assert!(h.query_by_label("Tool").is_none(), "no Tool row on a tool-less kind");
    // The side is one undo step.
    let side0 = w.side;
    assert!(h.query_by_label("Flood").is_none(), "a wire EDM has no coolant row");
    h.query_all_by_role(Role::ComboBox)
        .find(|n| n.accesskit_node().value().as_deref().is_some_and(|v| v.starts_with("Die")))
        .expect("the side combo")
        .click();
    h.run_steps(3);
    press(&mut h, "Punch - the wire runs outside the profile");
    press(&mut h, "Apply");
    let side = |h: &Harness<'static, ShellUi>| {
        let e = camcraft_app::model::find_by_name(&h.state().state.world, &name).expect("op");
        match camcraft_app::model::operation_of(&h.state().state.world, &e).map(|o| o.params) {
            Some(camcraft_app::operation::OpParams::WireContour(w)) => Some(w.side),
            _ => None,
        }
    };
    assert_ne!(side(&h), Some(side0), "Apply changed the side");
    h.state_mut().state.pending.push_back(CommandKind::Undo);
    h.run();
    assert_eq!(side(&h), Some(side0), "one undo restores it");
}
