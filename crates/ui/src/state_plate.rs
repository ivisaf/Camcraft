//! Fixture plate state (#819): the "Add fixture plate…" dialog's draft,
//! each fixture row's "Place on plate" draft (by fixture name, like the
//! vise rows' opening drafts), and the handlers their commands land
//! through - `InsertPlate`, `PlaceOnPlate` and `SetPlateGrid`, each one
//! undo unit. Drafts are view state, never history.
//! Provenance: our own dialog state over our own plate commands.

use camcraft_app::commands::{InsertPlate, PlaceOnPlate, PlatePlacement, SetPlateGrid};
use camcraft_app::fixture;
use camcraft_app::geometry::{Point3, quat_from_euler};
use camcraft_app::model;
use camcraft_app::plate::{self, Counterbore, Dowels, MountAt, PlateParams};
use camcraft_app::schema::Location;

use super::{ErrorScope, FixtureLibDialog, State};

/// "Add fixture plate…": the preset, its (editable) figures, the
/// optional dowel pattern and counterbores, and where the plate goes.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateDraft {
    /// The Fixtures node's setup (`""` - the implicit setup).
    pub setup: String,
    /// Index into `plate::PRESETS`.
    pub preset: usize,
    pub params: PlateParams,
    /// The dowel and counterbore figures, kept while switched off.
    pub dowels_on: bool,
    pub dowels: Dowels,
    pub counterbore_on: bool,
    pub counterbore: Counterbore,
    /// Centred on the machine table, lying on it.
    pub on_table: bool,
    /// The plate frame's origin (its corner on the top face), mm.
    pub pos: [f64; 3],
    pub yaw_deg: f64,
    pub error: Option<String>,
}

impl PlateDraft {
    /// A fresh draft on preset `preset`.
    #[must_use]
    pub fn new(setup: String, preset: usize, on_table: bool) -> Self {
        let mut d = Self {
            setup,
            preset: 0,
            params: PlateParams {
                size: String::new(),
                length: 0.0,
                width: 0.0,
                thickness: 0.0,
                pitch_x: 0.0,
                pitch_y: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
                thread: String::new(),
                thread_diameter: 0.0,
                dowels: None,
                counterbore: None,
            },
            dowels_on: false,
            dowels: Dowels { pitch_x: 50.0, pitch_y: 50.0, offset_x: 25.0, offset_y: 25.0, diameter: 10.0 },
            counterbore_on: false,
            counterbore: Counterbore { diameter: 16.0, depth: 10.0 },
            on_table,
            pos: [0.0; 3],
            yaw_deg: 0.0,
            error: None,
        };
        d.pick(preset);
        d
    }

    /// Pick preset `preset`: its figures replace the draft's.
    pub fn pick(&mut self, preset: usize) {
        let Some(row) = plate::PRESETS.get(preset) else { return };
        self.preset = preset;
        self.params = row.params();
        self.dowels_on = self.params.dowels.is_some();
        if let Some(d) = self.params.dowels.clone() {
            self.dowels = d;
        }
        self.counterbore_on = self.params.counterbore.is_some();
        if let Some(c) = self.params.counterbore.clone() {
            self.counterbore = c;
        }
        self.error = None;
    }

    /// The parameters Place sends: the switches decide the options.
    #[must_use]
    pub fn landed(&self) -> PlateParams {
        PlateParams {
            dowels: self.dowels_on.then(|| self.dowels.clone()),
            counterbore: self.counterbore_on.then(|| self.counterbore.clone()),
            ..self.params.clone()
        }
    }
}

/// One fixture row's "Place on plate" draft.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlatePlaceDraft {
    /// The plate picked (a fixture name).
    pub plate: String,
    /// On a dowel pair instead of a hole.
    pub dowels: bool,
    pub row: u32,
    pub col: u32,
    pub dowel_a: String,
    pub dowel_b: String,
    pub yaw_deg: f64,
}

impl PlatePlaceDraft {
    /// Where the draft puts the fixture.
    #[must_use]
    pub fn at(&self) -> MountAt {
        if self.dowels {
            MountAt::Dowels([self.dowel_a.clone(), self.dowel_b.clone()])
        } else {
            MountAt::Hole(plate::location_name(self.row, self.col))
        }
    }
}

