//! A mod-vise jaw block's solid (#820): one closed, outward shell - the
//! block's side profile (the body, and the jaw insert on its face)
//! extruded across the wider of the body and the jaw, centred on the
//! bolt pattern. A jaw narrower than the body is drawn as wide as the
//! body (and the other way round): a conservative envelope.
//!
//! Provenance: prism extrusion of a simple polygon; the cap is
//! triangulated by the vise's ear clipper (`vise::mesh::triangulate`,
//! Meisters 1975). Our own code.

use crate::geometry::{Point3, Triangle};
use crate::vise::mesh::triangulate;

use super::JawParams;

/// The side profile in the block frame's (y, z), counter-clockwise seen
/// from +X, with duplicate and collinear corners removed (a jaw as tall
/// as the body makes a plain rectangle). `p` is assumed checked.
#[must_use]
pub fn profile(p: &JawParams) -> Vec<(f64, f64)> {
    let face = p.face_y();
    let front = face - p.jaw_thickness;
    let rear = front - p.body_length;
    let raw = [
        (rear, 0.0),
        (face, 0.0),
        (face, p.jaw_height),
        (front, p.jaw_height),
        (front, p.body_height),
        (rear, p.body_height),
    ];
    simplify(&raw)
}

/// Drop repeated corners and corners on a straight run.
fn simplify(raw: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(raw.len());
    for &q in raw {
        if pts.last() != Some(&q) {
            pts.push(q);
        }
    }
    while pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    loop {
        let n = pts.len();
        if n < 4 {
            return pts;
        }
        let flat = (0..n).find(|&i| {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)) == 0.0
        });
        match flat {
            Some(i) => {
                pts.remove(i);
            }
            None => return pts,
        }
    }
}

/// The closed, outward shell of the block in its own frame. `p` is
/// assumed checked.
pub fn solid(p: &JawParams) -> Result<Vec<Triangle>, String> {
    let poly = profile(p);
    let caps = triangulate(&poly).map_err(|_| format!("the {} profile could not be triangulated (degenerate dimensions)", p.what()))?;
    let xc = p.centre_x();
    let h = 0.5 * p.body_width.max(p.jaw_width);
    let (x0, x1) = (xc - h, xc + h);
    let at = |x: f64, k: usize| Point3::new(x, poly[k].0, poly[k].1);
    let mut tris = Vec::with_capacity(2 * caps.len() + 2 * poly.len());
    for [a, b, c] in &caps {
        tris.push(Triangle::new(at(x1, *a), at(x1, *b), at(x1, *c)));
        tris.push(Triangle::new(at(x0, *a), at(x0, *c), at(x0, *b)));
    }
    let n = poly.len();
    for i in 0..n {
        let j = (i + 1) % n;
        tris.push(Triangle::new(at(x0, i), at(x0, j), at(x1, j)));
        tris.push(Triangle::new(at(x0, i), at(x1, j), at(x1, i)));
    }
    Ok(tris)
}
