//! #536 (part of #290): a constant wire taper, end to end. A tapered
//! punch generates poses (D2), passes the wire laws through the whole
//! height and posts a golden through the generic wire post on the
//! synthetic fixture (taper codes and guide planes invented, D10); a
//! zero taper is the vertical golden byte for byte; an angle past the
//! machine's maximum and a U/V offset past its stroke or guides are
//! refused by name; a taper whose lower edge burns the part blocks the
//! post. Not exercised on a real EDM control (D10).
//!
//! Gate map (#536): covers motion::wire_edm (taper), sim::verify::wire
//! (the leaning kerf), app::operation::wire_op (the taper's door),
//! app::machine::wire_edm (lean, guides, U/V), app::posting_wire
//! (WireU, WireV) and the wire-iso.cps taper words.
//! Provenance: our own tests over our own synthetic fixture.

use flecs_ecs::prelude::World;

use camcraft_app::commands::{CreateOperation, GenerateOperation, History, ImportModel, LoadMachine};
use camcraft_app::geometry::{Mesh, Point3, Triangle};
use camcraft_app::machine::parse_mch;
use camcraft_app::model;
use camcraft_app::operation::{Height, OpParams, SrcRef, WireContourOp, WireSide};
use camcraft_app::posting::{self, BUILTINS};
use camcraft_app::world::new_world;

const FIXTURE: &str = include_str!("edm/wire-iso-fixture.mch");
const WIRE: &str = "Generic ISO Wire EDM";
const VERTICAL: &str = "tests/golden/nc/wire-iso/wire_slug_punch.nc";
const NC: &str = "tests/golden/nc/wire-iso/wire_taper_punch.nc";
const CL: &str = "tests/golden/wire_taper_punch.cl";

fn cps() -> &'static str {
    BUILTINS.iter().find(|(n, _)| *n == WIRE).map(|(_, s)| *s).expect("registered")
}

/// A sharp 40 x 20 block from z = 0 to 12, centred on the origin.
fn block() -> Mesh {
    let pts = [(20.0, 10.0), (-20.0, 10.0), (-20.0, -10.0), (20.0, -10.0)];
    let p = |(x, y): (f64, f64), z: f64| Point3::new(x, y, z);
    let mut f = Vec::new();
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        f.push(Triangle::new(Point3::new(0.0, 0.0, 12.0), p(a, 12.0), p(b, 12.0)));
        f.push(Triangle::new(Point3::new(0.0, 0.0, 0.0), p(b, 0.0), p(a, 0.0)));
        f.push(Triangle::new(p(a, 0.0), p(b, 0.0), p(b, 12.0)));
        f.push(Triangle::new(p(a, 0.0), p(b, 12.0), p(a, 12.0)));
    }
    Mesh::from_facets(f)
}

/// The machine `text` mounted and a punch around the block created at
/// `taper` degrees; the generator's answer returned.
fn punch(text: &str, taper: f64) -> (World, Result<(), String>) {
    punch_with(text, taper, None)
}

/// [`punch`] with a no-core burnout (#540) stated.
fn punch_with(text: &str, taper: f64, burnout: Option<f64>) -> (World, Result<(), String>) {
    let world = new_world();
    let mut h = History::default();
    let m = parse_mch(text).expect("the fixture parses");
    h.execute(&world, LoadMachine::new(&world, "wire-iso.mch".into(), Some(m)).expect("mount"));
    h.execute(&world, ImportModel::from_mesh("block", block()));
    let op = WireContourOp {
        top_z: Height::fixed(12.0),
        bottom_z: Height::fixed(0.0),
        condition: "C110".into(),
        side: WireSide::Punch,
        start: Some([-30.0, 0.0]),
        tabs: None,
        // The punch drops into a stated capture (#539's hold law).
        capture: true,
        taper_deg: taper,
        burnout,
        clearance: camcraft_app::operation::DEFAULT_CLEARANCE,
    };
    let c = CreateOperation::new(&world, "Punch1", "", SrcRef::Entity("block".into()), OpParams::WireContour(op)).expect("op");
    h.execute(&world, c);
    let made = GenerateOperation::new(&world, "Punch1").map(|g| h.execute(&world, g)).map_err(|e| e.to_string());
    (world, made)
}

fn post(world: &World, tag: &str) -> Result<String, String> {
    let out = std::env::temp_dir().join(format!("camcraft-wire-taper-{tag}-{}.nc", std::process::id()));
    let posted = posting::post_script(world, &out, cps());
    let wrote = out.exists();
    let _ = std::fs::remove_file(&out);
    assert_eq!(wrote, posted.is_ok(), "{tag}: NC text is written only for a posted program");
    posted.map(|p| p.program)
}

fn golden(file: &str, text: &str) {
    if std::env::var_os("WRITE_GOLDEN").is_some() {
        std::fs::write(file, text).expect("write golden");
    }
    assert_eq!(text, std::fs::read_to_string(file).expect("golden present"), "{file} drifted");
}

