//! #672: the plasma through-cut laws - cut order, kerf crossing, tip-up,
//! and malformed programs - each tripped by one fixture and passed by
//! another.
//!
//! Gate map (#672): covers sim::verify::plasma.
//! Provenance: our own fixtures.

use camcraft_sim::geo2d::Point2;
use camcraft_sim::verify::plasma::{judge, CutRole, PlasmaCut, PlasmaFinding, PlasmaProgram};

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

/// A closed square, from its lower-left corner, side `s`.
fn square(x: f64, y: f64, s: f64) -> Vec<Point2> {
    vec![p(x, y), p(x + s, y), p(x + s, y + s), p(x, y + s), p(x, y)]
}

fn cut(contour: u32, role: CutRole, parent: Option<u32>, path: Vec<Point2>) -> PlasmaCut {
    PlasmaCut { contour, role, parent, small: false, path }
}

use CutRole::{Inner, Open, Outer};

/// A 100 mm plate part (outer 1) with a 20 mm hole (inner 2) in it.
fn plate(hole_first: bool) -> PlasmaProgram {
    let hole = cut(2, Inner, Some(1), square(40.0, 40.0, 20.0));
    let outer = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let cuts = if hole_first { vec![hole, outer] } else { vec![outer, hole] };
    PlasmaProgram { kerf: 1.5, cuts, rapids: vec![None, None] }
}

#[test]
fn holes_before_the_outline_pass_clean() {
    assert_eq!(judge(&plate(true)), vec![]);
}

#[test]
fn an_outline_before_its_hole_is_refused_by_name() {
    let f = judge(&plate(false));
    assert_eq!(f, vec![PlasmaFinding::CutOrder { contour: 2, enclosing: 1 }]);
    assert!(f[0].refuses());
}

#[test]
fn a_cut_within_a_kerf_of_an_earlier_one_is_warned() {
    // Two parts side by side, 1 mm apart with a 1.5 mm kerf: the second
    // outline runs inside the first one's open kerf.
    let a = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let b = cut(2, Outer, None, square(51.0, 0.0, 50.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a.clone(), b], rapids: vec![None, None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::KerfCrossing { contour: 2, earlier: 1, .. }]), "{f:?}");
    assert!(!f[0].refuses(), "a warning, not a refusal");
    // 5 mm apart: clear.
    let c = cut(2, Outer, None, square(55.0, 0.0, 50.0));
    assert_eq!(judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a, c], rapids: vec![None, None] }), vec![]);
}

#[test]
fn a_rapid_over_a_freed_part_is_warned() {
    // Part 1 (0..50) is cut free, then the rapid to part 3 (at x 120)
    // starts from the far side of part 1 and crosses it.
    let a = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let b = cut(3, Outer, None, square(120.0, 0.0, 30.0));
    let over = Some((p(-10.0, 25.0), p(120.0, 0.0)));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a.clone(), b.clone()], rapids: vec![None, over] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 3, freed: 1, .. }]), "{f:?}");
    // Around it, below the part: clear.
    let around = Some((p(0.0, -10.0), p(120.0, -10.0)));
    assert_eq!(judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a, b], rapids: vec![None, around] }), vec![]);
}

#[test]
fn a_malformed_program_is_refused_by_name() {
    for kerf in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let f = judge(&PlasmaProgram { kerf, ..plate(true) });
        assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("kerf")), "{kerf}: {f:?}");
    }
    let mut bad = plate(true);
    bad.cuts[0].path[1] = p(f64::NAN, 0.0);
    assert!(matches!(judge(&bad).as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("non-finite")));
    let mut empty = plate(true);
    empty.cuts[1].path.truncate(1);
    assert!(matches!(judge(&empty).as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("no cut path")));
}

/// Review of #675: an island (an outer contour inside a hole) cut after
/// its hole is refused - the hole freed the slug the island sits in.
#[test]
fn an_island_after_its_hole_is_refused() {
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let hole = cut(2, Inner, Some(1), square(30.0, 30.0, 40.0));
    let island = cut(3, Outer, Some(2), square(45.0, 45.0, 10.0));
    let good = PlasmaProgram { kerf: 1.5, cuts: vec![island.clone(), hole.clone(), outline.clone()], rapids: vec![None; 3] };
    assert_eq!(judge(&good), vec![], "island, hole, outline");
    let bad = PlasmaProgram { kerf: 1.5, cuts: vec![hole, island, outline], rapids: vec![None; 3] };
    assert_eq!(judge(&bad), vec![PlasmaFinding::CutOrder { contour: 3, enclosing: 2 }]);
}

