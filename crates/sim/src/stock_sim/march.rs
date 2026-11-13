//! The verdict march (M7.1 C): reading column heights off the field.
//! The walk and its bracket live here; its consumers - the
//! deviate/measure/sheet verdict readers - are `march_verdicts`,
//! split by the 400-line law.
//!
//! Provenance: sphere-tracing of a signed-distance field along a ray
//! is the published marching law for implicit surfaces; the exactness
//! here comes from this codebase's own parity (`column_crossings`, the
//! `solid_rest` reuse), which makes every centre's SIGN exact wherever
//! its magnitude is merely bounded. Nothing from vendor binaries.
//!
//! ## The laws here
//!
//! - **The verdict is a HEIGHT, in the column engine's units.** The
//!   march answers the Z of the top of remaining material on a cell
//!   center, so a `Deviation` or `StockReport` from either engine
//!   carries the same vocabulary - perpendicular offsets, the
//!   renderer's numbers, belong to the eikonal pass (M7.2), not here.
//! - **Only centres are probed, and a centre's sign is exact.** A
//!   centre stands in remaining material iff its own ray's parity says
//!   inside (`parity_of`, the half-open `[floor, top)` law: a centre on
//!   a floor plane is material, one on a top plane is air) and no cut
//!   has proven it removed (`cut_at > 0`). The stored magnitude never
//!   decides a sign: a cell whose centre lies ON a stock plane reads
//!   magnitude zero for every point of the cell, and a sign taken from
//!   it would call the whole cell "the surface" (#824, the review of
//!   #817: a flipped floor read 0.25 high). The walk visits centres
//!   from the world's end toward the stock and stops at the first
//!   material centre; the centre before it is air, so the surface lies
//!   inside that one-cell bracket, and is read from there: at the
//!   ray-entry distance the cut layer proves (`field_ray` - a
//!   conservative lower bound, so flanks and grooves answer at their
//!   own tilted geometry), or at the shell's own crossing inside the
//!   bracket (the cached parity column, exact to rounding for ANY
//!   plane, aligned with the lattice or not - #637 E2b), the one nearer
//!   the material centre where both bind. A sign flip with neither a
//!   crossing nor a record is a surface the field cannot place:
//!   `Unreadable`, never the boundary as a guess (a flipped bounded
//!   tool's bottom face). `march_up` is this descent's mirror, reading
//!   the floor through the same walk.
//! - **A skip is a proof, never a guess.** From an air centre the walk
//!   may skip the cells the field proves clear: a stored magnitude is
//!   at most the true distance plus sqrt(3) cells (`field_gen`), so a
//!   centre outside the shell proves its magnitude's cells less two
//!   clear of it; a cut claim is a lower bound on the tool's depth, so
//!   a claimed centre proves its claim's cells removed. Every skipped
//!   centre is air by one of the two, so the first material centre the
//!   walk meets is the stock's own end, and the centre before it is
//!   air. The old sphere trace stepped by the magnitude from points
//!   between centres and recovered with a climb; it could land past a
//!   thin web, and its last "air" probe was no proof under a claim.
//! - **No material centre is not "no material".** A walk that reaches
//!   the world's far end without a material centre has proven every
//!   centre air, not every point: a floor skin left under a cut, or a
//!   plate thinner than the lattice, stands between two air centres
//!   with no centre of its own. Where the shell crosses the column at
//!   all, the walk reads that material through the one sliver law
//!   (`field_floor::sliver`: the cut records of the two centres around
//!   each pair, the shell binding where no record does) and answers its
//!   top (a descent) or floor (an ascent); a column the shell never
//!   crosses is air by the shell's own word, and one whose records prove
//!   every pair cut through is `NoMaterial`. A sliver the records cannot
//!   place is `Unreadable` - never `NoMaterial` where the shell says
//!   material stands (the review of #817: a 0.25 skin under a slot read
//!   NaN on every verdict path that marched alone, and the rest mask
//!   would have treated that steel as air).
//! - **A march that cannot finish says so.** The walk budget turns
//!   into `Unreadable`, whose verdict is NaN - the no-verdict value the
//!   column engine already uses for air, never a zero dressed up as a
//!   clean bill of health. `NoMaterial` is NaN too: the field's stock
//!   has a bottom the semi-infinite column does not, and a column past
//!   it holds nothing to judge.
//! - **An `Open` column answers its ceiling.** Parity `Open` is
//!   material everywhere (stage A's never-hide rule), and the
//!   watertight gate makes it unreachable; should a future mesh ever
//!   be built around it, the march reports the world's top height -
//!   rest over-reported, the blessed direction.

use super::field::{cell_of, center_of, SdfField};
use super::field_floor::{shell_extent, ColumnSpan};
use super::kernel::{StockError, MAX_CELLS_PER_AXIS};

/// How one column's descent ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Read {
    /// The top of remaining material, bracketed inside its cell.
    Top(f64),
    /// The ray reached the world floor with material never found.
    NoMaterial,
    /// The march could not finish inside its budget: no verdict.
    Unreadable,
}

