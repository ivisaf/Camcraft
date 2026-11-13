//! The built-in fixture plate presets (#819): generic grid plates of our
//! own round figures - an inch plate on a 1 in grid, two metric plates
//! on 25 mm and 50 mm grids, and one sized from the Tormach 770MX
//! machine file's table (its X/Y travel spans, the table box the fixture
//! laws already derive). No vendor's plate model or dimension is copied
//! (`docs/decisions/819-fixture-plates.md`); the user edits any figure
//! to match their own plate.
//!
//! Provenance: our own round figures; common thread sizes (3/8-16 UNC,
//! M8, M12) from public thread standards.

use super::{Counterbore, Dowels, PlateParams};

const IN: f64 = 25.4;

/// One preset: a label and a builder (the figures are millimetres).
#[derive(Clone, Copy, Debug)]
pub struct PlatePreset {
    /// What the list shows.
    pub label: &'static str,
    build: fn() -> PlateParams,
}

impl PlatePreset {
    /// The preset's parameters.
    #[must_use]
    pub fn params(&self) -> PlateParams {
        (self.build)()
    }
}

/// An inch plate on a square 1 in grid of 3/8-16 holes, A1 half a pitch in.
fn inch_plate(size: &str, length_in: f64, width_in: f64) -> PlateParams {
    PlateParams {
        size: size.to_owned(),
        length: length_in * IN,
        width: width_in * IN,
        thickness: 0.75 * IN,
        pitch_x: IN,
        pitch_y: IN,
        offset_x: 0.5 * IN,
        offset_y: 0.5 * IN,
        thread: "3/8-16".to_owned(),
        thread_diameter: 0.375 * IN,
        dowels: None,
        counterbore: None,
    }
}

fn plate_12in() -> PlateParams {
    inch_plate("12 x 12 in", 12.0, 12.0)
}

/// 300 x 300 mm, M8 on 25 mm, with a 10 mm dowel lattice on 50 mm set
/// between the tapped holes.
fn plate_300mm() -> PlateParams {
    PlateParams {
        size: "300 x 300 mm".to_owned(),
        length: 300.0,
        width: 300.0,
        thickness: 20.0,
        pitch_x: 25.0,
        pitch_y: 25.0,
        offset_x: 12.5,
        offset_y: 12.5,
        thread: "M8".to_owned(),
        thread_diameter: 8.0,
        dowels: Some(Dowels { pitch_x: 50.0, pitch_y: 50.0, offset_x: 25.0, offset_y: 25.0, diameter: 10.0 }),
        counterbore: None,
    }
}

/// 400 x 300 mm, M12 on 50 mm, counterbored 18 mm x 12 mm.
fn plate_400mm() -> PlateParams {
    PlateParams {
        size: "400 x 300 mm".to_owned(),
        length: 400.0,
        width: 300.0,
        thickness: 30.0,
        pitch_x: 50.0,
        pitch_y: 50.0,
        offset_x: 25.0,
        offset_y: 25.0,
        thread: "M12".to_owned(),
        thread_diameter: 12.0,
        dowels: None,
        counterbore: Some(Counterbore { diameter: 18.0, depth: 12.0 }),
    }
}

/// Sized from the shipped `tormach-770mx.mch`: its table box is the X/Y
/// travel spans, 28 x 15 in (pinned against the machine file by the
/// tests), on a 1 in grid of 3/8-16.
fn plate_770mx() -> PlateParams {
    inch_plate("770MX table 28 x 15 in", 28.0, 15.0)
}

/// Every preset, in list order.
pub const PRESETS: &[PlatePreset] = &[
    PlatePreset { label: "12 x 12 in", build: plate_12in },
    PlatePreset { label: "300 x 300 mm", build: plate_300mm },
    PlatePreset { label: "400 x 300 mm", build: plate_400mm },
    PlatePreset { label: "770MX table 28 x 15 in", build: plate_770mx },
];

/// The preset labelled `label` (case and spaces ignored).
#[must_use]
pub fn preset(label: &str) -> Option<&'static PlatePreset> {
    let key = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    let want = key(label);
    PRESETS.iter().find(|p| key(p.label) == want)
}