/// Review of #675: parents, ids and rapids that can't be judged are
/// refused by name, never silently skipped.
#[test]
fn bad_parents_ids_and_rapids_are_refused_by_name() {
    let mal = |p: PlasmaProgram, word: &str| {
        let f = judge(&p);
        assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains(word)), "{word}: {f:?}");
    };
    let mut p = plate(true);
    p.cuts[0].parent = Some(9);
    mal(p, "no cut carries");
    let mut p = plate(true);
    p.cuts[0].parent = Some(2);
    mal(p, "itself");
    let mut p = plate(true);
    p.cuts[0].parent = None;
    mal(p, "no parent");
    let mut p = plate(true);
    p.cuts[1].contour = 2;
    mal(p, "twice");
    let mut p = plate(true);
    p.rapids.pop();
    mal(p, "one rapid entry per cut");
    let mut p = plate(true);
    p.rapids[1] = Some((p.cuts[0].path[0], Point2::new(f64::NAN, 0.0)));
    mal(p, "rapid into contour 1");
}

/// A centre-pierced small hole may be a single point; an open path is
/// never a freed area for tip-up.
#[test]
fn small_holes_and_open_paths() {
    let mut dot = cut(2, Inner, Some(1), vec![p(50.0, 50.0)]);
    dot.small = true;
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    assert_eq!(judge(&PlasmaProgram { kerf: 1.5, cuts: vec![dot, outline], rapids: vec![None, None] }), vec![]);
    // A slit (open) cut first, then a rapid across it: not a tip-up.
    let slit = cut(1, Open, None, vec![p(0.0, 0.0), p(50.0, 0.0), p(50.0, 50.0), p(0.0, 50.0), p(0.0, 0.0)]);
    let next = cut(3, Outer, None, square(120.0, 0.0, 30.0));
    let over = Some((p(-10.0, 25.0), p(120.0, 0.0)));
    assert_eq!(judge(&PlasmaProgram { kerf: 1.5, cuts: vec![slit, next], rapids: vec![None, over] }), vec![]);
}

// ---- #672 step 2: the review's notes and the laws the CL reader allows ----

/// Review of #675: `small` is the small-hole rule, and only a hole is
/// centre-pierced - a small OUTER (or OPEN) contour is refused by name.
#[test]
fn a_small_mark_on_anything_but_a_hole_is_refused() {
    let mut dot = cut(2, Outer, None, vec![p(50.0, 50.0)]);
    dot.small = true;
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![dot, outline], rapids: vec![None, None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("small hole") && m.contains("OUTER")), "{f:?}");
}

/// Review of #675: a one-point cut that is not a small hole has no path.
#[test]
fn a_one_point_cut_without_the_small_mark_is_refused() {
    let dot = cut(2, Inner, Some(1), vec![p(50.0, 50.0)]);
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![dot, outline], rapids: vec![None, None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("contour 2 has no cut path")), "{f:?}");
}

/// Review of #675: an open run encloses nothing, so a contour naming one
/// as its parent is malformed.
#[test]
fn an_open_parent_is_malformed() {
    let slit = cut(1, Open, None, vec![p(0.0, 0.0), p(50.0, 0.0)]);
    let hole = cut(2, Inner, Some(1), square(10.0, 10.0, 5.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![hole, slit], rapids: vec![None, None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("open run")), "{f:?}");
}

/// A closed contour's cut must return to its start; a polyline that
/// never closes under an OUTER or INNER role is refused by name.
#[test]
fn a_closed_contour_whose_cut_never_closes_is_refused() {
    let open = cut(1, Outer, None, vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)]);
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![open], rapids: vec![None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains("never returns to its start")), "{f:?}");
}

/// Review of #675: enclosure is recomputed from the rings, and a PARENT
/// the geometry disagrees with is refused by name - each way round.
#[test]
fn a_stated_parent_the_rings_disagree_with_is_refused() {
    let mal = |p: PlasmaProgram, word: &str| {
        let f = judge(&p);
        assert!(matches!(f.as_slice(), [PlasmaFinding::Malformed(m)] if m.contains(word) && m.contains("disagree")), "{word}: {f:?}");
    };
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    // A "hole" placed outside the outline it names.
    let away = cut(2, Inner, Some(1), square(200.0, 200.0, 20.0));
    mal(PlasmaProgram { kerf: 1.5, cuts: vec![away, outline.clone()], rapids: vec![None; 2] }, "no cut encloses it");
    // An outer contour inside the outline that names no parent.
    let unsaid = cut(3, Outer, None, square(40.0, 40.0, 20.0));
    mal(PlasmaProgram { kerf: 1.5, cuts: vec![unsaid, outline.clone()], rapids: vec![None; 2] }, "names no parent but");
    // An island naming the outline instead of the hole it sits in.
    let hole = cut(2, Inner, Some(1), square(30.0, 30.0, 40.0));
    let island = cut(3, Outer, Some(1), square(45.0, 45.0, 10.0));
    mal(
        PlasmaProgram { kerf: 1.5, cuts: vec![island, hole, outline], rapids: vec![None; 3] },
        "lies inside contour 2",
    );
}