/// Done means: a tapered punch posts and verifies; the CL golden shows
/// the poses and the NC golden the taper words from `edm.codes`.
#[test]
fn a_tapered_punch_posts_its_golden_and_its_cl_shows_the_poses() {
    let (world, made) = punch(FIXTURE, 3.0);
    made.expect("a 3 degree punch generates");
    let e = model::find_by_name(&world, "Punch1").expect("op");
    let path = model::toolpath_of(&world, &e).expect("generated").path;
    // Every side leans 3 degrees outward-down; the corners lean along
    // their mitre; the wire stands upright at the start hole.
    let lean = |a: [f64; 3]| a[0].hypot(a[1]).atan2(a[2]).to_degrees();
    let poses: Vec<[f64; 3]> = path.moves.iter().map(|m| m.axis.expect("every tapered move states its pose")).collect();
    assert!(poses.iter().all(|a| lean(*a) <= 15.0), "{poses:?}");
    assert!(poses.last().is_some_and(|a| lean(*a) == 0.0), "upright at the start hole");
    assert!(poses.iter().any(|a| (lean(*a) - 3.0).abs() < 1e-9), "a side point leans exactly the taper");
    let clause = posting::wire_clause(&world);
    assert!(clause.problems.is_empty(), "{:?}", clause.problems);
    golden(CL, &camcraft_app::cl::text::to_text(&path).expect("cl text"));
    let text = post(&world, "ok").expect("the tapered punch posts");
    golden(NC, &text);
    let lines: Vec<&str> = text.lines().collect();
    let at = |w: &str| lines.iter().position(|l| *l == w).unwrap_or_else(|| panic!("no {w} in\n{text}"));
    let (comp, on, off, cut) = (at("FX-COMP-OFF"), at("FX-TAPER-ON"), at("FX-TAPER-OFF"), at("FX-CUT"));
    assert!(comp < on && on < off && off < cut, "{text}");
    let moves = &lines[on + 1..off];
    assert!(moves.iter().all(|l| l.starts_with("G1 X") && l.contains(" U") && l.contains(" V")), "{text}");
    assert!(moves.last().is_some_and(|l| l.ends_with("U0 V0")), "the lead-out stands the wire upright: {text}");
}

/// Done means: a zero taper is the vertical cut's golden byte for byte,
/// on the same fixture that now states the taper's codes and guides.
#[test]
fn a_zero_taper_is_the_vertical_golden_byte_for_byte() {
    let (world, made) = punch(FIXTURE, 0.0);
    made.expect("generates");
    let text = post(&world, "zero").expect("posts");
    assert_eq!(text, std::fs::read_to_string(VERTICAL).expect("golden present"));
}

/// Done means: a 30 degree request on a 15 degree machine is refused by
/// name, and a machine stating no maximum refuses any taper.
#[test]
fn a_taper_past_the_machines_maximum_is_refused_by_name() {
    let (_, made) = punch(FIXTURE, 30.0);
    let err = made.expect_err("30 degrees on a 15 degree machine");
    assert!(err.contains("Punch1") && err.contains("30.000 degree taper") && err.contains("at most 15.000 degrees"), "{err}");
    let (_, made) = punch(&FIXTURE.replace("\"max_taper_deg\": 15.0,", ""), 3.0);
    let err = made.expect_err("no stated maximum");
    assert!(err.contains("states no edm.max_taper_deg"), "{err}");
}

/// Done means: a U/V offset outside the stroke, a machine with no U
/// axis or no guide planes, and a plane outside the guides' reach each
/// block the post by name.
#[test]
fn a_uv_offset_past_the_stroke_or_the_guides_is_refused_by_name() {
    let cases = [
        (FIXTURE.replace("\"min\": -5.0, \"max\": 5.0", "\"min\": -0.5, \"max\": 0.5"), "outside the U stroke -0.500..0.500"),
        (FIXTURE.replace("\"lower_guide_z\": -5.0,", "\"lower_guide_z\": 1.0,"), "outside the guides' reach"),
        (FIXTURE.replace("\"name\": \"U\"", "\"name\": \"W\""), "states no U axis"),
        (FIXTURE.replace("\"upper_guide_z\": 25.0,", ""), "states no edm.lower_guide_z and edm.upper_guide_z"),
    ];
    for (i, (text, want)) in cases.iter().enumerate() {
        let (world, made) = punch(text, 3.0);
        made.expect("the cut itself generates");
        let err = post(&world, &format!("uv{i}")).expect_err(want);
        assert!(err.contains("Punch1") && err.contains(want), "{want}: {err}");
    }
}

/// Done means: a taper whose lower edge burns into the part blocks the
/// post - the wire laws read the leaning kerf at every height.
#[test]
fn a_taper_whose_lower_edge_gouges_the_part_blocks_the_post() {
    // Narrowing downward: at the bottom the wire stands 12 * tan 3 deg
    // inside the block's vertical wall.
    let (world, made) = punch(FIXTURE, -3.0);
    made.expect("the generator reads the profile on the top plane only");
    let err = post(&world, "gouge").expect_err("a gouging taper never posts");
    assert!(err.starts_with("post blocked") && err.contains("Punch1: wire gouge"), "{err}");
}

/// #536 x #540: the burnout's rings are cut vertical, so a taper asked
/// with a burnout is refused by name - never cut upright silently.
#[test]
fn a_taper_with_a_no_core_burnout_is_refused_by_name() {
    let (_, made) = punch_with(FIXTURE, 3.0, Some(0.5));
    let err = made.expect_err("a tapered burnout");
    assert!(err.contains("Punch1") && err.contains("3.000 degree taper is asked with a no-core burnout"), "{err}");
    let (_, made) = punch_with(FIXTURE, 0.0, Some(0.5));
    assert!(made.as_ref().map_or_else(|e| !e.contains("taper"), |()| true), "a vertical burnout is not refused for a taper: {made:?}");
}
