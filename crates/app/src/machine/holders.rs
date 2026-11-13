//! #828: the holder library handed to the post crate's machine reader.
//! `camcraft-post` does not depend on `camcraft-tools` (maintainer,
//! PR #880: a crate edge carrying a couple of items is passed in by the
//! caller instead), so the app passes the library lookup to the reader
//! (`parse_mch_with`), the pane's validator (`tooling_refusals`) and the
//! stored-machine re-judge (`Machine::tooling_refusal`). Our own glue.

use camcraft_post::machine::{HolderInfo, Machine};

/// A built-in library holder by name (case and spacing ignored), as the
/// mount rule reads it; the library's own refusal when there is none.
pub fn holder_lookup(name: &str) -> Result<HolderInfo, String> {
    camcraft_tools::holder_library::find(name)
        .map(|h| HolderInfo { name: h.name(), taper: h.taper.map(|t| t.name.to_owned()) })
        .map_err(|e| e.to_string())
}

/// Parse a `.mch` machine file, its default holder judged against the
/// holder library - the app's one reader (shadows the post crate's
/// word-only `parse_mch`).
pub fn parse_mch(text: &str) -> Result<Machine, String> {
    camcraft_post::machine::parse_mch_with(text, Some(&holder_lookup))
}

/// Every refusal the tooling keys of a `.mch`'s JSON earn, as
/// (key, rule), judged against the holder library.
#[must_use]
pub fn tooling_refusals(v: &serde_json::Value) -> Vec<(String, String)> {
    camcraft_post::machine::tooling_refusals(v, Some(&holder_lookup))
}

/// Why a stored machine's tooling keys cannot be meant, by key.
#[must_use]
pub fn tooling_refusal(m: &Machine) -> Option<String> {
    m.tooling_refusal(Some(&holder_lookup))
}