/// Walk budget: a column's centres, plus slack - the walk visits each
/// centre of the world at most once (a skip only shortens it), so this
/// bounds a world taller than any lattice the field accepts.
pub(crate) const BRACKET_WALK: u32 = 8192;

/// The band magnitude's over-read, in cells: a stored value is at most
/// the truth plus sqrt(3) cells (`field_gen`'s construction law), so a
/// run proven clear of the shell is the magnitude's cells less two.
const MAG_SLACK_CELLS: i32 = 2;

/// The world the march runs in: one band above and below the shell,
/// so every probe's magnitude is answerable from inside it.
pub(crate) fn world(f: &SdfField) -> (f64, f64) {
    let b = f.bounds();
    let pad = f.cap() + f.cell();
    (b.max.z + pad, b.min.z - pad)
}

/// The stock's own XY lattice, origin-locked like its grid: the cells
/// the shell's bounding box covers, one row of indices each way.
pub(crate) struct Lattice {
    pub(crate) i0: i32,
    pub(crate) j0: i32,
    pub(crate) x0: f64,
    pub(crate) y0: f64,
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    pub(crate) cell: f64,
}

pub(crate) fn lattice_of(f: &SdfField) -> Result<Lattice, StockError> {
    let cell = f.cell();
    let b = f.bounds();
    let (i0, j0) = (cell_of(b.min.x, cell), cell_of(b.min.y, cell));
    let (mut i1, mut j1) = (
        (b.max.x / cell).ceil() as i32 - 1,
        (b.max.y / cell).ceil() as i32 - 1,
    );
    (i1, j1) = (i1.max(i0), j1.max(j0));
    let (nx, ny) = ((i1 - i0 + 1) as usize, (j1 - j0 + 1) as usize);
    if nx > MAX_CELLS_PER_AXIS || ny > MAX_CELLS_PER_AXIS {
        return Err(StockError::Invalid(format!(
            "grid would be {nx}x{ny} cells; at most {MAX_CELLS_PER_AXIS} per axis"
        )));
    }
    Ok(Lattice {
        i0,
        j0,
        x0: i0 as f64 * cell,
        y0: j0 as f64 * cell,
        nx,
        ny,
        cell,
    })
}

/// The shell's surface inside a bracket: the cached column's crossings
/// in `(lo, hi]` - the highest for a top read (`top`), the lowest for a
/// floor read - `None` where the parity never flips there. Exact to
/// rounding for any plane, lattice-aligned or not (#637 E2b).
pub(crate) fn shell_in(cx: &Option<Vec<f64>>, lo: f64, hi: f64, top: bool) -> Option<f64> {
    let it = cx.as_ref()?.iter().copied().filter(|z| z.is_finite() && *z > lo && *z <= hi);
    if top { it.reduce(f64::max) } else { it.reduce(f64::min) }
}

