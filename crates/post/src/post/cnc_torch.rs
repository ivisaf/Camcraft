//! The plasma records of the intermediate stream (#671): the `PLASMA`
//! header and the `TORCH ON` / `TORCH OFF` spans `cl::PlasmaCut`
//! states, in stream clothing - and the engine's laws over them.
//!
//! Provenance: our own line format (`docs/CNC-INTERMEDIATE.md`, "Plasma
//! records"); the words are the `.cl` v5 records #670 writes, so one
//! reader (`cl::parse_plasma_header` / `parse_torch_on` /
//! `parse_torch_off`) takes both the `.cnc` record and the `.cl` comment
//! form - every plasma golden stays a legal post input. The quantities
//! are the ones Tormach's public 1300PL manual UM10720 §7.1-7.2 lets a
//! program state (pierce height, cut height, pierce delay); behaviour
//! only, our own code.

use super::js::Script;
use super::PostError;
use crate::cl::{ContourRole, PlasmaCut, TorchSpan};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The section's plasma header: the cut's kerf and what the program
/// states (`None` = the controller's AutoFS decides). Lengths mm,
/// delay seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct PlasmaRec {
    pub kerf: f64,
    pub control_comp: bool,
    pub pierce_height: Option<f64>,
    pub cut_height: Option<f64>,
    pub pierce_delay: Option<f64>,
    pub auto_feed: bool,
    pub reversed: bool,
}

/// One torch event: `Some(span)` strikes the arc before the next
/// motion record, `None` puts out contour `contour` after the previous.
#[derive(Clone, Debug, PartialEq)]
pub struct TorchRec {
    pub contour: u32,
    /// The ON side: role, parent, small-hole factor. `None` is OFF.
    pub on: Option<(ContourRole, Option<u32>, Option<f64>)>,
}

impl PlasmaRec {
    #[must_use]
    pub fn from_cut(p: &PlasmaCut) -> Self {
        PlasmaRec {
            kerf: p.kerf,
            control_comp: p.control_comp,
            pierce_height: p.pierce_height,
            cut_height: p.cut_height,
            pierce_delay: p.pierce_delay,
            auto_feed: p.auto_feed,
            reversed: p.reversed,
        }
    }

    fn cut(&self) -> PlasmaCut {
        PlasmaCut {
            kerf: self.kerf,
            control_comp: self.control_comp,
            pierce_height: self.pierce_height,
            cut_height: self.cut_height,
            pierce_delay: self.pierce_delay,
            auto_feed: self.auto_feed,
            reversed: self.reversed,
            torches: Vec::new(),
        }
    }

    /// The canonical line: the `.cl` header without its `; `.
    #[must_use]
    pub fn line(&self) -> String {
        let h = self.cut().header_line();
        format!("{}\n", h.trim_start_matches("; "))
    }

    /// The properties the header publishes for the section (the engine
    /// keeps them until the section ends). An unstated height or delay
    /// is ABSENT, never 0 - absence is "the controller's AutoFS".
    #[must_use]
    pub fn props(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("PlasmaKerf".into(), json!(self.kerf));
        m.insert("PlasmaCompensation".into(), json!(if self.control_comp { "control" } else { "computer" }));
        if let Some(h) = self.pierce_height {
            m.insert("PierceHeight".into(), json!(h));
        }
        if let Some(h) = self.cut_height {
            m.insert("CutHeight".into(), json!(h));
        }
        if let Some(d) = self.pierce_delay {
            m.insert("PierceDelay".into(), json!(d));
        }
        m.insert("PlasmaFeed".into(), json!(if self.auto_feed { "auto" } else { "stated" }));
        m.insert("PlasmaReversed".into(), json!(self.reversed));
        m
    }

    #[must_use]
    pub fn label(&self) -> String {
        format!("plasma · kerf {:.3} mm{}", self.kerf, if self.control_comp { " · G41/G42" } else { "" })
    }
}

/// The keys [`PlasmaRec::props`] may set - cleared at the section end.
pub const KEYS: [&str; 7] =
    ["PlasmaKerf", "PlasmaCompensation", "PierceHeight", "CutHeight", "PierceDelay", "PlasmaFeed", "PlasmaReversed"];

impl TorchRec {
    #[must_use]
    pub fn on(t: &TorchSpan) -> Self {
        TorchRec { contour: t.contour, on: Some((t.role, t.parent, t.small)) }
    }

    #[must_use]
    pub fn off(contour: u32) -> Self {
        TorchRec { contour, on: None }
    }

    fn span(&self) -> Option<TorchSpan> {
        self.on.map(|(role, parent, small)| TorchSpan { contour: self.contour, role, parent, on: 0, off: 0, small })
    }

    /// The canonical line: the `.cl` record without its `; `.
    #[must_use]
    pub fn line(&self) -> String {
        let l = match self.span() {
            Some(s) => s.on_line(),
            None => TorchSpan { contour: self.contour, role: ContourRole::Open, parent: None, on: 0, off: 0, small: None }
                .off_line(),
        };
        format!("{}\n", l.trim_start_matches("; "))
    }

    #[must_use]
    pub fn props(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("Contour".into(), json!(self.contour));
        if let Some((role, parent, small)) = self.on {
            m.insert("command".into(), json!("TorchOn"));
            m.insert("ContourRole".into(), json!(role.word().to_ascii_lowercase()));
            if let Some(p) = parent {
                m.insert("ParentContour".into(), json!(p));
            }
            if let Some(f) = small {
                m.insert("SmallHole".into(), json!(f));
            }
        } else {
            m.insert("command".into(), json!("TorchOff"));
        }
        m
    }

