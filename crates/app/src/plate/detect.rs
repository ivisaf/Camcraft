//! A plate's grid from its own holes (#819): for an imported plate, the
//! holes found by the same recognition rule the Drill operation's
//! "Detect holes" (and MCP `detect_holes`) runs - closed, round walls on
//! the model's Z - are grouped by diameter, and the most numerous group
//! that forms a regular rectangular lattice becomes the tapped grid;
//! the next group that forms one becomes the dowel lattice. Nodes of the
//! lattice without a hole are recorded as gaps (placing there is
//! refused by name); a group that is not a lattice is refused with why.
//! A checkerboard lattice (holes on every other place, alternating row
//! by row - a staggered plate) is read as a staggered grid.
//!
//! The lattice is read in the plate's MODEL frame, axis-aligned: rows
//! along +Y from A, columns along +X from 1 (the `plate` convention),
//! the top face at the holes' entry. A hole within `ON_NODE` (0.05 mm)
//! of its lattice node is snapped to it. A plate whose holes do not
//! stand on its model Z, or whose grid is turned in its model frame, is
//! refused - including the one turn a lattice fit alone would accept:
//! a square grid turned 45 degrees reads as a checkerboard, exactly like
//! a staggered plate, so a staggered read also needs the plate's own
//! edges to run along X and Y (`square_edges`).
//!
//! Provenance: our own grouping and lattice fit over the repo's hole
//! recognition (`drilling::recognize`).

use crate::drilling::{self, Hole};
use crate::geometry::{Point3, Triangle};

use super::{Grid, MAX_HOLES, PlateDef, location_name};

/// Two hole diameters closer than this are one size (mm).
const SAME_SIZE: f64 = 0.005;
/// A hole within this of its lattice node is on it (mm).
pub(crate) const ON_NODE: f64 = 0.05;
/// At least this share of a lattice's nodes must hold a hole.
const MIN_FILL: f64 = 0.5;
/// What a refused fit adds when the likely cause is a turned plate.
const TURNED: &str = " - the holes don't line up with the model's X/Y axes: is the plate turned in its model, or are its holes irregular?";

/// Whether the plate's outline has straight edges along both model axes:
/// on each axis, the vertices at the least OR the greatest coordinate
/// span at least half the plate's extent the other way (#826: a clamp
/// ear or a chamfered corner spoils one side, not both). A plate turned
/// in its model meets its extremes at corners, not along an edge.
fn square_edges(tris: &[Triangle]) -> bool {
    let pts: Vec<Point3> = tris.iter().flat_map(|t| [t.a, t.b, t.c]).collect();
    let range = |v: &mut dyn Iterator<Item = f64>| v.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(x), b.max(x)));
    // Does the edge at the least (or greatest) `key` run at least half the
    // plate's extent along `along`?
    let edge = |key: fn(&Point3) -> f64, along: fn(&Point3) -> f64, greatest: bool| {
        let (lo, hi) = range(&mut pts.iter().map(key));
        let (amin, amax) = range(&mut pts.iter().map(along));
        let (emin, emax) = range(&mut pts.iter().filter(|p| if greatest { key(p) >= hi - ON_NODE } else { key(p) <= lo + ON_NODE }).map(along));
        let run = emax - emin;
        run.is_finite() && run >= 0.5 * (amax - amin)
    };
    (edge(|p| p.x, |p| p.y, false) || edge(|p| p.x, |p| p.y, true)) && (edge(|p| p.y, |p| p.x, false) || edge(|p| p.y, |p| p.x, true))
}

/// Distinct values of `v` (sorted), clustered within `ON_NODE`.
fn levels(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    let mut out: Vec<(f64, usize)> = Vec::new();
    for x in v {
        match out.last_mut() {
            Some((sum, n)) if (x - *sum / *n as f64).abs() <= ON_NODE => {
                *sum += x;
                *n += 1;
            }
            _ => out.push((x, 1)),
        }
    }
    out.into_iter().map(|(s, n)| s / n as f64).collect()
}

/// The pitch of sorted levels `l`, checked: every level on it.
fn pitch(l: &[f64], axis: &str) -> Result<(f64, u32), String> {
    let (Some(first), Some(last)) = (l.first(), l.last()) else {
        return Err(format!("no {axis} positions"));
    };
    if l.len() < 2 {
        return Err(format!("they stand in a single {}", if axis == "X" { "column" } else { "row" }));
    }
    let gap = l.windows(2).map(|w| w[1] - w[0]).fold(f64::MAX, f64::min);
    let steps = ((last - first) / gap).round();
    if !(steps.is_finite() && steps >= 1.0) || steps > MAX_HOLES as f64 {
        return Err(format!("their {axis} spacing is not regular{TURNED}"));
    }
    let p = (last - first) / steps;
    for x in l {
        let k = ((x - first) / p).round();
        if (x - (first + k * p)).abs() > ON_NODE {
            return Err(format!("a hole at {axis} {x:.3} mm is off the {p:.3} mm pitch{TURNED}"));
        }
    }
    Ok((p, steps as u32 + 1))
}

