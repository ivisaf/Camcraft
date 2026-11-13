//! The 3D Pocket under HSM Adaptive through the app (M11.4 owed
//! "Pocket3D port of the same planner"): the user's own part
//! (`testdata/models/Milling-test.step`, a D-shaped plate with a blind
//! bore) roughed through the commands with `strategy: Adaptive` - every
//! level planned by the 2D HSM planner inside the 3-D ladder's level
//! law - verifies clean against the part solid; a level the planner
//! refuses is said in the path's notes with its Z (the op's status
//! summary carries them), never silently swapped; the stamp tells
//! Adaptive from the rings (an old Adaptive op goes Stale once, an
//! OffsetRings op keeps its digest); and the default's output is
//! `golden_cl`'s business, unchanged.
//!
//! Gate map (M11.4): covers app::operation (params digest, generate's
//! Pocket3D arm), app::commands, app::verify.

use camcraft_app::cl::{MoveKind, Toolpath};
use camcraft_app::commands::{AddTool, CreateOperation, GenerateOperation, History, ImportModel, SetStock};
use camcraft_app::geometry::Point3;
use camcraft_app::links::Links;
use camcraft_app::machining2d::PocketStrategy;
use camcraft_app::machining3d::Entry3D;
use camcraft_app::operation::{self, Height, HeightReference, OpParams, SrcRef};
use camcraft_app::schema::Location;
use camcraft_app::stock::StockShape;
use camcraft_app::tools::catalog;
use camcraft_app::world::new_world;
use camcraft_app::model;
use flecs_ecs::prelude::World;

const TOOL: &str = "EM-1000-4FL";

fn pocket_op(strategy: PocketStrategy) -> OpParams {
    OpParams::Pocket3D {
        links: Links::ROUGH3D,
        stepover: 4.0,
        stock_to_leave: 0.0,
        clearance_z: Height::of(HeightReference::TopOfStock, 20.5),
        retract_z: Some(Height::of(HeightReference::TopOfStock, 2.0)),
        top_z: Height::of(HeightReference::TopOfModel, 0.0),
        bottom_z: Height::fixed(0.0),
        stepdown: 5.0,
        feed: 1200.0,
        plunge_feed: 300.0,
        climb: true,
        strategy,
        entry: Entry3D::Helix,
        helix_radius: 2.0,
        rest_rough: false,
        optimal_load: camcraft_app::load::DEFAULT_OPTIMAL_LOAD,
        feed_opt: false,
        shell: false,
    }
}

/// The user's part in a 70 x 70 block from Z -7, the 10 mm tool, one op
/// generated through the commands: its path and its warnings.
fn user_job(params: OpParams) -> (World, Toolpath, Vec<String>) {
    let world = new_world();
    let mut h = History::default();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/models/Milling-test.step");
    let imp = ImportModel::new(&path).expect("the user's STEP imports");
    let name = imp.name.clone();
    h.execute(&world, imp);
    let loc = Location { pos: Point3::new(0.0, 0.0, -7.0), ..Default::default() };
    h.execute(&world, SetStock::new(&world, StockShape::Box { dx: 70.0, dy: 70.0, dz: 27.0 }, loc).expect("stock"));
    let tool = catalog().into_iter().find(|(n, _)| *n == TOOL).map(|(_, t)| t).expect("catalog");
    h.execute(&world, AddTool::new(&world, TOOL, tool).expect("tool"));
    h.execute(&world, CreateOperation::new(&world, "Op1", TOOL, SrcRef::Entity(name), params).expect("create"));
    h.execute(&world, GenerateOperation::new(&world, "Op1").expect("generate"));
    let e = model::find_by_name(&world, "Op1").expect("op");
    let tp = model::toolpath_of(&world, &e).expect("cached path");
    let warns = tp.warnings.iter().map(|w| format!("{w:?}")).collect();
    (world, tp.path.clone(), warns)
}

