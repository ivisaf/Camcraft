//! Fixture plates (#819, part of #339): a grid (tooling) plate - a slab
//! with a regular lattice of tapped holes, an optional dowel pattern
//! and optional counterbores - placed as an ordinary fixture, and the
//! grid other fixtures are placed on by named location.
//!
//! Two kinds of plate share one grid model ([`PlateDef`]): a built-in,
//! parametric plate (`PlateParams`, its mesh generated as a SOLID slab -
//! holes drawn solid, which is conservative for the fixture laws), and
//! an imported plate whose grid is derived from its detected holes
//! (`detect`).
//!
//! **The grid convention.** Locations are named like a spreadsheet:
//! letters for the row, numbers for the column. In the plate's own
//! frame, columns count along +X from 1 and rows along +Y from A (A, B,
//! ..., Z, AA, AB, ...). A1 is the hole nearest the frame's origin
//! corner; B3 is the second row's third hole. A built-in plate's frame
//! has its origin at the plate's corner on its TOP face (x along the
//! length, y along the width, the slab below z = 0), so fixtures stand
//! on z = 0. Dowel holes form their own lattice, named the same way.
//!
//! Provenance: our own model and naming; grid plates with tapped and
//! dowel lattices are common shop workholding (machine-shop handbooks).
//! The presets are generic sizes of our own; no vendor model or
//! dimension is copied (`docs/decisions/819-fixture-plates.md`).

use serde::{Deserialize, Serialize};

use crate::geometry::{Point3, Triangle};

pub mod detect;
pub mod grid;
pub mod mount;
pub mod table;

pub use grid::{Grid, location_name, parse_location, row_letters};
pub use mount::{MountAt, PlateMount, RiderMove};
pub use table::{PRESETS, PlatePreset, preset};

/// The most holes one lattice may name (a bound on every loop over it).
pub const MAX_HOLES: u64 = 20_000;

/// An optional dowel-hole lattice on a built-in plate (mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dowels {
    pub pitch_x: f64,
    pub pitch_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub diameter: f64,
}

/// Optional counterbores on the grid holes (mm), recorded for the
/// markers and the setup sheet; the collision slab stays solid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Counterbore {
    pub diameter: f64,
    pub depth: f64,
}

/// A built-in plate's parameters (mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlateParams {
    /// The preset it started from; free text once edited.
    pub size: String,
    /// Along the plate frame's X.
    pub length: f64,
    /// Along the plate frame's Y.
    pub width: f64,
    pub thickness: f64,
    pub pitch_x: f64,
    pub pitch_y: f64,
    /// A1's distance from the origin corner, along X and Y. The lattice
    /// runs on while a hole keeps at least this margin to the far edge.
    pub offset_x: f64,
    pub offset_y: f64,
    /// The thread's name ("3/8-16", "M10").
    pub thread: String,
    /// The thread's major diameter.
    pub thread_diameter: f64,
    #[serde(default)]
    pub dowels: Option<Dowels>,
    #[serde(default)]
    pub counterbore: Option<Counterbore>,
}

/// What makes a fixture a plate: its hole grid (and dowel grid) in its
/// model frame, the top face fixtures stand on, and - for a built-in
/// plate - the parameters its mesh and grid were generated from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlateDef {
    #[serde(default)]
    pub params: Option<PlateParams>,
    pub top_z: f64,
    pub holes: Grid,
    #[serde(default)]
    pub dowels: Option<Grid>,
}

/// A fixture's plate roles, riding its `FixtureDef`: it may BE a plate,
/// and it may be placed ON one (a subplate is both). Default: neither.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlateLink {
    pub plate: Option<PlateDef>,
    pub mount: Option<PlateMount>,
}

/// Lattice nodes from `offset` while a node keeps `offset` to the far
/// edge `span` (the symmetric-margin rule), at least one.
fn count(span: f64, offset: f64, pitch: f64) -> u64 {
    let n = ((span - 2.0 * offset) / pitch + 1e-9).floor();
    if n.is_finite() && n >= 0.0 { (n as u64).saturating_add(1) } else { 0 }
}

