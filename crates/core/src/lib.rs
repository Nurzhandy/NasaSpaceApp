//! `astro_core` — the rules of "Junior Astronaut: Outpost" (2D). Pure Rust, no Bevy.
//!
//! THIS FILE IS THE CONTRACT between part A (rules, `crates/core`) and part B (Bevy app,
//! `crates/game`). Change it only in a small PR that both teammates review.
//!
//! Conventions:
//! * Coordinates are grid cells. Modules sit on integer cells (`x`, `y` = bottom-left cell);
//!   players, sites and the crewmate use `f32` cell units. `y` grows **up** (north).
//! * Time is seconds (`f32`). No colors are stored here.
//! * Every text a child sees comes from [`i18n`] (RU/EN).
//!
//! The crate is named `astro_core` (not `core`) because `core` clashes with Rust's built-in crate.
//! Until part A finishes the real implementation, [`Game`] is the stand-in [`mock::Game`].

pub mod catalog;
pub mod i18n;
pub mod mock;
pub mod profile;
pub mod weather;

pub use catalog::*;
pub use i18n::Lang;
pub use profile::Profile;
pub use weather::{Weather, WeatherEvent};

/// The game implementation used by the app. Part A: point this at the real `Game` when ready.
pub use mock::Game;

pub type ModuleId = u32;
pub type SiteId = u32;
pub type FeelingId = usize;
pub type MoodId = usize;

