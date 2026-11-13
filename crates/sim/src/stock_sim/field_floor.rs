//! A field column's two sides (#637 steps B, E1 and E2b): its top by
//! the down-march, its FLOOR exactly - from the shell where no cut
//! reaches it, under a cut from above by the monotone law, and under a
//! cut from BELOW (a baked flip, `field_turn`) by the up-march. The
//! column body's commonest re-clamp is a part flipped onto a face an
//! earlier setup machined, holes drilled through it included.
//!
//! ## The laws here
//!
//! - **A spanning tool is monotone on a vertical ray.** Every tool
//!   `Stock::simulate` builds spans the verdict world above its tip
//!   (`spanning_flank`), so if its swept solid holds `(x, y, z)` it holds
//!   `(x, y, z')` for every `z' > z` in the world; a flipped bake turns
//!   that solid upside down, so it holds every `z' < z` instead. The
//!   material left in a column is then ONE interval `[max(mesh floor,
//!   lower cut's top), min(mesh top, upper cut's bottom)]` or nothing.
//! - **The two ends are two marches.** The top is `march`'s (its
//!   `refine` reads `cut_ups`), the floor `march_up`'s (`cut_downs`),
//!   each reading an uncut end from the shell's own crossing - exact for
//!   any plane, lattice-aligned or not. Where no cut ever came from
//!   below (no `cut_downs` at all) the floor under a material centre is
//!   the shell's lowest crossing by the monotone law, with no march.
//! - **A column with no material centre is read from the records.** Cut
//!   through, or a sliver thinner than a cell: the material sits between
//!   two adjacent air centres, and those centres' records place it - the
//!   lower centre's `cut_ups` entry is the upper cut's bottom, the upper
//!   centre's `cut_downs` entry is the lower cut's top, the shell binds
//!   where no record does. Material stands iff floor `<=` top - the
//!   column body's own `top < floor` empty law, same strictness. An air
//!   centre inside the shell proves a cut, so a record that must then
//!   exist and does not is refused by name, never read as air. The scan
//!   is `sliver`, and it is the ONE sliver law: `march::walk` reads a
//!   centre-less column through it before it ever answers "no material"
//!   (the review of #817: a floor skin under a cut read as NaN on every
//!   verdict path that marched alone), so `column_span`, `Stock::top`,
//!   the deviate/measure verdicts, the sheet and the column masks all
//!   see the same skin.
//! - **A bounded tool suspends the laws.** A solid that does not span
//!   the world above its tip (the truncated taper, a bare ball, a lathe
//!   sweep, a posed axis) can leave material above a cut; the field marks
//!   itself (`bounded_cuts`) when one is subtracted, and a floor a cut
//!   came within a cell of is then refused by name (`CutFromBelow`), as
//!   step B did for every cut - never a guess.
//!
//! Provenance: ray-parity crossings of a watertight shell (the `solid_rest`
//! law this codebase already uses); monotonicity of an upward-spanning
//! solid along its axis (elementary set geometry); our own design (#637
//! research comments).

use super::field::{cell_of, center_of, SdfField};
use super::march::{march, world, Read};
use super::march_up::{march_up, ReadUp};

/// One column's floor read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloorRead {
    /// The lowest face of the stock under the column, exact to rounding.
    Floor(f64),
    /// The column crosses no stock at all, or every bit of it was cut away.
    NoMaterial,
    /// A bounded tool's cut has claimed the column near its floor: an
    /// upward read would not be exact, so none is given.
    CutFromBelow,
    /// The column cannot be read (an `Open` parity column, a march that
    /// cannot finish, a record that should exist and does not).
    Unreadable,
}

/// One column's material, both sides (#637 E1, E2b).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnSpan {
    /// Material stands from `floor` up to `top` (`top >= floor`).
    Material { top: f64, floor: f64 },
    /// No material stands on the column: air beside the stock, or cut
    /// through.
    Empty,
    /// A bounded tool's cut reached the floor: no exact read (the module
    /// law).
    CutFromBelow,
    /// The column cannot be read.
    Unreadable,
}

/// The shell's extent on a column: its lowest and highest finite
/// crossing - `None` where the ray crosses nothing (beside the stock).
/// A single crossing is both ends.
pub(crate) fn shell_extent(cross: &[f64]) -> Option<(f64, f64)> {
    let finite = cross.iter().copied().filter(|z| z.is_finite());
    let floor = finite.clone().reduce(f64::min)?;
    let ceiling = finite.reduce(f64::max).unwrap_or(floor);
    Some((floor, ceiling))
}

impl SdfField {
    /// The floor under `(x, y)`: `column_span`'s floor.
    #[must_use]
    pub fn floor_at(&self, x: f64, y: f64) -> FloorRead {
        match self.column_span(x, y) {
            ColumnSpan::Material { floor, .. } => FloorRead::Floor(floor),
            ColumnSpan::Empty => FloorRead::NoMaterial,
            ColumnSpan::CutFromBelow => FloorRead::CutFromBelow,
            ColumnSpan::Unreadable => FloorRead::Unreadable,
        }
    }

