//! Mission save & continue — entirely on the app side (part B), no changes to `ja-core`.
//!
//! The rules are deterministic: the same `MissionConfig` (seed), weather and the same sequence
//! of calls (`tick`, `set_player_pos`, `apply`, `set_paused`) always give the same `State`.
//! So instead of serialising the rules' internals we record every call ("ops") and, to continue,
//! rebuild the mission and replay the ops — it takes a fraction of a second.
//!
//! Saved when the child exits to the menu, closes the window or quits, and every few seconds.
//! The file is deleted when the mission ends (the result is already in the profile).

use crate::{AppState, Set, Setup, Sim};
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use ja_core::data::{MAPS, MODULES};
use ja_core::weather::WeatherEvent;
use ja_core::{Command, Game, GameEvent, MapId, MissionConfig, ModuleKind, Refusal, Status, Weather};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const VERSION: u32 = 2; // 2: big map, story tasks and finale in ja-core
const AUTOSAVE_EVERY: f32 = 5.0;

/// One recorded call into the rules.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Op {
    /// tick(dt)
    T(f32),
    /// set_player_pos(player, x, y)
    P(u8, f32, f32),
    /// apply(player, command)
    C(u8, Cmd),
    /// set_paused(on)
    Z(bool),
}

/// Serializable mirror of `ja_core::Command` (module kinds stored as their index in `MODULES`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Cmd {
    Build(u8, i32, i32),
    Demolish(i32, i32),
    SetPower(u32, u8),
    Repair(u32),
    Clean(u32),
    Collect(u32),
    Deliver,
    Enter,
    Exit,
    Harvest(bool),
    Cargo(bool),
    Feel(usize),
    Support(usize),
}

fn kind_idx(k: ModuleKind) -> u8 {
    MODULES.iter().position(|m| m.kind == k).unwrap_or(0) as u8
}
fn kind_of(i: u8) -> ModuleKind {
    MODULES.get(i as usize).map(|m| m.kind).unwrap_or(ModuleKind::Solar)
}

impl From<Command> for Cmd {
    fn from(c: Command) -> Self {
        match c {
            Command::Build { kind, x, y } => Cmd::Build(kind_idx(kind), x, y),
            Command::Demolish { x, y } => Cmd::Demolish(x, y),
            Command::SetPower { module, blocks } => Cmd::SetPower(module, blocks),
            Command::Repair { module } => Cmd::Repair(module),
            Command::Clean { module } => Cmd::Clean(module),
            Command::Collect { site } => Cmd::Collect(site),
            Command::DeliverSamples => Cmd::Deliver,
            Command::EnterShelter => Cmd::Enter,
            Command::ExitShelter => Cmd::Exit,
            Command::ChooseHarvest(b) => Cmd::Harvest(b),
            Command::ChooseCargo(b) => Cmd::Cargo(b),
            Command::NameFeeling(i) => Cmd::Feel(i),
            Command::Support(i) => Cmd::Support(i),
        }
    }
}
impl From<Cmd> for Command {
    fn from(c: Cmd) -> Self {
        match c {
            Cmd::Build(k, x, y) => Command::Build { kind: kind_of(k), x, y },
            Cmd::Demolish(x, y) => Command::Demolish { x, y },
            Cmd::SetPower(m, b) => Command::SetPower { module: m, blocks: b },
            Cmd::Repair(m) => Command::Repair { module: m },
            Cmd::Clean(m) => Command::Clean { module: m },
            Cmd::Collect(s) => Command::Collect { site: s },
            Cmd::Deliver => Command::DeliverSamples,
            Cmd::Enter => Command::EnterShelter,
            Cmd::Exit => Command::ExitShelter,
            Cmd::Harvest(b) => Command::ChooseHarvest(b),
            Cmd::Cargo(b) => Command::ChooseCargo(b),
            Cmd::Feel(i) => Command::NameFeeling(i),
            Cmd::Support(i) => Command::Support(i),
        }
    }
}

