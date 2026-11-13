//! The column read from below (#637 E2b): `march`'s mirror, rising
//! from the world floor to the LOWEST material centre and reading the
//! material's floor from there. After a baked flip (`field_turn`) the
//! field holds cuts on both sides - the earlier setup's solids are
//! monotone downward in the new frame, the next setup's monotone
//! upward - so a column's material is one interval `[max(mesh floor,
//! lower cut's top), min(mesh top, upper cut's bottom)]`: the top is
//! `march`'s, the floor is this module's.
//!
//! ## The laws here
//!
//! - **Same walk, same proof.** The ascent is `march::walk` run the
//!   other way: every centre's sign is the exact parity-and-claim sign,
//!   every skip is a proven-air run, and the first material centre met
//!   is the stock's own lowest - so the centre below it is air and the
//!   floor lies in that one-cell bracket. A stock plane on a cell
//!   centre reads by parity like any other (the half-open law: a
//!   centre on the floor plane is material, on the top plane air),
//!   never as "the surface is here" for the whole cell (the review of
//!   #817: a flipped floor read 0.25 high from exactly that shortcut).
//! - **The floor is read two ways, the HIGHER binding.** At the lowest
//!   material centre: the cut layer's downward entry (`cut_downs` -
//!   the distance DOWN from the centre to the ray's entry into a swept
//!   solid, a flipped bake's `cut_ups` record moved whole), and the
//!   shell's lowest crossing in the bracket (exact for any plane,
//!   lattice-aligned or not). A cut top above the mesh floor is the
//!   floor; a mesh floor above the cut's top is. Neither on record is
//!   `Unreadable` - a surface the field cannot place is never read as
//!   the cell boundary (a flipped bounded tool's bottom face).
//! - **Over-report, never under.** A floor is only ever read inside a
//!   bracket whose lower end is proven air, from a record that errs
//!   low or a crossing that is exact: material is never hidden.
//!
//! Provenance: sphere-tracing of a signed-distance field along a ray
//! (the published marching law); this codebase's own parity sign
//! (`solid_rest`); the mirror of `march` by construction. Nothing from
//! vendor binaries.

use super::field::SdfField;
use super::march::{walk, Read};

/// How one column's ascent ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ReadUp {
    /// The floor of the lowest material, bracketed inside its cell.
    Floor(f64),
    /// The ray reached the world ceiling with material never found.
    NoMaterial,
    /// The march could not finish inside its budget, or the floor's
    /// surface is one the field cannot place: no verdict.
    Unreadable,
}

/// Walk up `(x, y)` from `z_lo` until the steel's floor (or a bail
/// state names why there is none).
pub(crate) fn march_up(
    f: &SdfField,
    cx: &Option<Vec<f64>>,
    x: f64,
    y: f64,
    z_lo: f64,
    z_hi: f64,
    cuts: bool,
) -> ReadUp {
    match walk(f, cx, x, y, z_lo, z_hi, cuts) {
        Read::Top(z) => ReadUp::Floor(z),
        Read::NoMaterial => ReadUp::NoMaterial,
        Read::Unreadable => ReadUp::Unreadable,
    }
}