    /// The material standing on the column `(x, y)` after every cut, from
    /// both sides: see the module law.
    #[must_use]
    pub fn column_span(&self, x: f64, y: f64) -> ColumnSpan {
        if !(x.is_finite() && y.is_finite()) {
            return ColumnSpan::Unreadable;
        }
        let Some(cross) = self.shell_crossings(x, y) else {
            return ColumnSpan::Unreadable;
        };
        let Some((floor, _)) = shell_extent(&cross) else {
            return ColumnSpan::Empty;
        };
        if self.bounded_cuts && self.cut_near(x, y, floor) {
            return ColumnSpan::CutFromBelow;
        }
        let (z_hi, z_lo) = world(self);
        let cx = Some(cross);
        let top = match march(self, &cx, x, y, z_hi, z_lo, true) {
            Read::Top(top) if top.is_finite() => top,
            Read::Top(_) | Read::Unreadable => return ColumnSpan::Unreadable,
            // The walk has read the sliver itself (the module law): no
            // material means the records proved every pair cut through.
            Read::NoMaterial => return ColumnSpan::Empty,
        };
        // A material centre proves the interval reaches the shell floor
        // where every cut came from above (the monotone law); with cuts
        // from below the floor is the up-march's, exact the same way.
        let floor = if self.cut_downs.is_empty() {
            floor
        } else {
            match march_up(self, &cx, x, y, z_lo, z_hi, true) {
                ReadUp::Floor(f) if f.is_finite() => f,
                // The two marches read the same centres: a floor the
                // ascent cannot find under a top the descent found
                // breaks the parity promise and is no verdict.
                ReadUp::Floor(_) | ReadUp::NoMaterial | ReadUp::Unreadable => return ColumnSpan::Unreadable,
            }
        };
        if top >= floor {
            ColumnSpan::Material { top, floor }
        } else {
            ColumnSpan::Unreadable
        }
    }

    /// Has a cut claimed a cell within one cell of the floor point? The
    /// swept tool came within a cell of any centre in the 3x3x3 block
    /// around it (a claim is `-d_tool` at a centre: a claim above `-cell`
    /// means the tool passed within a cell). Every point of a cell lies
    /// within sqrt(3)/2 cell of its centre, so this catches a point inside
    /// a cut near its wall and a cut thinner than a cell (review of #649),
    /// conservatively: a tool that passed close without reaching the
    /// floor counts too, the safe direction.
    fn cut_near(&self, x: f64, y: f64, floor: f64) -> bool {
        let (ix, iy, iz) = (cell_of(x, self.cell), cell_of(y, self.cell), cell_of(floor, self.cell));
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let key = [ix.saturating_add(dx), iy.saturating_add(dy), iz.saturating_add(dz)];
                    if self.cut_at(key) > -self.cell {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The column holds no material centre: cut through, or a sliver
    /// thinner than a cell. Every centre from the one below the shell
    /// floor to the one above its ceiling is air; the material, if any,
    /// lies between two adjacent ones, and their records place it (the
    /// module law). The hull of what the pairs hold is the span - one
    /// interval for every spanning-tool program. `floor` and `ceiling`
    /// are the shell's own extent on the column (`shell_extent`).
    pub(crate) fn sliver(&self, x: f64, y: f64, floor: f64, ceiling: f64) -> ColumnSpan {
        let cell = self.cell;
        let (ix, iy) = (cell_of(x, cell), cell_of(y, cell));
        let mut k0 = cell_of(floor, cell);
        if center_of(k0, cell) >= floor {
            k0 = k0.saturating_sub(1);
        }
        // The half-open law `[floor, top)`: a centre ON the floor plane is
        // inside, so the centre below the floor is the first strictly
        // under it; a centre ON the ceiling is outside already, so it is
        // the centre over the ceiling (a pair above it holds nothing).
        let mut k1 = cell_of(ceiling, cell);
        if center_of(k1, cell) < ceiling {
            k1 = k1.saturating_add(1);
        }
        if i64::from(k1) - i64::from(k0) > i64::from(super::march::BRACKET_WALK) {
            return ColumnSpan::Unreadable; // a shell taller than any lattice the field accepts
        }
        let claimed = |z: i32| self.cut_at([ix, iy, z]) > f64::NEG_INFINITY;
        if self.bounded_cuts && (k0..=k1).any(claimed) {
            return ColumnSpan::CutFromBelow; // no law places a bounded tool's faces here
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for k in k0..k1 {
            let (c, cn) = (center_of(k, cell), center_of(k.saturating_add(1), cell));
            let (u, d) = (self.up_at([ix, iy, k]), self.down_at([ix, iy, k.saturating_add(1)]));
            // A centre proven inside a cut is that cut's whole half-line.
            if u == 0.0 || d == 0.0 {
                continue;
            }
            let (entry_up, entry_down) = (
                (u.is_finite() && u >= 0.0).then_some(c + u),
                (d.is_finite() && d >= 0.0).then_some(cn - d),
            );
            // An air centre strictly inside the shell is inside a cut: not
            // the upper one (its entry would be 0), so the lower one, whose
            // top must be on record above it - and the mirror.
            let pair_lo = if c > floor {
                match entry_down {
                    Some(a) => a,
                    None => return ColumnSpan::Unreadable,
                }
            } else {
                entry_down.map_or(floor, |a| a.max(floor))
            };
            let pair_hi = if cn < ceiling {
                match entry_up {
                    Some(b) => b,
                    None => return ColumnSpan::Unreadable,
                }
            } else {
                entry_up.map_or(ceiling, |b| b.min(ceiling))
            };
            // Both ends from the shell between two claimed centres: the
            // tool passed, and what it left here has no record - refused.
            if entry_up.is_none() && entry_down.is_none() && (claimed(k) || claimed(k.saturating_add(1))) {
                return ColumnSpan::Unreadable;
            }
            if !(pair_lo.is_finite() && pair_hi.is_finite()) {
                return ColumnSpan::Unreadable;
            }
            if pair_lo <= pair_hi {
                lo = lo.min(pair_lo);
                hi = hi.max(pair_hi);
            }
        }
        if lo <= hi {
            ColumnSpan::Material { top: hi, floor: lo }
        } else {
            ColumnSpan::Empty
        }
    }
}
