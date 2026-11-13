//! A carried field read back as a column body (#637 step D): per column
//! of a target lattice in the NEW frame, the top and the floor of the
//! carried, clipped solid - so the carried verdict, display and volume
//! run on either engine through one `Body` API, and an agreement lane
//! can hold the two engines to the same answer.
//!
//! Under a lattice turn a vertical column of the new frame is a vertical
//! column of the field's own (Z kept on +-Z), so each read is one column
//! span (`column_span`, #637 E1): the top by the field's down-march, the
//! floor exactly - the shell's own plane, or under a cut by the monotone
//! law. A flip swaps them: the new top is the old floor. A floor a
//! bounded tool's cut reached is refused by name - never a guess.
//!
//! Provenance: our own design (#637 research comment, steps A-E).

use super::carry::Body;
use super::dexel::Dexel;
use super::field_carry::{CarriedField, Clip};
use super::field_floor::ColumnSpan;
use crate::geometry::Point3;

/// The lattice a carried field is read back onto (new frame).
#[derive(Clone, Copy, Debug)]
pub struct Lattice {
    pub x0: f64,
    pub y0: f64,
    pub cell: f64,
    pub nx: usize,
    pub ny: usize,
}

/// One column's (top, floor) in the field's own frame; `None`: no material.
fn read_column(c: &CarriedField, x: f64, y: f64) -> Result<Option<(f64, f64)>, String> {
    match c.field.column_span(x, y) {
        // A NaN or infinite read is no verdict: refused, never an empty
        // column (review of #656 - an empty column is material the
        // verdict never sees).
        ColumnSpan::Material { top, floor } if !(top.is_finite() && floor.is_finite()) => {
            Err(format!("the carried stock read a non-finite top or floor at ({x:.4}, {y:.4}) - refused rather than guessed"))
        }
        ColumnSpan::Material { top, floor } => Ok(Some((top, floor))),
        ColumnSpan::Empty => Ok(None),
        ColumnSpan::CutFromBelow => Err(format!(
            "a bounded tool's cut reached the floor of the carried stock at ({x:.4}, {y:.4}) - an upward read there is not exact, refused rather than guessed"
        )),
        ColumnSpan::Unreadable => Err(format!("the carried field cannot read its column at ({x:.4}, {y:.4})")),
    }
}

impl CarriedField {
    /// The carried solid as a column body on `lat` (new frame).
    /// `judged_before` is the incoming body's floor flag; a flip anywhere
    /// in the carry chain sets it (the floor has faced the spindle), as
    /// `Body::carry` does - a flip already baked into the bricks included.
    ///
    /// # Errors
    /// A non-finite or empty lattice; a column the field cannot read; a
    /// floor a cut reached (see the module law).
    pub fn column_body(&self, lat: Lattice, judged_before: bool) -> Result<Body, String> {
        if !(lat.x0.is_finite() && lat.y0.is_finite() && lat.cell.is_finite() && lat.cell > 0.0) || lat.nx == 0 || lat.ny == 0 {
            return Err("the carried lattice is not a finite, non-empty grid".to_owned());
        }
        let n = lat.nx.checked_mul(lat.ny).ok_or("the carried lattice is too large")?;
        // The turn's Z: z' = s z + tz, with s = -1 on a flip.
        let (s, tz) = (if self.turn.flips() { -1.0 } else { 1.0 }, self.turn.apply(Point3::new(0.0, 0.0, 0.0)).z);
        let mut top = vec![f64::NEG_INFINITY; n];
        let mut floor = vec![f64::INFINITY; n];
        for jy in 0..lat.ny {
            for jx in 0..lat.nx {
                let cx = lat.x0 + (jx as f64 + 0.5) * lat.cell;
                let cy = lat.y0 + (jy as f64 + 0.5) * lat.cell;
                let (lo_z, hi_z) = match self.clip {
                    Some(Clip::Empty) => continue,
                    Some(Clip::Block(lo, hi)) => {
                        if !(cx > lo.x && cx < hi.x && cy > lo.y && cy < hi.y) {
                            continue;
                        }
                        (lo.z, hi.z)
                    }
                    None => (f64::NEG_INFINITY, f64::INFINITY),
                };
                let q = self.turn.unapply(Point3::new(cx, cy, 0.0));
                let Some((t, f)) = read_column(self, q.x, q.y)? else { continue };
                let (t, f) = if s > 0.0 { (t + tz, f + tz) } else { (tz - f, tz - t) };
                let (t, f) = (t.min(hi_z), f.max(lo_z));
                if t > f {
                    let j = jx + jy * lat.nx;
                    top[j] = t;
                    floor[j] = f;
                }
            }
        }
        let stock_z = top.iter().copied().filter(|v| v.is_finite()).fold(f64::NEG_INFINITY, f64::max);
        let stock_z = if stock_z.is_finite() { stock_z } else { 0.0 };
        let dex = Dexel::from_grid(lat.x0, lat.y0, lat.cell, lat.nx, lat.ny, stock_z, top);
        Body::from_parts(dex, floor, judged_before || self.flipped())
    }
}