impl PlateParams {
    fn named(&self) -> [(&'static str, f64); 7] {
        [
            ("length", self.length),
            ("width", self.width),
            ("thickness", self.thickness),
            ("pitch X", self.pitch_x),
            ("pitch Y", self.pitch_y),
            ("thread diameter", self.thread_diameter),
            ("first-hole offset X", self.offset_x),
        ]
    }

    /// The plate law. Every figure a positive, finite length; the first
    /// hole far enough in that the hole is on the plate; holes narrower
    /// than the pitch; dowels clear of the tapped holes; a counterbore
    /// wider than the thread, narrower than the pitch and shallower than
    /// the plate. Refusals name the figure.
    pub fn check(&self) -> Result<(), String> {
        let mut figs = self.named().to_vec();
        figs.push(("first-hole offset Y", self.offset_y));
        for (what, v) in figs {
            if !v.is_finite() {
                return Err(format!("plate {what} must be a finite length (got {v})"));
            }
            if v <= 0.0 {
                return Err(format!("plate {what} must be more than zero (got {v} mm)"));
            }
        }
        let r = 0.5 * self.thread_diameter;
        for (axis, off, span) in [("X", self.offset_x, self.length), ("Y", self.offset_y, self.width)] {
            if off < r {
                return Err(format!("plate first-hole offset {axis} {off:.3} mm puts the hole over the edge (thread radius {r:.3} mm)"));
            }
            if 2.0 * off > span {
                return Err(format!("plate first-hole offset {axis} {off:.3} mm is more than half the plate ({span:.3} mm)"));
            }
        }
        if self.thread_diameter >= self.pitch_x.min(self.pitch_y) {
            return Err(format!(
                "plate thread diameter {:.3} mm must be smaller than the pitch {:.3} mm",
                self.thread_diameter,
                self.pitch_x.min(self.pitch_y)
            ));
        }
        if self.thread.trim().is_empty() {
            return Err("plate thread needs a name (e.g. 3/8-16, M10)".into());
        }
        let holes = count(self.length, self.offset_x, self.pitch_x).saturating_mul(count(self.width, self.offset_y, self.pitch_y));
        if holes > MAX_HOLES {
            return Err(format!("plate grid has {holes} holes, more than {MAX_HOLES}"));
        }
        if let Some(c) = &self.counterbore {
            for (what, v) in [("counterbore diameter", c.diameter), ("counterbore depth", c.depth)] {
                if !(v.is_finite() && v > 0.0) {
                    return Err(format!("plate {what} must be a length more than zero (got {v})"));
                }
            }
            if c.diameter <= self.thread_diameter {
                return Err(format!("plate counterbore diameter {:.3} mm must be wider than the thread {:.3} mm", c.diameter, self.thread_diameter));
            }
            if c.diameter >= self.pitch_x.min(self.pitch_y) {
                return Err(format!("plate counterbore diameter {:.3} mm must be smaller than the pitch", c.diameter));
            }
            if c.depth >= self.thickness {
                return Err(format!("plate counterbore depth {:.3} mm must be less than the thickness {:.3} mm", c.depth, self.thickness));
            }
        }
        if let Some(d) = &self.dowels {
            self.check_dowels(d)?;
        }
        Ok(())
    }

    fn check_dowels(&self, d: &Dowels) -> Result<(), String> {
        for (what, v) in [
            ("dowel pitch X", d.pitch_x),
            ("dowel pitch Y", d.pitch_y),
            ("dowel offset X", d.offset_x),
            ("dowel offset Y", d.offset_y),
            ("dowel diameter", d.diameter),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("plate {what} must be a length more than zero (got {v})"));
            }
        }
        let r = 0.5 * d.diameter;
        if d.offset_x < r || d.offset_y < r || 2.0 * d.offset_x > self.length || 2.0 * d.offset_y > self.width {
            return Err("plate dowel offset puts the first dowel hole off the plate".into());
        }
        if d.diameter >= d.pitch_x.min(d.pitch_y) {
            return Err(format!("plate dowel diameter {:.3} mm must be smaller than the dowel pitch", d.diameter));
        }
        let n = count(self.length, d.offset_x, d.pitch_x).saturating_mul(count(self.width, d.offset_y, d.pitch_y));
        if n > MAX_HOLES {
            return Err(format!("plate dowel pattern has {n} holes, more than {MAX_HOLES}"));
        }
        // Clear of every tapped hole: the nearest lattice node to each
        // dowel, by rounding, is the only one that can touch it.
        let g = self.grid();
        let clear = 0.5 * (d.diameter + self.thread_diameter);
        for (name, x, y) in self.dowel_grid().map(|g| g.holes()).unwrap_or_default() {
            let c = ((x - g.x0) / g.pitch_x).round().clamp(0.0, f64::from(g.cols.saturating_sub(1)));
            let r = ((y - g.y0) / g.pitch_y).round().clamp(0.0, f64::from(g.rows.saturating_sub(1)));
            let (hx, hy) = g.node(r as u32, c as u32);
            if (x - hx).hypot(y - hy) < clear {
                return Err(format!(
                    "plate dowel hole {name} runs into tapped hole {} - move the dowel pattern",
                    location_name(r as u32, c as u32)
                ));
            }
        }
        Ok(())
    }

    /// The tapped-hole lattice (unchecked params give an empty one).
    #[must_use]
    pub fn grid(&self) -> Grid {
        let cols = u32::try_from(count(self.length, self.offset_x, self.pitch_x)).unwrap_or(0);
        let rows = u32::try_from(count(self.width, self.offset_y, self.pitch_y)).unwrap_or(0);
        Grid {
            x0: self.offset_x,
            y0: self.offset_y,
            pitch_x: self.pitch_x,
            pitch_y: self.pitch_y,
            cols,
            rows,
            diameter: self.thread_diameter,
            missing: Vec::new(),
            stagger: None,
        }
    }

    /// The dowel lattice, when the plate has one.
    #[must_use]
    pub fn dowel_grid(&self) -> Option<Grid> {
        let d = self.dowels.as_ref()?;
        Some(Grid {
            x0: d.offset_x,
            y0: d.offset_y,
            pitch_x: d.pitch_x,
            pitch_y: d.pitch_y,
            cols: u32::try_from(count(self.length, d.offset_x, d.pitch_x)).unwrap_or(0),
            rows: u32::try_from(count(self.width, d.offset_y, d.pitch_y)).unwrap_or(0),
            diameter: d.diameter,
            missing: Vec::new(),
            stagger: None,
        })
    }

    /// The checked plate's grid definition.
    pub fn def(&self) -> Result<PlateDef, String> {
        self.check()?;
        Ok(PlateDef { params: Some(self.clone()), top_z: 0.0, holes: self.grid(), dowels: self.dowel_grid() })
    }

    /// The checked plate's solid slab: x in [0, length], y in [0, width],
    /// z in [-thickness, 0]; 12 outward triangles, holes drawn solid.
    pub fn mesh(&self) -> Result<Vec<Triangle>, String> {
        self.check()?;
        Ok(slab(self.length, self.width, self.thickness))
    }

    /// The model name a fresh plate lands under: "Plate 12 x 12 in".
    #[must_use]
    pub fn base_name(&self) -> String {
        let s = self.size.trim();
        if s.is_empty() { "Fixture plate".to_owned() } else { format!("Plate {s}") }
    }
}

