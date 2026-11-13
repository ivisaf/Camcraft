//! The lifted-return law (issue #220, after #47's microlift): a return
//! lifted over the floor must travel in air - the cutter's disc, read on
//! the in-process stock the instant before the move runs, may
//! stand over no column still above the lift. The rim reading of
//! `quality` is the wrong yardstick here (it reads a cell and a half
//! OUTSIDE the cutter, so a return grazing the face it just cut reads a
//! few degrees of "contact" at zero depth); this reads INSIDE the disc,
//! two cells in, where a real collision would stand. Sampling law ours.

use crate::cl::{MoveKind, Toolpath};
use crate::geo2d::Point2;
use crate::geometry::Point3;
use crate::stock_sim::{Columns, Stock, StockError};

/// Height slack: a column within a micron of the lift is cut.
const Z_EPS: f64 = 1e-3;

/// Rim samples per ring of the disc read.
const RING: usize = 24;

/// The first point where a lifted planar feed of `path` (a planar feed
/// above the lowest plane fed since the last rapid) would stand its
/// disc of radius `r` over stock in `dex`, which is cut by every move
/// in order as the walk goes; `None` when every lifted move is in air.
/// A stock the reading cannot serve is an error, never a pass.
pub fn lifted_cut(path: &Toolpath, dex: &mut Stock, r: f64) -> Result<Option<Point3>, StockError> {
    let cell = dex.cell();
    let inner = (r - 2.0 * cell).max(0.0);
    let step = (r / 10.0).max(cell);
    let mut plane: Option<f64> = None;
    for m in &path.moves {
        if m.kind == MoveKind::Rapid {
            plane = None;
            continue;
        }
        let planar = (m.from.z - m.to.z).abs() <= 1e-9;
        if planar && m.kind == MoveKind::Feed {
            match plane {
                Some(z) if m.to.z > z + 1e-6 => {
                    if let Some(p) = first_stock(&dex.columns()?, m.segments3(0.01).as_slice(), inner, step) {
                        return Ok(Some(p));
                    }
                }
                _ => plane = Some(plane.map_or(m.to.z, |z| z.min(m.to.z))),
            }
        }
        dex.subtract_flat(m, r, 0.01)?;
    }
    Ok(None)
}

/// The first sample along `segs` whose disc (radius `inner`) stands
/// over a column above the sample's height.
fn first_stock(dex: &Columns<'_>, segs: &[(Point3, Point3)], inner: f64, step: f64) -> Option<Point3> {
    for &(a, b) in segs {
        let len = (b.x - a.x).hypot(b.y - a.y);
        let n = ((len / step).ceil() as usize).clamp(1, 1 << 16);
        for k in 0..=n {
            let t = k as f64 / n as f64;
            let p = Point3::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t);
            let mut pts = vec![Point2::new(p.x, p.y)];
            for rr in [0.5 * inner, inner] {
                for j in 0..RING {
                    let ang = j as f64 * std::f64::consts::TAU / RING as f64;
                    pts.push(Point2::new(p.x + rr * ang.cos(), p.y + rr * ang.sin()));
                }
            }
            if pts.iter().any(|&q| dex.col(q).is_some_and(|(ix, iy)| dex.top_at(ix, iy) > p.z + Z_EPS)) {
                return Some(p);
            }
        }
    }
    None
}
