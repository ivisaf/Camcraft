//! Modular plate-mounted vises (#820, part of #339): a "mod vise" is two
//! separate jaw blocks bolted to a fixture plate's grid (#819) - a FIXED
//! block and an ADJUSTABLE one. The opening is set by which grid holes
//! each block is bolted to; the adjustable block then slides along its
//! bolt slots until it touches the workpiece. Each block is an ordinary
//! meshed fixture wearing `FixtureDef::mod_jaw` (its parameters) and a
//! place on the plate (`FixtureDef::plate.mount`), so the fixture laws,
//! the riders rule and the project file treat it like any other fixture.
//!
//! **The block frame.** The origin is the block's ANCHOR bolt (the grid
//! hole it is placed on) at the block's underside. Local +Y points from
//! the block toward the jaw it clamps against: the clamping face (the
//! jaw insert's front) is at y = `face_offset + slide`, the insert behind
//! it, the body behind the insert. X runs across the jaw; the body is
//! centred across on the bolt pattern's mid-span. Z is up from the plate.
//! A pair stands face to face: the adjustable block is turned 180
//! degrees from the fixed one.
//!
//! **The solid** is the block's side profile (body plus insert) extruded
//! across the wider of the body and the jaw: a conservative envelope -
//! the fixture laws see at least the real steel, never less. Bolt and
//! pin holes are data (the mounting footprint), not drawn.
//!
//! Provenance: our own model and parameters from general workholding
//! practice (machine-shop handbooks: two-block modular vises on grid
//! plates). The presets are generic round figures of our own; no vendor
//! model or dimension is copied (`docs/decisions/820-mod-vises.md`).

use serde::{Deserialize, Serialize};

use crate::geometry::Triangle;

pub mod fit;
pub mod mesh;
pub mod reach;
pub mod table;

pub use fit::{PairFit, fit_pair};
pub use table::{ModVisePreset, PRESETS, preset};

/// The most bolts or pins one footprint may list.
pub const MAX_FOOTPRINT: usize = 16;

/// Which jaw block of a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JawSide {
    /// Bolted and pinned where it stands.
    Fixed,
    /// Bolted through slots: slides toward the workpiece.
    Adjustable,
}

impl JawSide {
    /// How the side reads in names and refusals.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            JawSide::Fixed => "fixed",
            JawSide::Adjustable => "adjustable",
        }
    }
}

/// The jaw insert: a hard (steel) or soft (machinable) jaw plate - the
/// same slab to the fixture laws, recorded for the setup sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JawInsert {
    #[default]
    Hard,
    Soft,
}

/// One jaw block's parameters (millimetres; see the module for the frame).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JawParams {
    /// The preset it started from; free text once edited.
    pub size: String,
    pub side: JawSide,
    /// Along the clamping direction (local Y), insert excluded.
    pub body_length: f64,
    /// Across the jaw (local X).
    pub body_width: f64,
    pub body_height: f64,
    pub jaw_width: f64,
    /// The insert's height above the plate.
    pub jaw_height: f64,
    /// The insert slab's depth along local Y.
    pub jaw_thickness: f64,
    #[serde(default)]
    pub insert: JawInsert,
    /// From the anchor bolt to the clamping face along local +Y, at no slide.
    pub face_offset: f64,
    pub bolt_diameter: f64,
    /// Bolt positions [x, y] in the block frame; one is the anchor [0, 0].
    pub bolts: Vec<[f64; 2]>,
    /// Locating pin positions [x, y] (into the plate's dowel bores).
    #[serde(default)]
    pub pins: Vec<[f64; 2]>,
    /// How far the body may slide along its bolt slots (0 for a fixed block).
    #[serde(default)]
    pub slot_travel: f64,
    /// How far it has slid toward its clamping face, within the travel.
    #[serde(default)]
    pub slide: f64,
}

/// Float dust allowed on a fit made by computation.
pub const TOL: f64 = 1e-6;

