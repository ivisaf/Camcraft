//! Placing a fixture on a plate's grid (#819): the fixture's model frame
//! goes on a named hole (or on a dowel pair) at the plate's top face,
//! turned by a yaw about the plate's Z. The location rides the placed
//! fixture's marker, so moving the plate carries every fixture on it
//! (transitively: a subplate carries its own riders) in the plate's
//! one command.
//!
//! - **At a hole:** the fixture frame's origin over the hole's centre,
//!   its axes the plate's turned by `yaw_deg`, and the fixture standing
//!   on the plate: its lowest point on the top face (a vise's base, an
//!   imported fixture's underside, whatever its model origin).
//! - **On a dowel pair:** the origin on the first dowel hole, its +X
//!   pointing at the second (then turned by `yaw_deg`) - the way a
//!   two-pin locator sits.
//! - **On the plate:** the fixture's footprint (its mesh's XY extent,
//!   placed) must lie within the plate's; one hanging off is refused
//!   with the side and the amount.
//!
//! Provenance: our own placement rules; rigid-frame composition from
//! published quaternion algebra.

use std::collections::HashSet;

use flecs_ecs::prelude::World;
use serde::{Deserialize, Serialize};

use crate::fixture;
use crate::geometry::{IDENTITY_Q, Point3, Triangle, quat_from_euler, transform_point};
use crate::model;
use crate::schema::{FixtureDef, Location};
use crate::setup_align::{compose, inverse};

use super::PlateDef;

/// Where on a plate a fixture stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountAt {
    /// A tapped-grid location ("C3").
    Hole(String),
    /// Two dowel-lattice locations: the origin on the first, +X toward
    /// the second.
    Dowels([String; 2]),
}

impl MountAt {
    /// How the location reads: "C3", or "dowels A1-A3".
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            MountAt::Hole(h) => h.trim().to_ascii_uppercase(),
            MountAt::Dowels([a, b]) => format!("dowels {}-{}", a.trim().to_ascii_uppercase(), b.trim().to_ascii_uppercase()),
        }
    }
}

/// A fixture's place on a plate: the plate's model name, the location,
/// and a turn about the plate's Z (degrees).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlateMount {
    pub plate: String,
    pub at: MountAt,
    #[serde(default)]
    pub yaw_deg: f64,
}

/// One carried fixture's pose before and after a plate command.
#[derive(Clone, Debug, PartialEq)]
pub struct RiderMove {
    pub name: String,
    pub before: Location,
    pub after: Location,
}

impl PlateDef {
    /// The fixture frame's pose in the plate's model frame for location
    /// `at` turned `yaw_deg`, or why not (named).
    pub fn local_pose(&self, at: &MountAt, yaw_deg: f64) -> Result<Location, String> {
        if !yaw_deg.is_finite() {
            return Err(format!("the turn on the plate must be finite (got {yaw_deg})"));
        }
        let (x, y, heading) = match at {
            MountAt::Hole(h) => {
                let (x, y) = self.holes.locate(h)?;
                (x, y, 0.0)
            }
            MountAt::Dowels([a, b]) => {
                let g = self.dowel_lattice()?;
                let (ax, ay) = g.locate(a).map_err(|e| format!("dowel {e}"))?;
                let (bx, by) = g.locate(b).map_err(|e| format!("dowel {e}"))?;
                if (bx - ax).hypot(by - ay) < 1e-9 {
                    return Err(format!("a dowel pair needs two different holes (got {} twice)", a.trim()));
                }
                (ax, ay, (by - ay).atan2(bx - ax))
            }
        };
        let rot = quat_from_euler(0.0, 0.0, heading + yaw_deg.to_radians());
        Ok(Location { pos: Point3::new(x, y, self.top_z), rot })
    }
}

/// XY bounds of `pts`.
fn xy_bounds(pts: impl Iterator<Item = Point3>) -> Option<[f64; 4]> {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let mut any = false;
    for p in pts {
        any = true;
        b[0] = b[0].min(p.x);
        b[1] = b[1].min(p.y);
        b[2] = b[2].max(p.x);
        b[3] = b[3].max(p.y);
    }
    (any && b.iter().all(|v| v.is_finite())).then_some(b)
}

/// How far a footprint may stand past the plate edge (float dust).
const EDGE_TOL: f64 = 1e-6;

/// The on-the-plate law: `fixture` (its own mesh) placed at `local` in
/// the plate's frame keeps its XY footprint inside `plate`'s (the
/// plate's own mesh). Refused with the side and the overhang.
pub fn on_the_plate(plate: &[Triangle], fixture: &[Triangle], local: Location) -> Result<(), String> {
    let pb = xy_bounds(plate.iter().flat_map(|t| [t.a, t.b, t.c])).ok_or("the plate has no geometry")?;
    let fb = xy_bounds(fixture.iter().flat_map(|t| [t.a, t.b, t.c]).map(|p| transform_point(p, local.pos, local.rot)))
        .ok_or("the fixture has no geometry")?;
    for (side, over) in [("-X", pb[0] - fb[0]), ("-Y", pb[1] - fb[1]), ("+X", fb[2] - pb[2]), ("+Y", fb[3] - pb[3])] {
        if over > EDGE_TOL {
            return Err(format!("it would hang {over:.3} mm off the plate's {side} edge"));
        }
    }
    Ok(())
}

