//! The CL reader of the plasma laws (#672 step 2): a [`PlasmaProgram`]
//! from a path's plasma record - the kerf and the torch spans of
//! `Toolpath::plasma` (#670), each span's lit moves chorded into its cut
//! path, and the torch-off travel between spans as the rapid into each
//! cut. Read from the records and the moves, never from re-parsed text.
//!
//! Provenance: our own reading of our own record (`camcraft_cl::cl::
//! PlasmaCut`, `TorchSpan`); no vendor material.

use super::{CutRole, PlasmaCut, PlasmaProgram};
use crate::cl::{ContourRole, Toolpath};
use crate::geo2d::Point2;
use crate::geometry::Point3;

/// The sagitta (mm) at which a lit arc is chorded for the laws: well
/// under any plasma kerf, so a chord never stands in for a crossing.
pub const CHORD_TOL: f64 = 0.01;

fn xy(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}

impl PlasmaProgram {
    /// The program a path states. Refused by name when the path carries
    /// no plasma record or a record that does not sit on its moves
    /// (`PlasmaCut::is_sane`): nothing is guessed from bare moves.
    pub fn from_toolpath(t: &Toolpath) -> Result<Self, String> {
        let rec = t
            .plasma
            .as_ref()
            .ok_or_else(|| "the path carries no plasma record - nothing says where the torch is lit".to_owned())?;
        if !rec.is_sane(&t.moves) {
            return Err("the plasma record does not sit on the path's moves".to_owned());
        }
        let mut cuts = Vec::with_capacity(rec.torches.len());
        let mut rapids = Vec::with_capacity(rec.torches.len());
        let mut prev_off: Option<Point2> = None;
        for sp in &rec.torches {
            let Some(moves) = t.moves.get(sp.on..=sp.off) else {
                return Err(format!("torch span C{} lies outside the path's moves", sp.contour));
            };
            let Some(first) = moves.first() else {
                return Err(format!("torch span C{} covers no move", sp.contour));
            };
            let mut path = vec![xy(first.from)];
            for m in moves {
                for (_, b) in m.chords(CHORD_TOL) {
                    if path.last().is_none_or(|q| (q.x - b.x).hypot(q.y - b.y) > 1e-9) {
                        path.push(b);
                    }
                }
            }
            rapids.push(prev_off.map(|a| (a, path[0])));
            prev_off = path.last().copied();
            cuts.push(PlasmaCut {
                contour: sp.contour,
                role: match sp.role {
                    ContourRole::Outer => CutRole::Outer,
                    ContourRole::Inner => CutRole::Inner,
                    ContourRole::Open => CutRole::Open,
                },
                parent: sp.parent,
                small: sp.small.is_some(),
                path,
            });
        }
        Ok(PlasmaProgram { kerf: rec.kerf, cuts, rapids })
    }
}
