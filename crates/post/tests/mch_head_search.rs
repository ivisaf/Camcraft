//! #785: `parse_mch` finds the head however the kinematics tree is
//! ordered - a leaf part (a 4th-axis rotary with no `parts`) listed ahead
//! of the head is passed over, never the end of the search. Pinned on a
//! hand-written tree, so a reorder of a shipped `.mch` cannot hide it.
//! Provenance: our own test over our own machine-file parser.
//!
//! Gate map (M4.4): covers camcraft_post::machine (parse).

use camcraft_post::machine::parse_mch;

fn machine(parts: &str) -> String {
    format!(r#"{{ "general": {{ "model": "T" }}, "kinematics": {{ "default": {{ "units": {{ "length": "mm" }}, "parts": {parts} }} }} }}"#)
}

const HEAD: &str = r#"{ "type": "head", "spindle": { "min_speed": 100, "max_speed": 10000 } }"#;
const LEAF: &str = r#"{ "type": "axis", "id": "A" }"#;

#[test]
fn a_leaf_ahead_of_the_head_does_not_end_the_head_search() {
    for parts in [
        format!("[{LEAF}, {HEAD}]"),
        format!(r#"[{LEAF}, {{ "type": "axis", "id": "Z", "parts": [{LEAF}, {HEAD}] }}]"#),
        format!("[{HEAD}, {LEAF}]"),
    ] {
        let m = parse_mch(&machine(&parts)).unwrap_or_else(|e| panic!("{parts}: {e}"));
        assert_eq!(m.spindle_max, Some(10000.0), "{parts}");
        assert_eq!(m.spindle_min, Some(100.0), "{parts}");
    }
}

#[test]
fn a_tree_with_no_head_states_no_spindle() {
    let m = parse_mch(&machine(&format!("[{LEAF}, {LEAF}]"))).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(m.spindle_max, None);
}
