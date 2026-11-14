//! The outpost: a grid base built from modules, a real-time resource simulation, a big map with
//! far regions, story tasks, a finale "boss" event, and game modes. Pure data — the Bevy app
//! (part B) only reads `state()`, sends `Command`s and reacts to `GameEvent`s.
//!
//! Units: time in real seconds (a sol is `SOL_LEN` s), positions in grid cells, rates "per sol".
//! The habitat sits at cells (-1..=1, -1..=1). Everything is serde-serializable so a host can send
//! `State` to other computers and receive their `Command`s (network co-op).
//!
//! Map layout (radius in cells from the habitat):
//! - `0..BUILD_R` — the base, modules can be built here;
//! - `..INNER_R` — open ground around the base;
//! - `INNER_R..WORLD_R` — three far regions, 120° each; region 0 needs the rover,
//!   region 1 the headlamp, region 2 the drill (see `data::regions`).

use crate::data::*;
use crate::rng::Rng;
use crate::survival::{build_cost, Body, EventReport, Survival, Tonight, Upgrade};
use crate::weather::{Weather, WeatherEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SOL_LEN: f32 = 100.0;
pub const O2_USE: f32 = 12.0;
pub const O2_PER: f32 = 6.0;
pub const FOOD_USE: f32 = 10.0;
pub const FOOD_PER: f32 = 5.0;
pub const SHIELD_PER: f32 = 0.3;
/// total shield blocks that count (−90%)
pub const SHIELD_CAP: u32 = 3;
pub const RTG_YIELD: f32 = 2.0;
pub const HABITAT_BATTERY: f32 = 5.0;
pub const BATTERY_PER_MODULE: f32 = 5.0;
/// how close (cells) a player must be to use a module or a site
pub const NEAR: f32 = 2.5;
pub const CARRY_MAX: u32 = 5;
/// modules can be built within this many cells of the habitat centre
pub const BUILD_R: i32 = 11;
/// open ground ends here; far regions start
pub const INNER_R: f32 = 40.0;
/// edge of the map (cells)
pub const WORLD_R: f32 = 115.0;
/// where the cargo ship lands and where players start (in front of the habitat door)
pub const CARGO_PAD: (f32, f32) = (0.5, 7.5);
pub const DOOR: (f32, f32) = (0.5, 2.5);
/// finds on the map are topped up to at most this many
pub const MAX_SITES: usize = 70;
/// finale waves: length and pause between them (fractions of a sol)
pub const WAVE_LEN: f32 = 0.2;
pub const WAVE_GAP: f32 = 0.05;

pub type ModuleId = u32;
pub type SiteId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// story tasks, far regions open step by step, a finale on the last sol
    Story,
    /// free building: no losing, everything open, no time limit (end with `Game::end_now`)
    Sandbox,
    /// straight to the finale with a ready base
    Challenge,
    /// the campaign on the Moon after landing: evening threats with a forecast, heat, upgrades,
    /// the astronauts' own stats (see `survival.rs`)
    Survival,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionConfig {
    pub map: MapId,
    /// age mode "Cadet 7–9": no food, accurate forecasts, lower science goal
    pub cadet: bool,
    pub coop: bool,
    /// use live DONKI events when the weather data is live
    pub live: bool,
    pub seed: u64,
}

impl MissionConfig {
    pub fn new(map: MapId) -> Self {
        Self { map, cadet: false, coop: false, live: true, seed: Rng::time_seed() }
    }
}

/// Everything a player can do. `player` is 0 or 1 (co-op).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Build { kind: ModuleKind, x: i32, y: i32 },
    Demolish { x: i32, y: i32 },
    SetPower { module: ModuleId, blocks: u8 },
    /// after holding the action key for ~3 s next to a broken generator or the leaking habitat
    Repair { module: ModuleId },
    /// after holding the action key next to a dusty solar panel
    Clean { module: ModuleId },
    /// pick up a sample / ore / wreck parts / ice at a site
    Collect { site: SiteId },
    DeliverSamples,
    EnterShelter,
    ExitShelter,
    /// true = eat together, false = send to the lab
    ChooseHarvest(bool),
    /// true = unload everything (costs battery), false = take only food
    ChooseCargo(bool),
    NameFeeling(usize),
    Support(usize),
    /// survival: buy the next level of an upgrade with coins
    Upgrade(Upgrade),
    /// the Space Beacon breathing calmed the astronaut's heartbeat
    Calm,
}

/// Why a command was refused. Show `crate::i18n::refusal_msg` to the child.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    MissionOver,
    NotEnoughPower,
    NotEnoughCoins,
    CellBusy,
    OutsideBase,
    EdgeOfMap,
    NotAvailableHere,
    OnlyOne,
    CannotDemolish,
    NoModuleThere,
    NoPowerSlot,
    TooFar,
    InsideShelter,
    NothingToRepair,
    NothingToClean,
    HandsFull,
    NothingToDeliver,
    NeedsModule(ModuleKind),
    /// the upgrade is already at the top level
    MaxLevel,
    NeedUnlock(Unlock),
    LabNeedsPower,
    NotReady,
    AlreadyInside,
    NotInside,
    NoMood,
    NameFeelingFirst,
    UnknownId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Forecast {
    Sure(EventKind),
    Maybe(EventKind),
    Night,
    Calm,
    /// the finale comes tomorrow
    Finale,
    /// the Sun is between Earth and the base (solar conjunction)
    NoSignal,
    /// the antenna has no power
    NoAntenna,
    /// survival: what comes this evening
    Tonight(Tonight),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cause {
    Oxygen,
    Food,
    Dose,
    /// survival: the habitat froze
    Cold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Playing,
    Won,
    Lost(Cause),
}

/// Something happened. Use `crate::i18n::event_msg` for the child-facing text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    NewSol(u32),
    Night,
    /// Mission Control sent a forecast; it arrives after the comms delay
    ForecastSent { arrives_in: f32 },
    ForecastArrived(Forecast),
    Flare { label_ru: String, label_en: String, date: String, live: bool },
    FlareEnded,
    DustStorm,
    DustGone,
    ModuleBroken(ModuleId),
    Leak(ModuleId),
    HarvestReady,
    /// new finds appeared on the map
    Discovery(Vec<SiteId>),
    CargoLanding { x: f32, y: f32 },
    CrewNeedsSupport(usize),
    SignalLost,
    Built { module: ModuleId, kind: ModuleKind },
    Demolished { kind: ModuleKind, refund: u32 },
    Repaired(ModuleId),
    Cleaned(ModuleId),
    Collected { site: SiteId, kind: SiteKind, region: Option<u8> },
    SamplesDelivered { count: u32, science: f32 },
    Sheltered { player: u8 },
    LeftShelter { player: u8 },
    Harvested { eat: bool },
    CargoUnloaded { all: bool },
    FeelingRight(usize),
    FeelingTryAgain(usize),
    Supported(usize),
    LowOxygen,
    BatteryEmpty,
    Unlocked(Unlock),
    QuestStarted(usize),
    QuestDone { quest: usize, coins: u32, unlock: Option<Unlock> },
    BossWave { wave: u8, of: u8, kind: Wave },
    WaveCleared { wave: u8, countered: bool },
    BossDefeated { countered: u8, of: u8 },
    /// survival: the evening threat starts
    Threat(Tonight),
    /// survival: the evening is over — what happened and why
    ThreatOver(EventReport),
    Upgraded { kind: Upgrade, level: u8 },
    MeteorHit { module: ModuleId, damage: f32 },
    /// a suit tank is low / empty (the astronaut is brought back to the airlock)
    SuitLow { player: u8 },
    SuitEmpty { player: u8 },
    Tired { player: u8 },
    Calmed { player: u8 },
    MissionEnded(MissionResult),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub kind: ModuleKind,
    /// top-left cell
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub power: u8,
    pub broken: bool,
    pub dusty: bool,
    /// structural strength %, meteorites lower it (survival)
    #[serde(default = "full_hp")]
    pub hp: f32,
}

fn full_hp() -> f32 {
    100.0
}

