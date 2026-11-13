//! The field's input gate (split from `field` by the 400-line law,
//! #779): a closed, finite, non-degenerate shell and a sane cell and band,
//! checked before any cell is written - the defect named, never a silent
//! repair.
//!
//! Provenance: edge-manifold watertightness (each edge shared by exactly
//! two facets), the closed-shell condition ray parity needs (`solid_rest`
//! law); our own code.

use std::collections::HashMap;

use crate::geometry::{Point3, Triangle};

use super::kernel::StockError;

/// Shared input gate for every field constructor: a non-empty mesh of
/// finite, non-degenerate facets whose every edge is used by exactly
/// two of them (the closed shell parity needs), and a sane cell and
/// band. The defect is named, never a silent repair.
pub(crate) fn validate_shell(facets: &[Triangle], cell: f64, band: u32) -> Result<(), StockError> {
    if facets.is_empty() {
        return Err(StockError::Invalid("sdf field: the mesh is empty".into()));
    }
    if !cell.is_finite() || cell <= 0.0 {
        return Err(StockError::Invalid(format!(
            "sdf field: cell size {cell} is not a positive finite number"
        )));
    }
    if band == 0 {
        return Err(StockError::Invalid("sdf field: band must be at least one cell wide".into()));
    }
    let mut edges: HashMap<[u64; 6], u32> = HashMap::new();
    for (n, t) in facets.iter().enumerate() {
        if !(t.a.is_finite() && t.b.is_finite() && t.c.is_finite()) {
            return Err(StockError::Invalid(format!(
                "sdf field: facet {n} has a non-finite vertex"
            )));
        }
        let (e0, e1) = (t.b - t.a, t.c - t.a);
        let cr = e0.cross(e1);
        if cr.dot(cr) <= 1e-24 {
            return Err(StockError::Invalid(format!(
                "sdf field: facet {n} has zero area (degenerate triangle)"
            )));
        }
        for (u, v) in [(t.a, t.b), (t.b, t.c), (t.c, t.a)] {
            *edges.entry(edge_key(u, v)).or_insert(0) += 1;
        }
    }
    if let Some((k, c)) = edges.iter().find(|entry| *entry.1 != 2) {
        return Err(StockError::Invalid(format!(
            "sdf field: shell is not watertight - the edge from ({}, {}, {}) to ({}, {}, {}) is used {} time(s); closed parity needs 2",
            k[0], k[1], k[2], k[3], k[4], k[5], c
        )));
    }
    Ok(())
}

/// Canonical edge key from two finite points: both endpoints, by bit
/// pattern, earlier one first. -0.0 folds to 0.0 so shared vertices
/// agree in bits the way they already agree in value.
fn edge_key(a: Point3, b: Point3) -> [u64; 6] {
    let canon = |q: Point3| {
        let f = |v: f64| if v == 0.0 { 0.0 } else { v };
        [f(q.x).to_bits(), f(q.y).to_bits(), f(q.z).to_bits()]
    };
    let (ka, kb) = (canon(a), canon(b));
    if ka <= kb {
        [ka[0], ka[1], ka[2], kb[0], kb[1], kb[2]]
    } else {
        [kb[0], kb[1], kb[2], ka[0], ka[1], ka[2]]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::field::{brick_of, in_brick, BRICK};
    use super::*;

    #[test]
    fn brick_slot_round_trips_across_negatives() {
        for k in [[-1i32, -2, 3], [0, 0, 0], [7, -8, 9], [-9_000, 1, -1]] {
            let b = brick_of(k);
            let slot = in_brick(k);
            let back = [
                b[0] * BRICK + slot.rem_euclid(BRICK as usize) as i32,
                b[1] * BRICK + (slot / BRICK as usize).rem_euclid(BRICK as usize) as i32,
                b[2] * BRICK + (slot / (BRICK * BRICK) as usize).rem_euclid(BRICK as usize) as i32,
            ];
            assert_eq!(back, k, "slot {slot} of brick {b:?} must name cell {k:?}");
        }
    }

    #[test]
    fn a_hole_in_the_shell_refuses_by_name() {
        // The box's 12 triangles minus one: a square seam used once.
        let mut f = box_facets();
        f.pop();
        let e = validate_shell(&f, 1.0, 4).unwrap_err();
        assert!(format!("{e}").contains("watertight"), "{e}");
    }

    #[test]
    fn degenerate_and_non_finite_inputs_refuse_by_name() {
        let e = validate_shell(&[], 1.0, 4).unwrap_err();
        assert!(format!("{e}").contains("empty"), "{e}");
        let f = vec![Triangle::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        )];
        let e = validate_shell(&f, 1.0, 4).unwrap_err();
        assert!(format!("{e}").contains("zero area"), "{e}");
        let mut g = box_facets();
        g[0].c.z = f64::NAN;
        let e = validate_shell(&g, 1.0, 4).unwrap_err();
        assert!(format!("{e}").contains("non-finite"), "{e}");
        let e = validate_shell(&box_facets(), 0.0, 4).unwrap_err();
        assert!(format!("{e}").contains("cell size"), "{e}");
        let e = validate_shell(&box_facets(), f64::NAN, 4).unwrap_err();
        assert!(format!("{e}").contains("cell size"), "{e}");
        let e = validate_shell(&box_facets(), 1.0, 0).unwrap_err();
        assert!(format!("{e}").contains("band"), "{e}");
    }

    /// The 4x4x4 box at the origin, 12 triangles.
    pub(crate) fn box_facets() -> Vec<Triangle> {
        let v = |i: usize| {
            Point3::new(
                [0.0, 4.0][i & 1],
                [0.0, 4.0][(i >> 1) & 1],
                [0.0, 4.0][(i >> 2) & 1],
            )
        };
        let quad = |a: usize, b: usize, c: usize, d: usize| -> [Triangle; 2] {
            [Triangle::new(v(a), v(b), v(c)), Triangle::new(v(a), v(c), v(d))]
        };
        let mut t = Vec::with_capacity(12);
        t.extend(quad(4, 5, 7, 6));
        t.extend(quad(0, 2, 3, 1));
        t.extend(quad(0, 1, 5, 4));
        t.extend(quad(2, 6, 7, 3));
        t.extend(quad(1, 3, 7, 5));
        t.extend(quad(2, 0, 4, 6));
        t
    }
}