/// The 2D gouge: a ring cutting through an earlier ring is refused;
/// two parts clear of each other pass.
#[test]
fn a_ring_crossing_an_earlier_ring_is_refused() {
    let a = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let b = cut(2, Outer, None, square(40.0, 10.0, 50.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a.clone(), b], rapids: vec![None, None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::CutCrossing { contour: 2, earlier: 1, .. }]), "{f:?}");
    assert!(f[0].refuses(), "a refusal");
    assert!(f[0].to_string().contains("near X"), "{}", f[0]);
    let clear = cut(2, Outer, None, square(60.0, 10.0, 50.0));
    assert_eq!(judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a, clear], rapids: vec![None, None] }), vec![]);
}

/// A rapid that leaves from the edge of the part just freed (a cut with
/// no lead-out ends on its ring) is not a tip-up; one that starts inside
/// the freed part is.
#[test]
fn a_rapid_leaving_a_freed_parts_edge_is_not_a_tip_up() {
    let a = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let b = cut(3, Outer, None, square(120.0, 0.0, 30.0));
    let from_edge = Some((p(50.0, 25.0), p(120.0, 0.0)));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a.clone(), b.clone()], rapids: vec![None, from_edge] });
    assert_eq!(f, vec![]);
    let from_inside = Some((p(49.0, 25.0), p(120.0, 0.0)));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a, b], rapids: vec![None, from_inside] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 3, freed: 1, .. }]), "{f:?}");
}

/// #791 review: a rapid in or out of a freed part through one of its
/// corners never crosses an edge properly, yet it runs over the part.
/// The rapid is split at every contact with the ring and judged piece
/// by piece, so each of these is a tip-up, placed inside the part: (a)
/// leaving from the edge, across the part, out at the (0,0) corner; (b)
/// from a corner along the whole diagonal; (c) in at one corner and
/// out at the other. A rapid running along the part's edge stays clear.
#[test]
fn a_rapid_through_a_freed_parts_corner_is_a_tip_up() {
    let a = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let b = cut(3, Outer, None, square(120.0, 0.0, 30.0));
    let run = |from: Point2, to: Point2| {
        judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a.clone(), b.clone()], rapids: vec![None, Some((from, to))] })
    };
    for (from, to) in [(p(50.0, 25.0), p(-50.0, -25.0)), (p(0.0, 0.0), p(70.0, 70.0)), (p(-10.0, -10.0), p(70.0, 70.0))] {
        let f = run(from, to);
        assert!(
            matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 3, freed: 1, slug: false, at }]
                if at.x > 0.0 && at.x < 50.0 && at.y > 0.0 && at.y < 50.0),
            "{from:?} -> {to:?}: {f:?}"
        );
    }
    // Along the bottom edge and on past the corner: never over the area.
    assert_eq!(run(p(50.0, 0.0), p(-20.0, 0.0)), vec![]);
}

/// #791 review: two overlapping rings that meet only at vertices and
/// along collinear edges never cross properly, but the later ring runs
/// through the earlier part all the same - the 2D gouge, refused. Two
/// parts sharing an edge (common-line cutting) enter nothing and stay
/// legal: the shared kerf is the torch-height warning, not a refusal.
#[test]
fn an_overlapping_ring_meeting_only_at_vertices_and_edges_is_refused() {
    let a = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let over = cut(2, Outer, None, square(25.0, 0.0, 50.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a.clone(), over], rapids: vec![None, None] });
    assert!(
        matches!(f.as_slice(), [PlasmaFinding::CutCrossing { contour: 2, earlier: 1, at }]
            if at.x > 0.0 && at.x < 50.0 && at.y > 0.0 && at.y < 50.0),
        "{f:?}"
    );
    assert!(f[0].refuses());
    let shared = cut(2, Outer, None, square(50.0, 0.0, 50.0));
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![a, shared], rapids: vec![None, None] });
    assert!(matches!(f.as_slice(), [PlasmaFinding::KerfCrossing { contour: 2, earlier: 1, .. }]), "{f:?}");
    assert!(!f[0].refuses(), "common-line cutting is legal");
}