/// The surface between a material center `cm` and the air center
/// beside it, from the cut layer's entry (`cm + entry` when the ray
/// meets a solid inside this cell: `cut_ups` for a top read, `cut_downs`
/// for a floor read) and the shell's crossing in the bracket: the
/// material's own end is the nearer of the two to `cm` (`min` for a
/// top, `max` for a floor). Neither on record: `None` - the field
/// cannot place that surface, and it never guesses the boundary.
pub(crate) fn face(entry: Option<f64>, shell: Option<f64>, top: bool) -> Option<f64> {
    match (entry, shell) {
        (Some(e), Some(s)) => Some(if top { e.min(s) } else { e.max(s) }),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}

/// One centre's exact sign, with the run of cells beyond it (either
/// way along the column) the field proves air - at least one, the
/// next centre, which the walk probes for itself.
struct Sign {
    material: bool,
    clear: i32,
}

/// A finite, non-negative length in whole cells, clamped to the walk.
fn cells_of(v: f64, cell: f64) -> i32 {
    (v / cell).floor().clamp(0.0, f64::from(BRACKET_WALK)) as i32
}

/// The sign at centre `j` of column `(ix, iy)` (the module law), `None`
/// where the field cannot say.
fn sign_at(f: &SdfField, cx: &Option<Vec<f64>>, ix: i32, iy: i32, j: i32, cuts: bool) -> Option<Sign> {
    let cell = f.cell();
    let key = [ix, iy, j];
    let mag = f.mag_at(key);
    if !mag.is_finite() {
        return None;
    }
    let inside = SdfField::parity_of(cx, center_of(j, cell));
    let claim = if cuts { f.cut_at(key) } else { f64::NEG_INFINITY };
    if claim.is_nan() {
        return None;
    }
    let claimed = claim > 0.0;
    let mut clear = 1;
    if !inside {
        clear = clear.max(cells_of(mag, cell) - MAG_SLACK_CELLS);
    }
    if claimed && claim.is_finite() {
        clear = clear.max(cells_of(claim, cell));
    }
    Some(Sign { material: inside && !claimed, clear })
}

/// The centre walk both marches share: from the centre holding
/// `z_from` toward the one holding `z_to` (a descent when `z_from` is
/// the higher, an ascent otherwise), skipping proven-air runs, to the
/// first material centre, whose surface toward the walk's origin is
/// then read from the bracket between it and the air centre before it
/// (the module law). The height in `Read::Top` is that surface: the
/// top for a descent, the floor for an ascent.
pub(crate) fn walk(
    f: &SdfField,
    cx: &Option<Vec<f64>>,
    x: f64,
    y: f64,
    z_from: f64,
    z_to: f64,
    cuts: bool,
) -> Read {
    if cx.is_none() {
        return Read::Top(z_from); // Open answers material, the `inside` law
    }
    if !(x.is_finite() && y.is_finite() && z_from.is_finite() && z_to.is_finite()) {
        return Read::Unreadable;
    }
    let down = z_from >= z_to;
    let cell = f.cell();
    let (ix, iy) = (cell_of(x, cell), cell_of(y, cell));
    let (j_from, j_to) = (cell_of(z_from, cell), cell_of(z_to, cell));
    let step: i32 = if down { -1 } else { 1 };
    let past_end = |j: i32| if down { j < j_to } else { j > j_to };
    let mut j = j_from;
    let mut cleared = false; // the centre the walk came from is proven air
    for _ in 0..BRACKET_WALK {
        let Some(s) = sign_at(f, cx, ix, iy, j, cuts) else {
            return Read::Unreadable;
        };
        if s.material {
            if !cleared {
                return Read::Unreadable; // the world's own end is material: no bracket
            }
            let cm = center_of(j, cell);
            let ca = center_of(j.saturating_sub(step), cell);
            // The uncut walk (mesh only) has no ray to enter; the cut
            // walk answers at the entry when this cell's ray met a solid
            // inside it. An entry past the mesh's own crossing (M13.1: a
            // curved tool bottom grazing the stock top from above) is air
            // already: the crossing binds, as the column's `min(top, cut)`
            // does.
            let entry = if cuts {
                let e = if down { f.up_at([ix, iy, j]) } else { f.down_at([ix, iy, j]) };
                (e.is_finite() && e >= 0.0 && e < cell).then_some(if down { cm + e } else { cm - e })
            } else {
                None
            };
            let shell = if down { shell_in(cx, cm, ca, true) } else { shell_in(cx, ca, cm, false) };
            return face(entry, shell, down).map_or(Read::Unreadable, Read::Top);
        }
        if past_end(j.saturating_add(step)) {
            return centreless(f, cx, x, y, down, cuts); // the world's end, every centre air
        }
        cleared = true;
        let next = j.saturating_add(step.saturating_mul(s.clear));
        if past_end(next) {
            return centreless(f, cx, x, y, down, cuts); // the proven run reaches the world's end
        }
        j = next;
    }
    Read::Unreadable
}

/// The walk proved every centre of the column air (the module law):
/// what stands is thinner than a cell and sits between two air
/// centres. Read it from the shell and the records - the surface
/// toward the walk's origin, the top for a descent (`down`) and the
/// floor for an ascent - or name why it cannot be read.
fn centreless(f: &SdfField, cx: &Option<Vec<f64>>, x: f64, y: f64, down: bool, cuts: bool) -> Read {
    let Some(cross) = cx.as_ref() else {
        return Read::Unreadable; // unreachable: `walk` answers `Open` before it walks
    };
    let Some((floor, ceiling)) = shell_extent(cross) else {
        return Read::NoMaterial; // the shell never crosses this column: air beside the stock
    };
    if !cuts {
        // Mesh only: the material's top is the shell's highest crossing
        // and its floor the lowest, exact for any plane.
        return Read::Top(if down { ceiling } else { floor });
    }
    match f.sliver(x, y, floor, ceiling) {
        ColumnSpan::Material { top, floor } => Read::Top(if down { top } else { floor }),
        ColumnSpan::Empty => Read::NoMaterial, // every pair proven cut through
        ColumnSpan::CutFromBelow | ColumnSpan::Unreadable => Read::Unreadable,
    }
}

/// Walk down `(x, y)` from `z_hi` until the steel's top (or a bail
/// state names why there is none).
pub(crate) fn march(
    f: &SdfField,
    cx: &Option<Vec<f64>>,
    x: f64,
    y: f64,
    z_hi: f64,
    z_lo: f64,
    cuts: bool,
) -> Read {
    walk(f, cx, x, y, z_hi, z_lo, cuts)
}

/// One column's remaining-material top, for `Stock::surface_z`.
pub(crate) fn column_top(f: &SdfField, x: f64, y: f64) -> f64 {
    if !(x.is_finite() && y.is_finite()) {
        return f64::NAN;
    }
    let (z_hi, z_lo) = world(f);
    match march(f, &f.shell_crossings(x, y), x, y, z_hi, z_lo, true) {
        Read::Top(z) => z,
        _ => f64::NAN,
    }
}
