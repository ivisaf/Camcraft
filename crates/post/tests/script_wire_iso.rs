//! The `wire-iso.cps` script suite (#533, the per-script-suite law of
//! #926): the post binary on every recorded edge-case stream
//! (`recorded/streams/*.cnc`), its program or its named refusal compared
//! with the golden in `recorded/golden/wire-iso/`. The recorded streams
//! are cutting streams posted machine-less (or on a mill), so the wire
//! post refuses each by name - it posts only wire EDM work on a machine
//! declared `wire_edm`; its wire program is pinned by app's `wire_post`.
//!
//! Regenerate after an intentional change, then read the diff:
//! `WRITE_GOLDEN=1 cargo test -p camcraft-post --test script_wire_iso`
//!
//! Gate map (M4.4): covers assets/posts/wire-iso.cps, camcraft_post::post::engine,
//! camcraft_post::post::cnc, the post binary.

#[macro_use]
mod recorded;

post_suite!("wire-iso.cps");
