//! The tool editor's drawn-cutter draft (#93 slice 2): a milling tool's
//! own cutter, neck and shank as a spun half-outline - Parametric (the
//! outline its numbers draw, regenerated as they change) or Custom
//! (hand-edited points that must keep agreeing with the numbers). The
//! draft is ephemeral pane state like `ToolProfileUi`; Apply cutter
//! queues the one command (`SetToolCutter`: a drawn outline, or a clear
//! back to the numbers' outline) and its refusal lands on the Tool pane
//! by name. Turning tools never get this draft (owner, #93).
//! Provenance: our own pane state over `app::tool_cutter`; no vendor
//! material consulted.

use camcraft_app::commands::SetToolCutter;
use camcraft_app::cutter_profile;
use camcraft_app::model;
use camcraft_app::spun::{SpunPoint, SpunProfile};
use camcraft_app::tool_cutter;

use super::{CommandKind, ErrorScope, State};

/// The head of an Apply cutter refusal.
pub(crate) const CUTTER_REFUSED: &str = "cutter profile not set: ";

/// The editor's drawn-cutter draft for the tool it is open on.
#[derive(Clone, Default)]
pub struct ToolCutterUi {
    /// The tool the draft belongs to (the editor's `tool_ui.name`).
    pub tool: String,
    /// The Profile tab shows the cutter (true) or the holder (view state).
    pub shown: bool,
    /// Custom: `points` are hand edits. Parametric: they are regenerated
    /// from the draft tool's numbers every frame.
    pub custom: bool,
    /// The drawn cutter outline, tip-up (z, r, arc).
    pub points: Vec<SpunPoint>,
    /// The world cutter the draft last mirrored: Apply, Undo, Redo or
    /// Open re-mirror when it changes.
    seen: Option<SpunProfile>,
}

impl State {
    /// The drawn cutter landed on the editor's tool, if any.
    pub(crate) fn editor_tool_cutter(&self) -> Option<SpunProfile> {
        let e = model::find_by_name(&self.world, &self.tool_ui.name)?;
        tool_cutter::cutter_of(&self.world, &e)
    }

    /// Keep the draft on the editor's tool: a new tool, or a world
    /// cutter that changed under it, re-mirrors - a drawn cutter
    /// re-edits as Custom, none starts Parametric. In Parametric the
    /// points follow the draft numbers.
    pub(crate) fn sync_tool_cutter_ui(&mut self) {
        let landed = self.editor_tool_cutter();
        if self.tool_cutter_ui.tool != self.tool_ui.name || self.tool_cutter_ui.seen != landed {
            let shown = self.tool_cutter_ui.shown;
            let custom = landed.is_some();
            let points = landed.as_ref().map(|s| s.points().to_vec()).unwrap_or_default();
            self.tool_cutter_ui = ToolCutterUi { tool: self.tool_ui.name.clone(), shown, custom, points, seen: landed };
        }
        if !self.tool_cutter_ui.custom {
            self.tool_cutter_ui.points = cutter_profile::parametric(&self.tool_ui.to_tool());
        }
    }

    /// Flip Parametric/Custom. Custom starts from the outline drawn now
    /// (the numbers' own), Parametric drops the edits.
    pub(crate) fn set_cutter_custom(&mut self, custom: bool) {
        self.tool_cutter_ui.custom = custom;
    }

    /// Insert a point after row `i`, midway to the next (a Custom edit).
    pub(crate) fn insert_cutter_point(&mut self, i: usize) {
        let d = &mut self.tool_cutter_ui;
        let (Some(a), Some(b)) = (d.points.get(i).copied(), d.points.get(i + 1).copied()) else {
            return;
        };
        d.points.insert(i + 1, SpunPoint::line((a.z + b.z) * 0.5, (a.r + b.r) * 0.5));
        d.custom = true;
    }

    /// Remove row `i` (a Custom edit); the validation names what is left.
    pub(crate) fn remove_cutter_point(&mut self, i: usize) {
        let d = &mut self.tool_cutter_ui;
        if i < d.points.len() {
            d.points.remove(i);
            d.custom = true;
        }
    }

    /// "Apply cutter": Custom lands the drawn outline; Parametric clears
    /// it back to the numbers' outline. One command either way.
    pub(crate) fn queue_apply_tool_cutter(&mut self) {
        let d = &self.tool_cutter_ui;
        let points = d.custom.then(|| d.points.clone());
        self.pending.push_back(CommandKind::SetToolCutter { name: self.tool_ui.name.clone(), points });
    }

    /// The command handler: validate now; a refusal lands on the Tool
    /// pane by name and the draft stays for correction.
    pub(crate) fn run_set_tool_cutter(&mut self, name: &str, points: Option<Vec<SpunPoint>>) {
        let clearing = points.is_none();
        match SetToolCutter::new(&self.world, name, points) {
            Ok(command) => {
                self.history.execute(&self.world, command);
                self.doc_version += 1;
                self.status = if clearing {
                    format!("tool {name}: cutter drawn from its numbers")
                } else {
                    format!("tool {name}: cutter profile set")
                };
            }
            Err(err) => self.refuse(ErrorScope::Tool, format!("{CUTTER_REFUSED}{err}")),
        }
    }
}