/// Fit one same-size group to a lattice.
fn lattice(group: &[Hole]) -> Result<Grid, String> {
    let xs = levels(group.iter().map(|h| h.xy.x).collect());
    let ys = levels(group.iter().map(|h| h.xy.y).collect());
    let (px, cols) = pitch(&xs, "X")?;
    let (py, rows) = pitch(&ys, "Y")?;
    let nodes = u64::from(cols) * u64::from(rows);
    if nodes > MAX_HOLES {
        return Err(format!("the lattice would hold {nodes} holes, more than {MAX_HOLES}{TURNED}"));
    }
    let (x0, y0) = (xs[0], ys[0]);
    let mut filled = vec![false; nodes as usize];
    for h in group {
        let c = ((h.xy.x - x0) / px).round();
        let r = ((h.xy.y - y0) / py).round();
        let k = (r as usize).saturating_mul(cols as usize).saturating_add(c as usize);
        match filled.get_mut(k) {
            Some(slot) if !*slot => *slot = true,
            Some(_) => return Err(format!("two holes stand at {}", location_name(r as u32, c as u32))),
            None => return Err("a hole falls outside the lattice".into()),
        }
    }
    let have = filled.iter().filter(|f| **f).count();
    let at = |r: u32, c: u32| filled[(r as usize) * cols as usize + c as usize];
    let all = || (0..rows).flat_map(move |r| (0..cols).map(move |c| (r, c)));
    // A checkerboard (staggered) plate: every hole on one row + column
    // parity, so half the full lattice's places are not places at all.
    let parity: Vec<u32> = all().filter(|&(r, c)| at(r, c)).map(|(r, c)| (r + c) % 2).collect();
    let stagger = match parity.first() {
        Some(&p) if parity.iter().all(|q| *q == p) => Some(p as u8),
        _ => None,
    };
    let places = all().filter(|&(r, c)| stagger.is_none_or(|p| (r + c) % 2 == u32::from(p))).count();
    if (have as f64) < MIN_FILL * places as f64 {
        return Err(format!("only {have} of the lattice's {places} places hold a hole{TURNED}"));
    }
    let missing = all()
        .filter(|&(r, c)| stagger.is_none_or(|p| (r + c) % 2 == u32::from(p)) && !at(r, c))
        .map(|(r, c)| location_name(r, c))
        .collect();
    let diameter = 2.0 * group.iter().map(|h| h.radius).sum::<f64>() / group.len() as f64;
    Ok(Grid { x0, y0, pitch_x: px, pitch_y: py, cols, rows, diameter, missing, stagger })
}

/// Holes grouped by diameter, the most numerous group first.
fn by_size(mut holes: Vec<Hole>) -> Vec<Vec<Hole>> {
    holes.sort_by(|a, b| a.radius.total_cmp(&b.radius));
    let mut groups: Vec<Vec<Hole>> = Vec::new();
    for h in holes {
        match groups.last_mut() {
            Some(g) if g.last().is_some_and(|l| 2.0 * (h.radius - l.radius) <= SAME_SIZE) => g.push(h),
            _ => groups.push(vec![h]),
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
    groups
}

/// The plate grid of mesh `tris` (the plate's own model frame), or why
/// its holes are not one.
pub fn grid_from_facets(tris: &[Triangle]) -> Result<PlateDef, String> {
    let holes: Vec<Hole> = drilling::recognize(tris).into_iter().filter(Hole::is_usable).collect();
    if holes.is_empty() {
        return Err("no holes found on its Z axis - is the plate's top face up (+Z) in its model?".into());
    }
    let groups = by_size(holes);
    let mut first_err = None;
    let mut found: Option<(Grid, f64)> = None;
    let mut dowels = None;
    for g in groups.iter().filter(|g| g.len() >= 4) {
        match lattice(g) {
            Ok(grid) => {
                if found.is_none() {
                    let top = g.iter().map(|h| h.entry_z).fold(f64::MIN, f64::max);
                    found = Some((grid, top));
                } else {
                    dowels = Some(grid);
                    break;
                }
            }
            Err(e) => {
                if first_err.is_none() {
                    let d = 2.0 * g[0].radius;
                    first_err = Some(format!("its {} holes of {d:.3} mm don't form a regular grid: {e}", g.len()));
                }
            }
        }
    }
    if found.as_ref().is_some_and(|(g, _)| g.stagger.is_some()) && !square_edges(tris) {
        return Err(
            "the holes don't line up with the model's X/Y axes - is the plate turned in its model? \
             (its holes read as a staggered grid, but its edges don't run along X and Y; \
             turn the plate square to X/Y and detect again)"
                .into(),
        );
    }
    let Some((holes, top_z)) = found else {
        return Err(first_err.unwrap_or_else(|| "its holes don't form a regular grid: fewer than four holes of any one size".into()));
    };
    let def = PlateDef { params: None, top_z, holes, dowels };
    def.check()?;
    Ok(def)
}