    #[must_use]
    pub fn label(&self) -> String {
        match self.on {
            Some((role, _, _)) => format!("torch on · C{} {}", self.contour, role.word().to_ascii_lowercase()),
            None => format!("torch off · C{}", self.contour),
        }
    }
}

/// Parse the words after `PLASMA` (the `.cnc` record or the `.cl`
/// comment): `cl`'s own reader, then the sanity the dump's door keeps.
pub fn parse_header(words: &str) -> Result<PlasmaRec, String> {
    let p = crate::cl::parse_plasma_header(words)?;
    let ok = |v: Option<f64>| v.is_none_or(|x| x.is_finite() && x >= 0.0);
    if !(p.kerf.is_finite() && p.kerf > 0.0) {
        return Err("PLASMA KERF must be positive".into());
    }
    if !ok(p.pierce_height) || !ok(p.cut_height) || !ok(p.pierce_delay) {
        return Err("PLASMA heights and delay must not be negative".into());
    }
    Ok(PlasmaRec::from_cut(&p))
}

/// Parse the words after `TORCH`: `ON C<id> <ROLE> ...` or `OFF C<id>`.
pub fn parse_torch(words: &str) -> Result<TorchRec, String> {
    let w = words.trim_start();
    if let Some(rest) = w.strip_prefix("ON ") {
        let t = crate::cl::parse_torch_on(rest, 0)?;
        if t.small.is_some_and(|f| !(f.is_finite() && f > 0.0 && f <= 1.0)) {
            return Err("TORCH SMALL must be a feed factor in (0, 1] (the SMALL word is that factor x 1000)".into());
        }
        return Ok(TorchRec::on(&t));
    }
    if let Some(rest) = w.strip_prefix("OFF") {
        return Ok(TorchRec::off(crate::cl::parse_torch_off(rest)?));
    }
    Err(format!("TORCH needs ON or OFF, found {w:?}"))
}

/// The engine's plasma state for one run: the header in force and the
/// lit contour. The laws the stream's records must keep - a torch with
/// no header, an ON while lit, an OFF that names another contour or
/// none, a section left lit, a rapid while lit - are counted refusals
/// by name, never repaired: a torch left burning across a rapid cuts
/// through the part.
#[derive(Default)]
pub struct TorchLaw {
    header: bool,
    lit: Option<u32>,
}

fn who(base: &BTreeMap<String, Value>) -> String {
    match base.get("OperationName").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        Some(op) => format!("operation {op}: "),
        None => String::new(),
    }
}

impl TorchLaw {
    /// A `PLASMA` header: publish it (the caller merges `props`) and
    /// refuse a script that cannot voice a torch at all.
    pub fn header(&mut self, s: &Script, base: &BTreeMap<String, Value>, warnings: &mut Vec<String>) {
        if self.header {
            warnings.push(format!("{}plasma: a second PLASMA header in one section - refused", who(base)));
        }
        self.header = true;
        if !(s.hook_exists("onTorchOn") && s.hook_exists("onTorchOff")) {
            warnings.push(format!(
                "{}is plasma cutting (a torch struck per contour), but this post script does not define onTorchOn / onTorchOff - use a plasma post",
                who(base)
            ));
        }
    }

    /// A `TORCH` record: true when the hook may fire.
    pub fn torch(&mut self, t: &TorchRec, base: &BTreeMap<String, Value>, warnings: &mut Vec<String>) -> bool {
        let w = who(base);
        if !self.header {
            warnings.push(format!("{w}plasma: TORCH C{} with no PLASMA header in force - refused", t.contour));
            return false;
        }
        match (t.on.is_some(), self.lit) {
            (true, Some(c)) => {
                warnings.push(format!("{w}plasma: C{} is struck while C{c} is still lit - refused", t.contour));
                false
            }
            (true, None) => {
                self.lit = Some(t.contour);
                true
            }
            (false, Some(c)) if c == t.contour => {
                self.lit = None;
                true
            }
            (false, lit) => {
                warnings.push(format!(
                    "{w}plasma: TORCH OFF C{} but {} - refused",
                    t.contour,
                    lit.map_or_else(|| "no torch is lit".to_owned(), |c| format!("C{c} is the lit contour"))
                ));
                false
            }
        }
    }

    /// A rapid: never while lit.
    pub fn rapid(&self, base: &BTreeMap<String, Value>, warnings: &mut Vec<String>) {
        if let Some(c) = self.lit {
            warnings.push(format!("{}plasma: a rapid move while C{c} is lit - refused", who(base)));
        }
    }

    /// The section ends: nothing may stay lit, and the header retires.
    pub fn section_end(&mut self, base: &mut BTreeMap<String, Value>, warnings: &mut Vec<String>) {
        if let Some(c) = self.lit.take() {
            warnings.push(format!("{}plasma: C{c} is never put out - refused", who(base)));
        }
        self.header = false;
        for k in KEYS {
            base.remove(k);
        }
    }

    /// The stream ends: a torch still lit there is the same refusal.
    pub fn close(&mut self, base: &mut BTreeMap<String, Value>, warnings: &mut Vec<String>) {
        if self.lit.is_some() {
            self.section_end(base, warnings);
        }
    }
}

/// Fire the torch hook for a record the law admitted.
pub fn fire(s: &mut Script, t: &TorchRec) -> Result<(), PostError> {
    s.call(if t.on.is_some() { "onTorchOn" } else { "onTorchOff" }, "")?;
    Ok(())
}