impl Module {
    pub fn covers(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
    pub fn center(&self) -> (f32, f32) {
        (self.x as f32 + self.w as f32 / 2.0, self.y as f32 + self.h as f32 / 2.0)
    }
    /// distance from a point to the module's rectangle (0 inside)
    pub fn dist(&self, p: (f32, f32)) -> f32 {
        let dx = (self.x as f32 - p.0).max(p.0 - (self.x + self.w) as f32).max(0.0);
        let dy = (self.y as f32 - p.1).max(p.1 - (self.y + self.h) as f32).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub id: SiteId,
    pub kind: SiteKind,
    pub x: f32,
    pub y: f32,
    /// far region index, `None` = open ground near the base
    pub region: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub pos: (f32, f32),
    pub inside: bool,
    pub carried: u32,
    /// suit oxygen, fatigue, suit condition, heartbeat (survival)
    #[serde(default)]
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flare {
    pub event: WeatherEvent,
    pub live: bool,
    /// fraction of a sol left
    pub t_left: f32,
    pub duration: f32,
    /// total dose % if fully exposed for the whole flare (before shield)
    pub spike: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrewMood {
    pub mood: usize,
    pub named: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct McMessage {
    pub forecast: Forecast,
    pub sent_at: f32,
    pub arrive_at: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolRecord {
    pub sol: u32,
    pub o2: f32,
    pub food: f32,
    pub dose: f32,
    pub science: f32,
}

/// The finale in progress. Show `wave`/`of` and `t_left` as the "boss bar".
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BossState {
    /// current wave, 1-based
    pub wave: u8,
    pub of: u8,
    pub kind: Wave,
    /// false during the pause between waves
    pub active: bool,
    /// fraction of a sol left in this wave (or pause)
    pub t_left: f32,
    /// waves the crew handled the right way so far
    pub countered: u8,
    sheltered: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flags {
    pub sheltered_in_flare: bool,
    pub storm: bool,
    pub night: bool,
    pub helped: bool,
    pub flares: u32,
    pub repairs: u32,
    pub built: u32,
    pub collected: u32,
    pub regions_visited: [bool; 3],
    /// survival: evening threats met / handled the right way
    pub threats: u32,
    pub countered: u32,
}

/// Snapshot for the debrief and for `progress::Profile::finish`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionResult {
    pub map: MapId,
    pub mode: Mode,
    pub won: bool,
    pub cause: Option<Cause>,
    pub stars: u32,
    pub points: u32,
    pub sol: u32,
    pub sols: u32,
    pub science: f32,
    pub goal: f32,
    pub dose: f32,
    pub history: Vec<SolRecord>,
    pub cadet: bool,
    pub coop: bool,
    pub live: bool,
    pub flags: Flags,
    pub samples_delivered: u32,
    pub quests_done: u32,
    pub boss_done: bool,
    pub boss_countered: u8,
}

/// Everything the app needs to draw. Read-only for part B.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub map: MapId,
    pub mode: Mode,
    pub cadet: bool,
    pub coop: bool,
    pub live: bool,
    pub sol: u32,
    pub sols: u32,
    /// progress inside the current sol, 0..1 (sunrise → sunset)
    pub t: f32,
    pub elapsed: f32,
    pub paused: bool,
    pub o2: f32,
    pub food: f32,
    pub dose: f32,
    pub morale: f32,
    pub science: f32,
    pub goal: f32,
    pub battery: f32,
    pub battery_cap: f32,
    pub coins: u32,
    /// power blocks per sol available right now / taken by modules
    pub supply: f32,
    pub demand: u32,
    pub modules: Vec<Module>,
    pub sites: Vec<Site>,
    pub players: Vec<Player>,
    /// tools that open far regions
    pub unlocks: Vec<Unlock>,
    pub flare: Option<Flare>,
    pub dust_left: u32,
    pub leak: bool,
    pub growth: f32,
    pub harvest_ready: bool,
    pub cargo: Option<(f32, f32)>,
    pub mood: Option<CrewMood>,
    /// last forecast that reached the base
    pub forecast: Option<Forecast>,
    /// a forecast still travelling from Earth
    pub mc: Option<McMessage>,
    pub silence_left: u32,
    pub samples_delivered: u32,
    /// current story task (index into `data::STORY`; == len when all are done)
    pub quest: usize,
    pub quest_progress: u32,
    pub quests_done: u32,
    pub boss: Option<BossState>,
    pub boss_done: bool,
    pub boss_countered: u8,
    pub status: Status,
    pub history: Vec<SolRecord>,
    pub flags: Flags,
    /// heat, air tightness, upgrades, tonight's threat (simulated in `Mode::Survival`)
    #[serde(default)]
    pub surv: Survival,
}

/// Which far region a point is in (`None` = the base and the open ground around it).
pub fn region_at(x: f32, y: f32) -> Option<u8> {
    if x.hypot(y) < INNER_R {
        return None;
    }
    let a = y.atan2(x).rem_euclid(std::f32::consts::TAU);
    Some(((a / (std::f32::consts::TAU / 3.0)) as u8).min(2))
}

impl State {
    pub fn def(&self) -> &'static MapDef {
        map(self.map)
    }
    pub fn is_night(&self) -> bool {
        self.def().night_sols.contains(&self.sol)
    }
    /// 0..1 how much sun is up (for lighting)
    pub fn daylight(&self) -> f32 {
        if self.is_night() {
            0.0
        } else {
            let d = (self.t * std::f32::consts::PI).sin();
            if self.dust_left > 0 { d * 0.35 } else { d }
        }
    }
    pub fn module(&self, id: ModuleId) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == id)
    }
    pub fn module_at(&self, x: i32, y: i32) -> Option<&Module> {
        self.modules.iter().find(|m| m.covers(x, y))
    }
    pub fn habitat(&self) -> &Module {
        self.modules.iter().find(|m| m.kind == ModuleKind::Habitat).expect("the habitat always exists")
    }
    pub fn count(&self, kind: ModuleKind) -> usize {
        self.modules.iter().filter(|m| m.kind == kind).count()
    }
    pub fn powered(&self, kind: ModuleKind) -> u32 {
        self.modules.iter().filter(|m| m.kind == kind && !m.broken).map(|m| m.power as u32).sum()
    }
    pub fn has(&self, u: Unlock) -> bool {
        self.unlocks.contains(&u)
    }
    /// Can the astronauts go into far region `r`?
    pub fn region_open(&self, r: u8) -> bool {
        self.has(regions(self.map)[r.min(2) as usize].need)
    }
    /// Can an astronaut stand here? Use it to stop walking at a locked region or the map edge.
    pub fn can_walk(&self, x: f32, y: f32) -> Result<(), Refusal> {
        if x.hypot(y) > WORLD_R {
            return Err(Refusal::EdgeOfMap);
        }
        match region_at(x, y) {
            Some(r) if !self.region_open(r) => Err(Refusal::NeedUnlock(regions(self.map)[r as usize].need)),
            _ => Ok(()),
        }
    }
    /// fraction of radiation the shield removes (0..0.9)
    pub fn shield_red(&self) -> f32 {
        self.powered(ModuleKind::Shield).min(SHIELD_CAP) as f32 * SHIELD_PER
    }
    /// power blocks that can be handed out right now
    pub fn available(&self) -> u32 {
        self.supply.max(0.0).floor() as u32 + self.battery.max(0.0).floor() as u32
    }
    pub fn has_finale(&self) -> bool {
        self.mode != Mode::Sandbox
    }
    pub fn stars(&self) -> u32 {
        if self.status != Status::Won {
            return 0;
        }
        1 + (self.science >= self.goal) as u32 + (self.dose < 50.0) as u32
    }
    pub fn points(&self) -> u32 {
        let quests: u32 = STORY.iter().take(self.quests_done as usize).map(|q| q.reward.points).sum();
        if self.mode == Mode::Sandbox {
            return quests;
        }
        let per = [0, 20, 30, 40, 50, 60][self.def().tier as usize];
        let threats = self.surv.countered * 3;
        let base = match self.status {
            Status::Won => per * self.stars() + if self.live { 10 } else { 0 } + if self.coop { 10 } else { 0 },
            _ => 10 + (self.sol.saturating_sub(1) * 3).min(20),
        };
        base + quests + self.boss_countered as u32 * 5 + threats
    }
}

pub struct Game {
    pub(crate) s: State,
    pub(crate) rng: Rng,
    pub(crate) weather: Weather,
    /// survival: tonight's threat for every sol (index = sol)
    pub(crate) threat_plan: Vec<Option<Tonight>>,
    schedule: HashMap<u32, (EventKind, bool)>,
    false_alarm: Option<u32>,
    pending: Vec<EventKind>,
    next_id: u32,
    low_o2_warned: bool,
    battery_warned: bool,
    /// network guest: the state comes from the host, commands go back to it
    mirror: bool,
    outbox: Vec<(u8, Command)>,
}

impl Game {
    /// A story mission (the default).
    pub fn new(cfg: MissionConfig, weather: &Weather) -> Game {
        Self::with_mode(cfg, Mode::Story, weather)
    }

    /// A mission in a chosen mode: story, free building (sandbox) or the finale challenge.
    pub fn with_mode(cfg: MissionConfig, mode: Mode, weather: &Weather) -> Game {
        Self::with_players(cfg, mode, if cfg.coop { 2 } else { 1 }, weather)
    }

    /// Up to 4 astronauts (expeditions). `cfg.coop` is ignored; 2+ players count as co-op.
    pub fn with_players(cfg: MissionConfig, mode: Mode, players: u8, weather: &Weather) -> Game {
        let players_n = players.clamp(1, 4);
        let cfg = MissionConfig { coop: players_n > 1, ..cfg };
        let m = map(cfg.map);
        let mut rng = Rng::new(cfg.seed);
        let live = cfg.live && weather.live && cfg.map != MapId::Earth;
        let sols = match mode {
            Mode::Story | Mode::Survival => m.sols,
            Mode::Sandbox => 99,
            Mode::Challenge => 1,
        };
        // ---- event schedule (one event on most sols, never two of the same kind in a row);
        // the last sol of a story belongs to the finale
        let mut schedule = HashMap::new();
        let last_free = if mode == Mode::Sandbox { sols } else { sols.saturating_sub(1) };
        let mut last: Option<EventKind> = None;
        let mut used_once: Vec<EventKind> = vec![];
        for sol in 2..=last_free {
            if m.night_sols.contains(&sol) || !rng.chance(0.65) {
                continue;
            }
            // survival: threats come in the evening (survival.rs); the day keeps the small events
            let pool: Vec<EventKind> = if mode == Mode::Survival { m.events.iter().copied().filter(|k| matches!(k, EventKind::Harvest | EventKind::Emotion | EventKind::Supply | EventKind::Discovery)).collect() } else { m.events.to_vec() };
            let mut kind = pool[rng.below(pool.len())];
            let mut tries = 0;
            while (Some(kind) == last || (mode != Mode::Sandbox && used_once.contains(&kind))) && tries < 20 {
                kind = pool[rng.below(pool.len())];
                tries += 1;
            }
            if matches!(kind, EventKind::Dust | EventKind::Conjunction | EventKind::Supply) {
                used_once.push(kind);
            }
            let surprise = !cfg.cadet && kind == EventKind::Flare && rng.chance(0.2);
            schedule.insert(sol, (kind, surprise));
            last = Some(kind);
        }
        // a real high-risk week guarantees an early flare
        if live && last_free >= 2 && mode != Mode::Survival {
            let sol = 2 + rng.below(2.min(last_free as usize - 1)) as u32;
            if weather.risk() >= 2 || !schedule.contains_key(&sol) {
                schedule.insert(sol, (EventKind::Flare, false));
            }
        }
        let mut false_alarm = None;
        if !cfg.cadet && cfg.map != MapId::Earth && mode != Mode::Survival && rng.chance(0.5) && last_free >= 2 {
            let free: Vec<u32> = (2..=last_free).filter(|s| !schedule.contains_key(s) && !m.night_sols.contains(s)).collect();
            if !free.is_empty() {
                false_alarm = Some(free[rng.below(free.len())]);
            }
        }

        let players = (0..players_n).map(|i| Player { pos: (DOOR.0 - 1.2 + i as f32 * 0.8, DOOR.1 + 1.0), inside: false, carried: 0, body: Body::default() }).collect();
        let open_all = mode != Mode::Story;
        let s = State {
            map: cfg.map,
            mode: mode,
            cadet: cfg.cadet,
            coop: cfg.coop,
            live,
            sol: 1,
            sols,
            t: 0.0,
            elapsed: 0.0,
            paused: false,
            o2: 65.0,
            food: if cfg.cadet { 100.0 } else { 65.0 },
            dose: 0.0,
            morale: 70.0,
            science: 0.0,
            goal: (m.sols as f32 * if cfg.cadet { 0.8 } else { 1.2 }).round(),
            battery: m.start_battery,
            battery_cap: HABITAT_BATTERY,
            coins: match mode {
                Mode::Story => m.start_coins,
                Mode::Sandbox => 40,
                Mode::Challenge => m.start_coins + 10,
                Mode::Survival => m.start_coins + 8,
            },
            supply: 0.0,
            demand: 0,
            modules: vec![],
            sites: vec![],
            players,
            unlocks: if open_all { vec![Unlock::Vehicle, Unlock::Headlamp, Unlock::Drill] } else { vec![] },
            flare: None,
            dust_left: 0,
            leak: false,
            growth: 0.3,
            harvest_ready: false,
            cargo: None,
            mood: None,
            forecast: None,
            mc: None,
            silence_left: 0,
            samples_delivered: 0,
            quest: if mode == Mode::Challenge { STORY.len() } else { 0 },
            quest_progress: 0,
            quests_done: 0,
            boss: None,
            boss_done: false,
            boss_countered: 0,
            status: Status::Playing,
            history: vec![],
            flags: Flags::default(),
            surv: Survival::default(),
        };
        let mut g = Game { s, rng, weather: weather.clone(), threat_plan: vec![], schedule, false_alarm, pending: vec![], next_id: 1, low_o2_warned: false, battery_warned: false, mirror: false, outbox: vec![] };
        g.starting_base(cfg.cadet, mode == Mode::Challenge);
        g.starting_sites();
        if mode == Mode::Survival {
            g.plan_threats();
        }
        g.refresh_power();
        let _ = g.begin_sol();
        g
    }

    pub(crate) fn spawn_site_pub(&mut self, kind: SiteKind) -> SiteId {
        self.spawn_site(kind, None)
    }
    pub(crate) fn refresh_power_pub(&mut self) {
        self.refresh_power()
    }
    pub(crate) fn start_flare_pub(&mut self, event: WeatherEvent, live: bool, duration: f32, boost: f32) {
        self.start_flare(event, live, duration, boost)
    }

    pub fn state(&self) -> &State {
        &self.s
    }

    /// Network guest: show the host's state. From now on `tick` does nothing (the host simulates)
    /// and `apply` queues commands for the host instead of running them (see `take_outbox`).
    pub fn mirror(&mut self, state: State) {
        self.s = state;
        self.mirror = true;
    }
    pub fn is_mirror(&self) -> bool {
        self.mirror
    }
    /// Commands a network guest made since the last call, to send to the host.
    pub fn take_outbox(&mut self) -> Vec<(u8, Command)> {
        std::mem::take(&mut self.outbox)
    }

    /// Pause the world (menus, the Space Beacon, solo modal windows).
    pub fn set_paused(&mut self, paused: bool) {
        self.s.paused = paused;
    }

    /// Part B reports where each astronaut stands (cells). Needed for "walk up and act".
    pub fn set_player_pos(&mut self, player: u8, x: f32, y: f32) {
        if let Some(p) = self.s.players.get_mut(player as usize) {
            p.pos = (x, y);
        }
    }

    /// Finish now (the "go home" button in sandbox mode). Returns the result for the debrief.
    pub fn end_now(&mut self) -> MissionResult {
        if self.s.status == Status::Playing {
            self.s.status = Status::Won;
        }
        self.result()
    }

    fn add_module(&mut self, kind: ModuleKind, x: i32, y: i32, power: u8) -> ModuleId {
        let d = module_def(kind);
        let id = self.next_id;
        self.next_id += 1;
        self.s.modules.push(Module { id, kind, x, y, w: d.w, h: d.h, power, broken: false, dusty: false, hp: 100.0 });
        id
    }

    fn starting_base(&mut self, cadet: bool, ready: bool) {
        let m = self.s.def();
        self.add_module(ModuleKind::Habitat, -1, -1, 0);
        for i in 0..m.start_panels as i32 {
            self.add_module(ModuleKind::Solar, -4 + i * 2, -5, 0);
        }
        for i in 0..m.start_rtg as i32 {
            self.add_module(ModuleKind::Rtg, 5, -4 + i * 2, 0);
        }
        self.add_module(ModuleKind::Battery, -5, 0, 0);
        self.add_module(ModuleKind::OxygenGen, -5, 2, 2);
        self.add_module(ModuleKind::Greenhouse, 3, 3, if cadet { 0 } else { 2 });
        self.add_module(ModuleKind::Antenna, 4, 0, 1);
        if ready {
            // challenge: a shield and a second battery are already there
            self.add_module(ModuleKind::Shield, -7, -2, 0);
            self.add_module(ModuleKind::Battery, -7, 0, 0);
        }
    }

    fn spawn_site(&mut self, kind: SiteKind, region: Option<u8>) -> SiteId {
        let (a, r) = match region {
            None => (self.rng.range(0.0, std::f32::consts::TAU), self.rng.range(BUILD_R as f32 + 2.0, INNER_R - 2.0)),
            Some(i) => {
                let sector = std::f32::consts::TAU / 3.0;
                (self.rng.range(i as f32 * sector + 0.08, (i + 1) as f32 * sector - 0.08), self.rng.range(INNER_R + 3.0, WORLD_R - 4.0))
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        self.s.sites.push(Site { id, kind, x: a.cos() * r, y: a.sin() * r, region });
        id
    }

    fn spawn_region_find(&mut self, r: u8) -> SiteId {
        let finds = regions(self.s.map)[r as usize].finds;
        let kind = finds[self.rng.below(finds.len())];
        self.spawn_site(kind, Some(r))
    }

    fn starting_sites(&mut self) {
        for _ in 0..4 {
            self.spawn_site(SiteKind::Sample, None);
        }
        for _ in 0..5 {
            self.spawn_site(SiteKind::Ore, None);
        }
        self.spawn_site(SiteKind::Wreck, None);
        if self.s.def().ice {
            self.spawn_site(SiteKind::Ice, None);
        }
        for r in 0..3 {
            for _ in 0..6 {
                self.spawn_region_find(r);
            }
        }
    }

    /// New finds every sol: the robot BEEP scans, meteorites fall. The map never runs dry.
    fn top_up_sites(&mut self) -> Option<GameEvent> {
        if self.s.sites.len() >= MAX_SITES {
            return None;
        }
        let mut ids = vec![];
        let near = if self.rng.chance(0.5) { SiteKind::Ore } else { SiteKind::Sample };
        ids.push(self.spawn_site(near, None));
        ids.push(self.spawn_site(SiteKind::Ore, None));
        for r in 0..3 {
            ids.push(self.spawn_region_find(r));
        }
        Some(GameEvent::Discovery(ids))
    }

    fn compute_supply(&self) -> f32 {
        let s = &self.s;
        let m = s.def();
        let night = s.is_night();
        let mut g = 0.0;
        let survival = s.mode == Mode::Survival;
        // survival: the landing site sets the sunlight; dust stays until someone cleans it
        let sun = if survival { [0.8, 1.0, 1.2][crate::landing::SITES[s.surv.site as usize].sun as usize - 1] * s.surv.crisis } else { 1.0 };
        for md in &s.modules {
            match md.kind {
                ModuleKind::Solar if md.broken => {}
                ModuleKind::Solar if !night => g += m.panel_yield * sun * if (s.dust_left > 0 || survival) && md.dusty { 0.45 } else { 1.0 },
                ModuleKind::Rtg => g += RTG_YIELD,
                _ => {}
            }
        }
        if s.cadet {
            g = (g - 1.0).max(2.0f32.min(g));
        }
        if survival {
            g -= s.surv.upkeep();
        }
        g
    }

    fn refresh_power(&mut self) {
        self.s.supply = self.compute_supply();
        self.s.demand = self.s.modules.iter().map(|m| m.power as u32).sum();
        self.s.battery_cap = HABITAT_BATTERY + BATTERY_PER_MODULE * self.s.count(ModuleKind::Battery) as f32;
        self.s.battery = self.s.battery.min(self.s.battery_cap);
    }

    fn begin_sol(&mut self) -> Vec<GameEvent> {
        let mut ev = vec![GameEvent::NewSol(self.s.sol)];
        self.s.t = 0.0;
        if self.s.is_night() {
            self.s.flags.night = true;
            ev.push(GameEvent::Night);
        }
        if self.s.silence_left > 0 {
            self.s.silence_left -= 1;
        }
        if self.s.sol > 1 {
            ev.extend(self.top_up_sites());
        }
        if let Some((kind, _)) = self.schedule.get(&self.s.sol).copied() {
            self.pending.push(kind);
        }
        // forecast for tomorrow, delayed like real comms
        let m = self.s.def();
        let next = self.s.sol + 1;
        let survival_fc = if self.s.mode == Mode::Survival { self.tonight_plan() } else { None };
        let fc = if self.s.powered(ModuleKind::Antenna) == 0 {
            Forecast::NoAntenna
        } else if let Some(t) = survival_fc {
            // survival: the morning forecast is about this evening
            Forecast::Tonight(t)
        } else if self.s.silence_left > 0 {
            Forecast::NoSignal
        } else if self.s.has_finale() && (next == self.s.sols || self.s.sols == 1) {
            Forecast::Finale
        } else if next > self.s.sols {
            Forecast::Calm
        } else if m.night_sols.contains(&next) && !self.s.is_night() {
            Forecast::Night
        } else if self.false_alarm == Some(next) {
            Forecast::Maybe(EventKind::Flare)
        } else {
            match self.schedule.get(&next).copied() {
                Some((kind, false)) if matches!(kind, EventKind::Flare | EventKind::Dust | EventKind::Failure | EventKind::Leak) => {
                    if self.s.cadet || kind != EventKind::Flare { Forecast::Sure(kind) } else { Forecast::Maybe(kind) }
                }
                _ => Forecast::Calm,
            }
        };
        if survival_fc.is_some() || self.s.mode == Mode::Survival {
            // survival: yesterday's evening forecast is stale this morning
            self.s.forecast = None;
        }
        let now = self.s.elapsed;
        self.s.mc = Some(McMessage { forecast: fc, sent_at: now, arrive_at: now + m.delay_s });
        ev.push(GameEvent::ForecastSent { arrives_in: m.delay_s });
        ev
    }

    fn start_flare(&mut self, event: WeatherEvent, live: bool, duration: f32, boost: f32) {
        let factor = match self.s.map {
            MapId::Moon => 1.0,
            MapId::Mars => 0.8,
            MapId::Orbit => 1.25,
            MapId::Europa => 1.1,
            MapId::Earth => 0.0,
        };
        let spike = (12.0 + 18.0 * event.power) * factor * boost;
        self.s.flare = Some(Flare { event, live, t_left: duration, duration, spike });
        self.s.flags.flares += 1;
    }

    fn break_generator(&mut self) -> Option<GameEvent> {
        let gens: Vec<ModuleId> = self.s.modules.iter().filter(|m| m.kind == ModuleKind::OxygenGen && !m.broken).map(|m| m.id).collect();
        if gens.is_empty() {
            return None;
        }
        let id = gens[self.rng.below(gens.len())];
        if let Some(m) = self.s.modules.iter_mut().find(|m| m.id == id) {
            m.broken = true;
        }
        Some(GameEvent::ModuleBroken(id))
    }

    fn dust_up(&mut self) {
        self.s.dust_left = self.s.dust_left.max(1);
        self.s.flags.storm = true;
        for m in self.s.modules.iter_mut().filter(|m| m.kind == ModuleKind::Solar) {
            m.dusty = true;
        }
    }

    fn fire_pending(&mut self) -> Vec<GameEvent> {
        if self.s.t < 0.08 || self.pending.is_empty() {
            return vec![];
        }
        let kind = self.pending.remove(0);
        match kind {
            EventKind::Flare => {
                let (event, live) = self.weather.pick_flare(&mut self.rng);
                let live = live && self.s.live;
                let e = GameEvent::Flare { label_ru: event.label_ru.clone(), label_en: event.label_en.clone(), date: event.date.clone(), live };
                self.start_flare(event, live, 0.4, 1.0);
                vec![e]
            }
            EventKind::Dust => {
                self.dust_up();
                self.s.dust_left = 2;
                vec![GameEvent::DustStorm]
            }
            EventKind::Failure => self.break_generator().into_iter().collect(),
            EventKind::Leak => {
                self.s.leak = true;
                vec![GameEvent::Leak(self.s.habitat().id)]
            }
            EventKind::Harvest => {
                if self.s.count(ModuleKind::Greenhouse) == 0 {
                    return vec![];
                }
                self.s.growth = 1.0;
                self.s.harvest_ready = true;
                vec![GameEvent::HarvestReady]
            }
            EventKind::Discovery => {
                let ids = (0..3).map(|i| self.spawn_site(if i == 2 { SiteKind::Ore } else { SiteKind::Sample }, None)).collect();
                vec![GameEvent::Discovery(ids)]
            }
            EventKind::Supply => {
                self.s.cargo = Some(CARGO_PAD);
                vec![GameEvent::CargoLanding { x: CARGO_PAD.0, y: CARGO_PAD.1 }]
            }
            EventKind::Emotion => {
                let mood = self.rng.below(MOODS.len());
                self.s.mood = Some(CrewMood { mood, named: false });
                vec![GameEvent::CrewNeedsSupport(mood)]
            }
            EventKind::Conjunction => {
                self.s.silence_left = 2;
                vec![GameEvent::SignalLost]
            }
        }
    }

    fn start_wave(&mut self, wave: u8, countered: u8) -> Vec<GameEvent> {
        let def = boss(self.s.map);
        let of = def.waves.len() as u8;
        let kind = def.waves[(wave - 1) as usize];
        let mut ev = vec![GameEvent::BossWave { wave, of, kind }];
        match kind {
            Wave::Flare => {
                // a historical/finale event, never labeled as live NASA data
                let (date, power) = if self.s.map == MapId::Moon { ("1859-09-01", 1.0) } else { ("", 0.9) };
                let event = WeatherEvent { kind: "FINALE".into(), date: date.into(), label_ru: def.name_ru.into(), label_en: def.name_en.into(), power };
                self.start_flare(event, false, WAVE_LEN, 1.0);
            }
            Wave::Dust => self.dust_up(),
            Wave::Meteor => {
                self.s.leak = true;
                ev.push(GameEvent::Leak(self.s.habitat().id));
                ev.extend(self.break_generator());
            }
        }
        self.s.boss = Some(BossState { wave, of, kind, active: true, t_left: WAVE_LEN, countered, sheltered: false });
        ev
    }

    fn boss_tick(&mut self, ds: f32) -> Vec<GameEvent> {
        let mut ev = vec![];
        if !self.s.has_finale() || self.s.boss_done || self.s.sol < self.s.sols || self.s.status != Status::Playing {
            return ev;
        }
        let Some(mut b) = self.s.boss else {
            if self.s.t >= 0.15 {
                ev.extend(self.start_wave(1, 0));
            }
            return ev;
        };
        b.t_left -= ds;
        if b.active {
            let inside = self.s.players.iter().any(|p| p.inside);
            b.sheltered |= inside;
            let done_right = match b.kind {
                Wave::Flare => false,
                Wave::Dust => !self.s.modules.iter().any(|m| m.dusty),
                Wave::Meteor => !self.s.leak && !self.s.modules.iter().any(|m| m.broken),
            };
            let over = done_right || b.t_left <= 0.0 || (b.kind == Wave::Flare && self.s.flare.is_none());
            if over {
                let countered = done_right || (b.kind == Wave::Flare && b.sheltered);
                if b.kind == Wave::Dust {
                    // the storm passes
                    self.s.dust_left = 0;
                    for m in self.s.modules.iter_mut() {
                        m.dusty = false;
                    }
                }
                b.countered += countered as u8;
                ev.push(GameEvent::WaveCleared { wave: b.wave, countered });
                b.active = false;
                b.t_left = WAVE_GAP;
            }
            self.s.boss = Some(b);
        } else if b.t_left <= 0.0 {
            if b.wave >= b.of {
                self.s.boss = None;
                self.s.boss_done = true;
                self.s.boss_countered = b.countered;
                ev.push(GameEvent::BossDefeated { countered: b.countered, of: b.of });
            } else {
                ev.extend(self.start_wave(b.wave + 1, b.countered));
            }
        } else {
            self.s.boss = Some(b);
        }
        ev
    }

    /// Story progress from what just happened. Only the current task counts.
    fn quest_tick(&mut self, events: &[GameEvent]) -> Vec<GameEvent> {
        let mut out = vec![];
        let mut events = events;
        if self.s.mode == Mode::Challenge {
            return out;
        }
        while let Some(q) = STORY.get(self.s.quest) {
            let (need, progress) = match q.goal {
                Goal::Collect(kind, n) => (n, self.s.quest_progress + events.iter().filter(|e| matches!(e, GameEvent::Collected { kind: k, .. } if *k == kind)).count() as u32),
                Goal::Deliver(n) => (n, self.s.quest_progress + events.iter().map(|e| if let GameEvent::SamplesDelivered { count, .. } = e { *count } else { 0 }).sum::<u32>()),
                Goal::Explore(r, n) => (n, self.s.quest_progress + events.iter().filter(|e| matches!(e, GameEvent::Collected { region: Some(x), .. } if *x == r)).count() as u32),
                Goal::Build(kind) => (1, (self.s.count(kind) > 0) as u32),
                Goal::Boss => (1, self.s.boss_done as u32),
            };
            self.s.quest_progress = progress.min(need);
            if progress < need {
                break;
            }
            // done: reward, then the next task starts (events are not counted twice)
            let r = q.reward;
            self.s.coins += r.coins;
            if let Some(u) = r.unlock {
                if !self.s.has(u) {
                    self.s.unlocks.push(u);
                    out.push(GameEvent::Unlocked(u));
                }
            }
            out.push(GameEvent::QuestDone { quest: self.s.quest, coins: r.coins, unlock: r.unlock });
            self.s.quest += 1;
            self.s.quests_done += 1;
            self.s.quest_progress = 0;
            events = &[];
            if self.s.quest < STORY.len() {
                out.push(GameEvent::QuestStarted(self.s.quest));
            }
        }
        out
    }

    /// Advance the world by `dt` real seconds.
    pub fn tick(&mut self, dt: f32) -> Vec<GameEvent> {
        let mut ev = vec![];
        if self.mirror || self.s.status != Status::Playing || self.s.paused || dt <= 0.0 {
            return ev;
        }
        self.s.elapsed += dt;
        let ds = dt / SOL_LEN;
        self.refresh_power();
        let s = &mut self.s;
        let m = map(s.map);

        // power: taking more than the sun gives drains the battery; spare power charges it
        let (supply, demand) = (s.supply, s.demand as f32);
        let mut mult = 1.0;
        if demand > supply {
            if s.battery <= 0.0 {
                mult = if demand > 0.0 { supply / demand } else { 1.0 };
            }
            s.battery = (s.battery - (demand - supply) * ds).max(0.0);
        } else {
            s.battery = (s.battery + (supply - demand) * ds).min(s.battery_cap);
        }
        // oxygen and food
        let survival = s.mode == Mode::Survival;
        let o2_made: f32 = s.modules.iter().filter(|m| m.kind == ModuleKind::OxygenGen).map(|m| m.power as f32 * O2_PER * if m.broken { 0.5 } else { 1.0 } * if survival && m.dusty { 0.6 } else { 1.0 }).sum();
        let o2_made = if survival { o2_made * self_o2_factor(s) } else { o2_made };
        let leak = if s.leak { 4.0 } else { 0.0 };
        s.o2 += (o2_made * mult - O2_USE - leak) * ds;
        let green = s.powered(ModuleKind::Greenhouse) as f32;
        if !s.cadet {
            s.food += (green * FOOD_PER * mult - FOOD_USE) * ds;
        }
        let was_ready = s.harvest_ready;
        s.growth = (s.growth + green * mult * 0.3 * ds).min(1.0);
        if s.growth >= 1.0 && green > 0.0 {
            s.harvest_ready = true;
        }
        if s.harvest_ready && !was_ready {
            ev.push(GameEvent::HarvestReady);
        }
        // radiation: the base is shielded, a suit outside is not
        let red = s.shield_red();
        let n = s.players.len().max(1) as f32;
        let inside = s.players.iter().filter(|p| p.inside).count() as f32 / n;
        let mut dose_rate = m.dose * (1.0 - red) + m.dose * 0.5 * (1.0 - inside);
        if let Some(f) = &mut s.flare {
            let exposure = (1.0 - inside) + inside * 0.15;
            dose_rate += f.spike / f.duration * (1.0 - red) * exposure;
            f.t_left -= ds;
            if inside > 0.0 {
                s.flags.sheltered_in_flare = true;
            }
            if f.t_left <= 0.0 {
                s.flare = None;
                ev.push(GameEvent::FlareEnded);
            }
        }
        s.dose += dose_rate * ds;
        // morale and science
        s.morale -= 4.0 * ds * if survival { 1.0 - 0.25 * s.surv.level(Upgrade::Comfort) as f32 } else { 1.0 };
        let sci_mult = if s.morale < 30.0 { 0.5 } else if s.morale >= 80.0 { 1.3 } else { 1.0 };
        s.science += s.powered(ModuleKind::Lab) as f32 * mult * sci_mult * ds;
        if s.mode == Mode::Sandbox {
            // free building: the crew never has to leave
            s.o2 = s.o2.max(5.0);
            s.food = s.food.max(5.0);
            s.dose = s.dose.min(90.0);
        }
        s.o2 = s.o2.clamp(0.0, 100.0);
        s.food = s.food.clamp(0.0, 100.0);
        s.dose = s.dose.clamp(0.0, 100.0);
        s.morale = s.morale.clamp(0.0, 100.0);
        // warnings, once per dip
        if s.o2 < 25.0 && !self.low_o2_warned {
            self.low_o2_warned = true;
            ev.push(GameEvent::LowOxygen);
        } else if s.o2 > 35.0 {
            self.low_o2_warned = false;
        }
        if s.battery <= 0.0 && demand > supply && !self.battery_warned {
            self.battery_warned = true;
            ev.push(GameEvent::BatteryEmpty);
        } else if s.battery > 1.0 {
            self.battery_warned = false;
        }
        // the forecast reaches the base after the comms delay
        if let Some(mc) = s.mc {
            if s.elapsed >= mc.arrive_at {
                s.forecast = Some(mc.forecast);
                s.mc = None;
                ev.push(GameEvent::ForecastArrived(mc.forecast));
            }
        }
        // end conditions — the crew always evacuates safely
        if s.o2 <= 0.0 {
            s.status = Status::Lost(Cause::Oxygen);
        } else if !s.cadet && s.food <= 0.0 {
            s.status = Status::Lost(Cause::Food);
        } else if s.dose >= 100.0 {
            s.status = Status::Lost(Cause::Dose);
        }
        s.t += ds;
        if survival && self.s.status == Status::Playing {
            self.survival_tick(ds, mult, &mut ev);
        }
        ev.extend(self.fire_pending());
        ev.extend(self.boss_tick(ds));
        if self.s.status == Status::Playing && self.s.t >= 1.0 {
            let finale_running = self.s.has_finale() && self.s.sol >= self.s.sols && !self.s.boss_done;
            if finale_running {
                // the last sol lasts until the finale is over
                self.s.t = 0.999;
            } else {
                if self.s.mode == Mode::Survival {
                    self.finish_threat(&mut ev);
                }
                let s = &mut self.s;
                s.history.push(SolRecord { sol: s.sol, o2: s.o2, food: s.food, dose: s.dose, science: s.science });
                if s.sol >= s.sols {
                    s.status = Status::Won;
                } else {
                    s.sol += 1;
                    if s.dust_left > 0 {
                        s.dust_left -= 1;
                        if s.dust_left == 0 {
                            for m in s.modules.iter_mut() {
                                m.dusty = false;
                            }
                            ev.push(GameEvent::DustGone);
                        }
                    }
                    ev.extend(self.begin_sol());
                }
            }
        }
        let q = self.quest_tick(&ev);
        ev.extend(q);
        if self.s.status != Status::Playing {
            ev.push(GameEvent::MissionEnded(self.result()));
        }
        ev
    }

    pub fn result(&self) -> MissionResult {
        let s = &self.s;
        MissionResult {
            map: s.map,
            mode: s.mode,
            won: s.status == Status::Won,
            cause: if let Status::Lost(c) = s.status { Some(c) } else { None },
            stars: s.stars(),
            points: s.points(),
            sol: s.sol,
            sols: s.sols,
            science: s.science,
            goal: s.goal,
            dose: s.dose,
            history: s.history.clone(),
            cadet: s.cadet,
            coop: s.coop,
            live: s.live,
            flags: Flags { threats: s.surv.threats, countered: s.surv.countered, ..s.flags.clone() },
            samples_delivered: s.samples_delivered,
            quests_done: s.quests_done,
            boss_done: s.boss_done,
            boss_countered: s.boss_countered,
        }
    }

    // ---------------------------------------------------------------- building

    /// Can `kind` go at (x, y)? Use it to colour the build ghost green or red.
    pub fn can_build(&self, kind: ModuleKind, x: i32, y: i32) -> Result<(), Refusal> {
        let d = module_def(kind);
        if kind == ModuleKind::Habitat {
            return Err(Refusal::OnlyOne);
        }
        if !d.only.is_empty() && !d.only.contains(&self.s.map) {
            return Err(Refusal::NotAvailableHere);
        }
        if x < -BUILD_R || y < -BUILD_R || x + d.w - 1 > BUILD_R || y + d.h - 1 > BUILD_R {
            return Err(Refusal::OutsideBase);
        }
        let probe = Module { id: 0, kind, x, y, w: d.w, h: d.h, power: 0, broken: false, dusty: false, hp: 100.0 };
        let overlaps = self.s.modules.iter().any(|m| m.x < x + d.w && x < m.x + m.w && m.y < y + d.h && y < m.y + m.h);
        let on_site = self.s.sites.iter().any(|st| probe.dist((st.x, st.y)) < 0.5);
        let on_pad = probe.dist(CARGO_PAD) < 1.0 || probe.dist(DOOR) < 0.8;
        if overlaps || on_site || on_pad {
            return Err(Refusal::CellBusy);
        }
        if self.s.coins < build_cost(kind, &self.s.surv) {
            return Err(Refusal::NotEnoughCoins);
        }
        Ok(())
    }

    /// The nearest free spot for `kind` (for "auto-place", hints and tests).
    pub fn find_free_spot(&self, kind: ModuleKind) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        for y in -BUILD_R..=BUILD_R {
            for x in -BUILD_R..=BUILD_R {
                let ok = matches!(self.can_build(kind, x, y), Ok(()) | Err(Refusal::NotEnoughCoins));
                let d = x * x + y * y;
                if ok && best.map_or(true, |b| d < b.2) {
                    best = Some((x, y, d));
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    fn near_module(&self, player: usize, kind: ModuleKind) -> Option<ModuleId> {
        let p = self.s.players.get(player)?;
        self.s.modules.iter().filter(|m| m.kind == kind && m.dist(p.pos) <= NEAR).map(|m| m.id).next()
    }

    fn require_outside(&self, player: usize) -> Result<(), Refusal> {
        if self.s.players.get(player).map_or(false, |p| p.inside) {
            Err(Refusal::InsideShelter)
        } else {
            Ok(())
        }
    }

    fn require_near_module(&self, player: usize, id: ModuleId) -> Result<(), Refusal> {
        self.require_outside(player)?;
        let m = self.s.module(id).ok_or(Refusal::UnknownId)?;
        let p = self.s.players.get(player).ok_or(Refusal::UnknownId)?;
        if m.dist(p.pos) > NEAR {
            return Err(Refusal::TooFar);
        }
        Ok(())
    }

    /// A player's action. On success returns what happened (for sounds and messages).
    pub fn apply(&mut self, player: u8, cmd: Command) -> Result<Vec<GameEvent>, Refusal> {
        if self.s.status != Status::Playing {
            return Err(Refusal::MissionOver);
        }
        if self.mirror {
            // the host decides; its answer comes back with the next state
            self.outbox.push((player, cmd));
            return Ok(vec![]);
        }
        let pi = (player as usize).min(self.s.players.len() - 1);
        let ev = match cmd {
            Command::Build { kind, x, y } => {
                self.can_build(kind, x, y)?;
                self.s.coins -= build_cost(kind, &self.s.surv);
                let module = self.add_module(kind, x, y, 0);
                self.s.flags.built += 1;
                let mut ev = vec![GameEvent::Built { module, kind }];
                if kind == ModuleKind::Garage && !self.s.has(Unlock::Vehicle) {
                    self.s.unlocks.push(Unlock::Vehicle);
                    ev.push(GameEvent::Unlocked(Unlock::Vehicle));
                }
                ev
            }
            Command::Demolish { x, y } => {
                let m = self.s.module_at(x, y).ok_or(Refusal::NoModuleThere)?.clone();
                if m.kind == ModuleKind::Habitat {
                    return Err(Refusal::CannotDemolish);
                }
                let refund = module_def(m.kind).cost / 2;
                self.s.coins += refund;
                self.s.modules.retain(|x| x.id != m.id);
                vec![GameEvent::Demolished { kind: m.kind, refund }]
            }
            Command::SetPower { module, blocks } => {
                let m = self.s.module(module).ok_or(Refusal::UnknownId)?;
                let max = if m.kind == ModuleKind::Habitat { self.heater_slots() } else { module_def(m.kind).max_power };
                if max == 0 {
                    return Err(Refusal::NoPowerSlot);
                }
                let blocks = blocks.min(max);
                let old = m.power;
                if blocks > old {
                    self.refresh_power();
                    let new_demand = self.s.demand - old as u32 + blocks as u32;
                    if new_demand > self.s.available() {
                        return Err(Refusal::NotEnoughPower);
                    }
                }
                if let Some(m) = self.s.modules.iter_mut().find(|m| m.id == module) {
                    m.power = blocks;
                }
                vec![]
            }
            Command::Repair { module } => {
                self.require_near_module(pi, module)?;
                let kind = self.s.module(module).map(|m| (m.kind, m.broken)).ok_or(Refusal::UnknownId)?;
                if self.s.mode == Mode::Survival {
                    self.survival_repair(module)?;
                    self.s.coins += 2;
                    self.s.flags.repairs += 1;
                    return self.finish_apply(vec![GameEvent::Repaired(module)]);
                }
                match kind {
                    (ModuleKind::OxygenGen, true) => {
                        if let Some(m) = self.s.modules.iter_mut().find(|m| m.id == module) {
                            m.broken = false;
                        }
                        self.s.battery = (self.s.battery - 2.0).max(0.0);
                    }
                    (ModuleKind::Habitat, _) if self.s.leak => {
                        self.s.leak = false;
                        self.s.battery = (self.s.battery - 1.0).max(0.0);
                    }
                    _ => return Err(Refusal::NothingToRepair),
                }
                self.s.flags.repairs += 1;
                vec![GameEvent::Repaired(module)]
            }
            Command::Clean { module } => {
                self.require_near_module(pi, module)?;
                let m = self.s.modules.iter_mut().find(|m| m.id == module).ok_or(Refusal::UnknownId)?;
                let survival = self.s.mode == Mode::Survival;
                let cleanable = m.kind == ModuleKind::Solar || (survival && m.kind == ModuleKind::OxygenGen);
                if !cleanable || !m.dusty {
                    return Err(Refusal::NothingToClean);
                }
                m.dusty = false;
                if survival {
                    self.s.coins += 1;
                }
                vec![GameEvent::Cleaned(module)]
            }
            Command::Collect { site } => {
                self.require_outside(pi)?;
                let st = self.s.sites.iter().find(|x| x.id == site).ok_or(Refusal::UnknownId)?.clone();
                if let Some(r) = st.region {
                    if !self.s.region_open(r) {
                        return Err(Refusal::NeedUnlock(regions(self.s.map)[r as usize].need));
                    }
                }
                let p = &self.s.players[pi];
                if (p.pos.0 - st.x).hypot(p.pos.1 - st.y) > NEAR {
                    return Err(Refusal::TooFar);
                }
                match st.kind {
                    SiteKind::Sample => {
                        if p.carried >= CARRY_MAX {
                            return Err(Refusal::HandsFull);
                        }
                        self.s.players[pi].carried += 1;
                    }
                    SiteKind::Ore => self.s.coins += 3,
                    SiteKind::Wreck => self.s.coins += 6,
                    SiteKind::Ice => self.s.o2 = (self.s.o2 + 10.0).min(100.0),
                }
                self.s.sites.retain(|x| x.id != site);
                self.s.flags.collected += 1;
                if let Some(r) = st.region {
                    self.s.flags.regions_visited[r as usize] = true;
                }
                vec![GameEvent::Collected { site, kind: st.kind, region: st.region }]
            }
            Command::DeliverSamples => {
                self.require_outside(pi)?;
                if self.s.count(ModuleKind::Lab) == 0 {
                    return Err(Refusal::NeedsModule(ModuleKind::Lab));
                }
                self.near_module(pi, ModuleKind::Lab).ok_or(Refusal::TooFar)?;
                let count = self.s.players[pi].carried;
                if count == 0 {
                    return Err(Refusal::NothingToDeliver);
                }
                if self.s.powered(ModuleKind::Lab) == 0 {
                    return Err(Refusal::LabNeedsPower);
                }
                let science = count as f32 * 1.5;
                self.s.science += science;
                if self.s.mode == Mode::Survival {
                    self.s.coins += count;
                }
                self.s.samples_delivered += count;
                self.s.players[pi].carried = 0;
                vec![GameEvent::SamplesDelivered { count, science }]
            }
            Command::EnterShelter => {
                if self.s.players[pi].inside {
                    return Err(Refusal::AlreadyInside);
                }
                self.near_module(pi, ModuleKind::Habitat).ok_or(Refusal::TooFar)?;
                self.s.players[pi].inside = true;
                if self.s.flare.is_some() {
                    self.s.flags.sheltered_in_flare = true;
                }
                vec![GameEvent::Sheltered { player: pi as u8 }]
            }
            Command::ExitShelter => {
                if !self.s.players[pi].inside {
                    return Err(Refusal::NotInside);
                }
                self.s.players[pi].inside = false;
                self.s.players[pi].pos = DOOR;
                vec![GameEvent::LeftShelter { player: pi as u8 }]
            }
            Command::ChooseHarvest(eat) => {
                self.require_outside(pi)?;
                if !self.s.harvest_ready {
                    return Err(Refusal::NotReady);
                }
                self.near_module(pi, ModuleKind::Greenhouse).ok_or(Refusal::TooFar)?;
                let s = &mut self.s;
                s.harvest_ready = false;
                s.growth = 0.0;
                s.o2 = (s.o2 + 3.0).min(100.0);
                if eat {
                    s.food = (s.food + 12.0).min(100.0);
                    s.morale = (s.morale + 10.0).min(100.0);
                } else {
                    s.food = (s.food + 4.0).min(100.0);
                    s.science += 2.0;
                }
                vec![GameEvent::Harvested { eat }]
            }
            Command::ChooseCargo(all) => {
                self.require_outside(pi)?;
                let pad = self.s.cargo.ok_or(Refusal::NotReady)?;
                let p = self.s.players[pi].pos;
                if (p.0 - pad.0).hypot(p.1 - pad.1) > NEAR + 1.0 {
                    return Err(Refusal::TooFar);
                }
                let s = &mut self.s;
                s.cargo = None;
                if all {
                    s.battery = (s.battery - 2.0).max(0.0);
                    s.food = (s.food + 20.0).min(100.0);
                    s.o2 = (s.o2 + 8.0).min(100.0);
                    s.coins += 4;
                } else {
                    s.food = (s.food + 8.0).min(100.0);
                }
                vec![GameEvent::CargoUnloaded { all }]
            }
            Command::NameFeeling(f) => {
                let mood = self.s.mood.ok_or(Refusal::NoMood)?;
                let feeling = FEELINGS.get(f).ok_or(Refusal::UnknownId)?;
                if MOODS[mood.mood].ok.contains(&feeling.id) {
                    self.s.mood = Some(CrewMood { named: true, ..mood });
                    vec![GameEvent::FeelingRight(f)]
                } else {
                    // a gentle "maybe, what else?" — never a penalty
                    vec![GameEvent::FeelingTryAgain(f)]
                }
            }
            Command::Support(i) => {
                let mood = self.s.mood.ok_or(Refusal::NoMood)?;
                if !mood.named {
                    return Err(Refusal::NameFeelingFirst);
                }
                if i >= MOODS[mood.mood].support.len() {
                    return Err(Refusal::UnknownId);
                }
                let s = &mut self.s;
                s.mood = None;
                s.battery = (s.battery - 1.0).max(0.0);
                s.morale = (s.morale + 20.0).min(100.0);
                s.flags.helped = true;
                vec![GameEvent::Supported(i)]
            }
            Command::Upgrade(u) => self.apply_upgrade(u)?,
            Command::Calm => {
                let b = &mut self.s.players[pi].body;
                b.heart = (b.heart - 25.0).max(60.0);
                vec![GameEvent::Calmed { player: pi as u8 }]
            }
        };
        self.finish_apply(ev)
    }

    fn finish_apply(&mut self, mut ev: Vec<GameEvent>) -> Result<Vec<GameEvent>, Refusal> {
        self.refresh_power();
        let q = self.quest_tick(&ev);
        ev.extend(q);
        Ok(ev)
    }

    #[cfg(test)]
    pub(crate) fn force_event(&mut self, kind: EventKind) -> Vec<GameEvent> {
        self.pending.push(kind);
        self.s.t = self.s.t.max(0.1);
        self.fire_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_mode(map_id: MapId, cadet: bool, mode: Mode) -> Game {
        Game::with_mode(MissionConfig { map: map_id, cadet, coop: false, live: false, seed: 42 }, mode, &Weather::archive())
    }

    fn game(map_id: MapId, cadet: bool) -> Game {
        game_mode(map_id, cadet, Mode::Story)
    }

    fn id_of(g: &Game, kind: ModuleKind) -> ModuleId {
        g.state().modules.iter().find(|m| m.kind == kind).unwrap().id
    }

    fn stand_at(g: &mut Game, id: ModuleId) {
        let c = g.state().module(id).unwrap().center();
        g.set_player_pos(0, c.0, c.1 + 2.0);
    }

    fn collect(g: &mut Game, site: &Site) -> Result<Vec<GameEvent>, Refusal> {
        g.set_player_pos(0, site.x, site.y);
        g.apply(0, Command::Collect { site: site.id })
    }

    fn run_to_end(g: &mut Game) -> Vec<GameEvent> {
        let mut all = vec![];
        let mut guard = 0;
        while g.state().status == Status::Playing && guard < 200_000 {
            all.extend(g.tick(0.5));
            guard += 1;
        }
        all
    }

    #[test]
    fn starting_base_is_sane() {
        for m in MAPS {
            let g = game(m.id, false);
            let s = g.state();
            assert_eq!(s.count(ModuleKind::Habitat), 1);
            assert!(s.demand as f32 <= s.supply + s.battery, "{:?} starts over budget", m.id);
            assert!(s.sites.len() >= 28, "{:?}: the big map has finds everywhere", m.id);
            for r in 0..3u8 {
                assert!(s.sites.iter().any(|x| x.region == Some(r) && region_at(x.x, x.y) == Some(r)));
            }
            assert!(s.sites.iter().all(|x| x.x.hypot(x.y) <= WORLD_R));
        }
    }

    #[test]
    fn regions_are_120_degree_sectors_outside_the_open_ground() {
        assert_eq!(region_at(10.0, 0.0), None);
        assert_eq!(region_at(60.0, 1.0), Some(0));
        assert_eq!(region_at(-60.0, 1.0), Some(1));
        assert_eq!(region_at(0.0, -60.0), Some(2));
    }

    #[test]
    fn far_regions_open_with_tools() {
        let mut g = game(MapId::Moon, false);
        assert_eq!(g.state().can_walk(60.0, 1.0), Err(Refusal::NeedUnlock(Unlock::Vehicle)));
        assert_eq!(g.state().can_walk(0.0, 200.0), Err(Refusal::EdgeOfMap));
        assert_eq!(g.state().can_walk(20.0, 0.0), Ok(()));
        let far = g.state().sites.iter().find(|s| s.region == Some(0)).unwrap().clone();
        assert_eq!(collect(&mut g, &far), Err(Refusal::NeedUnlock(Unlock::Vehicle)));
        g.s.coins = 20;
        let (x, y) = g.find_free_spot(ModuleKind::Garage).unwrap();
        let ev = g.apply(0, Command::Build { kind: ModuleKind::Garage, x, y }).unwrap();
        assert!(ev.contains(&GameEvent::Unlocked(Unlock::Vehicle)));
        let ev = collect(&mut g, &far).unwrap();
        assert!(matches!(ev[0], GameEvent::Collected { region: Some(0), .. }));
        assert!(g.state().flags.regions_visited[0]);
    }

    #[test]
    fn the_map_never_runs_dry() {
        let mut g = game(MapId::Earth, false);
        // take everything near the base
        let near: Vec<Site> = g.state().sites.iter().filter(|s| s.region.is_none()).cloned().collect();
        for s in &near {
            let _ = collect(&mut g, s);
        }
        assert!(g.state().sites.iter().all(|s| s.region.is_some() || s.kind == SiteKind::Sample));
        let mut fresh = false;
        for _ in 0..(SOL_LEN / 0.5) as usize + 4 {
            fresh |= g.tick(0.5).iter().any(|e| matches!(e, GameEvent::Discovery(_)));
        }
        assert!(fresh && g.state().sites.iter().any(|s| s.region.is_none()), "new finds appear every sol");
    }

    #[test]
    fn story_tasks_advance_and_give_rewards() {
        let mut g = game(MapId::Moon, false);
        assert_eq!(g.state().quest, 0);
        let ores: Vec<Site> = g.state().sites.iter().filter(|s| s.kind == SiteKind::Ore && s.region.is_none()).take(3).cloned().collect();
        let mut done = vec![];
        for s in &ores {
            done.extend(collect(&mut g, s).unwrap());
        }
        assert!(done.iter().any(|e| matches!(e, GameEvent::QuestDone { quest: 0, .. })));
        assert_eq!(g.state().quest, 1);
        g.s.coins = 30;
        let (x, y) = g.find_free_spot(ModuleKind::Lab).unwrap();
        let ev = g.apply(0, Command::Build { kind: ModuleKind::Lab, x, y }).unwrap();
        assert!(ev.contains(&GameEvent::QuestStarted(2)));
        // deliver 3 samples → headlamp
        let samples: Vec<Site> = g.state().sites.iter().filter(|s| s.kind == SiteKind::Sample && s.region.is_none()).take(3).cloned().collect();
        for s in &samples {
            collect(&mut g, s).unwrap();
        }
        let lab = id_of(&g, ModuleKind::Lab);
        g.apply(0, Command::SetPower { module: lab, blocks: 1 }).unwrap();
        stand_at(&mut g, lab);
        let ev = g.apply(0, Command::DeliverSamples).unwrap();
        assert!(ev.contains(&GameEvent::Unlocked(Unlock::Headlamp)));
        assert!(g.state().region_open(1));
        assert_eq!(g.state().quests_done, 3);
    }

    #[test]
    fn the_finale_has_waves_and_rewards_doing_the_right_thing() {
        let mut g = game_mode(MapId::Moon, false, Mode::Challenge);
        assert_eq!(g.state().sols, 1);
        let mut waves = 0;
        let mut defeated = None;
        let mut guard = 0;
        while g.state().status == Status::Playing && guard < 10_000 {
            guard += 1;
            for e in g.tick(0.5) {
                match e {
                    GameEvent::BossWave { kind, .. } => {
                        waves += 1;
                        // a careful crew: hide from flares, fix meteor damage
                        match kind {
                            Wave::Flare => {
                                g.set_player_pos(0, DOOR.0, DOOR.1);
                                let _ = g.apply(0, Command::EnterShelter);
                            }
                            Wave::Meteor => {
                                let _ = g.apply(0, Command::ExitShelter);
                                let hab = g.state().habitat().id;
                                stand_at(&mut g, hab);
                                g.apply(0, Command::Repair { module: hab }).unwrap();
                                let broken: Vec<ModuleId> = g.state().modules.iter().filter(|m| m.broken).map(|m| m.id).collect();
                                for id in broken {
                                    stand_at(&mut g, id);
                                    g.apply(0, Command::Repair { module: id }).unwrap();
                                }
                            }
                            Wave::Dust => {}
                        }
                    }
                    GameEvent::BossDefeated { countered, of } => defeated = Some((countered, of)),
                    _ => {}
                }
            }
        }
        assert_eq!(waves, 3);
        assert_eq!(defeated, Some((3, 3)), "all three waves handled");
        assert_eq!(g.state().status, Status::Won);
        assert!(g.state().history.len() == 1 && g.state().boss_done);
    }

    #[test]
    fn sandbox_never_loses_and_has_everything_open() {
        let mut g = game_mode(MapId::Europa, false, Mode::Sandbox);
        assert!(g.state().region_open(0) && g.state().region_open(1) && g.state().region_open(2));
        let ox = id_of(&g, ModuleKind::OxygenGen);
        g.apply(0, Command::SetPower { module: ox, blocks: 0 }).unwrap();
        for _ in 0..(3.0 * SOL_LEN / 0.5) as usize {
            g.tick(0.5);
        }
        assert_eq!(g.state().status, Status::Playing);
        assert!(g.state().o2 >= 5.0);
        let r = g.end_now();
        assert!(r.won && g.state().status == Status::Won);
    }

    #[test]
    fn a_mirrored_game_shows_the_host_and_forwards_commands() {
        let mut host = game(MapId::Moon, false);
        let mut guest = game(MapId::Moon, false);
        for _ in 0..20 {
            host.tick(0.5);
        }
        guest.mirror(host.state().clone());
        assert!(guest.is_mirror());
        assert_eq!(guest.tick(0.5), vec![]);
        assert_eq!(guest.state(), host.state(), "the guest never simulates on its own");
        let (x, y) = host.find_free_spot(ModuleKind::Solar).unwrap();
        let cmd = Command::Build { kind: ModuleKind::Solar, x, y };
        assert_eq!(guest.apply(1, cmd), Ok(vec![]));
        let out = guest.take_outbox();
        assert_eq!(out, vec![(1, cmd)]);
        assert!(guest.take_outbox().is_empty());
        host.apply(out[0].0, out[0].1).unwrap();
        guest.mirror(host.state().clone());
        assert_eq!(guest.state().count(ModuleKind::Solar), host.state().count(ModuleKind::Solar));
    }

    #[test]
    fn state_and_commands_travel_over_the_network() {
        let g = game(MapId::Mars, true);
        let json = serde_json::to_string(g.state()).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, g.state());
        let cmd = Command::Build { kind: ModuleKind::Garage, x: 3, y: -7 };
        assert_eq!(serde_json::from_str::<Command>(&serde_json::to_string(&cmd).unwrap()).unwrap(), cmd);
    }

    #[test]
    fn cannot_build_on_busy_cell_or_without_materials() {
        let mut g = game(MapId::Moon, false);
        assert_eq!(g.apply(0, Command::Build { kind: ModuleKind::Shield, x: 0, y: 0 }), Err(Refusal::CellBusy));
        assert_eq!(g.apply(0, Command::Build { kind: ModuleKind::Shield, x: 40, y: 0 }), Err(Refusal::OutsideBase));
        assert_eq!(g.apply(0, Command::Build { kind: ModuleKind::Habitat, x: 6, y: 6 }), Err(Refusal::OnlyOne));
        assert_eq!(g.apply(0, Command::Build { kind: ModuleKind::Rtg, x: 6, y: 6 }), Err(Refusal::NotAvailableHere));
        let (x, y) = g.find_free_spot(ModuleKind::Shield).unwrap();
        let before = g.state().coins;
        assert!(g.apply(0, Command::Build { kind: ModuleKind::Shield, x, y }).is_ok());
        assert_eq!(g.state().coins, before - 6);
        assert_eq!(g.apply(0, Command::Build { kind: ModuleKind::Shield, x, y }), Err(Refusal::CellBusy));
        g.s.coins = 0;
        let (x, y) = g.find_free_spot(ModuleKind::Lab).unwrap();
        assert_eq!(g.apply(0, Command::Build { kind: ModuleKind::Lab, x, y }), Err(Refusal::NotEnoughCoins));
    }

    #[test]
    fn demolish_refunds_half_and_keeps_the_habitat() {
        let mut g = game(MapId::Earth, false);
        assert_eq!(g.apply(0, Command::Demolish { x: 0, y: 0 }), Err(Refusal::CannotDemolish));
        let b = g.state().modules.iter().find(|m| m.kind == ModuleKind::Battery).unwrap().clone();
        let before = g.state().coins;
        g.apply(0, Command::Demolish { x: b.x, y: b.y }).unwrap();
        assert_eq!(g.state().coins, before + 2);
        assert_eq!(g.state().battery_cap, HABITAT_BATTERY);
    }

    #[test]
    fn power_cannot_exceed_supply_plus_battery() {
        let mut g = game(MapId::Mars, false);
        let lab_spot = g.find_free_spot(ModuleKind::Lab).unwrap();
        g.apply(0, Command::Build { kind: ModuleKind::Lab, x: lab_spot.0, y: lab_spot.1 }).unwrap();
        let lab = id_of(&g, ModuleKind::Lab);
        let ox = id_of(&g, ModuleKind::OxygenGen);
        g.apply(0, Command::SetPower { module: ox, blocks: 3 }).unwrap();
        assert!(g.apply(0, Command::SetPower { module: lab, blocks: 3 }).is_ok());
        g.s.battery = 0.0;
        assert_eq!(g.apply(0, Command::SetPower { module: ox, blocks: 3 }), Ok(vec![]), "keeping the same level is fine");
        let sh = g.find_free_spot(ModuleKind::Shield).unwrap();
        g.apply(0, Command::Build { kind: ModuleKind::Shield, x: sh.0, y: sh.1 }).unwrap();
        let shield = id_of(&g, ModuleKind::Shield);
        assert_eq!(g.apply(0, Command::SetPower { module: shield, blocks: 1 }), Err(Refusal::NotEnoughPower));
        assert_eq!(g.apply(0, Command::SetPower { module: id_of(&g, ModuleKind::Solar), blocks: 1 }), Err(Refusal::NoPowerSlot));
    }

    #[test]
    fn lunar_night_without_battery_starves_oxygen() {
        let mut g = game(MapId::Moon, true);
        g.s.sol = 4;
        g.s.battery = 0.0;
        g.refresh_power();
        assert_eq!(g.state().supply, 0.0);
        let o2 = g.state().o2;
        for _ in 0..100 {
            g.tick(0.5);
        }
        assert!(g.state().o2 < o2 - 5.0, "no sun and no battery: oxygen must fall");
    }

    #[test]
    fn samples_need_a_powered_lab() {
        let mut g = game(MapId::Earth, false);
        let site = g.state().sites.iter().find(|s| s.kind == SiteKind::Sample && s.region.is_none()).unwrap().clone();
        g.set_player_pos(0, 0.0, 0.0);
        assert_eq!(g.apply(0, Command::Collect { site: site.id }), Err(Refusal::TooFar));
        collect(&mut g, &site).unwrap();
        assert_eq!(g.state().players[0].carried, 1);
        assert_eq!(g.apply(0, Command::DeliverSamples), Err(Refusal::NeedsModule(ModuleKind::Lab)));
        let (x, y) = g.find_free_spot(ModuleKind::Lab).unwrap();
        g.apply(0, Command::Build { kind: ModuleKind::Lab, x, y }).unwrap();
        let lab = id_of(&g, ModuleKind::Lab);
        stand_at(&mut g, lab);
        assert_eq!(g.apply(0, Command::DeliverSamples), Err(Refusal::LabNeedsPower));
        g.apply(0, Command::SetPower { module: lab, blocks: 1 }).unwrap();
        let ev = g.apply(0, Command::DeliverSamples).unwrap();
        assert_eq!(ev[0], GameEvent::SamplesDelivered { count: 1, science: 1.5 });
    }

    #[test]
    fn collecting_ore_gives_materials_and_removes_the_site() {
        let mut g = game(MapId::Moon, false);
        let ore = g.state().sites.iter().find(|s| s.kind == SiteKind::Ore && s.region.is_none()).unwrap().clone();
        let before = g.state().coins;
        collect(&mut g, &ore).unwrap();
        assert_eq!(g.state().coins, before + 3);
        assert!(g.state().sites.iter().all(|s| s.id != ore.id));
    }

    #[test]
    fn repairs_need_to_be_next_to_the_module() {
        let mut g = game(MapId::Moon, false);
        let ev = g.force_event(EventKind::Failure);
        let GameEvent::ModuleBroken(id) = ev[0] else { panic!("{ev:?}") };
        g.set_player_pos(0, 30.0, 30.0);
        assert_eq!(g.apply(0, Command::Repair { module: id }), Err(Refusal::TooFar));
        stand_at(&mut g, id);
        assert_eq!(g.apply(0, Command::Repair { module: id }).unwrap(), vec![GameEvent::Repaired(id)]);
        assert_eq!(g.apply(0, Command::Repair { module: id }), Err(Refusal::NothingToRepair));
    }

    #[test]
    fn shelter_cuts_flare_dose() {
        let run = |hide: bool| {
            let mut g = game(MapId::Moon, false);
            g.force_event(EventKind::Flare);
            if hide {
                g.set_player_pos(0, DOOR.0, DOOR.1);
                g.apply(0, Command::EnterShelter).unwrap();
            }
            while g.state().flare.is_some() {
                g.tick(0.5);
            }
            g.state().dose
        };
        let (out, inside) = (run(false), run(true));
        assert!(inside < out * 0.5, "inside {inside} vs outside {out}");
    }

    #[test]
    fn crew_support_needs_the_feeling_named_first() {
        let mut g = game(MapId::Earth, false);
        g.force_event(EventKind::Emotion);
        let mood = g.state().mood.unwrap().mood;
        assert_eq!(g.apply(0, Command::Support(0)), Err(Refusal::NameFeelingFirst));
        let wrong = FEELINGS.iter().position(|f| !MOODS[mood].ok.contains(&f.id)).unwrap();
        assert_eq!(g.apply(0, Command::NameFeeling(wrong)).unwrap(), vec![GameEvent::FeelingTryAgain(wrong)]);
        let right = FEELINGS.iter().position(|f| MOODS[mood].ok.contains(&f.id)).unwrap();
        g.apply(0, Command::NameFeeling(right)).unwrap();
        let morale = g.state().morale;
        g.apply(0, Command::Support(1)).unwrap();
        assert!(g.state().morale > morale && g.state().flags.helped && g.state().mood.is_none());
    }

    #[test]
    fn forecast_arrives_after_the_comms_delay() {
        let mut g = game(MapId::Mars, false);
        assert!(g.state().mc.is_some());
        g.tick(1.0);
        assert!(g.state().forecast.is_none(), "Mars delay is 6 s in game");
        let mut arrived = false;
        for _ in 0..12 {
            arrived |= g.tick(0.5).iter().any(|e| matches!(e, GameEvent::ForecastArrived(_)));
        }
        assert!(arrived);
        let ant = id_of(&g, ModuleKind::Antenna);
        g.apply(0, Command::SetPower { module: ant, blocks: 0 }).unwrap();
        let mut last = None;
        for _ in 0..(SOL_LEN as i32 * 2 + 40) {
            for e in g.tick(0.5) {
                if let GameEvent::ForecastArrived(f) = e {
                    last = Some(f);
                }
            }
        }
        assert_eq!(last, Some(Forecast::NoAntenna));
    }

    #[test]
    fn same_seed_same_mission() {
        let a = game(MapId::Mars, false);
        let b = game(MapId::Mars, false);
        assert_eq!(a.state(), b.state());
        assert_eq!(a.schedule, b.schedule);
    }

    #[test]
    fn mission_ends_with_a_result() {
        let mut g = game(MapId::Earth, true);
        let end = run_to_end(&mut g).into_iter().find_map(|e| if let GameEvent::MissionEnded(r) = e { Some(r) } else { None });
        let r = end.expect("the mission ends");
        assert_eq!(r.won, g.state().status == Status::Won);
        assert!(r.boss_done || !r.won, "a won story mission went through the finale");
        assert_eq!(g.apply(0, Command::DeliverSamples), Err(Refusal::MissionOver));
    }
}

fn self_o2_factor(s: &State) -> f32 {
    let cold = if s.surv.temp < 0.0 { 0.5 } else { 1.0 };
    cold * (1.0 + 0.2 * s.surv.level(Upgrade::LifeSupport) as f32)
}