/// Serializable copy of the space weather the mission started with (so flares replay identically).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavedWeather {
    live: bool,
    events: Vec<(String, String, String, String, f32)>,
    counts: (usize, usize, usize, usize),
    kp_max: f32,
    range: String,
}
impl From<&Weather> for SavedWeather {
    fn from(w: &Weather) -> Self {
        Self {
            live: w.live,
            events: w.events.iter().map(|e| (e.kind.clone(), e.date.clone(), e.label_ru.clone(), e.label_en.clone(), e.power)).collect(),
            counts: w.counts,
            kp_max: w.kp_max,
            range: w.range.clone(),
        }
    }
}
impl SavedWeather {
    fn to_weather(&self) -> Weather {
        let mut w = Weather::archive();
        w.live = self.live;
        w.events = self.events.iter().map(|(kind, date, label_ru, label_en, power)| WeatherEvent { kind: kind.clone(), date: date.clone(), label_ru: label_ru.clone(), label_en: label_en.clone(), power: *power }).collect();
        w.counts = self.counts;
        w.kp_max = self.kp_max;
        w.range = self.range.clone();
        w
    }
}

/// Everything needed to rebuild a mission.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Recording {
    version: u32,
    map: u8,
    cadet: bool,
    coop: bool,
    live: bool,
    seed: u64,
    pub plan: u8,
    weather: SavedWeather,
    /// shown on the "Continue" button without replaying
    pub sol: u32,
    pub sols: u32,
    ops: Vec<Op>,
    #[serde(skip)]
    last_pos: Vec<(f32, f32)>,
}

impl Recording {
    pub fn new(cfg: MissionConfig, weather: &Weather, plan: u8) -> Self {
        Self {
            version: VERSION,
            map: MAPS.iter().position(|m| m.id == cfg.map).unwrap_or(0) as u8,
            cadet: cfg.cadet,
            coop: cfg.coop,
            live: cfg.live,
            seed: cfg.seed,
            plan,
            weather: weather.into(),
            sol: 1,
            sols: 1,
            ops: vec![],
            last_pos: vec![],
        }
    }
    pub fn map_id(&self) -> MapId {
        MAPS.get(self.map as usize).map(|m| m.id).unwrap_or(MapId::Earth)
    }
    pub fn coop(&self) -> bool {
        self.coop
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    fn config(&self) -> MissionConfig {
        MissionConfig { map: self.map_id(), cadet: self.cadet, coop: self.coop, live: self.live, seed: self.seed }
    }
}

impl Sim {
    /// Start a new mission (recorded from the first moment).
    pub fn start(cfg: MissionConfig, weather: &Weather, plan: u8) -> Self {
        let game = Game::new(cfg, weather);
        let mut rec = Recording::new(cfg, weather, plan);
        rec.sols = game.state().sols;
        Sim(game, rec)
    }
    /// Rebuild a saved mission by replaying its recording.
    pub fn resume(mut rec: Recording) -> Self {
        let mut game = Game::new(rec.config(), &rec.weather.to_weather());
        for op in &rec.ops {
            match *op {
                Op::T(dt) => {
                    let _ = game.tick(dt);
                }
                Op::P(i, x, y) => game.set_player_pos(i, x, y),
                Op::C(i, c) => {
                    let _ = game.apply(i, c.into());
                }
                Op::Z(on) => game.set_paused(on),
            }
        }
        rec.last_pos = game.state().players.iter().map(|p| p.pos).collect();
        Sim(game, rec)
    }

