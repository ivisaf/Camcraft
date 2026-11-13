+++
title = "Setups"
description = "Stock, work offsets, machines, posts and fixtures, per setup."
weight = 3
+++

A setup is one way the part is held on one machine: its stock, its work
offset, its machine and post, its fixtures and the tools it uses. A new
document has no setups yet: its one stock and one work offset act as the
first setup. Click **Create first setup** (Setup > Setups) to make it
explicit, and **New setup** to add more, for example to flip the part and
machine the other side.

## The setup in the browser

Each setup appears in the browser with its own pages:

- **Settings:** whether it is the active setup, where its stock comes from,
  how the part is aligned, and the WCS label the post writes.
- **Machine:** the setup's machine. It can follow the document's machine
  or carry its own. **Load machine (.mch)** loads a machine file.
- **Post:** the post processor the setup writes with: a built-in post, or
  **Load post (.cps)** for a script of your own.
- **Fixtures:** vises, clamps and other workholding, imported or placed
  from the fixture library.
- **Tools:** the tools this setup's operations use, in change order.

Below the pages, **Operations** lists the setup's operations.

## Work offset

Each setup owns one work coordinate system. G54 to G59 is only the label
the post writes for it. Heights that are measured from the work offset read
the setup's offset.

## Carrying stock to the next setup

**New setup** with **Carry stock and WCS over** ticked starts the new
setup from a copy of the active setup's stock and work-offset placement;
re-place the copy to describe how the part is re-clamped.

On the Settings page of any setup after the first, the stock source can
instead be what the setup before it leaves behind. Verification, rest
machining and the fixture checks of that setup then see the material the
earlier setup left, not the raw stock. CamCraft refuses that choice by
name when it cannot verify the carry, for example between setups on
different machines.

## Machines

The workspace button on the Setup group mounts a built-in machine for
Milling, Turning or Mill-turning, and the ribbon shows only the operations
that machine can run. CamCraft also ships machine files for several
Tormach mills, and **View > Panels > Machines** browses, validates and
edits `.mch` machine files.

A machine file states facts such as axis travel, spindle speed range and
rigid-tapping support. CamCraft checks a program against the facts the
mounted machine states before it posts. A machine file is not a guarantee
that a real machine is safe to run: check its numbers against your
machine's manual.

## Setups on different machines

Setups can sit on different machines or posts. Each such group generates on
its own machine, and Post writes one NC file per machine and post, each
checked against its own machine.

## Setup sheets

The Setups page lists each setup's tools in change order, with cut and
rapid lengths and the estimated time. **Export setup sheets…** writes one
text page per setup. Posting does not write them.
