//! The stock edits a generator makes across the seam (M7.3 4/6): the
//! flat atom a planner's feed sampler and quality yardstick cut with,
//! and the column-top rewrite their measuring stocks are seeded by -
//! so motion holds a `Stock`, never a `Dexel`.
//!
//! Provenance: the column arm is `Dexel::subtract` / `Dexel::cut_chord`
//! / `Dexel::set` unchanged (M2.1); the field arm cuts the same flat
//! tool through `SdfField::subtract` with the spanning flank of
//! `backend`. Nothing from vendor binaries.
//!
//! ## The laws here
//!
//! - **The column arm is the old call, bit for bit.** A generator that
//!   moved onto the seam reads and writes exactly the heights it did
//!   through `Dexel`, so every golden stays byte-identical.
//! - **Upright or refused.** The column atom stands on +Z; a posed
//!   cutting move is refused by name (`POSED_REFUSAL`) before a column
//!   is touched, never cut as if upright.
//! - **Column tops are a column word.** Rewriting a column's top has
//!   no field meaning short of a solid union, so the field arm refuses
//!   `set_tops` by name rather than pretend.

use super::backend::{named_op, spanning_flank, Stock};
use super::cut::{first_posed, POSED_REFUSAL};
use super::field_cut::SdfTool;
use super::kernel::StockError;

use crate::cl::{Move, MoveKind};
use crate::geo2d::Point2;
use crate::geometry::Point3;

/// The field arm's refusal of a column-top rewrite, said once.
pub const SET_TOPS_FIELD_REFUSAL: &str =
    "column tops are rewritten on the column arm only - the field arm takes material as a solid, not as heights";

impl Stock {
    /// Lower the stock by one move of the upright flat tool of radius
    /// `r`, arcs walked at `chord_tol` - `Dexel::subtract` on the
    /// column arm. A rapid or non-finite move cuts nothing.
    pub fn subtract_flat(&mut self, m: &Move, r: f64, chord_tol: f64) -> Result<(), StockError> {
        if let Some(k) = first_posed(std::slice::from_ref(m)) {
            return Err(StockError::Invalid(format!("move {k}: {POSED_REFUSAL}")));
        }
        match self {
            Stock::Column(d) => {
                d.subtract(m, r, chord_tol);
                Ok(())
            }
            Stock::Field(f) => {
                if m.kind == MoveKind::Rapid || !m.is_finite() {
                    return Ok(());
                }
                let tool = SdfTool::Flat { r, flank: spanning_flank(f) };
                f.subtract(m, &tool, chord_tol).map_err(|e| named_op(0, e))
            }
        }
    }

    /// The flat atom along one straight, Z-linear chord `a -> b` -
    /// `Dexel::cut_chord` on the column arm. Non-finite input cuts
    /// nothing, as there.
    pub fn cut_chord(&mut self, a: Point3, b: Point3, r: f64) -> Result<(), StockError> {
        match self {
            Stock::Column(d) => {
                d.cut_chord(a, b, r);
                Ok(())
            }
            Stock::Field(f) => {
                if !(a.is_finite() && b.is_finite()) || !(r.is_finite() && r >= 0.0) {
                    return Ok(());
                }
                let chord_tol = (f.cell() * 0.25).max(1e-9);
                let tool = SdfTool::Flat { r, flank: spanning_flank(f) };
                f.subtract(&Move::new(MoveKind::Feed, a, b, None), &tool, chord_tol)
            }
        }
    }

    /// Rewrite every column's top: `top(k, center, z)` answers the new
    /// height of column `k` (row-major), centred at `center`, standing
    /// at `z` now. A column whose answer is its current top is left
    /// untouched (no tile is split for it). Column arm only - see
    /// [`SET_TOPS_FIELD_REFUSAL`].
    pub fn set_tops(&mut self, mut top: impl FnMut(usize, Point2, f64) -> f64) -> Result<(), StockError> {
        match self {
            Stock::Column(d) => {
                for iy in 0..d.ny {
                    for ix in 0..d.nx {
                        let k = ix + iy * d.nx;
                        let cur = d.top_at(k);
                        let z = top(k, d.center(ix, iy), cur);
                        if z.to_bits() != cur.to_bits() {
                            d.set(k, z);
                        }
                    }
                }
                Ok(())
            }
            Stock::Field(_) => Err(StockError::Invalid(SET_TOPS_FIELD_REFUSAL.into())),
        }
    }
}