    pub fn apply(&mut self, player: u8, cmd: Command) -> Result<Vec<GameEvent>, Refusal> {
        if !self.0.is_mirror() {
            self.1.ops.push(Op::C(player, cmd.into()));
        }
        self.0.apply(player, cmd)
    }
    pub fn tick(&mut self, dt: f32) -> Vec<GameEvent> {
        if self.0.is_mirror() {
            return vec![];
        }
        self.1.ops.push(Op::T(dt));
        let ev = self.0.tick(dt);
        self.1.sol = self.0.state().sol;
        ev
    }
    pub fn set_player_pos(&mut self, player: u8, x: f32, y: f32) {
        let i = player as usize;
        if self.1.last_pos.len() <= i {
            self.1.last_pos.resize(i + 1, (f32::NAN, f32::NAN));
        }
        if self.1.last_pos[i] != (x, y) {
            self.1.last_pos[i] = (x, y);
            if !self.0.is_mirror() {
                self.1.ops.push(Op::P(player, x, y));
            }
            self.0.set_player_pos(player, x, y);
        }
    }
    pub fn set_paused(&mut self, on: bool) {
        if self.0.is_mirror() {
            return;
        }
        self.1.ops.push(Op::Z(on));
        self.0.set_paused(on);
    }
    fn unfinished(&self) -> bool {
        // a network guest's mission belongs to the host: nothing to save here
        self.0.state().status == Status::Playing && !self.0.is_mirror()
    }
}

// ------------------------------------------------------------------ files

fn path() -> PathBuf {
    if let Some(p) = std::env::var_os("JA_SAVE") {
        return p.into();
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| ".".into());
    home.join(".junior_astronaut_mission.json")
}

/// The screenshot tour must not touch a real save.
fn enabled() -> bool {
    std::env::var_os("JA_SHOTS").is_none() || std::env::var_os("JA_SAVE").is_some()
}

pub fn load() -> Option<Recording> {
    let s = std::fs::read_to_string(path()).ok()?;
    let r: Recording = serde_json::from_str(&s).ok()?;
    (r.version == VERSION).then_some(r)
}

fn write(sim: &Sim) -> bool {
    if !enabled() || !sim.unfinished() {
        return false;
    }
    let Ok(s) = serde_json::to_string(&sim.1) else { return false };
    // write to a temp file first so a crash never leaves a half-written save
    let tmp = path().with_extension("json.tmp");
    std::fs::write(&tmp, s).is_ok() && std::fs::rename(&tmp, path()).is_ok()
}

fn delete() {
    if enabled() {
        let _ = std::fs::remove_file(path());
    }
}

// ------------------------------------------------------------------ plugin

/// What the "Continue" button shows (None = nothing saved).
#[derive(Resource, Default, Clone)]
pub struct SavedMission(pub Option<(MapId, u32, u32, bool)>);

/// Set by the menu: the next mission continues the saved one instead of starting fresh.
#[derive(Resource, Default)]
pub struct Resume(pub bool);

#[derive(Component)]
struct SavedBadge(f32);

pub struct SavePlugin;
impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SavedMission>()
            .init_resource::<Resume>()
            .add_systems(Startup, refresh_summary)
            .add_systems(Update, (autosave, mission_over).in_set(Set::View).run_if(in_state(AppState::Playing)))
            .add_systems(Update, (on_close, fade_badge))
            .add_systems(Last, on_app_exit)
            .add_systems(OnExit(AppState::Playing), (save_on_leave, drop_sim, refresh_summary).chain());
    }
}

fn summary(r: &Recording) -> (MapId, u32, u32, bool) {
    (r.map_id(), r.sol, r.sols, r.coop)
}

fn refresh_summary(mut saved: ResMut<SavedMission>) {
    saved.0 = load().map(|r| summary(&r));
}

