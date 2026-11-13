//! The Model page's "Detect pockets / slots" (#511): one recognition pass
//! per click (`app::recognize::pockets_of`), the list held as view state
//! for the page, and each row's Create turned into the operation that
//! machines it (`app::recognize::plan_operation`, shared with the MCP
//! `detect_pockets {create}` tool) - one `CreateOperation` through the
//! command queue, one undo step. No recognition or planning logic lives
//! in `ui`: this module only calls into `app` (layering ui -> app).
//!
//! Provenance: our own UI glue over this repo's recognize module; no
//! vendor material.

use camcraft_app::operation::{tool_gate, KindInfo};
use camcraft_app::recognize::{self, Pocket};

use super::{CommandKind, ErrorScope, State, UiTarget};

/// The last Detect pockets / slots pass: which model, at which document
/// version, and what it found. View state, never authoritative - a
/// document change after the scan marks it stale rather than offering
/// coordinates the part no longer has.
#[derive(Clone, Debug, Default)]
pub struct PocketScan {
    pub model: String,
    pub doc_version: u64,
    pub found: Vec<Pocket>,
}

impl State {
    /// Run recognition on `model` and hold the list for the page.
    pub(crate) fn detect_pockets(&mut self, model: &str) {
        match recognize::pockets_of(&self.world, model) {
            Ok(found) => {
                self.status = match found.len() {
                    0 => format!("{model}: no pockets or slots found"),
                    n => format!("{model}: {n} pocket(s) / slot(s) found"),
                };
                self.pocket_scan = PocketScan { model: model.to_owned(), doc_version: self.doc_version, found };
            }
            Err(e) => {
                self.pocket_scan = PocketScan::default();
                self.refuse(ErrorScope::Operation, format!("pockets not detected: {e}"));
            }
        }
    }

    /// The held scan for `model`, when it is still current.
    #[must_use]
    pub(crate) fn pocket_scan_for(&self, model: &str) -> Option<&[Pocket]> {
        let s = &self.pocket_scan;
        (s.model == model && s.doc_version == self.doc_version).then_some(s.found.as_slice())
    }

    /// The tool a Create would use for feature `i`: the last document
    /// tool that fits the feature's kind, as the ribbon's quick-add picks.
    #[must_use]
    pub(crate) fn pocket_tool(&self, p: &Pocket) -> Option<String> {
        let kind = recognize::op_kind_of(p) as usize;
        KindInfo::ALL
            .get(kind)
            .map(|k| tool_gate::fitting_tools(&self.world, k))
            .unwrap_or_default()
            .last()
            .cloned()
            .or_else(|| self.tool_names().pop())
    }

    /// Create the operation that machines feature `i` of the held scan,
    /// then open its page. Refused by name when the scan is stale, the
    /// index is gone, there is no tool, or the feature cannot be planned.
    pub(crate) fn create_from_pocket(&mut self, model: &str, i: usize) {
        let Some(p) = self.pocket_scan_for(model).and_then(|f| f.get(i)).cloned() else {
            self.refuse(ErrorScope::Operation, format!("{model}: detect pockets again - the list is out of date"));
            return;
        };
        let Some(tool) = self.pocket_tool(&p) else {
            self.refuse(ErrorScope::Operation, String::from("operation not created: no tool in the document"));
            return;
        };
        match recognize::plan_operation(&self.world, &p, &tool, None) {
            Ok(plan) => {
                let name = plan.name.clone();
                self.pending.push_back(CommandKind::CreateOperation {
                    name: plan.name,
                    tool,
                    src: plan.src,
                    params: plan.params,
                });
                self.pending.push_back(CommandKind::Select(Some(name.clone())));
                self.ui_target = UiTarget::Operation(name);
            }
            Err(e) => self.refuse(ErrorScope::Operation, format!("operation not created: {e}")),
        }
    }
}