impl State {
    /// The plates of the document, by name (`except` left out).
    #[must_use]
    pub fn plate_names(&self, except: &str) -> Vec<String> {
        model::named_entities(&self.world)
            .into_iter()
            .filter(|(n, e)| n != except && fixture::def_of(&self.world, e).is_some_and(|d| d.plate.plate.is_some()))
            .map(|(n, _)| n)
            .collect()
    }

    /// Fixture `name`'s place-on-plate draft, made on first use.
    pub fn plate_place_draft(&mut self, name: &str) -> &mut PlatePlaceDraft {
        self.plate_place.entry(name.to_owned()).or_insert_with(|| PlatePlaceDraft {
            dowel_a: "A1".into(),
            dowel_b: "A2".into(),
            ..PlatePlaceDraft::default()
        })
    }

    /// Drop the place drafts of fixtures that are gone.
    pub fn retire_plate_place_drafts(&mut self) {
        let world = &self.world;
        self.plate_place.retain(|name, _| model::find_by_name(world, name).is_some());
    }

    /// Open "Add fixture plate…" over `setup`'s Fixtures node: the inch
    /// preset for an inch document, 300 mm otherwise; on the table when
    /// the setup's machine states one.
    pub fn open_plate_dialog(&mut self, setup: String) {
        let want = if self.units() == camcraft_app::units::Unit::Inches { "12 x 12 in" } else { "300 x 300 mm" };
        let preset = plate::PRESETS.iter().position(|p| p.label == want).unwrap_or(0);
        let declared = (!setup.is_empty()).then_some(setup.as_str());
        let on_table = fixture::table_box(&self.world, declared).is_some();
        self.fixture_lib = Some(FixtureLibDialog::Plate(PlateDraft::new(setup, preset, on_table)));
    }

    /// `CommandKind::InsertFixturePlate`: one `InsertPlate`. Closes the
    /// dialog on success; a refusal is named in it and in the status.
    pub(super) fn insert_fixture_plate(&mut self, setup: Option<String>, params: &PlateParams, on_table: bool, pos: [f64; 3], yaw_deg: f64) {
        let placement = if on_table {
            PlatePlacement::OnTable
        } else {
            PlatePlacement::At(Location {
                pos: Point3::new(pos[0], pos[1], pos[2]),
                rot: quat_from_euler(0.0, 0.0, yaw_deg.to_radians()),
            })
        };
        match InsertPlate::new(&self.world, params, setup.as_deref(), placement) {
            Ok(command) => {
                let name = command.name().to_owned();
                self.history.execute(&self.world, command);
                self.doc_version += 1;
                self.status = format!("placed fixture plate {name}");
                self.fixture_lib = None;
            }
            Err(err) => {
                let msg = format!("plate not placed: {err}");
                if let Some(FixtureLibDialog::Plate(d)) = self.fixture_lib.as_mut() {
                    d.error = Some(msg.clone());
                }
                self.refuse(ErrorScope::Setup, msg);
            }
        }
    }

    /// `CommandKind::PlaceOnPlate`: one `PlaceOnPlate`.
    pub(super) fn place_fixture_on_plate(&mut self, fixture: &str, plate: &str, at: MountAt, yaw_deg: f64) {
        let label = at.label();
        match PlaceOnPlate::new(&self.world, fixture, plate, at, yaw_deg) {
            Ok(command) => {
                self.history.execute(&self.world, command);
                self.doc_version += 1;
                self.status = format!("{fixture} placed at {label} on {plate}");
            }
            Err(err) => self.refuse(ErrorScope::Setup, format!("not placed on the plate: {err}")),
        }
    }

    /// `CommandKind::DetectPlateGrid`: one `SetPlateGrid` (detect, or clear).
    pub(super) fn detect_plate_grid(&mut self, name: &str, clear: bool) {
        let planned = if clear { SetPlateGrid::clear(&self.world, name) } else { SetPlateGrid::detect(&self.world, name) };
        match planned {
            Ok(command) => {
                let what = command.plate().map(|d| d.holes.extent());
                self.history.execute(&self.world, command);
                self.doc_version += 1;
                self.status = match what {
                    Some(e) => format!("{name}: a fixture plate, {e}"),
                    None => format!("{name}: no longer a fixture plate"),
                };
            }
            Err(err) => self.refuse(ErrorScope::Setup, format!("no plate grid: {err}")),
        }
    }
}
