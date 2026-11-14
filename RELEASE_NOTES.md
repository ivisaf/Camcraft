# CamCraft 0.19.0 (clean-room implementation)

Released 2026-11-13.

## Breaking
- **The document's unit is the unit every post writes** (#902, PR #981).
  Posted programs are in the document's mm or inch (G21/G20, Siemens 840D
  G710/G700), never the machine file's unit. The grblHAL and TinyG posts follow
  it; the Carvera, Heidenhain and Fanuc Lathe posts refuse an inch document by
  name. Older projects open on the unit they posted in. MCP gains `set_units`.
  Check the first program you post on each machine.
- **Thread milling is reworked** (#894, PR #1005). Thread mills are tool kinds
  (multi-form and single-form), and the Thread operation cuts internal threads
  by helical interpolation (radial passes, climb or conventional, right or left
  hand) and posts G2/G3 with Z. A Thread operation no longer accepts a Tap-kind
  tool, so projects that used one need a thread mill.
- **Air coolant and machine default coolant** (#869, PR #961). An operation can
  ask for Air (M7 air blast), resolved from the machine's coolant list or
  refused by name. A new operation starts on its machine's default coolant (Air
  on the Carvera), and the Carvera post posts the operation's own coolant
  instead of the first Flood entry. The machine editor adds coolants with their
  codes and no longer saves a coolant without one.
- **Project file format is v59** (#1056, PR #1341). Projects saved before mesh
  fidelity was recorded (v58 or older) open with every model marked "fidelity
  unknown", and their operations are refused by name until the model is
  re-imported. Files saved by 0.19.0 do not open in 0.18.0.

A minor release headlined by **wire EDM and sinker EDM**, **more turning**
(threading infeed, workholding rpm limits, lathe holder bodies), **new posts**
(grblHAL, TinyG, generic ISO wire and sinker EDM), **a much larger MCP surface**
and **stricter verification** of imported STEP models. 237 pull requests merged
since 0.18.0.

## Highlights
### Wire EDM
- Machine files can declare `wire_edm` and `sinker_edm` machines with an `edm`
  section and an EDM condition table; a program whose operation its setup's
  machine cannot run is refused at post (#530, #560).
- 2-axis Wire Contour: kerf offset to a stated punch or die side, start hole,
  corner refusal, open runs cut in from the stock's edge, tabs with a tab
  cut-off (an unheld slug is refused), no-core burnout by offset rings, and
  constant taper with posed moves and max-angle and U/V refusals.
- Wire verification: kerf, threading and slug laws block the post.
- A tool-less Wire Contour page and machine section in the UI.

### Sinker EDM
- Sinker burn to depth with one electrode, rough and finish with undersize from
  their conditions, orbit patterns (circular, vector, planetary), side burn,
  electrode wear compensation in Z, and an electrode generated from a cavity's
  picked faces (`electrode_from_faces`).

### Turning
- Threading infeed method with a decreasing-depth pass schedule (#516).
- A setup's workholding max rpm caps G96 and refuses G97 above it.
- The index swing of any work-turning rotary is judged about a level line, and
  a rotary subplate raises the A centerline.
- Lathe holder bodies can be placed from the tool editor and over MCP, are read
  on their own side of the axis, and are judged against chuck jaws and standing
  steel. The simulation draws the turning tool's lathe body.

### Milling, threads and finishing
- External threads on a boss at the operation level, and thread-milling pass
  control (decreasing passes, finish and spring passes, depth allowance).
- Finish 3D gains a land model that fills closed openings on the drive surface.
- Part alignment from two perpendicular edge picks (full 3D frame), the WCS from
  any set of feature picks, and the WCS anchored on a picked circle's centre.
- Tools carry part number, vendor, coating, helix angle, notes and cutting data
  per work material; the setup sheet tool list prints them. Tool library import
  has add, update and replace modes, and `.camtools` carries the tool body.
- A self-centering grip vise jaw, and editing and re-fitting a placed mod vise
  pair.

### Posts
- New posts: grblHAL (3-axis mill), Synthetos TinyG (3-axis mill), generic ISO
  wire EDM and generic ISO sinker EDM (every EDM code is machine data). Sienci
  AltMill machines run on the grblHAL post.
- Fanuc Lathe writes inch programs; the document's program number and comment
  are written per dialect. A user's post script states the machine kinds it
  serves. Rigid tapping is refused by name at the program gate when the machine
  does not state it.
- An opt-in **Explain G-code** option writes plain-English comments on blocks
  for posts that declare a comment form; other posts are refused by name.

### MCP tools
`set_tool_holder`, `list_holder_library`, `rename_operation`,
`reorder_operation`, `set_operation_coolant`, `set_setup_post`, `edit_machine`,
`edit_tool`, `set_wcs_from_features`, `set_wcs_circle_centre`, `lay_part_on_face`,
`set_tool_number`, `set_work_material`, `apply_cutting_data`, `delete_tool`,
`set_operation_setup`, `delete_setup`, `electrode_from_faces`, `set_units`, and
`add_operation` for the Wire Contour with no tool. `pick_in_box` takes a
`through` argument. The skill documents every registered tool.

### Verification and collision
- A STEP model imported coarser than 0.005 mm is flagged as reduced fidelity:
  gouge checks are tightened by the excess and its operations are refused at the
  post once it is too coarse. The facet budget is a setting carried with the
  import. A STEP import can go through solidcore behind a seam, and a machining
  mesh with a measured deviation is available from it.
- A from-model stock's source is judged unless marked raw stock only. The rapid
  law reads the stock under the rapid, and the mid-turn sweeps read the whole
  turn rather than five poses.
- Refusal ghosts show where a part-way refusal stopped.

### UI
- The Setups page lists setups only; alignment, work offset, workholding and
  tools live on the setup. A viewport bottom control bar holds the view buttons,
  and a "Pick through" toggle lets picks reach the far side.
- The G-code window has syntax colour, selectable text, a highlighted trace line
  and click-to-trace. Error text is selectable and copyable everywhere.
- Original SVG operation icons, tool kind glyphs and status badges, with an
  SVG-to-PNG icon pipeline. The tool editor edits each milling tool kind by its
  parameters. Viewport origin gizmos wear their names.
- The browser (wasm32) build is published under `/app/` on the project site.

## Fixes
91 fixes landed; the ones most likely to matter when posting:
- Posts: canned-cycle posts traverse between holes at the stream's hop and seat
  an off-seat cycle by the same law; Sinumerik writes work offsets 5 and 6 as
  G505 and G506; an absent rotary takes no angle word; lathe posts state G97
  before a section that is not constant surface speed; a milling operation is
  never posted on a lathe's work spindle; a cut with the spindle stopped, or a
  canned cycle with no feed, is refused in every spindle mill dialect; a touch
  probe section starts no coolant; a post that does not serve its setup's machine
  kind is refused by name; the sinker ISO post writes the document's unit.
- Rotary and A-axis: posts cap the feed of a block that turns a rotary at the
  machine's stated rotary rate and linear feed (Fanuc, Doosan DNM, Fanuc Lathe;
  #885), and the Doosan DNM post writes the first A word of every operation, so
  an A0 operation after a tilted one no longer cuts at the old angle (#950).
- Swarf: the mid-turn sweep no longer refuses a blade-lying link that turns B
  through vertical (#1404).
- Verification: a tool cutting into a flat is a gouge; a necked tool is judged at
  its drawn neck; non-finite or zero tool axes are refused; crossing checks read
  a shared vertex a rounding apart as a touch; the hole-wall excuse and tilted
  band laws were corrected.
- Import and project: a STEP record nested past 64 parentheses is refused by name
  instead of overflowing the stack; STEP parse errors name the entity and line;
  opening a project over MCP replaces the document.
- Adaptive roughing no longer drops a level to Offset rings because the entry
  orbit's closing point sat a rounding error off its start (#980).
- UI: no console window on a Windows double-click, native dialogs are owned by
  the shell window, and the plasma refusal reads plainly.

## Not exercised on a real control
These were checked by simulation, recorded streams and tests only, not on a
machine or a vendor control:
- The generic ISO wire EDM post and the generic ISO sinker EDM post are
  synthetic: their codes are machine data, not taken from a real control.
- Wire taper, tabs, no-core burnout and 4-axis wire as merged, and the sinker
  orbit, side burn and wear compensation moves.
- The turning thread infeed plan (#516).
- The rotary feed cap (#885) and the Doosan DNM A-state change (#950) merged
  without a PathPilot Hub run or a real Doosan control.
- The grblHAL and TinyG posts, and the Sienci AltMill machine files.
- Thread milling by helical interpolation was verified in simulation only.

## Process changes
Contributor and agent process only; no effect on the program. PROCESS_VERSION
went from 16 to 54 (numbers 23 and 27 were not announced):
- **17** (#1117): the `gh-issues` skill is in the repo.
- **18, 20, 22, 25, 28, 34, 35, 40, 41, 43-53**: camcraft-mcp skill rows for new
  MCP tools and arguments.
- **19, 21, 33, 37, 38, 42**: xtask gate maps and the quick gate follow moved
  modules, changed post scripts and machine files, `#[path]` files, and the
  motion generator; probe and arc post suites and twelve other test files moved
  into the crates that own them.
- **24, 26** (#1116, #1179): `cargo xtask icons` rasterises the SVG masters.
- **29** (#1193): the priority scale is `P1`..`P9`.
- **30** (#1214): the gate reports VOID when the checkout moves during a run.
- **31, 32** (#1227, #1321): refresh only at merge time; any collaborator's
  review counts and both may merge.
- **36, 39** (#991, #1225): code placement rules (`app` holds the document, new
  crate dependencies need a decision) and `crates/ui/AGENTS.md`.
- **54** (#1391): the cut ends once the Pages job is green and the site shows the
  new version.

## Known issues
- The browser build is a first rung; the file picker is still owed.
- No installer is built on the cut machine; the zip is the release and the
  published artifact is built by the release workflow from the tag.

---

# CamCraft 0.18.0 (clean-room implementation)

Released 2026-11-12.

Breaking: none. The project file format stays at v58, so files saved by
0.17.0 open unchanged. **Posted
G-code does change for some machines**, so check the first program you post on
each: the Fanuc, Haas, Mazak and Doosan DNM posts now write `G21` and the
Siemens 840D post writes `G710` before any motion (#901); every mill post
writes an explicit spindle stop after a probe's tool change (#574); lathe
posts reset constant surface speed at the end of a section (#772); and the
post gate judges arcs and tool bodies more strictly in places (below), so a
program that posted before may now be refused by name.

A minor release headlined by **CamCraft in a browser** (a first web runner),
**C-axis unwind** for continuous live tooling, **default holders on machine
files**, **a large fixture and holder library** (plates, mod vises, 5-axis
vises, accessories, BT/CAT/ER/TTS holders), **theme as a user preference**,
a Trochoidal slot operation, Tormach microARC 4 and plasma posts, and
two new lathe and mill posts. Thanks to mkinney, vilts, tbrasselle, ahmetv10,
JSwetz and seelesturm for their contributions.

## This cut
The UI snapshot goldens were regenerated for this cut and accepted by the
maintainer without the usual manual look at the diff images (maintainer's
decision, 2026-11-12).

## New
- **Runs in a browser** (#31, #841, #847): the shell builds for wasm32 and runs
  on a web canvas with an awaited GPU start-up and a page built by
  `web/build-app-wasm.sh`. A first rung only, and **not part of the release
  download yet**: the release workflow does not build or attach it, only the
  manual CI workflow compiles it. The file picker is still owed.
- **C-axis unwind** (#33): continuous C-axis live tooling now unwinds C at
  clear links against the machine's stated C travel. Rotary Contour unwinds;
  Rotary Parallel and Rotary Offset refuse by name where C would need a
  rewind. Each rewind is an orbit the verifier and the post both see, and the
  lathe post caps every C block's feed by the machine's stated linear feed and
  C rate.
- **Machine default holders** (#828): a machine file can name a spindle mount
  and a default library holder (`tooling.spindle_mount`,
  `tooling.default_holder`; refused by name when unknown or mismatched). A new
  tool added in the GUI or over MCP starts with that holder in the same undo
  step. The Tormach MX mills default to BT30 ER20, the PCNC/M mills to TTS ER20.
- **Theme is your preference** (#877, vilts): the theme and colour-vision mode
  survive restarts, and New, Open and project files no longer change them. The
  light theme now has a light viewport with toolpath, overlay, view cube, hover
  and title-bar colours that read on it (#698, vilts).
- **Holder library** (#792, #793, #794, #799): BT30, BT40 and CAT40 taper
  holders in ER collet chuck and Weldon versions; standalone ER11-ER40 collet
  chucks and BT40 ER40 from one ER table, with the flange flare and Weldon
  clamp screw enveloped; Tormach TTS holders (ER20, ER16 and set-screw) with
  the collar face as the gauge line. MCP gains `set_tool_holder` and
  `list_holder_library`. A vendor tool STEP can be aligned by its ISO 13399 tool
  coordinate systems (#370, JSwetz).
- **Tool editor and simulation** (#93, #743): draw a milling tool's cutter,
  neck and shank as a spun outline (Profile tab, validated against the stated
  diameter and lengths, undoable, over MCP as `set_tool_cutter_profile` /
  `get_tool_cutter_profile`). The simulation draws the tool and holder as the
  same revolved body the collision laws judge.
- **Fixture library** (#787, #788, #789, #819, #820): parametric milling vises
  (3-8 in, 100-200 mm); 5-axis self-centering vises (52, 75, 80, 125 mm)
  centred on the stock; accessories (parallels, 1-2-3 and step blocks, strap
  clamps, soft jaws); fixture plates (12 x 12 in, 300/400 mm, 770MX table) with
  any fixture placed at a named grid hole or dowel pair; mod vises (fixed and
  adjustable jaw blocks on a plate, MCP `add_mod_vise_pair`). Each is placed in
  one undoable step and judged by the fixture mesh laws. Links and retracts
  that would strike a fixture are raised over the steel instead of only being
  reported; a clamp taller than the machine's Z stroke refuses generation by
  name (#339).
- **Trochoidal slot operation** (#220): an engagement-controlled trochoid along
  a typed centre line, with the measured contact angle held under a limit by
  adapting the step, microlift or retract returns, and named refusals. It also
  refuses by name a forward step past the loop cap, a microlift above the
  clearance plane and a bore past the helix-turn cap (#774).
- **Pocket and slot detection** (#511): the Model page detects pockets and
  slots and creates their operations with one click.
- **Posts and machines**: Tormach microARC 4 and PathPilot A indexing between
  operations on the new 770MX + microARC 4 machine (#785; simultaneous 4-axis,
  unreachable tilts and A past travel are refused by name); a PathPilot plasma
  post for the Tormach 1300PL (#671); a PathPilot lathe post (#612); a Doosan
  DNM + Fanuc post aligned with a real sample program (#614, JSwetz; see
  Known issues, #950, before using it with a 4th axis);
  documented Heidenhain drilling and tapping cycles with explicit floating
  holder intent (#51, seelesturm).
- **Plasma** (#672, #801): the through-cut laws judge every Plasma Profile at
  the post gate (cut order, cut crossing, a malformed record block the post by
  name); a slot or keyway narrower than the kerf is refused at generation; the
  laws warn on a rapid over a freed hole slug.
- **Refused generations show their moves** (#261): the moves built before a
  refusal are drawn as a dashed ghost in the refusal colour (display only;
  never cached, simulated or posted), and the script/MCP error names its move
  count.
- **UI polish** (ahmetv10): larger ribbon cells with readable labels,
  capitalised tab titles, no internal status text, an empty-viewport hint and a
  Z=0 ground grid. The Machines pane's kind dropdown filters the list (#816).
  The starter tool catalog is now a data file, `assets/tools/catalog.json`
  (#741).
- **Documentation** (#688, #291, tbrasselle): a Zola user-guide and developer
  site published to GitHub Pages at each release tag; a first-part example
  project with tutorial images.

## Safety and correctness
- **Posted programs**: the mm unit word (`G21`, `G710` on the 840D) is written
  before any motion on the Fanuc, Haas, Mazak, Doosan DNM and Sinumerik posts,
  so a control left in inch never reads an mm program as inches (#901). Every
  mill post writes an explicit spindle stop (`M5`; `MOTOFF` on Mazak) after a
  probe's tool change and length offset (#574). Lathe posts (Fanuc, PathPilot)
  no longer carry a section's G96 into a later section that states none (#772).
  The Carvera post names a per-operation smoothing ask with a one-time
  "SMOOTHING NOT WRITTEN" note instead of dropping it (#757).
- **STEP chord cap and arc fitting** (#689): a bore at a STEP hole's own size
  posts clean. STEP meshes are capped at 0.005 mm chord error, arc fitting
  keeps vertices within half the tolerance, closed rings fit whole, and the
  gouge law reads arcs at a quarter of its tolerance. A nominal arc round a
  convex wall now passes; an arc bowing past the floor between its old chord
  joints is now caught.
- **Reach and tool-body laws** (#903, #835): reach laws (c)/(c3) no longer
  excuse a leaning shank or holder standing in the part as cutter kerf. A tool
  body (imported or drawn) is judged by the same band stack the simulation
  draws, so a stepped holder whose flange stands clear is no longer warned;
  laws that still use the enclosing cylinder say so; a mid-turn swing through
  the part is no longer hidden behind a stock shank warning on the same move.
- **STEP files with empty parameter lists import** (#887, tbrasselle): files
  exported by HOOPS Exchange, anything with an empty `()`, no longer fail as
  corrupt.
- **4th-axis A tables** (#797, #804, #813): the machine check reads an A-about-X
  table's pose through the tool axis (the underside A0 B180 posts as A180 on the
  770MX + microARC 4). The program gate refuses an A index when the retracted
  tool tip cannot clear the swept stock, part and turning fixtures by 5 mm. The
  swing check also judges a work-turning A whose machine file states no axis
  vector (an A is about X), and refuses one with neither a vector nor an
  RS-274 letter. A machine whose work-turning A states no `nose_to_axis` has
  every operation posted on it refused by name at the program gate until the
  machine file states it.
- **Adaptive** (#850): the growth planner refuses by name a plan that has
  worked heavy ring geometry over a long walk, so an islanded area it cannot
  finish falls back to the named Offset rings in about 22 s instead of about a
  minute. Plans that finish today are unchanged.
- **Gouge law (b) in Boundary mode** (#905): an arc move in contour, trace and
  face is checked along its whole length against wall corners, not only at
  sample points, so a gouge between two samples is caught. Arcs in the XZ and
  YZ planes are still read at their chord joints (#946).
- **Picking over MCP** (#692): `locate_part` refuses a mate id no click could
  pick, like the other pick tools.

## Fixed
- **Fixtures**: a vise or accessory key this build cannot read is dropped with
  a load note and the project still opens; a strap clamp's stud must be
  narrower than its strap; the parallels fit marks the height it sets (#812).
  Dropped plate and mod-vise keys show in the load notes, a located plate's
  riders are clash-checked, a plate whose mesh changes drops its grid, and
  staggered plates with a clamp ear or chamfers are no longer refused as
  turned (#826).
- **Setup page** (#866, #872, #873): it names the turn that stops the stock
  carry and says what to do; the slow-frame log no longer blames time spent in
  a file dialog.
- **Plasma**: the profile refuses a zero-width slit in a picked contour instead
  of dropping it (#801).
- **Build**: the main branch compiles again after a Trochoid/Stock merge slip
  (#844).

## Changed
- The UI says "work offset (G54-G59)" plainly, and refusals say what a setup
  owns in plain words instead of "furniture" (#876, #878).
- The dev build profile is lighter and the gate can use a compile cache
  (#900); the release build is untouched.

## Known issues
- **#950 (P0)**: the new Doosan DNM post, on a 4th axis without TCP, assumes
  the table stands at A0 at program start and after every tool change. An
  operation at A0 that follows one at another angle is then posted with no A
  word and cuts at the previous angle. Until it is fixed, do not post programs
  through the Doosan DNM post that mix A0 with other A angles, or add the A0
  move by hand and check every A word after a tool change.
- **#980 (P1)**: STEP models are now meshed much finer (the chord cap of
  #689), and on some parts the Adaptive (HSM) plan of a level then crosses
  itself and that level is cut with Offset rings instead - named in the
  operation's notes, and the path still verifies clean. On the project's own
  test part one more level falls back than in 0.17.0 and the path has fewer
  arcs (222, was 600). Expect longer, less smooth roughing on imported STEP
  parts until the planner is fixed.
- **#934 (P0)**: the Sinumerik post writes work offsets 5 and 6 as G58 / G59,
  which on the 840D are programmable offsets, not settable frames. Check
  offsets 5 and 6 by hand on that post.
- **#946 (P1)**: gouge law (b) in Boundary mode still reads arcs in the XZ and
  YZ planes only at their chord joints, so a sharp wall corner between two
  joints is not judged there.
- **#864 (P1)**: an over-budget STEP mesh is imported at reduced fidelity with only a
  log line; gouge verdicts on it can pass a real gouge on a convex wall.
- **#906, #885 (P1)**: a machine that states a C travel but no C rate gets an
  uncapped rewind feed, and a combined linear + C move on the Fanuc and Doosan
  posts is posted at the mill feed without a cap from the C rate.
- **#938 (P1)**: a tool tilted by a rounding error (1e-10 degrees) is judged as
  tilted, and a holder wider than the cutter then reports a false collision.

## Process changes
Contributor and agent process only; no effect on the program.
- **PROCESS_VERSION 11** (#765): the xtask test scanner ignores braces in
  strings, chars and comments.
- **12** (#846): the `classify-effort` skill, one way to choose an issue's
  `effort:*` label.
- **13** (#855): the docs-picture goldens move to the release snapshot lane
  (xtask plan gains a step; a fork re-syncs xtask).
- **14** (#881): a contributor's claim must be live; stale after 48 hours with
  no open PR, branch commit or comment, and an ended claim is never mirrored
  back.
- **15** (#913): `cargo xtask gate` uses sccache when present; a lighter dev
  profile; agents must not export `CARGO_TARGET_DIR`.
- **16** (#944): agents prefer `cargo xtask gate`, set `RUSTC_WRAPPER=sccache`
  only when it is installed, and the release build runs without a compiler
  wrapper.

---

# CamCraft 0.17.0 (clean-room implementation)

Not a breaking release: the project file format stays at v58, so files saved
by 0.16.0 open unchanged and 0.16.0 opens 0.17.0 files. **Posted G-code does
change for existing machines**, so check the first program you post on each
machine. Every mill dialect's Face and other clearance-seeded paths now open
with an approach rapid on the clearance plane (#690, #691). Probe sections
start no spindle and descend to the safe plane before searching (#523, #570).
Lathe posts write the tool's control point for its orientation, threads cut
right-hand toward the chuck by default, and grooving and parting blades post
their touched-off corner (#629, #630, #632). Per-operation smoothing (#616) is
opt-in and changes nothing until you switch it on.

A minor release headlined by **plasma cutting** (a plasma machine kind, the
Tormach 1300PL and a Plasma Profile operation), **one picking rule for the
whole app**, new ways to align the part (three points, an edge or two
points, turning about the WCS origin), a fixture library, pocket and slot
recognition, per-operation smoothing in every mill post, and many new
machines. Thanks to mkinney, austin62jake, tbrasselle, vilts and JSwetz for
their contributions.

## New
- **Plasma** (#669, #670, #672): a `plasma` machine kind (X/Y cut, Z torch
  lifter, no spindle) and the Tormach 1300PL machine file; a Plasma Profile
  operation with a plasma torch tool (kerf = diameter): kerf offset computed
  or by G41/G42, outer contours clockwise and holes counter-clockwise
  (overridable), arc leads, the small-hole rule, and holes cut before their
  outer contour. Milling and turning operations are refused by name on a
  plasma setup.
- **One picking rule everywhere** (#692): faces, edges and points are picked
  from one feature graph - whole faces, real feature edges and corner points
  only, the visible one under the cursor first, never a tessellation edge or
  a hidden edge through the part.
- **Part alignment** on the Setups page: lay the part on a plane through three
  picked points (#555); set the in-plane axes by an edge or two points (#557, #554);
  alignment turns about the WCS origin and lays the picked face down (#606).
  The setup page previews the stock it takes over from the previous setup as a
  translucent body (#406).
- **Fixtures**: a fixture library to save, insert and delete your own fixtures
  (#339); moving a part warns when it or its stock would sink into a fixture
  (#582); a square stock stays square when the part is turned (#571).
- **Pocket and slot recognition** (#35): recognised pockets and slots turn
  into operations; an inside-out or open mesh is refused by name (#718).
- **Posting**: opt-in per-operation smoothing in every mill dialect (G64 P,
  G5.1, G187, CYCLE832, G62; #616); per-setup post options and a per-setup
  tool-list export (#389); Tormach programs can end in the operator's units
  (#596); probe programs report the measured deviation on the control (#575); Haas
  probing on an inverted X/Y axis (#392); comment text is escaped per dialect (#594).
- **Machines**: Tormach 8L, 15L and 15LX Slant-PRO lathes (#611, #619), 24R
  and xsTECH Pro routers (#620), 770M+ and 1100M+ (#618), and the Makera
  Carvera and Carvera Air with a Carvera post (#707). The Machines pane's
  "Use for this setup" assigns the selected machine (#384), and the machine overview
  is a readable two-column grid (#745).
- **MCP**: the tool table is served live from inside the GUI (#183); idle
  live-session clients are dropped after a timeout (#567); new `move_model` and
  `repair_model` tools (#716, #717).
- **UI**: recent projects and models on Open and Model (#662); a refused generation
  marks the spot it names, and drilling refusals mark their hole (#261, #677).

## Fixed
- **Safety and the post gate**: mill paths seeded on the clearance plane always
  open with an approach rapid (#690, #691, #697); a step too small to ladder is
  refused by name instead of allocated (#695); MCP `verify` no longer reports
  a job as clean while an operation's own warnings hold the post (#722); the
  program sim sweeps a tap at its thread's minor diameter, so a tap-drill-size
  hole is no longer a false gouge (#266, partial); an edge-break record is
  credited only within its operation's stated width (#529).
- **Probing**: a touch probe starts no spindle on any mill post (#523), is not
  judged against the machine's spindle range (#565), searches a short way past the
  face and feeds below the safe plane (#570); a probe-only program's stock sim
  judges the uncut stock (#592).
- **Turning**: threads cut right-hand toward the chuck by default and state
  their hand (#629); lathe posts write the tool's control point (#630);
  grooving and parting blades post their touched-off corner (#632, #641); facing
  approaches at rapid and lifts off the face before retracting; the lathe
  stroke and thread travel checks judge the posted radius (#631, #633, #647).
- **Setup page**: lay-on-plane controls stay on screen so a picked face is
  turned flat (#330); another setup's WCS triad stays on its spot (#558);
  setup picks disarm when their page is left and the pick sweep runs every
  frame (#664, #667); a hidden Properties pane drops the locate draft (#673).
- **Units**: status lines, refusals and the gate's gouge clause speak the
  document's units (#578).
- **Views**: orthographic depth maps into WebGPU's clip volume (#600), and
  orthographic picking and stock tracing cast parallel rays (#609, #610); view-cube pole
  clicks keep the roll (#622).
- **Machines**: the tool-number clash is judged per machine (#438); a saved
  machine file keeps its key order (#480).

## Changed
- **Smoothing, picking and alignment** are described above; the stock engine
  builds a block stock in closed form (faster, same result).

## Process changes
Contributor and agent process only; no effect on the program.
- **PROCESS_VERSION 1-4** (#682, #683, #685, #687): process files carry a
  version and are announced on the pinned issue #681 so forks pick them up;
  the `/resolve-conflicts` skill and the conflict rule.
- **5** (#742): MCP issues carry `area:mcp` and P1.
- **6** (#747): every issue is weighted `effort:low|medium|high|max` and
  worked by the matching `issue-worker` agent.
- **7-9** (#750, #751, #753): `cargo xtask gate|release|package` replace the
  PowerShell/bash scripts on every OS; the scripts are wrappers.
- **10** (#726): the camcraft-mcp skill's tool signatures match the server.

## Known issues
- **#738**: setup-page alignment re-applies from the import pose instead of
  composing on the current one, and picks are hard to see (fix in review,
  PR #746).
- **#757**: the Carvera post ignores per-operation smoothing without a note.
- **#74**: GPU selection can fail on hybrid Intel/NVIDIA laptops.

---

# CamCraft 0.16.0 (clean-room implementation)

BREAKING: the project file format moved from v57 to v58 (PR #381: a tool can
carry a modelled holder body). Files saved by 0.15.0 or earlier open here and
are migrated when saved, but a v58 file saved by 0.16.0 is refused by older
builds (save a copy first if you still need 0.15.0). Two post behaviours also
tighten and can refuse a program that used to post: the Tormach posts now
retract to the machine file's `safe_retract` and refuse a non-Tormach machine
that states none, the Siemens 840D post refuses any machine that states none
(#495, #503), and a probing post script must declare
`onProbeGuard` or the Probe op is refused by name (#174). The built-in VMC,
tilt-head, Lathe and Mill-turn machines now state their strokes in program
coordinates (#457, #493), so programs they used to refuse now post.

A minor release headlined by fixtures judged against their real mesh, tools
that carry modelled holder bodies, one NC file per mount for multi-setup jobs,
a Machines pane that edits machine files, and a much larger MCP surface. It
also lands a long run of sim, gouge-gate and post fixes. Thanks to mkinney,
seelesturm, austin62jake and tbrasselle for their contributions.

## New
- **One NC file per mount** (#388): setups on different machines or posts
  generate on their own machine, and the post writes one NC file per
  machine/post, each judged against its own machine; the program sim refuses
  by name to carry stock between setups on different machines (#404).
  Pause, optional stop and go-home entries can sit between operations (#155).
- **Fixtures judged against their real mesh** (#339): the box stays the broad
  phase, so a path between vise jaws no longer refuses the post; tilted,
  rotary, spindle-C and XZ/YZ-arc moves are judged too, each arc chord padded
  by its sagitta (#491), and spindle-C moves are judged at the station pose
  (#486). An upright rapid checks the whole tool stack and a ramp each band
  over its true height span (#424, #425). The fixture UX (PR #549, replacing
  #482, mkinney): mark or import a fixture from the browser, see whether each
  is judged by mesh or box and why, and see which fixture a warned move
  strikes. The Fixtures page rows wrap and Place has labelled fields (#509).
- **Tools with holder bodies**: a tool carries a modelled holder (STEP/STL/OBJ)
  placed by axis and tip, judged by its revolved envelope (#368); import,
  place and draw it in the tool editor and sim (#371); draw a milling holder
  as a revolved outline (#93). Lathe tools carry a holder or boring-bar body
  judged against the revolved stock (#369) with an optional insert zone
  (#396). Export and import a `.camtools` library (#300, PR #487 replacing
  #466).
- **MCP server**: pick faces, edge chains and loops for an operation (#180);
  `detect_holes` (#312, PR #456 replacing #314); `render_stock` PNG with rest
  and gouge volume (#182); setups, work offsets, machines and fixtures (#181);
  tool-library export/import (#355, PR #490 replacing #467); stdio hardening
  against null ids, oversize and non-UTF-8 lines (#443).
- **Machines pane** (View > Machines): browse and validate machine files
  (#287), and create, edit and save `.mch` files with per-field validation
  and atomic writes (#383). Machine files for the Tormach PCNC 770 and 1100
  (#170).
- **Probing**: seed a Probe op from a picked face, edge, vertex or round wall
  (#171); positioning moves post as protected stop-on-contact moves (#174);
  Fanuc probing on an inverted X/Y axis (#392).
- **UI**: orthographic (parallel) projection with a Projection row and bare P
  (#434, tbrasselle); warned toolpath moves are marked in the viewport and
  framed from the op page (#261, PR #454 replacing #349); a refused post's
  reason stays in a window (#265, PR #451 replacing #296); the Post dialog
  opens where the document last posted (#258, austin62jake); the G-code
  viewer lists from the left edge (#256, austin62jake); the Stock page and
  Theme button split (#441, #442); macOS Cmd+Q asks about unsaved changes
  (#144); the browser tree reads on the light theme (#506).
- **Licence**: CamCraft is now Apache-2.0 (was MIT), copyright held by its
  contributors (#452).

## Fixed
- **Post gate false gouges and misses**: XY arcs are cut on the true circle,
  not inscribed chords, in the column engine (#402), the field engine and the
  envelope scan (#413); a column within tolerance of a wall is judged against
  the floor beside it (#299); a typed Slot's round ends no longer read as a
  gouge (#510); a 2D Chamfer or Deburr is judged against its declared edge
  break (#303); the scallop check reads to the ball diameter (#212).
- **Crash**: an intermittent SIGSEGV/SIGABRT when threads created their first
  world at once is gone (#379).
- **Alignment** picks refuse by name on an inside-out, open or non-manifold
  body instead of silently flipping the part (#366).
- **Turning**: multi-pass, chamfer and taper turning exit a longer bar safely
  (#323, seelesturm); the rotary wrap turns its seam clear of a D-flat or
  keyway (#226); rotary Adaptive no longer sorts its leave-limit level by
  float noise (#55); swarf raster lines keep their lying angle on rapids
  (#48).
- **Posts**: the Fanuc probe result no longer writes a doubled sign (#377); a
  trailing Go home is written once (#504); the Tormach and Siemens 840D
  retract from the machine file (#495, #503).

## Known issues
- **#554**: face picking on the setup page keeps snapping to an edge.
- **#330**: Align by picking a face or two edges moves the part but does not
  fully rotate it onto the chosen G17/G18/G19 plane.
- **#558**: another setup's WCS can be drawn in the wrong place when the part
  is aligned differently.
- **#523**: posts start the spindle (S0 M3) on a probe tool.
- **#74**: GPU selection can fail on hybrid Intel/NVIDIA laptops.

---
# CamCraft 0.15.0 (clean-room implementation)

BREAKING: the project file format moved from v50 to v57, and two of those
steps change what a project means. Files saved by 0.14.0 or earlier are
migrated when opened, but a file saved by 0.15.0 is refused by older builds
(save a copy first if you still need 0.14.0).
- **One WCS per setup** (#342): a setup now owns a single work coordinate
  system, and G54-G59 is only the label the post writes for it. Placed slots
  that no setup posted are dropped on load, each with a load note naming the
  entity and where it sat; single-WCS projects post byte for byte as before.
  The `SetWcsOrigin` command lost its slot argument.
- **The setup defines the work offset, not each height** (#329): a height
  measured from the work offset reads its setup's offset, and the Heights
  page no longer offers a per-height WCS choice. A project from 0.14.0 or
  earlier whose operations used different G54-G59 slots inside one setup is
  refused at load by name (open it in 0.14.0, make the slots agree, save,
  then open it here).

A minor release headlined by multi-setup machining: a setup tree in the
browser, setups placed relative to one another, the stock carried from one
setup into the next (including rest machining, verification and fixtures),
plus inverted machine axes. Thanks to austin62jake, bherbruck, linuxguy123,
mkinney and seelesturm for their contributions.

## New
- **Setups as a tree** (#343): the browser shows each setup as a node with an
  active-setup radio and its own WCS, Stock, Machine, Post, Fixtures, Tool
  list and Operations; a setup can have its own machine and post.
- **Part alignment per setup** (#326, #330): hold the part flipped or turned
  (presets or typed angles) with the stock following square to the setup
  frame, or lay it on a G17/G18/G19 plane by picking a flat face or two
  straight edges. Face, 2D Contour, 2D Pocket and 2D Adaptive generate and
  post in an aligned setup; every other kind is refused by name.
- **Place a setup relative to another** (#316): same way up, flipped, turned
  or by angles; changing the previous setup's alignment carries into the
  linked setups, and the transform between two setups is derived from their
  alignments.
- **Stock carried between setups**: the program sim carries the machined body
  through the setup transform (#317); rest machining in a later setup reads
  it, and an upstream change marks the later setup Stale (#318); rapids,
  entries and links are judged against it (#319); a setup after the first can
  take its own stock or what the setup before leaves behind, and the setup
  page shows the incoming stock (#320); fixtures read the carried stock, a
  lathe or mill-turn part flip is refused by name, and the carried body is
  cached (#321).
- **Pick the work offset a setup posts at** (#196) on the Setups page; the
  setup sheet names it and conflicting picks are refused by name.
- **Inverted machine axes**: the machine file states a per-axis direction
  (#373); the Fanuc and Haas posts write the program mirrored (#374: signs,
  I/J/K, G2/G3, G41/G42, work-offset origin; an inverted Z, helical arcs,
  threading, rotary, live tooling, probing and every other post are refused by
  name); the axis is shown on the viewport triad, the setup sheet and the NC
  header (#375).
- **Pause or home after an operation, suppress an operation, "Skip pauses
  and homes" post option and one NC file per operation** (#110, #111).
- **G41/G42 cutter compensation on Haas, Mazak and Siemens 840D** (#39).
- **Continuous C-axis live tooling** (#33): rotary kinds on a mill-turn's
  spindle state the C angle per move and post through the Fanuc lathe post.
- **Comments in posted programs** naming the setup, each operation and each
  tool, in every dialect's own syntax (#257, mkinney).
- **Machine files for the other Tormach mills** (#170, mkinney): 770M, 1100MX,
  1500MX and PCNC 440; a .mch can state spindle belt ranges and rigid-tap
  ability.
- **Lathe parts with a large keyway or flat** turn instead of being refused
  (#113, mkinney); grooving, parting and form tools are cut with their true
  blade profile in the lathe stock sim (#21).
- **Probe**: the stylus stem and body are judged against the part (#172,
  mkinney).
- **Stock view and sim**: dual contouring for the machined-stock mesh keeps a
  pocket's sharp edges (#23); the mesh paints deviation classes and lights the
  hovered facet (#25); the hidden-rest scan answers past its old budget (#24).
- **UI**: the Stock page shows stock only and the theme and colour-vision
  picks have their own Theme ribbon button (#441); build version, time and
  commit show in the toolbar and a Debug info window with a Copy button opens
  from the status strip (#352, linuxguy123; #157, bherbruck); spindle speed is
  editable as rpm or surface speed (#225, bherbruck); a refusal on the status
  strip is drawn as an alert (#192, bherbruck); ribbon section and machine
  doors wear glyphs (#207, bherbruck); a new document can reach the Setups page
  to create its first setup (#322).
- **Logging**: the launching terminal logs save, open, post, generation and
  commands (#163, bherbruck; CAMCRAFT_LOG / RUST_LOG).
- **Operation tops below the remaining stock are explained** during upright
  2D/3D generation (#104, seelesturm).
- **Packaging and tooling**: Arch and FreeBSD package targets (#72,
  linuxguy123); the Windows installer's embedded uninstaller is signed when a
  certificate is configured (#29); headless dev binaries per crate (#32); the
  kernel seam gains union/intersection and edge blends, benched only (#27).

## Fixed
- **Cutter compensation** is engaged in G17, never in a stale G18/G19, on the
  Fanuc, Haas, Mazak and Siemens 840D posts (#391).
- **Work offsets**: a rotated work offset (3-2-1 alignment) is refused by name
  instead of posting unrotated (#315); a non-finite or zero-length rotation
  or a non-finite origin is refused by name (#327, mkinney).
- **Turning**: a pass that ends inside a longer bar leaves its cut at feed
  (#229, austin62jake); a plain two-ring bar fits its true axis and wraps
  (#228, austin62jake).
- **Deburr** leaves each edge run straight up before travelling (#304,
  austin62jake).
- **STEP import** reads the file's length unit (#306, austin62jake).
- **CL text dump** keeps a C-axis path's spindle angle (#295, austin62jake).
- **Facing** tells concave returns from narrow gaps between separate holes
  (#260, seelesturm); cross-drill topology is stable under roundoff (#151,
  seelesturm).
- A fresh depth operation bottoms out at the part's floor, not the G54 plane
  (#214, mkinney).
- A loaded stock keeps its geometry revision, so a project resaves
  byte-identically (#340, mkinney).
- A ball nose's corner radius is its ball, shown and saved as half the
  diameter (#263, austin62jake); Fit to Models' cylinder stock is snug (#230,
  bherbruck).

## Known issues
- **#424 and #425 (P0, fixture law)**: an upright rapid checks only the flute
  against fixtures, so the shank, holder and spindle housing can hit a clamp
  unreported; and a ramp is judged at its lowest z, so the top of the tool
  stack at the high end can hit a clamp unreported. Both predate 0.15.0 and
  are being worked. Until they land, check clamp clearance for holder and
  spindle by hand.

---
# CamCraft 0.14.0 (clean-room implementation)

A minor release: a Probe operation, per-operation coolant, G76 threading,
wear-mode cutter compensation, a Tormach post, 3-2-1 work-offset alignment,
an MCP server for AI agents, and a long run of rapid, drilling and lathe
safety fixes. Thanks to bherbruck, mkinney and seelesturm for their
contributions.

## New
- **Probe operation** (#108): face, edge, outside-corner and bore/boss-centre
  finding with a touch probe. The Fanuc post writes G31 skip touches with a
  no-contact alarm, can set G54-G59 or stop for the operator, and stops when a
  result is out of tolerance. Posts that cannot probe refuse it by name.
- **Coolant per operation** (#112): Off, Flood (default), Mist or Through
  tool. Every post writes the machine's own code, with M9 before each tool
  change and at program end. A coolant the machine has no code for blocks the
  post by name.
- **G76 threading** (#34): the Fanuc Lathe post can write a thread as a
  two-block G76 cycle (post option "Threading", default G32 passes).
- **Wear-mode cutter compensation** (#38): the contour's tool-centre path
  posts under G41/G42 Dn / G40, with the register holding only the wear delta
  (Fanuc; other posts refuse by name).
- **Tormach PathPilot post and 770MX machine** (#131, mkinney). A machine
  file in inches now posts inch (G20) programs through posts that read the
  machine's units.
- **Controller tool numbers** (#221, mkinney): a tool can carry its own
  T/H number; two tools posting one number are refused.
- **Align a work offset 3-2-1** (#187, bherbruck) from a picked face, edge
  and vertex or bore.
- **Box stock per-side allowance** (#198, bherbruck), and the Stock page
  shows the models' size and extents (bherbruck).
- **`camcraft-cli mcp`** (#128): a stdio MCP server that lets an AI agent
  open or import a model, set stock and tools, edit operations, generate,
  verify and post - every change undoable.
- **Unsaved changes** (#107): the title shows `*`, and closing, Ctrl+Q, New
  or Open over unsaved work asks Save / Don't Save / Cancel.
- **3D finish scallop ceiling** (#37): a stepover at or past the ball
  diameter, or a tolerance at or past half the ball radius, is refused by
  name.
- **UI**: Create cells darken without stock and open the Stock page (#216);
  a chunkier, readable origin triad (#195); pinning ribbon cells is undoable
  (#143, mkinney).
- **Stock view**: the rotary simulation states (and can enforce) its
  flute-rotation bound (#20); shading reads a true distance gradient (#22).

## Fixed
- **Rapids are judged more strictly**: against the tool's whole footprint
  (#106), along the whole length of sideways rapids (#140), and for turning
  in the meridian plane against the revolved bar (#103). Any rapid into
  uncut stock blocks the post.
- **Ops go Stale when what they read changes**: a stock edit (#156) or an
  earlier op of the setup changing (#109) marks dependent ops Stale, and the
  post refuses them until they are regenerated; one Regenerate All brings a
  job current. Saved projects may show some ops Stale once after upgrading.
- **Drilling** reads the stock earlier operations left (#89). A cross drill
  is judged against the finished part (#117) and enters box stock at the face
  (#85). It no longer gets the axial-hole check on a lathe (#80), reads its
  holder per station (#119) and unwelded STL rounds at their true surface
  (#121), and hops clear between off-centre holes (bherbruck, #81).
- **Lathe**: turning ops approach from, and rough down from, a bar wider
  than the part (#130). A boring bar's body is checked on its cutting feeds
  (#146). Facing faces the stock allowance above a finished part (#122,
  bherbruck).
- **Axis fit**: it no longer leans on one cross hole (#116), reads short bars
  from their facing flats (#152), and refuses normals that only meet at one
  point (#208). Fine-meshed balls (#237) and D-flat bars (#246) read
  correctly again (bherbruck). The rotary wrap names a seam feature or skin
  hole instead of "the mesh is open" (#54).
- New operations post after their setup's existing ones, not in name order
  (#167, bherbruck).
- Project files round-trip every float exactly (#169).
- The Heidenhain DIN/ISO post declares the TNC's `.I` extension (#52,
  mkinney).
- Idle frames no longer redraw, resizing stays responsive, and rotated face
  and edge picks no longer freeze (#164).

## Changed
- With a single part (or its named source model), moving or rotating the
  part now carries a non-model stock along in the same undo step (#164).

Known limits: the probe op is not yet created from a picked face, edge or
cylinder (#171); on CPU-rendered stock previews a paused scrub may stay stale
until the mouse moves.

---

# CamCraft 0.13.0 (clean-room implementation)

A minor release: CamCraft now ships for Linux and macOS as well as Windows,
three collision checks are tighter (issues #84, #91, #96), and a keyed shaft
is no longer refused on the lathe (issue #71).

## New
- **Packages for Linux and macOS** (contributed by linuxguy123): macOS ZIP
  and DMG, AppImage, Snap, Flatpak, and Linux x86_64 TAR.GZ, DEB and RPM,
  next to the Windows ZIP and installer; an Arch Linux source package
  (PKGBUILD) is in `aur/`. This is the first release these packages are
  built for on GitHub - the macOS, Snap and Flatpak builds have not run on a
  real runner before, so report anything that does not install or start. The
  macOS build is not notarized yet - Gatekeeper may call a downloaded DMG
  damaged until its quarantine flag is cleared.

## Fixed
- **Drilling checks the tool can reach** (#84): a hole deeper from the stock
  surface than the drill's flute is refused by name (a bore or tap in an
  existing hole only needs a shank that fits it), and the holder is proved
  clear of the stock (including the cross drill's swing over the end face).
- **A drill R plane below the stock or part top is refused** (#91): the rapid
  down to R used to cut through material and post silently.
- **A rapid straight down into uncut stock blocks the post** (#96), for every
  milling and drilling operation - unless an earlier operation of the setup
  already removed that stock. Turning rapids along the spindle axis keep their
  old reading for now (#103).
- **Keyed and flatted shafts turn** (#71): the lathe axis is read on the
  turned surface, so a correctly placed keyed shaft is no longer refused; a
  note names the share of the surface (keyways, flats, holes) left for a later
  setup.
- **The AppImage starts** (linuxguy123).

Known limits: with stock declared, a hole on an already-machined pocket floor,
or face-then-drill, may be refused by the drilling checks until they learn what
earlier operations removed (#89); a large keyway is still refused as "not
revolved" (#110).

---

# CamCraft 0.12.1 (clean-room implementation)

A patch release: four fixes, one of them a collision (issues #77, #79, #67,
#74).

## Fixed
- **Cross drilling no longer rapids through uncut stock** (#77). On raw stock
  wider than the finished part, a mill-turn cross drill used to hop between
  stations inside the stock at rapid feed, and nothing caught it. Hops and
  approaches now stand off the raw stock (and a shank wider than the drill),
  hole mouths start at the raw surface, every rapid is proved clear or the job
  is refused by name, and resizing the stock marks the drilling stale.
- **The post check now covers cross-drill rapids** (#79). The rapid-in-stock
  check that blocks a post was switched off for radial programs; it now
  judges them against the raw stock (retracts up a drilled hole stay clean).
- **Cross drilling measures about the machine spindle** (#67). On a bar off
  the world origin under a moved work offset, clearance and hops were measured
  about the wrong axis. A mill-turn bar off its work offset, a hole plunging
  away from the spindle, and on a lathe a part that cannot be read as turned
  are refused by name.
- **GPU startup on dual-GPU laptops** (#74, in progress). Every adapter is
  listed and tried in turn (your `--gpu <name>` choice, then the
  high-performance GPU, discrete before integrated, DX12 before Vulkan on
  Windows); each must draw a test frame, and a failure moves on to the next.
  If none works, a dialog names every adapter tried. Frames over 100 ms are
  logged by phase - please send those log lines if you see lag.

---

# CamCraft 0.12.0 (clean-room implementation)

A minor release, breaking first: Heidenhain NC output and the project file
format change (issues #50, #45, #62, #9, #19).

## Breaking
- **Heidenhain DIN/ISO programs take the TNC's own block form** (#50):
  numbered blocks ending `*`, signed words, the `%NAME G71` frame; drilling
  posts as plain moves with the dwell at the bottom, because G73/G80/G54/G55
  mean something else on a TNC. Tapping and any work offset but the first are
  refused by name on this post until the native cycles land (#51).
- **Project files are v50** (#45 part, contributed by linuxguy123): older
  builds refuse them; v49 files load and migrate.

## New
- **Project and operation workflow** (#45 part, PR #58): Save As and kept
  filenames; rename, reorder, copy and paste operations, with playback in
  their order; browser collapse and viewport visibility controls.
- **Adaptive on the rotary wrap** (#9): each developed level clears on the
  adaptive planner under the load and kink laws; a level it refuses is named
  by its radius and cut by Offset rings.
- **Turning programs in the stock simulation** (#19): facing, profile,
  contour and boring simulate about the part's spindle; blade, thread and form
  tools are refused by name until their sweep lands (#21).

## Fixed
- **A turning part whose axis is not the machine spindle is refused** (#62):
  a part leaning even slightly, or off the work offset, used to post a taper
  or an eccentric with no warning. It is now refused by name at generation
  and again at posting (so moving the work offset afterwards is caught), to
  within 0.0005 mm over the whole part.

---

# CamCraft 0.11.0 (clean-room implementation)

A minor release, breaking first: Heidenhain NC output changes for every XY
arc (issues #49, #17, #18).

## Breaking
- **Heidenhain DIN/ISO arcs post their true centre** (#49). The TNC reads
  I/J as the absolute circle centre, entered in its own block before the
  G02/G03 (TNC 640 ISO programming manual). The post wrote the centre
  relative to the arc start, so a real control would cut around the wrong
  centre. Every Heidenhain XY arc's output changes; an arc without a centre
  is now refused by name.

## Fixed
- **Swarf links half as often and never tilts under level** (#17): about
  half the air links on the blade, the lowest pose level instead of 7-9
  degrees under, and a ball tool's re-entry into the cut it just left is no
  longer refused as a plunge.

## Decided
- **Heidenhain vertical arcs and oblique entries stay chord fans** (#18):
  the manuals give no rotation sense for off-plane arcs, so a native arc
  would be a guess; the fan is exact to the output resolution.

---

# CamCraft 0.10.0 (clean-room implementation)

A minor release: one new capability and two fixes (issues #15, #14, #16).

## New
- **Rotary Parallel and Swarf ramp in on their first entry** (#15). Where a
  straight plunge is refused, the first infeed now comes in on a sloped ramp,
  each one proved clear of the part and the stock before it is kept.

## Fixed
- **A lead-in outside a round bore is seated inside it** (#14): one descent
  per level instead of a plunge into material beside the bore.
- **Rotary Offset is judged against the real round bar** (#16). Its plunge
  check now uses the true stock cylinder about the rotary axis instead of a
  box around it, so moves in the air beside the bar are no longer refused.

---

# CamCraft 0.9.0 (clean-room implementation)

A minor release: one new capability and one measurement fix (issues #11, #10).

## New
- **Adaptive arc output covers the growing spiral** (#11). Where a single
  arc could not follow the spiral's changing curvature, the path is now
  fitted with pairs of tangent arcs (biarcs), each proved inside the area,
  crossing-free and kink-free - about a third fewer moves: rectangle
  18,372 → 12,730, L-pocket 51,788 → 34,582, slot 23,382 → 14,748.

## Fixed
- **The load reader reads chord paths at their true engagement** (#10): it
  faces the path's real direction at each corner, not the lagging chord, so
  readings on straight-segment paths dropped (e.g. the L-pocket from 22.2 %
  to 21.4 % of the tool diameter at the coarse grid).

---

# CamCraft 0.8.4 (clean-room implementation)

A patch release: three fixes (issues #13, #36, #46).

## Fixed
- **2D Adaptive no longer hangs on a long thin band** (#13). A channel too
  long and narrow for the adaptive spiral is refused in about a second with
  its size named, and Offset rings cut it - instead of planning for minutes.
- **Finishing passes ride over mesh folds** (#36). On coarse meshes the
  rows no longer skip a strip at every fold, so the scallop stays within
  tolerance on curved parts.
- **Raster passes reach the rim** (#46). Each pass ends with a row where its
  ground ends, so tilted and flat shelves are finished to the edge.

## Also in this build
- The 2D Adaptive trochoid fallback is wired but still refused by name in
  every case (its landings are #47); output is unchanged.

---

# CamCraft 0.8.3 (clean-room implementation)

A patch release: four fixes, three of them safety (issues #12, #42, #43, #40).

## Fixed
- **A whole-part Pocket or 2D Adaptive no longer cuts into the part** (#12).
  With the whole model as the source, it now clears only the stock around
  the part; if the part fills the stock outline, it refuses and says so.
- **The verifier catches a straight cut across a part's corner** (#42).
  Pocket-type operations are now checked along every move, not only at its
  end - a 0.3 mm corner intrusion reads as a gouge and blocks the post.
- **G41/G42 compensation refuses contours the controller cannot offset**
  (#40): inside fillets tighter than the tool, edges too short for the
  offset, and necks narrower than the tool - each named with its position.

- **Contour moves are checked against the part's walls along their whole
  length** (#43), so a straight cut across a convex corner of the wall is a
  gouge; a Face's rows crossing its open rim stay legal.
- The pocket check from #42 now judges only the stretch of a move below the
  stock top: a ramp crossing the wall line in the air is not a gouge.

---

# CamCraft 0.8.2 (clean-room implementation)

A bug-fix release from hand-testing 0.8.0/0.8.1.

## Fixed
- **Closing an edge loop.** In Edges mode, Shift+click takes the whole loop
  an edge lies on, and Ctrl+click adds or removes one edge. The Selection
  group shows the hint, and the "open run" error says how to close the loop.
- **The Bottom height after picking a face.** Switching Bottom's *From* to
  Selection kept the previous reference's offset (e.g. +18.7), which put the
  floor above the picked face. It now starts at offset 0, so the floor is the
  face.
- **Containment Outside on 2D Adaptive/Pocket.** It no longer machines a
  recess inside the part: only the outer edge is grown, holes stay protected,
  and pockets the band past the outline does not reach are left alone. With
  Outside alone the band is exactly one tool wide, so 2D Adaptive refuses it
  by name ("no room for an entry") - add a Boundary extension.
- **Circular follows the selection.** Pick a bore rim, a round pocket floor,
  or a face around one round hole, and the rings are centred and sized from
  it (shown as "From selection: Ø… at (x, y)"). A selection that is not round
  is refused with where it deviates; with no selection the typed centre and
  radius work as before.

## Known
- 2D Adaptive on a wide Outside band can be slow to plan (minutes); 2D
  Offset plans the same area quickly.

---

# CamCraft 0.8.1 (clean-room implementation)

A packaging release on top of 0.8.0 - the program is unchanged.

- The repository and the release now carry the project's MIT `LICENSE`.
- New issue templates on GitHub: **Toolpath / path planning bug** and
  **UI / UX bug**.
- The repository no longer carries local tooling, agent settings or
  research material.

The 0.8.0 notes below still describe everything in this build. Fixes for
the bugs reported against 0.8.0 (edge-loop picking, face heights and
Outside containment in 2D Adaptive, Circular following the selection) are
in progress for the next release.

---

# CamCraft 0.8.0 (clean-room implementation)

Eighth public release, cut early (2026-11-06) to park a stable build. It
carries what was planned for three trains (0.8.0, 0.9.0 and most of 0.10.0),
every piece through the full gate, heavy lanes included. Everything a
generator cannot prove safe is still refused by name, never posted.

This is an independent, clean-room reimplementation of user-visible HSMWorks
behavior, specced from published documentation and general machining knowledge.
"HSMWorks" is a trademark of Autodesk; this project is unaffiliated (see
AGENTS.md). Third-party notices: `assets/ui/LICENSES.md`.

Please start with `TESTING.md`: a short hand-test list for what changed.

## What's new since 0.7.0

### Machining
- **G41/G42 cutter compensation** on Contour: *Compensation: In control* emits
  the part contour with `G41/G42 D<tool>` / `G40` through the Fanuc post. It
  needs a straight lead of at least the tool radius; anything else is refused
  by name, and posts that cannot emit it refuse instead of writing a centre path.
- **Arc output for adaptive clearing**: G2/G3 over the HSM passes, each arc
  proven inside the area, crossing-free and kink-free (a bore went from 6,650
  to 1,200 moves).
- **3D Pocket uses the HSM planner** per level (*Strategy: Adaptive*); a level
  it cannot plan is cut with offset rings and the summary names its Z.
- **Facing goes around holes** (*Holes: Lift*, the default); *Cut across air*
  and *Through* (the old behaviour) are choices.
- **Contours across two models**: picked edges that meet end to end across
  bodies become one ring, and the depth ladder cuts every level of the union.
- **Waterline stepdown reads the slope**, and the finishing march no longer
  drops rows across mesh fold gaps - most scallop readings are now inside the
  tolerance.
- **Boundaries everywhere they apply**: on 3+2-indexed operations (read in the
  tilted plane), on every 3D finish, and on Trace/Engrave; a clipped finish
  ramps back in.
- **Lathe**: axial drilling on the spindle axis, and the first lathe post
  (`Fanuc Lathe`: diameter X, G96/G97/G50, G99, G83/G84/G85, G32). The Setup
  pane offers the matching post when you mount a lathe; it never switches it
  for you. C-axis live tooling is not in this release (refused by name).

### Safety
- **5-axis swarf** no longer drags the tool's flank through the part on its
  in-air links (it retracts along the surface normal) and never falls back to
  an upright pose inside a wall.
- **Fixtures are checked along the whole move**, not just at its ends, for
  every pose change.
- **Scallop is reported** (a non-blocking note) where a finish leaves more
  than its tolerance.

### Simulation and display
- **The machined stock** is drawn in the viewport and can be picked; rotary
  programs now show their stock too.
- **Playback jumps** no longer stall, and the CPU fallback renders off the UI
  thread.

### Models and output
- **Repair mesh** (Model page): weld vertices and fill planar holes, one undo
  step, never automatic.
- **The setup sheet** says whether each fixture is fixed or turns with the part.
- **Windows installer** (preview) beside the zip, and an optional signing step;
  this build is unsigned.

### Known limits
- C-axis live tooling, G41/G42 wear mode, and comp on posts other than Fanuc.
- Lathe operations are not shown in the field stock simulation yet (refused
  by name; the turning verdict at generation still applies).
- The adaptive planner still falls back to offset rings on island pockets.

---

# CamCraft 0.7.0 (clean-room implementation)

Seventh public release: the user-review release. Most of what is new came
straight from hand-testing 0.6.0:
- the operation dialog now reads like a CAM dialog;
- simulation playback no longer locks up;
- lead in / lead out is complete;
- adaptive clearing is a real HSM spiral;
- every boundary says what it does.

Under it, a run of safety work: wherever the verifier had a blind spot, it was
closed. Where a plan could not be made safe, it is refused by name instead of
posted.

This is an independent, clean-room reimplementation of user-visible HSMWorks
behavior, specced from published documentation and general machining knowledge.
"HSMWorks" is a trademark of Autodesk; this project is unaffiliated (see
AGENTS.md). Third-party notices: `assets/ui/LICENSES.md`.

Please start with `TESTING.md`: a short hand-test list for what changed.

## What's new since 0.6.0

### From your review
- **The operation dialog, Fusion-style.**
  - Label and value share a row, units sit beside the value, and fields are
    grouped in collapsible sections.
  - Only the fields the current choice uses are shown.
  - "0 = auto" fields became explicit Auto toggles, and help moved to tooltips.
  - A one-line status banner sits at the top; a fixed action bar at the bottom
    has Generate as the primary action and Delete set apart.
  - The tabs stay.
- **The boundary says what it does.**
  - One plain sentence per setting, a "Machine the other side" button, and a
    notice when a setting has no effect on the kind.
  - The viewport fills the area actually swept; open edges are dashed.
  - Contour now rides an open boundary as one open pass.
- **Simulation that keeps up.** Playback runs on a worker thread: a UI frame
  went from 182 ms to 0.13 ms, and the stock follows the cutter. Each picture
  uploads only the changed bricks (2 MB to 57 KB).
- **Source is the stock or a part.** G54, setups, tools and fixtures are no
  longer offered, and are refused if a file names them.
- **Pick a vertex, set the origin there.** A new "Vertices" pick mode; "Set
  origin" on a picked vertex or face (centroid, or Z only on a horizontal
  face). It is undoable.
- **Panels always come back.** There is a permanent viewport, a Panels… menu
  and Reset layout. Operation-page errors clear, and Clear toolpath removes the
  path, not the operation.

### Machining
- **HSM adaptive.**
  - A constant-load spiral with no kinks past 5° and one entry per level.
  - The optimal load is set in % of D.
  - Narrow slots join their pieces with lifted returns (floor air 11 m to
    0.9 m).
  - A plan that can't be made sane falls back to offset rings by name.
- **Lead in / lead out, complete.** Ramp angle and length, a continuous helix,
  and arc radius and sweep, all in one tab.
  - Every entry has extensions, entry feeds and an engagement limit.
  - Heights can be measured from the selection.
  - A plunge height of 2 mm applies.
- **Plane-aware transitions.** Arcs carry their plane: G17/G18/G19 in the CL
  and every post. Arc entries post as true vertical arcs.
- **Keep-down links** and plunge, ramp, helix or arc level transitions on every
  ladder operation; how levels link is your choice.
- **New entries.**
  - A horizontal (G17) arc entry for pockets.
  - Face entries run out to air for tools that are not centre-cutting.
  - 3D ladders slow only the first diameter of cut.
- **Area control.** Inside, on or outside containment, boundary offset and
  extension, extra include/exclude boundaries, and open boundaries that start
  outside the stock.
- **Smoother paths.** Tangent arc blends at corners and links (line-arc and
  arc-arc too), and facing reversals become single arcs.
- **More finishing.** Flowline, steep-and-shallow, flat area, pencil and corner,
  ramp, morphed spiral, geodesic, blend, ruled loft, project drape, deburr, 3D
  offset shells.
- **2D additions.** Engrave, chamfer, circular, standalone bore, helical thread.
- **Rotary and 5-axis.**
  - Posed ball tools post the tip, so they land on plan (they were 4.2 mm off
    on 5-axis).
  - Rotary programs open on the clearance pose, and Rotary Parallel links run
    radially.
- **Machine limits and cutting data.** Spindle speed and power are checked, and
  speeds and feeds are suggested per material.

### Safety (the Prime Directive)
- The verifier judges the tool against the part's real solid, with the tool's
  real tip. A program it can't judge says so.
- Closed blind spots:
  - stock under overhangs;
  - a positive stock-to-leave that hid a gouge of that depth;
  - cuts that only happen later in the program;
  - tilted holders sweeping through a chuck;
  - the spindle on tilted rapids.
- Tools that are not centre-cutting never plunge into solid, anywhere: 2D, 3D,
  drilling, Face and every multiaxis kind. They ramp, helix, or are refused by
  name.
- No level under a blind floor; 2D pockets and contours see what hangs above
  them.

## Projects
- The project format goes from v42 to v49. Older projects load. New fields are
  written only when used, so a re-saved 0.6.0 project only grows where you
  change something.
- The persistence of picks, open boundaries, vertex picks, fixed fixtures and
  the pocket arc entry is round-trip tested.

## Known limits (tracked in ROADMAP.md)
- Adaptive island pockets fall back to offset rings, by name.
- Rotary Parallel's radial infeeds and Swarf's first raster entry always plunge.
  With a tool that is not centre-cutting they are refused rather than ramped.
- The fixed-fixture flag has no UI toggle yet, and the generate-time fixture
  check still uses the stricter older law; the post gate applies the new one.
- Building is Windows-first. Linux and macOS build from the same source (M4.2);
  code signing and an installer are still owed (M13.3).
