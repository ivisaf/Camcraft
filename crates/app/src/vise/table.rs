//! The built-in vise size table (#788): typical figures for generic
//! milling-machine vises at the common jaw widths, 3-8 in and 100-200 mm.
//! Every row is our own rounded, generic proportion inside the ranges
//! public catalog spec sheets print for vises of that jaw width (see
//! `docs/decisions/788-builtin-vises.md`) - no vendor's model is copied.
//! The user edits any figure to match their own vise.
//!
//! Provenance: our own table from published catalog ranges (jaw width,
//! jaw height, max opening, overall length/width/height).

use super::{ViseParams, ViseStyle};

/// One row: a label and its figures, all millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViseSize {
    /// What the list shows ("6 in", "150 mm").
    pub label: &'static str,
    pub jaw_width: f64,
    pub jaw_height: f64,
    pub jaw_thickness: f64,
    /// Each jaw's solid body depth (plate included): `BODY_PER_PLATE`
    /// jaw thicknesses, a conservative default the user edits.
    pub jaw_body_depth: f64,
    pub max_opening: f64,
    pub length: f64,
    pub width: f64,
    pub base_height: f64,
    pub housing_length: f64,
    pub housing_height: f64,
    /// Standard, or self-centering with its clamping range floor (#787).
    pub style: ViseStyle,
}

const IN: f64 = 25.4;

/// A jaw's body depth (plate plus the steel behind it, along the screw)
/// as a multiple of its plate thickness. 2x is the default: it fits every
/// row's length with two such bodies, its max opening and housing
/// (`docs/decisions/788-builtin-vises.md`). Edit it to the real vise.
pub const BODY_PER_PLATE: f64 = 2.0;

/// An imperial row, written in inches.
#[allow(clippy::too_many_arguments)]
const fn inch(
    label: &'static str,
    jaw_width: f64,
    jaw_height: f64,
    jaw_thickness: f64,
    max_opening: f64,
    length: f64,
    width: f64,
    base_height: f64,
    housing_length: f64,
    housing_height: f64,
) -> ViseSize {
    ViseSize {
        label,
        jaw_width: jaw_width * IN,
        jaw_height: jaw_height * IN,
        jaw_thickness: jaw_thickness * IN,
        jaw_body_depth: BODY_PER_PLATE * jaw_thickness * IN,
        max_opening: max_opening * IN,
        length: length * IN,
        width: width * IN,
        base_height: base_height * IN,
        housing_length: housing_length * IN,
        housing_height: housing_height * IN,
        style: ViseStyle::Standard,
    }
}

/// A metric row, written in millimetres.
#[allow(clippy::too_many_arguments)]
const fn mm(
    label: &'static str,
    jaw_width: f64,
    jaw_height: f64,
    jaw_thickness: f64,
    max_opening: f64,
    length: f64,
    width: f64,
    base_height: f64,
    housing_length: f64,
    housing_height: f64,
) -> ViseSize {
    ViseSize {
        label,
        jaw_width,
        jaw_height,
        jaw_thickness,
        jaw_body_depth: BODY_PER_PLATE * jaw_thickness,
        max_opening,
        length,
        width,
        base_height,
        housing_length,
        housing_height,
        style: ViseStyle::Standard,
    }
}

/// The built-in sizes, imperial first, each in ascending jaw width.
/// Columns: jaw width, jaw height, jaw thickness, max opening, overall
/// length, overall width, base height (floor to table), screw-housing
/// length and height (above the floor).
pub const SIZES: &[ViseSize] = &[
    inch("3 in", 3.0, 1.3125, 1.0, 4.75, 12.0, 4.0, 2.0, 1.5, 0.75),
    inch("4 in", 4.0, 1.25, 1.25, 6.0, 14.0, 5.25, 2.25, 1.75, 1.0),
    inch("5 in", 5.0, 1.5, 1.375, 7.0, 16.0, 6.25, 2.5, 2.0, 1.0),
    inch("6 in", 6.0, 1.75, 1.5, 9.0, 18.5, 7.5, 2.875, 2.25, 1.25),
    inch("8 in", 8.0, 2.0, 1.75, 8.25, 20.0, 9.5, 3.25, 2.5, 1.5),
    mm("100 mm", 100.0, 32.0, 25.0, 110.0, 300.0, 125.0, 55.0, 35.0, 20.0),
    mm("125 mm", 125.0, 40.0, 30.0, 160.0, 360.0, 150.0, 65.0, 40.0, 25.0),
    mm("150 mm", 150.0, 45.0, 35.0, 200.0, 420.0, 180.0, 70.0, 45.0, 30.0),
    mm("160 mm", 160.0, 50.0, 38.0, 220.0, 450.0, 190.0, 75.0, 50.0, 30.0),
    mm("200 mm", 200.0, 62.0, 45.0, 260.0, 550.0, 240.0, 85.0, 60.0, 35.0),
];

/// Every built-in row: the standard table, then the 5-axis
/// self-centering vises (#787). The dialog and the MCP size list read
/// this; an index into it is stable for a build.
pub fn presets() -> impl Iterator<Item = &'static ViseSize> {
    SIZES.iter().chain(super::self_centering::SIZES.iter())
}

/// Row `k` of `presets()`.
#[must_use]
pub fn preset(k: usize) -> Option<&'static ViseSize> {
    presets().nth(k)
}

/// The row labelled `label` among `presets()` (exact, then case- and
/// space-blind).
#[must_use]
pub fn size(label: &str) -> Option<&'static ViseSize> {
    let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    presets().find(|s| s.label == label).or_else(|| presets().find(|s| squash(s.label) == squash(label)))
}

impl ViseSize {
    /// This size's parameters with the jaws `opening` mm apart.
    #[must_use]
    pub fn params(&self, opening: f64) -> ViseParams {
        ViseParams {
            size: self.label.to_owned(),
            jaw_width: self.jaw_width,
            jaw_height: self.jaw_height,
            jaw_thickness: self.jaw_thickness,
            jaw_body_depth: self.jaw_body_depth,
            max_opening: self.max_opening,
            length: self.length,
            width: self.width,
            base_height: self.base_height,
            housing_length: self.housing_length,
            housing_height: self.housing_height,
            opening,
            style: self.style,
        }
    }

    /// This size, jaws half open (a fresh vise's starting opening).
    #[must_use]
    pub fn half_open(&self) -> ViseParams {
        self.params(0.5 * self.max_opening)
    }
}