/// A closed box [0, l] x [0, w] x [-t, 0], outward normals.
#[must_use]
pub fn slab(l: f64, w: f64, t: f64) -> Vec<Triangle> {
    let p = |i: usize| {
        Point3::new(if i & 1 == 0 { 0.0 } else { l }, if i & 2 == 0 { 0.0 } else { w }, if i & 4 == 0 { -t } else { 0.0 })
    };
    // Each face as a quad (a, b, c, d) counter-clockwise seen from outside.
    let quads = [
        [0, 2, 3, 1], // bottom (-z)
        [4, 5, 7, 6], // top (+z)
        [0, 1, 5, 4], // -y
        [2, 6, 7, 3], // +y
        [0, 4, 6, 2], // -x
        [1, 3, 7, 5], // +x
    ];
    quads
        .iter()
        .flat_map(|q| [Triangle::new(p(q[0]), p(q[1]), p(q[2])), Triangle::new(p(q[0]), p(q[2]), p(q[3]))])
        .collect()
}

impl PlateDef {
    /// The law a loaded or derived plate keeps: its grids are sane, and a
    /// built-in plate's grids are the ones its parameters make.
    pub fn check(&self) -> Result<(), String> {
        if !self.top_z.is_finite() {
            return Err("plate top must be finite".into());
        }
        self.holes.check("plate grid")?;
        if let Some(d) = &self.dowels {
            d.check("plate dowel pattern")?;
        }
        if let Some(p) = &self.params {
            p.check()?;
            if p.grid() != self.holes || p.dowel_grid() != self.dowels {
                return Err("plate grid does not match its parameters".into());
            }
        }
        Ok(())
    }

    /// The dowel lattice, or why the plate has none.
    pub fn dowel_lattice(&self) -> Result<&Grid, String> {
        self.dowels.as_ref().ok_or_else(|| "it has no dowel pattern".to_owned())
    }
}