/// #791 delta review: a later part laid inside an earlier one and
/// touching its edge has nothing outside it, yet its inner edge (x = 25)
/// runs through the freed part - the 2D gouge, refused, not the
/// torch-height warning. #801: the touching case that stays legal is
/// pinned on a program that reaches the laws - a part nested in a hole
/// and touching the hole's wall, cut first (the vertex-touching hole the
/// old pin used is malformed before any law runs, so it proved nothing).
#[test]
fn a_ring_inside_an_earlier_part_touching_its_edge_is_refused() {
    let e = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let l = cut(2, Outer, None, vec![p(0.0, 0.0), p(25.0, 0.0), p(25.0, 50.0), p(0.0, 50.0), p(0.0, 0.0)]);
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![e, l], rapids: vec![None, None] });
    assert!(
        matches!(f.as_slice(), [PlasmaFinding::CutCrossing { contour: 2, earlier: 1, at }]
            if at.x > 0.0 && at.x < 50.0 && at.y > 0.0 && at.y < 50.0),
        "{f:?}"
    );
    assert!(f[0].refuses());
    let f = judge(&nested(true, &[0, 1, 2]));
    assert!(matches!(f.as_slice(), [PlasmaFinding::KerfCrossing { contour: 2, earlier: 3, .. }]), "{f:?}");
}

// ---- #801: the third review of #791 ----

/// A plate (outer 1, 0..100) with a hole (inner 2, 20..80) and a part
/// (outer 3) nested in the hole - clear of its wall, or (`touching`)
/// laid against the hole's left wall. `order` indexes [part, hole,
/// outline].
fn nested(touching: bool, order: &[usize]) -> PlasmaProgram {
    let part = if touching {
        cut(3, Outer, Some(1), vec![p(20.0, 30.0), p(50.0, 30.0), p(50.0, 60.0), p(20.0, 60.0), p(20.0, 30.0)])
    } else {
        cut(3, Outer, Some(2), square(30.0, 30.0, 40.0))
    };
    let all = [part, cut(2, Inner, Some(1), square(20.0, 20.0, 60.0)), cut(1, Outer, None, square(0.0, 0.0, 100.0))];
    let cuts: Vec<PlasmaCut> = order.iter().map(|&k| all[k].clone()).collect();
    PlasmaProgram { kerf: 1.5, rapids: vec![None; cuts.len()], cuts }
}

/// #801 (the reverse of the delta review's case): L is cut first, then
/// E around it. L's x = 25 edge already ran through E's material, so
/// the later part's cut is refused, not only warned. Nesting and common
/// lines stay as they were - the third review's probe set.
#[test]
fn a_part_cut_around_an_earlier_ring_running_through_it_is_refused() {
    let e = cut(1, Outer, None, square(0.0, 0.0, 50.0));
    let l = cut(2, Outer, None, vec![p(0.0, 0.0), p(25.0, 0.0), p(25.0, 50.0), p(0.0, 50.0), p(0.0, 0.0)]);
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![l, e], rapids: vec![None, None] });
    assert!(
        matches!(f.as_slice(), [PlasmaFinding::CutCrossing { contour: 1, earlier: 2, at }]
            if at.x > 0.0 && at.x < 50.0 && at.y > 0.0 && at.y < 50.0),
        "{f:?}"
    );
    assert!(f[0].refuses());
    // Part in the hole, then the hole, then the outline: clean.
    assert_eq!(judge(&nested(false, &[0, 1, 2])), vec![]);
    // The nested part's own hole first, too: clean.
    let mut ph = nested(false, &[0, 1, 2]);
    ph.cuts.insert(0, cut(4, Inner, Some(3), square(45.0, 45.0, 10.0)));
    ph.rapids.push(None);
    assert_eq!(judge(&ph), vec![]);
    // The nested part touching the hole's wall, cut first: the shared
    // wall is the torch-height warning only (common-line cutting).
    let f = judge(&nested(true, &[0, 1, 2]));
    assert!(f.iter().all(|x| !x.refuses()), "{f:?}");
    // The nested part cut after its hole: the cut-order refusal, as before.
    assert_eq!(judge(&nested(false, &[1, 2, 0])), vec![PlasmaFinding::CutOrder { contour: 3, enclosing: 2 }]);
    // Side by side, sharing an edge or a corner, either order: warnings.
    for b in [square(50.0, 0.0, 50.0), square(50.0, 50.0, 50.0)] {
        let (a, b) = (cut(1, Outer, None, square(0.0, 0.0, 50.0)), cut(2, Outer, None, b));
        for cuts in [vec![a.clone(), b.clone()], vec![b, a]] {
            let f = judge(&PlasmaProgram { kerf: 1.5, cuts, rapids: vec![None, None] });
            assert!(!f.is_empty() && f.iter().all(|x| matches!(x, PlasmaFinding::KerfCrossing { .. })), "{f:?}");
        }
    }
}

