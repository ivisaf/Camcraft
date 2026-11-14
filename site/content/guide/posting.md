+++
title = "Posting"
description = "Writing the NC program, and what is checked first."
weight = 6
+++

Posting turns the document's operations into an NC program for one
machine controller. Click **Post** (Ctrl+P, Cmd+P on macOS), choose where to save the
program, and CamCraft checks the program, writes it, and opens it in the
G-code viewer.

## Post processors

The post is chosen on the **Post** page: a setup can pick its own, and a
setup that picks none uses the document's. CamCraft ships posts for Fanuc,
Haas, Siemens 840D, Heidenhain, Mazak, Tormach PathPilot, Makera Carvera and
Fanuc Lathe. A post is a `.cps` script, so you can load your own with
**Load post (.cps)**.

The Post dialog opens where this document last posted, or beside the saved
project with the project's name and the `.nc` extension.

## What is checked before anything is written

The post is a gate. It writes the program only when all of these hold:

- every operation has a tool that exists and can cut that kind of
  operation;
- every operation has been generated, and nothing it depends on has
  changed since (no **stale** operations);
- no operation carries a warning;
- the program as a whole verifies: the simulated stock shows no gouge into
  the part, no move strikes a fixture, and no move exceeds a limit the
  mounted machine states, such as axis travel or spindle speed.

If any check fails, **nothing is written**; a program already at that path
is left unchanged. See [Reading a refusal](@/guide/refusals.md).

## More than one machine

When setups use different machines or posts, Post writes one NC file per
machine and post, and checks each one against its own machine.

## Before you run it

Passing the gate means CamCraft found nothing wrong with the program
against the model, stock, fixtures and machine facts it knows. It does not
know everything about your machine and setup. Read the program, run it in
your control's own simulation if it has one, and dry-run it with the
spindle off and Z raised before cutting material.

## Headless posting

The command-line tool `camcraft-cli` can drive CamCraft without the window,
including as an MCP server for AI agents (see [AI agents (MCP)](@/guide/mcp.md)). The standalone `post` tool runs a
`.cps` script over a cutter-location file; it does not run the document
checks above, so its output is not verified.

Which unit the program is written in, and where its coordinates are
measured from, is on [Units and coordinates](@/guide/units-and-coordinates.md).
