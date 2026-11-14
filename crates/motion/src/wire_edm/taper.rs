//! Wire EDM taper contour (#536, part of #290): one closed profile cut
//! by a wire leaning at a constant angle from vertical. Per D2 the lean
//! is the move's pose - a unit vector from the wire's point on the
//! lower programmed plane (`to`) toward the upper guide - so the moves
//! stay one programmed plane and the wire between two moves sweeps the
//! ruled surface between their lower points and their upper points.
//!
//! The profile is read on the TOP plane (#532: the plane the profile is
//! read on): there the wire-centre path is the profile's parallel at
//! the kerf offset measured across the leaning wire, so the horizontal
//! offset is `(wire radius + gap) / cos(angle)`. Every side then leans
//! by the angle in the plane square to it: its lower edge is the top
//! edge moved `height * tan(angle)` outward of the ring (a positive
//! angle widens the profile downward - a die's relief, a drafted
//! punch's base), and each corner moves along its mitre so the two
//! sides' faces stay planar and meet in a straight corner line.
//!
//! Provenance: plane trigonometry (the horizontal section of a cylinder
//! leaning at `a` is an ellipse whose half-width across the lean is
//! `r / cos(a)`; a polygon's sides moved by `d` meet at the vertex
//! moved by `d * (n1 + n2) / (1 + n1.n2)`, the textbook mitre), over the
//! umbrella's decisions D2 and D7. Our own code; no vendor post,
//! control manual or CAM product behaviour was consulted.
//!
//! Refused by name, never trimmed: a taper on an open run (#1109,
//! which has no ring to lean), a taper the machine does not state a
//! maximum for, an angle (or a corner's lean) above that maximum, a
//! corner too sharp to lean, and a side the lean folds over.

use super::{notes, wire_contour, WireParams, WireSide};
use crate::cl::{MoveKind, PathBuilder, Toolpath};
use crate::geo2d::{Loop, Point2};
use crate::geometry::Point3;
use crate::machining2d::GenError;

/// The sharpest turn (as `1 + n1.n2`) a leaning corner may make: a
/// turn of about 140 degrees, past which the mitre runs away.
const MITRE_FLOOR: f64 = 0.25;

/// The constant taper of a Wire Contour, resolved to plain numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct WireTaper {
    /// Degrees from vertical; positive widens the profile downward.
    pub angle_deg: f64,
    /// The top plane, where the profile is read.
    pub top_z: f64,
    /// The machine's stated maximum taper, degrees; `None` = not stated.
    pub max_deg: Option<f64>,
    /// The machine's name, as the refusals say it.
    pub machine: String,
}

fn invalid(why: String) -> GenError {
    GenError::Invalid(format!("wire contour: {why}"))
}

impl WireTaper {
    /// The door: the angle and the machine's maximum, by name.
    ///
    /// # Errors
    /// A non-finite angle, an unstated maximum, or an angle above it.
    pub fn door(&self) -> Result<(), GenError> {
        let a = self.angle_deg;
        if !(a.is_finite() && a.abs() < 90.0) {
            return Err(invalid(format!("the taper angle {a} is not an angle between -90 and 90 degrees")));
        }
        let Some(max) = self.max_deg else {
            return Err(invalid(format!(
                "a {a:.3} degree taper is asked, and machine {} states no edm.max_taper_deg - a taper is cut only on a machine that states how far it leans",
                self.machine
            )));
        };
        if a.abs() > max + 1e-9 {
            return Err(invalid(format!(
                "a {:.3} degree taper is asked, and machine {} leans the wire at most {max:.3} degrees (edm.max_taper_deg)",
                a.abs(),
                self.machine
            )));
        }
        Ok(())
    }
}

