//! The plasma program's door (#672): what the laws refuse by name before
//! judging anything - the numbers, the ids, the roles, and the stated
//! nesting checked against the rings themselves. The record's `PARENT`
//! is a claim; enclosure is recomputed from the geometry here and a
//! disagreement is a refusal, never a trusted word.
//!
//! Provenance: our own rules over the plasma record's contract
//! (`camcraft_cl::cl::PlasmaCut::is_sane`); no vendor material.

use super::geom::{encloses, ring_area};
use super::{CutRole, PlasmaCut, PlasmaProgram};
use crate::geo2d::Point2;

/// The program's shape, refused by name when it cannot be judged. On
/// success, each cut's parent as the geometry confirms it (the record's
/// word, verified).
pub(super) fn well_formed(p: &PlasmaProgram) -> Result<Vec<Option<u32>>, String> {
    if !(p.kerf.is_finite() && p.kerf > 0.0) {
        return Err(format!("plasma kerf {} is not a positive width", p.kerf));
    }
    if p.rapids.len() != p.cuts.len() {
        return Err(format!("{} rapids for {} cuts - one rapid entry per cut", p.rapids.len(), p.cuts.len()));
    }
    let finite = |q: &Point2| q.x.is_finite() && q.y.is_finite();
    let mut ids = std::collections::HashSet::new();
    for (c, r) in p.cuts.iter().zip(&p.rapids) {
        if !ids.insert(c.contour) {
            return Err(format!("contour {} appears twice", c.contour));
        }
        if c.path.is_empty() || (c.path.len() < 2 && !c.small) {
            return Err(format!("contour {} has no cut path", c.contour));
        }
        if !c.path.iter().all(finite) {
            return Err(format!("contour {} has a non-finite point", c.contour));
        }
        if let Some((a, b)) = r
            && !(finite(a) && finite(b))
        {
            return Err(format!("the rapid into contour {} has a non-finite point", c.contour));
        }
        if c.small && c.role != CutRole::Inner {
            return Err(format!(
                "contour {} is marked a small hole but is {} - only a hole is centre-pierced",
                c.contour,
                c.role.word()
            ));
        }
    }
    for c in &p.cuts {
        match c.parent {
            Some(parent) if parent == c.contour => return Err(format!("contour {} names itself as its parent", c.contour)),
            Some(parent) => match p.cuts.iter().find(|e| e.contour == parent) {
                None => return Err(format!("contour {} names parent {parent}, which no cut carries", c.contour)),
                Some(e) if e.role == CutRole::Open => {
                    return Err(format!(
                        "contour {} names parent {parent}, which is an open run - an open run encloses nothing",
                        c.contour
                    ));
                }
                Some(_) => {}
            },
            None if c.role == CutRole::Inner => return Err(format!("inner contour {} has no parent", c.contour)),
            None => {}
        }
    }
    let rings: Vec<Option<&[Point2]>> = p.cuts.iter().map(PlasmaCut::ring).collect();
    for (c, ring) in p.cuts.iter().zip(&rings) {
        if ring.is_none() && c.role != CutRole::Open && !c.small {
            return Err(format!(
                "contour {} is {} but its cut never returns to its start - a closed contour must close",
                c.contour,
                c.role.word()
            ));
        }
    }
    // Enclosure from the rings: the smallest ring standing around every
    // point of the cut (its ring when it has one, else its path).
    let mut parents = Vec::with_capacity(p.cuts.len());
    for (k, c) in p.cuts.iter().enumerate() {
        let probe: &[Point2] = rings[k].unwrap_or(&c.path);
        let my_area = rings[k].map(ring_area);
        let mut best: Option<(u32, f64)> = None;
        for (j, e) in p.cuts.iter().enumerate() {
            let Some(ring) = rings[j] else { continue };
            if j == k {
                continue;
            }
            let area = ring_area(ring);
            if my_area.is_some_and(|a| area <= a) {
                continue;
            }
            if encloses(ring, probe) && best.is_none_or(|(_, a)| area < a) {
                best = Some((e.contour, area));
            }
        }
        let found = best.map(|(id, _)| id);
        if found != c.parent {
            return Err(match (c.parent, found) {
                (Some(said), Some(seen)) => format!(
                    "contour {} names parent {said} but its cut lies inside contour {seen} - the record and the geometry disagree",
                    c.contour
                ),
                (Some(said), None) => format!(
                    "contour {} names parent {said} but no cut encloses it - the record and the geometry disagree",
                    c.contour
                ),
                (None, Some(seen)) => format!(
                    "contour {} names no parent but its cut lies inside contour {seen} - the record and the geometry disagree",
                    c.contour
                ),
                // Equal parents never reach here; said for the compiler.
                (None, None) => format!("contour {} nesting could not be confirmed", c.contour),
            });
        }
        parents.push(found);
    }
    Ok(parents)
}
