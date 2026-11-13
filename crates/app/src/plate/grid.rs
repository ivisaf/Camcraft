//! A plate's hole lattice and its names (#819): rows lettered along +Y
//! from A (A..Z, AA, AB, ...), columns numbered along +X from 1; A1 is
//! the node nearest the frame's origin corner. See `plate` for the frame.
//!
//! Provenance: our own naming (spreadsheet-style bijective base 26).

use serde::{Deserialize, Serialize};

use super::MAX_HOLES;

/// A rectangular lattice of holes in a plate's model frame: A1 at
/// (`x0`, `y0`), columns `pitch_x` apart along +X, rows `pitch_y` apart
/// along +Y. `missing` names lattice nodes with no hole (an imported
/// plate's gaps); placing there is refused by name. A STAGGERED grid
/// (`stagger: Some(p)`, an imported checkerboard plate) has holes only
/// at the nodes whose row + column parity is `p`; the names stay those
/// of the full lattice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub x0: f64,
    pub y0: f64,
    pub pitch_x: f64,
    pub pitch_y: f64,
    pub cols: u32,
    pub rows: u32,
    pub diameter: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stagger: Option<u8>,
}

/// Row letters for zero-based row `r`: 0 -> A, 25 -> Z, 26 -> AA.
#[must_use]
pub fn row_letters(r: u32) -> String {
    let mut n = u64::from(r) + 1;
    let mut out = Vec::new();
    while n > 0 {
        let k = (n - 1) % 26;
        out.push(b'A' + k as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// The name of zero-based (`row`, `col`): "B3" is row 1, column 2.
#[must_use]
pub fn location_name(row: u32, col: u32) -> String {
    format!("{}{}", row_letters(row), u64::from(col) + 1)
}

/// Zero-based (row, col) of a location name (case and spaces ignored);
/// `None` when it is not letters then a column number from 1.
#[must_use]
pub fn parse_location(s: &str) -> Option<(u32, u32)> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
    let split = s.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = s.split_at(split);
    if letters.is_empty() || letters.len() > 4 || !letters.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut row: u64 = 0;
    for b in letters.bytes() {
        row = row * 26 + u64::from(b - b'A') + 1;
    }
    let col: u64 = digits.parse().ok()?;
    if col == 0 {
        return None;
    }
    Some((u32::try_from(row - 1).ok()?, u32::try_from(col - 1).ok()?))
}

impl Grid {
    /// The grid's span of names, as refusals say it: "rows A-L, columns 1-12".
    #[must_use]
    pub fn extent(&self) -> String {
        format!(
            "rows A-{}, columns 1-{}",
            row_letters(self.rows.saturating_sub(1)),
            self.cols
        )
    }

    /// The (x, y) of a node, unchecked.
    #[must_use]
    pub fn node(&self, row: u32, col: u32) -> (f64, f64) {
        (self.x0 + f64::from(col) * self.pitch_x, self.y0 + f64::from(row) * self.pitch_y)
    }

    /// The (x, y) of location `name`, or why not (an unknown name, a
    /// node off the lattice, or a node with no hole).
    pub fn locate(&self, name: &str) -> Result<(f64, f64), String> {
        let (row, col) =
            parse_location(name).ok_or_else(|| format!("'{name}' is not a grid location (letters for the row, then the column number, e.g. B3)"))?;
        if row >= self.rows || col >= self.cols {
            return Err(format!("there is no location {} ({})", name.trim(), self.extent()));
        }
        let canon = location_name(row, col);
        if self.off_stagger(row, col) {
            return Err(format!("location {canon} has no hole (the grid is staggered: every other place along a row)"));
        }
        if self.missing.contains(&canon) {
            return Err(format!("location {canon} has no hole"));
        }
        Ok(self.node(row, col))
    }

    /// Whether node (`row`, `col`) is a gap of a staggered grid.
    #[must_use]
    pub fn off_stagger(&self, row: u32, col: u32) -> bool {
        self.stagger.is_some_and(|p| (u64::from(row) + u64::from(col)) % 2 != u64::from(p))
    }

    /// Every hole: (name, x, y), row by row from A1. Bounded by
    /// [`MAX_HOLES`] (a checked grid never reaches it).
    #[must_use]
    pub fn holes(&self) -> Vec<(String, f64, f64)> {
        let mut out = Vec::new();
        'rows: for r in 0..self.rows {
            for c in 0..self.cols {
                if out.len() as u64 >= MAX_HOLES {
                    break 'rows;
                }
                let name = location_name(r, c);
                if self.off_stagger(r, c) || self.missing.contains(&name) {
                    continue;
                }
                let (x, y) = self.node(r, c);
                out.push((name, x, y));
            }
        }
        out
    }

    /// The grid law: finite, positive pitches and diameter, holes clear
    /// of each other (the nearest two more than a diameter apart), at
    /// least one node each way, at most [`MAX_HOLES`] nodes.
    pub fn check(&self, what: &str) -> Result<(), String> {
        for (k, v) in [("x", self.x0), ("y", self.y0)] {
            if !v.is_finite() {
                return Err(format!("{what} first hole {k} must be finite"));
            }
        }
        for (k, v) in [("pitch X", self.pitch_x), ("pitch Y", self.pitch_y), ("hole diameter", self.diameter)] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("{what} {k} must be a length more than zero (got {v})"));
            }
        }
        if self.stagger.is_some_and(|p| p > 1) {
            return Err(format!("{what} stagger must be 0 or 1"));
        }
        // Holes must not run into each other, and the pitch must stand
        // clear of the detection tolerance (#819 review): the nearest
        // two holes of the lattice - along a row or column, or across
        // the diagonal of a staggered one - more than a hole apart.
        let near = match self.stagger {
            None => self.pitch_x.min(self.pitch_y),
            Some(_) => (2.0 * self.pitch_x).min(2.0 * self.pitch_y).min(self.pitch_x.hypot(self.pitch_y)),
        };
        if self.pitch_x.min(self.pitch_y) <= 2.0 * super::detect::ON_NODE {
            return Err(format!("{what} pitch must be more than {:.2} mm", 2.0 * super::detect::ON_NODE));
        }
        if near <= self.diameter {
            return Err(format!(
                "{what} holes of {:.3} mm would run into each other {near:.3} mm apart - the pitch must be more than the hole diameter",
                self.diameter
            ));
        }
        if self.rows == 0 || self.cols == 0 {
            return Err(format!("{what} has no holes"));
        }
        if u64::from(self.rows) * u64::from(self.cols) > MAX_HOLES {
            return Err(format!("{what} has more than {MAX_HOLES} holes"));
        }
        Ok(())
    }
}