/// [`wire_contour`] with a constant taper. A zero angle is the vertical
/// cut itself, byte for byte.
///
/// # Errors
/// Every refusal of [`wire_contour`], plus the taper's own (module head).
pub fn wire_contour_tapered(loops: &[Loop], chains: &[Vec<Point2>], p: &WireParams, t: &WireTaper) -> Result<Toolpath, GenError> {
    if t.angle_deg == 0.0 {
        return wire_contour(loops, chains, p);
    }
    // An open run (#1109) has no ring to lean: reading its polyline as
    // one would close it with a chord back across the part. Refused by
    // name before anything is leaned.
    if !chains.is_empty() || matches!(p.side, WireSide::Left | WireSide::Right) {
        return Err(invalid(format!(
            "a {:.3} degree taper is asked on an open run - a taper is cut only around one closed profile (die or punch); cut the open run vertical (taper 0)",
            t.angle_deg
        )));
    }
    t.door()?;
    let h = t.top_z - p.plane_z;
    if !(h.is_finite() && h > 0.0) {
        return Err(invalid(format!("the top plane ({:.3}) does not stand above the programmed plane ({:.3})", t.top_z, p.plane_z)));
    }
    let a = t.angle_deg.to_radians();
    // The top-plane path at the kerf measured across the leaning wire:
    // every law of the vertical cut (cover, start hole, kerf) runs there.
    let s = 1.0 / a.cos();
    let top = WireParams { wire_diameter: p.wire_diameter * s, gap: p.gap * s, plane_z: t.top_z, ..p.clone() };
    let flat = wire_contour(loops, chains, &top)?;
    let Some(start) = p.start else {
        return Err(invalid("no start hole is stated".into()));
    };
    // The ring: every move's end but the lead-out, its closing repeat
    // of the first point dropped.
    let ends: Vec<Point2> = flat.moves.iter().map(|m| Point2::new(m.to.x, m.to.y)).collect();
    let ring = match ends.len().checked_sub(2) {
        Some(n) if n >= 3 => &ends[..n],
        _ => return Err(invalid("the wire path lays down no cut".into())),
    };
    let d = h * a.tan();
    let lean = leans(ring, d, h, t)?;
    let z = p.plane_z;
    let mut b = PathBuilder::new(Point3::new(start.x, start.y, z));
    for i in (0..ring.len()).chain(std::iter::once(0)) {
        let (lo, axis) = lean[i];
        b.set_axis(Some(axis));
        b.push(MoveKind::Feed, Point3::new(lo.x, lo.y, z), None);
    }
    // The lead-out stands the wire upright again at the start hole.
    b.set_axis(Some([0.0, 0.0, 1.0]));
    b.push(MoveKind::Feed, Point3::new(start.x, start.y, z), None);
    let mut path = b.finish();
    let lead = super::ring::dist(start, lean[0].0);
    let mut said = notes(p, start, lead);
    said.pop();
    said.push(format!(
        "wire EDM: a {:.3} degree taper ({}) - the profile and the kerf held on the top plane Z{:.3}, each side leaning in the plane square to it, the moves' poses the wire toward the upper guide; vertical at the start hole; no feed (the control's generator sets the rate)",
        t.angle_deg,
        if t.angle_deg > 0.0 { "the profile widens downward" } else { "the profile narrows downward" },
        t.top_z
    ));
    path.notes.extend(said);
    Ok(path)
}

/// Each ring point's lower-plane point and pose: moved `d` along its
/// mitre, outward of the counter-clockwise ring.
fn leans(ring: &[Point2], d: f64, h: f64, t: &WireTaper) -> Result<Vec<(Point2, [f64; 3])>, GenError> {
    let n = ring.len();
    let normal = |a: Point2, b: Point2| {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l = dx.hypot(dy);
        (l > 0.0).then(|| (dy / l, -dx / l))
    };
    let at = |q: Point2| Point3::new(q.x, q.y, t.top_z);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (prev, q, next) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (Some(n1), Some(n2)) = (normal(prev, q), normal(q, next)) else {
            return Err(invalid("the wire path repeats a point - it cannot be leaned".into()));
        };
        let c = 1.0 + n1.0 * n2.0 + n1.1 * n2.1;
        if c < MITRE_FLOOR {
            return Err(GenError::invalid_at(
                format!("wire contour: the corner at X{:.3} Y{:.3} is too sharp to taper - its mitre runs away", q.x, q.y),
                at(q),
            ));
        }
        let lo = Point2::new(q.x + d * (n1.0 + n2.0) / c, q.y + d * (n1.1 + n2.1) / c);
        let (ux, uy) = (q.x - lo.x, q.y - lo.y);
        let lean = ux.hypot(uy).atan2(h).to_degrees();
        if let Some(max) = t.max_deg
            && lean > max + 1e-9
        {
            return Err(GenError::invalid_at(
                format!(
                    "wire contour: at the corner at X{:.3} Y{:.3} the wire leans {lean:.3} degrees, and machine {} leans it at most {max:.3} degrees (edm.max_taper_deg)",
                    q.x, q.y, t.machine
                ),
                at(q),
            ));
        }
        let len = (ux * ux + uy * uy + h * h).sqrt();
        out.push((lo, [ux / len, uy / len, h / len]));
    }
    // A side the lean turns over is a side the lower plane cannot cut.
    for i in 0..n {
        let j = (i + 1) % n;
        let (q0, q1, l0, l1) = (ring[i], ring[j], out[i].0, out[j].0);
        if (l1.x - l0.x) * (q1.x - q0.x) + (l1.y - l0.y) * (q1.y - q0.y) <= 0.0 {
            return Err(GenError::invalid_at(
                format!(
                    "wire contour: the taper folds the side from X{:.3} Y{:.3} over on the lower plane - the side is shorter than the lean ({:.4} mm)",
                    q0.x,
                    q0.y,
                    d.abs()
                ),
                at(q0),
            ));
        }
    }
    Ok(out)
}
