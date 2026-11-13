+++
title = "Reading a refusal"
description = "Why CamCraft refused, and how to fix it."
weight = 6
+++

CamCraft refuses rather than guess. A refusal is not a crash: the document
is unchanged and nothing unsafe was written. Every refusal names what is
wrong, and usually what to do about it.

## Where refusals appear

- **A refused post** opens the **Post refused** window with the reason. It
  says that nothing was written to the file, and that a program already
  there is unchanged. The window stays until you close it, post
  successfully, or open another document. If you edit the document while
  it is open, it tells you the document has changed since, so post again
  to check it.
- **A refused change** (an edit, a new setup, a stock change) shows its
  reason on the page you edited or on the status line, and the change is
  not made.
- **A refused generation** shows its reason on the operation's page. When
  the reason names a place on the part, it is underlined: click it to
  frame that spot in the viewport.
- **A greyed-out button** says in its tooltip why it is unavailable.

## Common post refusals

| The message says | What it means | What to do |
|---|---|---|
| `<operation>: no toolpath yet - generate it first` | The operation was never generated. | Select it and click **Generate**. |
| `<operation>: inputs moved since generation - regenerate before posting` | The model, stock, tool, work offset or the operation's settings changed after it was generated. | Generate it again. |
| `post blocked - <operation>: <n> warning(s): ...` | The operation's own checks found something unsafe. | Read the warnings. The flagged moves are marked in the viewport; fix the setting or geometry they name. |
| `gouge: program cuts below the model ...` | Simulating the whole program found material removed from the part. | Use **Simulate** and **Deviate** to find where, then fix the operation that cuts there. |
| `... - fix the tool before posting` | An operation's tool is missing or cannot cut that kind of operation. | Pick a suitable tool on the operation's Geometry tab. |

A machine-limit refusal names the axis or setting and the limit the
mounted machine states. Either the program really is outside the machine's
range (move the work offset or the part, or choose another machine), or
the machine file's number is wrong (check it against the machine's
manual).

## When you think a refusal is wrong

Report it on the
[issue tracker](https://github.com/NeoTech/Camcraft/issues) with the
message and, if you can, the project file. A refusal that blocks a safe
program is a bug. A program that would gouge or collide and was **not**
refused is a worse one: please report those first.
