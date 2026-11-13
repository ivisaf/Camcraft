//! The built-in vise's solid (#788): one closed, outward-facing shell -
//! the vise's side profile (base, fixed jaw, moving jaw at the opening,
//! screw and handle housing) extruded across the vise's overall width.
//! Jaws drawn as wide as the body are a conservative envelope: the
//! fixture laws see at least the real steel, never less.
//!
//! Provenance: prism extrusion of a simple polygon; the cap is
//! triangulated by ear clipping (Meisters, "Polygons have ears", 1975 -
//! the textbook two-ears method). Our own code.

use crate::geometry::{Point3, Triangle};

use super::ViseParams;

/// The side profile in the vise frame's (y, z), counter-clockwise seen
/// from +X: y = 0 is the fixed jaw's clamping face (its body behind,
/// toward +y), the moving jaw's face at y = -opening, z = 0 the floor
/// the part sits on, the base below it. `p` is assumed checked.
#[must_use]
pub fn profile(p: &ViseParams) -> Vec<(f64, f64)> {
    // Each jaw is drawn as its whole body, not just the plate (#802
    // review): steel behind a plate below jaw height is in the solid.
    let t = p.jaw_body_depth;
    let o = p.opening;
    let front = t - p.length;
    let housing = front + p.housing_length;
    vec![
        (t, -p.base_height),
        (t, p.jaw_height),
        (0.0, p.jaw_height),
        (0.0, 0.0),
        (-o, 0.0),
        (-o, p.jaw_height),
        (-o - t, p.jaw_height),
        (-o - t, 0.0),
        (housing, 0.0),
        (housing, p.housing_height),
        (front, p.housing_height),
        (front, -p.base_height),
    ]
}

/// Twice the signed area of a polygon (positive: counter-clockwise).
fn area2(poly: &[(f64, f64)]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum()
}

fn cross(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// `q` inside or on triangle `abc` (counter-clockwise).
fn covers(a: (f64, f64), b: (f64, f64), c: (f64, f64), q: (f64, f64)) -> bool {
    cross(a, b, q) >= 0.0 && cross(b, c, q) >= 0.0 && cross(c, a, q) >= 0.0
}

/// Ear-clip a simple counter-clockwise polygon into index triangles,
/// each counter-clockwise. A polygon with no ear left (degenerate or
/// self-crossing) is refused, never looped on.
pub fn triangulate(poly: &[(f64, f64)]) -> Result<Vec<[usize; 3]>, String> {
    if poly.len() < 3 {
        return Err("the vise profile has fewer than three corners".into());
    }
    let mut ring: Vec<usize> = (0..poly.len()).collect();
    let mut out = Vec::with_capacity(poly.len().saturating_sub(2));
    while ring.len() > 3 {
        let m = ring.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (ring[(i + m - 1) % m], ring[i], ring[(i + 1) % m]);
            cross(poly[a], poly[b], poly[c]) > 0.0
                && !ring.iter().any(|&k| k != a && k != b && k != c && covers(poly[a], poly[b], poly[c], poly[k]))
        });
        let Some(i) = ear else {
            return Err("the vise profile could not be triangulated (degenerate dimensions)".into());
        };
        out.push([ring[(i + m - 1) % m], ring[i], ring[(i + 1) % m]]);
        ring.remove(i);
    }
    if let [a, b, c] = ring[..] {
        if cross(poly[a], poly[b], poly[c]) <= 0.0 {
            return Err("the vise profile could not be triangulated (degenerate dimensions)".into());
        }
        out.push([a, b, c]);
    }
    Ok(out)
}

/// The closed, outward shell of the vise in its own frame: the profile
/// extruded over x in [-width/2, width/2]. `p` is assumed checked.
pub fn solid(p: &ViseParams) -> Result<Vec<Triangle>, String> {
    let mut poly = profile(p);
    if area2(&poly) < 0.0 {
        poly.reverse();
    }
    let caps = triangulate(&poly)?;
    let h = 0.5 * p.width;
    let at = |x: f64, k: usize| Point3::new(x, poly[k].0, poly[k].1);
    let mut tris = Vec::with_capacity(2 * caps.len() + 2 * poly.len());
    for [a, b, c] in &caps {
        tris.push(Triangle::new(at(h, *a), at(h, *b), at(h, *c)));
        tris.push(Triangle::new(at(-h, *a), at(-h, *c), at(-h, *b)));
    }
    let n = poly.len();
    for i in 0..n {
        let j = (i + 1) % n;
        tris.push(Triangle::new(at(-h, i), at(-h, j), at(h, j)));
        tris.push(Triangle::new(at(-h, i), at(h, j), at(h, i)));
    }
    Ok(tris)
}