impl JawParams {
    fn named(&self) -> [(&'static str, f64); 7] {
        [
            ("body length", self.body_length),
            ("body width", self.body_width),
            ("body height", self.body_height),
            ("jaw width", self.jaw_width),
            ("jaw height", self.jaw_height),
            ("jaw thickness", self.jaw_thickness),
            ("bolt diameter", self.bolt_diameter),
        ]
    }

    /// "fixed jaw block" / "adjustable jaw block", as refusals say it.
    #[must_use]
    pub fn what(&self) -> String {
        format!("{} jaw block", self.side.word())
    }

    /// The body's centre across (local X): the bolt pattern's mid-span.
    #[must_use]
    pub fn centre_x(&self) -> f64 {
        let (lo, hi) = self.bolts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), b| (lo.min(b[0]), hi.max(b[0])));
        if lo.is_finite() && hi.is_finite() { 0.5 * (lo + hi) } else { 0.0 }
    }

    /// The clamping face's local Y at the current slide.
    #[must_use]
    pub fn face_y(&self) -> f64 {
        self.face_offset + self.slide
    }

    /// The block law. Every size a positive, finite length; the slot
    /// travel and slide finite, the slide within the travel, a fixed
    /// block with neither; an anchor bolt at [0, 0]; every bolt and pin
    /// inside the body by its radius at every slide (a slotted body must
    /// still cover its bolts at full travel). Refusals name the figure.
    pub fn check(&self) -> Result<(), String> {
        let what = self.what();
        for (fig, v) in self.named() {
            if !v.is_finite() {
                return Err(format!("{what} {fig} must be a finite length (got {v})"));
            }
            if v <= 0.0 {
                return Err(format!("{what} {fig} must be more than zero (got {v} mm)"));
            }
        }
        for (fig, v) in [("face offset", self.face_offset), ("slot travel", self.slot_travel), ("slide", self.slide)] {
            if !v.is_finite() {
                return Err(format!("{what} {fig} must be finite (got {v})"));
            }
        }
        if self.slot_travel < 0.0 || self.slide < 0.0 {
            return Err(format!("{what} slot travel and slide must not be negative"));
        }
        if self.side == JawSide::Fixed && (self.slot_travel > 0.0 || self.slide > 0.0) {
            return Err(format!("a {what} has no slot travel (got {:.3} mm)", self.slot_travel.max(self.slide)));
        }
        if self.slide > self.slot_travel + TOL {
            return Err(format!("{what} slide {:.3} mm is past its slot travel {:.3} mm", self.slide, self.slot_travel));
        }
        if self.bolts.is_empty() || self.bolts.len() > MAX_FOOTPRINT || self.pins.len() > MAX_FOOTPRINT {
            return Err(format!("{what} needs 1 to {MAX_FOOTPRINT} bolts and at most {MAX_FOOTPRINT} pins"));
        }
        if !self.bolts.iter().any(|b| b[0] == 0.0 && b[1] == 0.0) {
            return Err(format!("{what} needs its anchor bolt at [0, 0] (the grid hole it is placed on)"));
        }
        let r = 0.5 * self.bolt_diameter;
        let xc = self.centre_x();
        let half = 0.5 * self.body_width;
        // The body's Y span at no slide; it moves up by the slide.
        let front = self.face_offset - self.jaw_thickness;
        let rear = front - self.body_length;
        for (kind, pts) in [("bolt", &self.bolts), ("pin", &self.pins)] {
            for p in pts.iter() {
                if !(p[0].is_finite() && p[1].is_finite()) {
                    return Err(format!("{what} {kind} position must be finite"));
                }
                let inside = (p[0] - xc).abs() + r <= half + TOL && p[1] - r >= rear + self.slot_travel - TOL && p[1] + r <= front + TOL;
                if !inside {
                    return Err(format!(
                        "{what} {kind} at [{:.3}, {:.3}] mm is not inside the body{} - check the face offset, body length and slot travel",
                        p[0],
                        p[1],
                        if self.slot_travel > 0.0 { " at every slide" } else { "" }
                    ));
                }
            }
        }
        Ok(())
    }

    /// The same block slid `slide` along its slots.
    #[must_use]
    pub fn with_slide(&self, slide: f64) -> Self {
        Self { slide, ..self.clone() }
    }

    /// The checked block's closed shell in its own frame.
    pub fn mesh(&self) -> Result<Vec<Triangle>, String> {
        self.check()?;
        mesh::solid(self)
    }

    /// The model name a fresh block lands under (before the free-name
    /// suffix): "Mod vise fixed 1 in grid".
    #[must_use]
    pub fn base_name(&self) -> String {
        let s = self.size.trim();
        if s.is_empty() { format!("Mod vise {}", self.side.word()) } else { format!("Mod vise {} {s}", self.side.word()) }
    }
}
