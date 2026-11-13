//! Where a placed jaw block's clamping face stands against the stock
//! (#820 review): after a plate edit re-seats a block on its named hole
//! with its slide kept, the stock does not move - the face may now reach
//! into it, or stand off it. Judged in the block's own frame against the
//! stock's world box (the same box the pair fit uses).
//!
//! Provenance: rigid-frame algebra from published quaternion identities;
//! the face law is our own (`modvise` module docs).

use crate::geometry::{Aabb, Point3, transform_point};
use crate::schema::Location;
use crate::setup_align::inverse;

use super::JawParams;

/// The stock's near side minus the clamping face, along the block's
/// clamping direction (local +Y): negative when the face reaches into the
/// stock (by that much), positive when it stands off. `None` when the
/// stock is not in front of the block at all (beside it, above it,
/// behind it) or a figure is not finite.
#[must_use]
pub fn face_gap(jaw: &JawParams, pose: Location, stock: &Aabb) -> Option<f64> {
    let inv = inverse(pose);
    let pts: [Point3; 8] = std::array::from_fn(|k| {
        let c = Point3::new(
            if k & 1 == 0 { stock.min.x } else { stock.max.x },
            if k & 2 == 0 { stock.min.y } else { stock.max.y },
            if k & 4 == 0 { stock.min.z } else { stock.max.z },
        );
        transform_point(c, inv.pos, inv.rot)
    });
    let span = |f: &dyn Fn(&Point3) -> f64| pts.iter().map(f).fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let (x_lo, x_hi) = span(&|p| p.x);
    let (t_lo, t_hi) = span(&|p| p.y);
    let (z_lo, z_hi) = span(&|p| p.z);
    if ![x_lo, x_hi, t_lo, t_hi, z_lo, z_hi].iter().all(|v| v.is_finite()) {
        return None;
    }
    let xc = jaw.centre_x();
    let half = 0.5 * jaw.body_width.max(jaw.jaw_width);
    let tall = jaw.body_height.max(jaw.jaw_height);
    let face = jaw.face_y();
    // Above the insert only the body's front stands: the insert's back.
    let front = if z_lo < jaw.jaw_height { face } else { face - jaw.jaw_thickness };
    let rear = face - jaw.jaw_thickness - jaw.body_length;
    let across = x_hi > xc - half && x_lo < xc + half;
    let up = z_hi > 0.0 && z_lo < tall;
    (across && up && t_hi > rear).then_some(t_lo - front)
}
