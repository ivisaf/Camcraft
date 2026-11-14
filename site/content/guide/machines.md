+++
title = "Machine types"
description = "Mill, router, lathe, mill-turn, plasma and EDM: what each allows and how it changes setups and posting."
weight = 7
+++

A machine file (`.mch`) declares a **kind**: `mill`, `lathe`, `mill-turn`,
`plasma`, `wire_edm` or `sinker_edm`. A file that declares nothing is read
from its axes (a mill unless it has a lathe's X/Z pair or a turning
rotary); plasma and EDM are never guessed, only declared. The kind decides
which operations the document accepts, which posts the picker lists and
which limits the program is checked against. See [Setups](@/guide/setups.md)
for mounting a machine and [Reading a refusal](@/guide/refusals.md) for the
messages below.

| Kind | Operation families allowed | Posts that serve it | Shipped machine files |
|---|---|---|---|
| Mill | Milling 2D, Milling 3D, Drilling, Multi-axis | every milling post | VMC 3-axis, Tilt-head 5-axis, Tormach, Carvera |
| Lathe | Turning, plus the axial Drill | Fanuc Lathe, Tormach PathPilot Lathe | Turning center 2-axis, Tormach 8L, 15L, 15LX |
| Mill-turn | milling and turning | every post except plasma | Multitasking 5-axis |
| Plasma | Plasma Profile only | Tormach PathPilot Plasma | Tormach 1300PL |
| Wire / sinker EDM | Wire Contour (wire EDM); Sinker Burn, Side Burn (sinker EDM) | none yet | none yet |

The rules are `Family::admits` in `crates/app/src/operation/kinds.rs` and
`post_serves` in `crates/app/src/posting.rs`. A picker lists posts by this
table, but picking another post is not blocked there: a post refuses by
name what it cannot express.

## Mill

The default. The ribbon shows the milling sections (2D, 3D, Drilling,
Multi-axis). Rotary axes in the file unlock the rotary and multi-axis
operations; an axis that states a leaning direction (a tilting head or
trunnion) tilts the cutter, while a C table only turns the part. A program
that asks for a tilt the machine cannot hold is refused by name.

For setting up 3+2, continuous 4-axis and 5-axis work, which machines and
posts allow it and what is checked, see
[4-axis and 5-axis machining](@/guide/multi-axis/_index.md).

Checked against the machine file at post, when it states them: axis travel
(arc bulges included), feed ceilings, spindle rpm range, spindle power,
tool length and diameter against the tool table, changer capacity, and
rotary letters and angles.

Posts: Fanuc (the default), Haas, Siemens 840D, Heidenhain, Mazak,
Tormach PathPilot, Doosan DNM and Makera Carvera. Machine files:
`3axis-vmc`, `vmc`, `tilthead`, `doosan-dnm`, the Carvera pair and the
Tormach mills.

Common refusals: `a <kind> operation cannot run on its setup's milling
machine` (a turning or plasma operation on a mill), `move <i> tilts the
tool; <machine> states no tilting rotary`, and `move <i> commands <axis>
<angle> deg, the travel is <lo>..<hi>`.

## Router

CamCraft has no separate router kind: a router is a machine file that
declares `mill`, so everything above applies. Nothing in the code treats a
router differently, so there are no router-only operations or limits; its
travel, spindle range and tool table are whatever its file states. The
Tormach 24R and xsTECH files in `assets/machines/` are mills in this sense.
Sheet-goods practice (spoilboard, tabs, hold-down) is handled by the
ordinary milling operations and the setup's fixtures, not by the machine.

## Lathe

The work spindle turns the part; X is a diameter. The ribbon shows Drilling
(the axial Drill) and Turning. Milling operations are refused on a lathe,
and the Drill is admitted only because a hole down the spindle axis is the
turret's simplest hole: a hole off the axis is refused by name.

A lathe setup can state a **workholding rpm limit**, the chuck or collet's
rated maximum. Under constant surface speed (G96) the posted rpm ceiling is
the lowest of the machine's, the tool's and the workholding's. Under
constant rpm (G97) an operation over the limit is refused at the program
gate, never silently capped. Slide strokes are judged on the posted path.

Posts: Fanuc Lathe (the default for a lathe) and Tormach PathPilot Lathe.

Common refusals: `a Drill operation would be posted on the work spindle
with a hole off the spindle axis (X.. Y.. from the work offset)`, `turns
the work at <rpm> rpm (G97), over setup '<name>''s workholding maximum of
<limit> rpm`, `move <i> reaches <v> in <axis>, the stroke is <lo>..<hi>`,
and `<post> does not serve <machine>, a turning machine` (a milling post
on a lathe setup).

## Mill-turn

Both families are admitted: turning on the work spindle and milling with
live tooling on the tool spindle (the C axis). Its posts are every post
except plasma. For a live-tool move the machine's C rate is what is judged,
and the work's chuck rpm limit applies to sections that run on the work
spindle. Each shipped post script declares its dialect in
`capabilities` (1 = milling, 2 = turning).

Common refusals: `<post> is not a turning post - on <machine>, a mill-turning
machine, these sections turn the work` (turning sections through a milling
post), the same off-axis Drill refusal as a lathe, and `turns the work on
the C axis at up to <rpm> rpm ... over setup '<name>''s workholding
maximum`.

## Plasma

A tool-less cut: no spindle, so rpm and spindle power are not judged, and a
plasma machine file that states a spindle or a rotary is refused. Z is the
torch lifter, owned by the control (pierce and cut height), so Z depth and
Z travel are skipped with a named note. XY travel, XY feed and the changer
count are checked. Only the Plasma Profile operation exists, and it is
admitted on a plasma table only (not on a mill-turn). A spindle word, a
tilted or a turned move is refused, and the plasma laws (cut order,
crossing, kerf, tip-up) run at the post gate. The 3D stock simulation does
not run for a torch.

Post: Tormach PathPilot Plasma, the default, and the only post that serves
plasma. Machine file: `tormach-1300pl`.

Common refusals: `a <kind> operation is <family> work - a plasma table
cuts only plasma operations`, `tool <n> states <rpm> rpm - a plasma table
has no spindle to turn`, `move <i> tilts or turns the tool - <machine>
states no rotary axis`, and, when the machine file is read, `a plasma
machine has no spindle - a torch states no rpm, power, speed range or
tapping` and `a plasma table cuts flat sheet in X and Y - it states no
rotary axis`.

## Wire and sinker EDM

The kinds exist (`wire_edm`, `sinker_edm`), and the Wire Contour operation
(`OpType::WireContour`) is admitted on a wire EDM machine. What does not ship
yet is an EDM machine file. Status: Wire Contour can be created and
generated on a machine file you write that declares `wire_edm`; posting it
stops with `a Wire Contour operation posts through a wire EDM post, which
is not built yet - nothing is posted`. The sinker EDM kind admits Sinker
Burn and Sinker Side Burn on a machine file you write that declares
`sinker_edm`; a straight Sinker Burn posts through the "Sinker EDM (ISO)"
post, which takes every EDM code from the file's `edm.codes`.
The operations, their settings and their refusals are in
[Wire and sinker EDM](@/guide/operations-edm.md).

## Changing a setup's machine

Operations are judged against the machine at post, so switching a setup to
another machine does not rewrite its operations; an operation the new kind
does not admit, or a limit the new file states, is refused when you post.
A post that does not serve the machine's kind is filtered out of the
picker and, if mounted anyway, refused by name at post (`<post> does not
serve <machine>, a <kind> machine`).