/// The fixtures placed directly on plate `plate`, by name.
#[must_use]
pub fn riders(world: &World, plate: &str) -> Vec<String> {
    model::named_entities(world)
        .into_iter()
        .filter(|(_, e)| fixture::def_of(world, e).and_then(|d| d.plate.mount).is_some_and(|m| m.plate == plate))
        .map(|(n, _)| n)
        .collect()
}

/// The fixtures riding on `plate` - directly or on a subplate of it -
/// each moved by the rigid `delta` (world frame). `skip` names fixtures
/// the caller moves itself. Cycles end at the first revisit.
#[must_use]
pub fn carry(world: &World, plate: &str, delta: Location, skip: &[&str]) -> Vec<RiderMove> {
    let mut seen: HashSet<String> = skip.iter().map(|s| (*s).to_owned()).collect();
    seen.insert(plate.to_owned());
    let mut out = Vec::new();
    let mut queue = vec![plate.to_owned()];
    while let Some(p) = queue.pop() {
        for name in riders(world, &p) {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(e) = model::find_by_name(world, &name) else { continue };
            let before = model::transform_of(world, &e).unwrap_or_default();
            out.push(RiderMove { name: name.clone(), before, after: compose(delta, before) });
            queue.push(name);
        }
    }
    out
}

/// The rigid move from pose `prev` to `next`: `next ∘ prev⁻¹`.
#[must_use]
pub fn delta(prev: Location, next: Location) -> Location {
    compose(next, inverse(prev))
}

/// Write the carried poses: `after` forward, `before` on undo.
pub fn write(world: &World, moves: &[RiderMove], forward: bool) {
    for m in moves {
        if let Some(e) = model::find_by_name(world, &m.name) {
            model::set_transform(world, &e, if forward { m.after } else { m.before });
        }
    }
}

/// `local` lifted so `fixture` (its own mesh) stands on the plate top:
/// its lowest point at `local`'s z. A yaw about Z keeps heights, so the
/// mesh's own lowest z is the one that lands.
#[must_use]
pub fn stand(local: Location, fixture: &[Triangle]) -> Location {
    let low = fixture.iter().flat_map(|t| [t.a.z, t.b.z, t.c.z]).fold(f64::INFINITY, f64::min);
    if !low.is_finite() {
        return local;
    }
    Location { pos: Point3::new(local.pos.x, local.pos.y, local.pos.z - low), rot: local.rot }
}

/// The direct riders of a plate that is going away - deleted, untagged,
/// or re-imported without its grid (#819 review): their markers as they
/// were. Forward, each rider loses its mount (it stays where it stands);
/// on undo the markers come back verbatim. Without it a rider kept the
/// gone plate's NAME, and the next plate given that name would carry it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Unmounts(Vec<(String, FixtureDef)>);

impl Unmounts {
    /// Snapshot `plate`'s direct riders now (command construction).
    #[must_use]
    pub fn of(world: &World, plate: &str) -> Self {
        Unmounts(
            riders(world, plate)
                .into_iter()
                .filter_map(|n| model::find_by_name(world, &n).and_then(|e| fixture::def_of(world, &e)).map(|d| (n, d)))
                .collect(),
        )
    }

    /// The riders' names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(n, _)| n.as_str())
    }

    /// Clear the mounts (`forward`) or put the markers back.
    pub fn write(&self, world: &World, forward: bool) {
        for (name, def) in &self.0 {
            if let Some(e) = model::find_by_name(world, name) {
                let mut d = def.clone();
                if forward {
                    d.plate.mount = None;
                }
                e.entity_view(world).set(d);
            }
        }
    }
}

/// The load-time half of the law above: a mount naming a fixture that is
/// not a plate (gone, untagged, or no grid) is dropped and logged - a
/// file saved before the delete-clears-riders fix, or edited by hand.
/// Returns the fixtures it unmounted.
pub fn drop_dangling(world: &World) -> Vec<String> {
    let is_plate = |name: &str| {
        model::find_by_name(world, name).and_then(|e| fixture::def_of(world, &e)).is_some_and(|d| d.plate.plate.is_some())
    };
    let mut out = Vec::new();
    for (name, e) in model::named_entities(world) {
        let Some(mut d) = fixture::def_of(world, &e) else { continue };
        let Some(m) = &d.plate.mount else { continue };
        if m.plate != name && is_plate(&m.plate) {
            continue;
        }
        tracing::warn!(model = %name, plate = %m.plate, "fixture plate location dropped on load: no such plate");
        d.plate.mount = None;
        e.entity_view(world).set(d);
        out.push(name);
    }
    out
}

/// No turn.
#[must_use]
pub fn identity() -> Location {
    Location { pos: Point3::ZERO, rot: IDENTITY_Q }
}
