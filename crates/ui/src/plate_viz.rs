//! Fixture plate markers in the viewport (#819): every plate's grid
//! holes drawn as small crosses at its top face (dowel holes as
//! diamonds), and the location names along row A and column 1, so the
//! A1 / B3 convention reads straight off the model. Screen-space marks
//! (egui strokes), projected through the same camera math as the
//! boundary marks; bounded per plate so a huge grid stays a bounded draw.
//!
//! Provenance: our own display design.

use camcraft_app::geometry::{Point3, transform_point};
use camcraft_app::{fixture, model};

use crate::interact::project_uv;
use crate::state::State;

/// Holes marked per plate, at most.
const MAX_MARKED: usize = 4000;

/// What one frame draws: stroke segments (viewport uv) with a dowel
/// flag, and labels (uv, text).
#[derive(Default, Debug)]
pub struct PlateMarks {
    pub segs: Vec<([f32; 2], [f32; 2], bool)>,
    pub labels: Vec<([f32; 2], String)>,
}

/// The markers of every plate in the document.
#[must_use]
pub fn plate_marks(state: &State, aspect: f64) -> PlateMarks {
    let mut out = PlateMarks::default();
    let mvp = state.cam.mvp(aspect);
    for (_, e) in model::named_entities(&state.world) {
        let Some(def) = fixture::def_of(&state.world, &e).and_then(|d| d.plate.plate) else { continue };
        let at = model::transform_of(&state.world, &e).unwrap_or_default();
        let place = |x: f64, y: f64| {
            let p = transform_point(Point3::new(x, y, def.top_z), at.pos, at.rot);
            [p.x, p.y, p.z]
        };
        for (grid, dowel) in [(Some(&def.holes), false), (def.dowels.as_ref(), true)] {
            let Some(g) = grid else { continue };
            let s = 0.2 * g.pitch_x.min(g.pitch_y);
            for (name, x, y) in g.holes().into_iter().take(MAX_MARKED) {
                let arms: [((f64, f64), (f64, f64)); 2] = if dowel {
                    [((x - s, y), (x + s, y)), ((x, y - s), (x, y + s))]
                } else {
                    [((x - s, y - s), (x + s, y + s)), ((x - s, y + s), (x + s, y - s))]
                };
                for ((ax, ay), (bx, by)) in arms {
                    if let (Some(a), Some(b)) = (project_uv(&mvp, place(ax, ay)), project_uv(&mvp, place(bx, by))) {
                        out.segs.push((a, b, dowel));
                    }
                }
                let edge = name.starts_with('A') && name[1..].bytes().all(|b| b.is_ascii_digit())
                    || name.ends_with('1') && !name[..name.len() - 1].bytes().any(|b| b.is_ascii_digit());
                if !dowel && edge && let Some(uv) = project_uv(&mvp, place(x + s, y + s)) {
                    out.labels.push((uv, name));
                }
            }
        }
    }
    out
}
