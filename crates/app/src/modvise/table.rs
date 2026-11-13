//! The built-in mod-vise presets (#820): generic jaw-block pairs of our
//! own round figures for three common grids - 3/8-16 on 1 in (the
//! built-in 12 in and 770MX plates), 1/2-13 on 2 in, and M12 on 50 mm
//! (the built-in 400 x 300 mm plate). No vendor's model or dimension is
//! copied (`docs/decisions/820-mod-vises.md`); the user edits any figure
//! to match their own blocks.
//!
//! Each pair: a fixed block with its face a little in front of its bolt
//! row, and an adjustable block whose slots travel one grid pitch - so
//! any stock between the jaws is reached from the nearest hole that
//! clears it. Both bolt to two holes side by side across the jaw.
//!
//! Provenance: our own round figures; thread sizes from public thread
//! standards.

use super::{JawInsert, JawParams, JawSide};

const IN: f64 = 25.4;

/// One preset pair: a label and its figures (millimetres).
#[derive(Clone, Copy, Debug)]
pub struct ModVisePreset {
    /// What the list shows.
    pub label: &'static str,
    /// The grid pitch the footprint is made for.
    pub pitch: f64,
    /// The bolt spacing across the jaw.
    bolt_span: f64,
    bolt_diameter: f64,
    body_width: f64,
    body_height: f64,
    jaw_height: f64,
    jaw_thickness: f64,
    fixed_length: f64,
    fixed_face: f64,
    adjustable_length: f64,
    adjustable_face: f64,
}

impl ModVisePreset {
    fn block(&self, side: JawSide) -> JawParams {
        let (body_length, face_offset, slot_travel) = match side {
            JawSide::Fixed => (self.fixed_length, self.fixed_face, 0.0),
            JawSide::Adjustable => (self.adjustable_length, self.adjustable_face, self.pitch),
        };
        JawParams {
            size: self.label.to_owned(),
            side,
            body_length,
            body_width: self.body_width,
            body_height: self.body_height,
            jaw_width: self.body_width,
            jaw_height: self.jaw_height,
            jaw_thickness: self.jaw_thickness,
            insert: JawInsert::Hard,
            face_offset,
            bolt_diameter: self.bolt_diameter,
            bolts: vec![[0.0, 0.0], [self.bolt_span, 0.0]],
            pins: Vec::new(),
            slot_travel,
            slide: 0.0,
        }
    }

    /// The fixed block.
    #[must_use]
    pub fn fixed(&self) -> JawParams {
        self.block(JawSide::Fixed)
    }

    /// The adjustable block (no slide yet).
    #[must_use]
    pub fn adjustable(&self) -> JawParams {
        self.block(JawSide::Adjustable)
    }
}

/// Every preset, in list order.
pub const PRESETS: &[ModVisePreset] = &[
    ModVisePreset {
        label: "1 in grid 3/8-16",
        pitch: IN,
        bolt_span: 2.0 * IN,
        bolt_diameter: 0.375 * IN,
        body_width: 5.0 * IN,
        body_height: 1.5 * IN,
        jaw_height: 1.25 * IN,
        jaw_thickness: 0.5 * IN,
        fixed_length: 2.5 * IN,
        fixed_face: 1.75 * IN,
        adjustable_length: 3.5 * IN,
        adjustable_face: 1.0 * IN,
    },
    ModVisePreset {
        label: "2 in grid 1/2-13",
        pitch: 2.0 * IN,
        bolt_span: 2.0 * IN,
        bolt_diameter: 0.5 * IN,
        body_width: 6.0 * IN,
        body_height: 2.0 * IN,
        jaw_height: 1.75 * IN,
        jaw_thickness: 0.5 * IN,
        fixed_length: 3.0 * IN,
        fixed_face: 2.0 * IN,
        adjustable_length: 4.5 * IN,
        adjustable_face: 1.25 * IN,
    },
    ModVisePreset {
        label: "50 mm grid M12",
        pitch: 50.0,
        bolt_span: 50.0,
        bolt_diameter: 12.0,
        body_width: 150.0,
        body_height: 50.0,
        jaw_height: 45.0,
        jaw_thickness: 12.0,
        fixed_length: 75.0,
        fixed_face: 50.0,
        adjustable_length: 112.0,
        adjustable_face: 30.0,
    },
];

/// The preset labelled `label` (case and spaces ignored).
#[must_use]
pub fn preset(label: &str) -> Option<&'static ModVisePreset> {
    let key = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    let want = key(label);
    PRESETS.iter().find(|p| key(p.label) == want)
}