/// #801: a hole's slug is cut free like a part and can tip up into the
/// torch. Three holes in a row cut middle first: the rapid from the
/// left hole to the right one runs over the middle slug - a tip-up
/// naming the hole. Leaving the hole just cut (its lead-out ends on its
/// own slug) is the departure, not judged.
#[test]
fn a_rapid_over_a_freed_hole_slug_is_a_tip_up_naming_the_hole() {
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let hole = |id: u32, x: f64| cut(id, Inner, Some(1), square(x, 40.0, 20.0));
    let (left, mid, right) = (hole(2, 10.0), hole(3, 40.0), hole(4, 70.0));
    let run = |cuts: Vec<PlasmaCut>, rapids| judge(&PlasmaProgram { kerf: 1.5, cuts, rapids });
    // Left, middle, right, then the outline: each rapid leaves the hole
    // just cut (from inside its slug) and crosses no other - clean.
    let rapids = vec![None, Some((p(20.0, 50.0), p(50.0, 50.0))), Some((p(50.0, 50.0), p(80.0, 50.0))), Some((p(80.0, 50.0), p(110.0, 50.0)))];
    assert_eq!(run(vec![left.clone(), mid.clone(), right.clone(), outline.clone()], rapids), vec![]);
    // Middle, left, right: the rapid left -> right crosses the middle slug.
    let rapids = vec![None, Some((p(50.0, 50.0), p(20.0, 45.0))), Some((p(20.0, 50.0), p(80.0, 50.0))), Some((p(80.0, 50.0), p(110.0, 50.0)))];
    let f = run(vec![mid, left, right, outline], rapids);
    assert!(
        matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 4, freed: 3, slug: true, at }]
            if at.x > 40.0 && at.x < 60.0),
        "{f:?}"
    );
    assert!(!f[0].refuses(), "a warning, as the part's tip-up is");
    assert!(f[0].to_string().contains("slug of hole C3"), "{}", f[0]);
}

/// #801: the departure from the hole just cut is only its first stretch.
/// An L-shaped hole's rapid leaves the upright arm and then runs back
/// over the foot - over the slug just freed, a tip-up placed in the foot.
#[test]
fn a_rapid_leaving_a_hole_and_running_back_over_its_slug_is_a_tip_up() {
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let l = vec![p(10.0, 10.0), p(30.0, 10.0), p(30.0, 15.0), p(15.0, 15.0), p(15.0, 30.0), p(10.0, 30.0), p(10.0, 10.0)];
    let hole = cut(2, Inner, Some(1), l);
    let f = judge(&PlasmaProgram { kerf: 1.5, cuts: vec![hole, outline], rapids: vec![None, Some((p(12.0, 28.0), p(25.0, -5.0)))] });
    assert!(
        matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 1, freed: 2, slug: true, at }]
            if at.y > 10.0 && at.y < 15.0),
        "{f:?}"
    );
}

/// #814 review: a hole with no lead-out (a small hole, or `lead_out = 0`)
/// ends ON its own wall. A rapid starting there is judged in full: across
/// the slug it is a tip-up - from an edge's middle or from a corner - and
/// only a rapid straight outward from the wall stays clear.
#[test]
fn a_rapid_from_a_holes_wall_across_its_slug_is_a_tip_up() {
    let outline = cut(1, Outer, None, square(0.0, 0.0, 100.0));
    let hole = cut(2, Inner, Some(1), square(40.0, 40.0, 20.0));
    let run = |a: Point2, b: Point2| {
        judge(&PlasmaProgram { kerf: 1.5, cuts: vec![hole.clone(), outline.clone()], rapids: vec![None, Some((a, b))] })
    };
    for (a, b) in [(p(40.0, 50.0), p(110.0, 50.0)), (p(40.0, 40.0), p(110.0, 110.0))] {
        let f = run(a, b);
        assert!(
            matches!(f.as_slice(), [PlasmaFinding::TipUp { contour: 1, freed: 2, slug: true, at }]
                if at.x > 40.0 && at.x < 60.0 && at.y > 40.0 && at.y < 60.0),
            "{a:?} -> {b:?}: {f:?}"
        );
    }
    assert_eq!(run(p(40.0, 50.0), p(-10.0, 50.0)), vec![]);
}