#[derive(Clone, Copy, Debug)]
pub struct MissionConfig {
    pub map: MapId,
    pub cadet: bool,
    pub coop: bool,
    pub live: bool,
    pub seed: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModuleKind {
    Habitat,
    Solar,
    Battery,
    Oxygen,
    Greenhouse,
    Shield,
    Lab,
    Antenna,
}

impl ModuleKind {
    /// What the child can place in build mode (the habitat is pre-built).
    pub const BUILDABLE: [ModuleKind; 7] = [
        ModuleKind::Solar,
        ModuleKind::Battery,
        ModuleKind::Oxygen,
        ModuleKind::Greenhouse,
        ModuleKind::Shield,
        ModuleKind::Lab,
        ModuleKind::Antenna,
    ];
}

/// Static description of a module type (values owned by part A).
#[derive(Clone, Copy, Debug)]
pub struct ModuleSpec {
    pub kind: ModuleKind,
    /// footprint in cells
    pub w: i32,
    pub h: i32,
    /// materials needed to build
    pub materials: u32,
    /// how many power blocks it can take (0 = needs no power, e.g. solar, battery)
    pub max_power: u8,
    /// icon name in `assets/icons`
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    pub what_ru: &'static str,
    pub what_en: &'static str,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub id: ModuleId,
    pub kind: ModuleKind,
    pub x: i32,
    pub y: i32,
    /// power blocks currently assigned
    pub power: u8,
    pub broken: bool,
    pub leaking: bool,
    /// solar panel covered by dust (repair = clean)
    pub dusty: bool,
    /// greenhouse has a harvest ready
    pub ripe: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SiteKind {
    /// rock sample → science at the lab
    Sample,
    /// water ice → oxygen
    Ice,
    /// old debris → building materials
    Wreck,
}

#[derive(Clone, Debug)]
pub struct Site {
    pub id: SiteId,
    pub kind: SiteKind,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerState {
    pub x: f32,
    pub y: f32,
    /// samples carried
    pub carried: u32,
    pub in_shelter: bool,
}

#[derive(Clone, Debug)]
pub struct FlareInfo {
    pub label_ru: String,
    pub label_en: String,
    pub date: String,
    /// true = recent event from the live DONKI API, false = archive
    pub live: bool,
    /// remaining fraction 0..1
    pub left: f32,
}

/// A Mission Control radio message. It is created when sent and shown when `arrives_in <= 0`
/// (the comms delay of the map).
#[derive(Clone, Debug)]
pub struct Radio {
    pub icon: &'static str,
    pub text_ru: String,
    pub text_en: String,
    pub arrives_in: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Playing,
    Won,
    Lost(Cause),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    Oxygen,
    Food,
    Dose,
}

#[derive(Clone, Copy, Debug)]
pub struct SolRecord {
    pub sol: u32,
    pub o2: f32,
    pub food: f32,
    pub dose: f32,
    pub science: f32,
}

/// Everything the app needs to draw. Read-only for part B.
#[derive(Clone, Debug)]
pub struct State {
    pub map: MapId,
    pub cadet: bool,
    pub coop: bool,
    pub sol: u32,
    pub sols: u32,
    /// progress through the current sol, 0..1
    pub day_t: f32,
    /// sunlight 0..1 (night and dust lower it)
    pub daylight: f32,
    pub night: bool,
    pub dust_storm: bool,
    /// no Mission Control forecasts (solar conjunction)
    pub silence: bool,
    // resources, percent unless noted
    pub o2: f32,
    pub food: f32,
    pub dose: f32,
    pub science: f32,
    pub goal: f32,
    pub morale: f32,
    pub battery: f32,
    pub battery_max: f32,
    pub materials: u32,
    /// power blocks produced / assigned this moment
    pub power_gen: u32,
    pub power_used: u32,
    // world
    pub world_w: i32,
    pub world_h: i32,
    /// center of the base and the radius (cells) where building is allowed
    pub base_x: i32,
    pub base_y: i32,
    pub base_r: i32,
    pub modules: Vec<Module>,
    pub sites: Vec<Site>,
    pub players: Vec<PlayerState>,
    /// radius (cells) protected by a powered shield, 0 if none
    pub shield_radius: f32,
    pub flare: Option<FlareInfo>,
    pub radio: Option<Radio>,
    pub harvest_ready: bool,
    /// landed cargo pod position
    pub cargo: Option<(f32, f32)>,
    /// crewmate needing support
    pub crew_mood: Option<MoodId>,
    pub crew_pos: (f32, f32),
    pub status: Status,
    pub history: Vec<SolRecord>,
}

#[derive(Clone, Debug)]
pub struct MissionResult {
    pub map: MapId,
    pub won: bool,
    pub cause: Option<Cause>,
    pub stars: u32,
    pub points: u32,
    pub sol: u32,
    pub sols: u32,
    pub science: f32,
    pub goal: f32,
    pub dose: f32,
    pub coop: bool,
    pub live: bool,
    pub history: Vec<SolRecord>,
    /// badge ids earned during the mission (the profile decides which are new)
    pub badges: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub enum Command {
    Build { kind: ModuleKind, x: i32, y: i32 },
    Demolish { x: i32, y: i32 },
    SetPower { module: ModuleId, blocks: u8 },
    /// repair a broken module, patch a leak or clean a dusty panel (after "hold F 3 s")
    Repair { module: ModuleId },
    PickSample { site: SiteId },
    DeliverSamples,
    EnterShelter,
    ExitShelter,
    /// true = eat, false = send to the lab
    ChooseHarvest(bool),
    /// true = unload everything, false = essentials only
    ChooseCargo(bool),
    NameFeeling(FeelingId),
    Support(usize),
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    Flare { label_ru: String, label_en: String, date: String, live: bool },
    FlareEnded,
    NewSol(u32),
    ModuleBroken(ModuleId),
    Leak(ModuleId),
    DustStorm,
    HarvestReady,
    CargoLanding,
    CrewNeedsSupport(MoodId),
    ForecastArrived(Radio),
    Built(ModuleId),
    Demolished,
    Repaired(ModuleId),
    Picked(SiteKind),
    SamplesDelivered { science: f32 },
    FeelingChecked { ok: bool },
    Supported,
    BadgeEarned(&'static str),
    MissionEnded(MissionResult),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Refusal {
    NotEnoughPower,
    NotEnoughMaterials,
    CellBusy,
    TooFar,
    OutsideBase,
    NeedsModule(ModuleKind),
    LabNeedsPower,
    CannotDemolish,
    NothingToDo,
    MaxPower,
}

/// The API part B relies on. Implemented by `mock::Game` now and by the real game later.
pub trait GameApi {
    fn new(cfg: MissionConfig, weather: &Weather) -> Self;
    /// Advance time by `dt` seconds; returns what happened (sounds, animations, alerts).
    fn tick(&mut self, dt: f32) -> Vec<GameEvent>;
    /// A player action. The error explains to the child why it is not possible.
    fn apply(&mut self, player: u8, cmd: Command) -> Result<Vec<GameEvent>, Refusal>;
    /// Dry run of `apply` for previews (green/red build ghost). Must not change state.
    fn check(&self, player: u8, cmd: &Command) -> Result<(), Refusal>;
    /// Report where a player stands (the app moves players; the rules need positions
    /// for distance checks and radiation exposure).
    fn set_player(&mut self, player: u8, x: f32, y: f32);
    fn state(&self) -> &State;
}