#[test]
#[ignore = "heavy lane: HSM Adaptive over every level of the user's STEP bore, verified against the part solid, ~220 s debug (release tier); the planner laws' debug twin is motion pocket3d_hsm::an_open_level_holds_the_planner_laws"]
fn the_users_bore_roughs_adaptive_level_by_level_and_verifies_clean() {
    let (_, path, warns) = user_job(pocket_op(PocketStrategy::Adaptive));
    assert!(warns.is_empty(), "every level verifies clean: {warns:?}\nnotes {:?}", path.notes);
    let planar = path.moves.iter().filter(|m| m.kind == MoveKind::Feed && m.from.z == m.to.z).count();
    assert!(planar > 0, "the ladder cuts");
    // The HSM plan's arcs ride the levels (or the level says why not).
    let arcs = path.moves.iter().filter(|m| m.arc.is_some() && m.from.z == m.to.z).count();
    let fell: Vec<&String> = path.notes.iter().filter(|n| n.contains("Offset rings cut the level at Z")).collect();
    assert!(arcs > 0 || !fell.is_empty(), "{:?}", path.notes);
    // Never a silent switch: a rings level names its Z in the notes,
    // and the op's status line carries the notes.
    for n in &fell {
        assert!(n.contains("Z ") && n.starts_with("adaptive: the HSM plan"), "{n}");
        let line = path.status_summary("Op1", 0, camcraft_app::units::Unit::Millimeters);
        assert!(line.contains("Offset rings"), "{line}");
    }
    // Issue #850: the growth planner's budgets never refuse a level of
    // the bore (its finely tessellated outline is heavy geometry over a
    // short walk); only the levels whose plan would cross itself go to
    // the rings. A budget that cut a level would change the path silently
    // under the assertions above.
    assert!(!path.notes.iter().any(|n| n.contains("planning budget") || n.contains("too detailed")), "{:?}", path.notes);
    // Which levels fall back (measured on main f490458a): Z 20 and Z 10,
    // each because its HSM plan would cross itself. Until PR #839 (merge
    // 575f02d8, issue #689) only Z 20 did: #839 caps the STEP chord at
    // 0.005 mm (it was ~0.1% of the model extent), so the bore's wall is
    // meshed far finer, the level outlines the planner works on change,
    // and the Z 10 plan now crosses itself at (-1.99, -0.13). Getting
    // that level's HSM plan back is Refs #980 (part 2).
    let mut fell_z: Vec<&str> = fell.iter().filter_map(|n| ["Z 20.000", "Z 10.000"].into_iter().find(|z| n.contains(z))).collect();
    fell_z.sort_unstable();
    assert!(fell.len() == 2 && fell_z == ["Z 10.000", "Z 20.000"], "{fell:?}");
    assert!(fell.iter().all(|n| n.contains("crosses itself")), "{fell:?}");
    // The path as main plans it (measured on main f490458a, identical over
    // repeated runs, and on 575f02d8 alike: 6500 moves, 6464 planar feeds, 222
    // arcs; before #839, on 575f02d8^1: 5530 / 5497 / 600). #839's finer
    // mesh moves them: a second rings level (no arcs, more chords) and
    // fewer fitted arcs on the HSM levels. A change to the planner or the
    // mesh that moves these re-measures them and says why.
    println!("moves {} planar {planar} arcs {arcs} notes {:?}", path.moves.len(), path.notes);
    assert_eq!((path.moves.len(), planar, arcs), (6500, 6464, 222), "the bore's path as main plans it: {:?}", path.notes);
}

#[test]
fn the_stamp_tells_the_hsm_plan_from_the_rings() {
    let world = new_world();
    let mut h = History::default();
    h.execute(&world, SetStock::new(&world, StockShape::Box { dx: 60.0, dy: 60.0, dz: 20.0 }, Default::default()).expect("stock"));
    let tool = catalog().into_iter().find(|(n, _)| *n == TOOL).map(|(_, t)| t).expect("catalog");
    h.execute(&world, AddTool::new(&world, TOOL, tool).expect("tool"));
    let stamp = |s: PocketStrategy| operation::current_stamp(&world, TOOL, &SrcRef::Stock, &pocket_op(s), "X", None);
    let hsm = stamp(PocketStrategy::Adaptive);
    assert!(hsm.is_some());
    assert_ne!(hsm, stamp(PocketStrategy::OffsetRings), "the HSM plan is a new generation of the word");
    assert_ne!(hsm, stamp(PocketStrategy::Waterline));
}