fn autosave(mut commands: Commands, time: Res<Time>, sim: Option<Res<Sim>>, mut t: Local<f32>, art: Res<crate::art::Art>, prof: Res<crate::Prof>, badges: Query<Entity, With<SavedBadge>>) {
    let Some(sim) = sim else { return };
    *t += time.delta_secs();
    if *t < AUTOSAVE_EVERY {
        return;
    }
    *t = 0.0;
    if write(&sim) && badges.is_empty() {
        // a small "saved" mark under the pause button
        let l = prof.lang();
        commands
            .spawn((Node { position_type: PositionType::Absolute, right: Val::Px(14.0), top: Val::Px(78.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() }, BackgroundColor(Color::srgba(0.05, 0.06, 0.16, 0.8)), BorderRadius::all(Val::Px(6.0)), GlobalZIndex(11), SavedBadge(1.6), StateScoped(AppState::Playing)))
            .with_children(|p| {
                crate::kit::icon(p, &art, "check", 18.0);
                crate::kit::text(p, l.pick("Сохранено", "Saved"), &art.body, 15.0, crate::kit::C_GOOD);
            });
    }
}

fn fade_badge(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut SavedBadge)>) {
    for (e, mut b) in &mut q {
        b.0 -= time.delta_secs();
        if b.0 <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn mission_over(mut ev: EventReader<crate::world::Happened>) {
    for crate::world::Happened(e) in ev.read() {
        if matches!(e, GameEvent::MissionEnded(_)) {
            delete();
        }
    }
}

fn save_on_leave(sim: Option<Res<Sim>>) {
    if let Some(sim) = sim {
        write(&sim);
    }
}

fn drop_sim(mut commands: Commands, mut resume: ResMut<Resume>) {
    commands.remove_resource::<Sim>();
    resume.0 = false;
}

fn on_close(mut ev: EventReader<WindowCloseRequested>, sim: Option<Res<Sim>>) {
    if ev.read().count() > 0 {
        if let Some(sim) = sim {
            write(&sim);
        }
    }
}

fn on_app_exit(mut ev: EventReader<AppExit>, sim: Option<Res<Sim>>) {
    if ev.read().count() > 0 {
        if let Some(sim) = sim {
            write(&sim);
        }
    }
}

/// Used by `main::start_mission`: continue the saved mission if asked, else start a new one.
pub fn begin(commands: &mut Commands, setup: &mut Setup, resume: bool, cfg: MissionConfig, weather: &Weather) {
    if resume {
        if let Some(rec) = load() {
            setup.map = rec.map_id();
            setup.coop = rec.coop();
            setup.plan = rec.plan;
            commands.insert_resource(Sim::resume(rec));
            return;
        }
    }
    commands.insert_resource(Sim::start(cfg, weather, setup.plan));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ja_core::data::MAPS;
    use ja_core::game::BUILD_R;

    /// Play a mission with a mix of every kind of call, save, load, replay — states must match.
    fn roundtrip(map: MapId, coop: bool, seed: u64) {
        let cfg = MissionConfig { map, cadet: false, coop, live: false, seed };
        let mut sim = Sim::start(cfg, &Weather::archive(), 2);
        let mut x = seed;
        let mut rnd = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for step in 0..4000u32 {
            let r = rnd();
            match r % 10 {
                0..=5 => {
                    sim.tick(0.016 + (r % 7) as f32 * 0.003);
                }
                6 | 7 => {
                    let p = (r % 2) as u8 * coop as u8;
                    let a = (r % 628) as f32 / 100.0;
                    let d = (rnd() % 1500) as f32 / 100.0;
                    sim.set_player_pos(p, a.cos() * d, a.sin() * d);
                }
                8 => {
                    let st = sim.0.state();
                    let cmd = match rnd() % 6 {
                        0 => Command::Build { kind: ModuleKind::Battery, x: (rnd() % (2 * BUILD_R as u64)) as i32 - BUILD_R, y: (rnd() % (2 * BUILD_R as u64)) as i32 - BUILD_R },
                        1 => match st.modules.get(rnd() as usize % st.modules.len()) {
                            Some(m) => Command::SetPower { module: m.id, blocks: (rnd() % 4) as u8 },
                            None => Command::DeliverSamples,
                        },
                        2 => match st.sites.first() {
                            Some(s) => Command::Collect { site: s.id },
                            None => Command::EnterShelter,
                        },
                        3 => Command::EnterShelter,
                        4 => Command::ExitShelter,
                        _ => Command::NameFeeling((rnd() % 6) as usize),
                    };
                    let _ = sim.apply(0, cmd);
                }
                _ => sim.set_paused(step % 3 == 0),
            }
            if sim.0.state().status != Status::Playing {
                break;
            }
        }
        let json = serde_json::to_string(&sim.1).unwrap();
        let rec: Recording = serde_json::from_str(&json).unwrap();
        let back = Sim::resume(rec);
        assert_eq!(back.0.state(), sim.0.state(), "{map:?} coop={coop} seed={seed}: continued mission differs");
    }

    #[test]
    fn continue_restores_the_exact_mission() {
        for (i, m) in MAPS.iter().enumerate() {
            roundtrip(m.id, i % 2 == 1, 1000 + i as u64);
        }
    }
}
