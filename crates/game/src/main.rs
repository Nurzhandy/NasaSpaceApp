//! SPACE EXPLORERS — 2D pixel-art desktop game (docs/PLAN_V2.md).
//! Everything visible and audible lives here; the rules come from `ja-core` (part A).
// Release builds on Windows open no black console window next to the game.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod art;
mod audio;
mod build;
mod dialog_ui;
mod encyclopedia;
mod planets;
mod minigames;
mod ship;
mod touch_panel;
mod camera;
mod hud;
mod kit;
mod landing;
mod net;
mod player;
mod save;
mod screens;
mod story;
mod survival_ui;
mod touch;
mod tour;
mod windows;
mod world;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use ja_core::progress::Rewards;
use ja_core::{progress, weather, Lang, MapId, MissionConfig, MissionResult, Profile, Weather};

use std::sync::{mpsc, Mutex};

/// One cell of the base grid on screen: 16 px art × 3.
pub const CELL: f32 = 48.0;
pub const PX: f32 = 3.0;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    Maps,
    Briefing,
    /// new expedition: survival or creative, Junior or Advanced, 1–2 players
    Expedition,
    /// landing on the Moon: choose a site, fly down (landing.rs)
    Landing,
    Shop,
    /// six achievement directions + mission patches
    Achievements,
    Info,
    /// choose / create / delete a profile
    Profiles,
    /// the active profile: level, stars, history
    ProfilePage,
    /// look and expedition role
    Character,
    Inventory,
    Settings,
    /// network game: create or join (net.rs)
    Lobby,
    /// the flight to the Moon (part A, ship/)
    Ship,
    /// planets, physics cards, crew voices (part A, encyclopedia.rs)
    Encyclopedia,
    Playing,
    Debrief,
}

/// Settings for the next mission, chosen in the menus.
#[derive(Resource)]
pub struct Setup {
    pub map: MapId,
    pub coop: bool,
    pub live: bool,
    pub plan: u8,
    pub shop_tab: u8,
    /// bump to force the current menu page to rebuild
    pub refresh: u32,
    /// the mission mode: `Survival` (campaign) or `Sandbox` (creative); `Story` for old maps
    pub mode: ja_core::Mode,
    /// landing site and the coins the landing earned (survival)
    pub site: u8,
    pub landing_coins: u32,
    /// crew roles for the next expedition (player 1 takes the profile's role)
    pub roles: [ja_core::Role; 2],
    /// achievement tiers reached since the expedition started (landing + mission)
    pub new_tiers: Vec<(ja_core::Track, u8)>,
    /// profile waiting for the second "delete" press
    pub confirm_delete: Option<usize>,
}
impl Default for Setup {
    fn default() -> Self {
        Self { map: MapId::Moon, coop: false, live: true, plan: 0, shop_tab: 0, refresh: 0, mode: ja_core::Mode::Survival, site: 1, landing_coins: 0, roles: [ja_core::Role::None; 2], new_tiers: vec![], confirm_delete: None }
    }
}

/// The running mission: the rules (`ja-core`) plus a recording of every call, used to save
/// and continue the mission later (see `save.rs`). Read state via `sim.0.state()`; change it
/// only through `Sim::apply / tick / set_player_pos / set_paused` so the recording stays complete.
#[derive(Resource)]
pub struct Sim(pub ja_core::Game, pub save::Recording);

/// Time multiplier (the screenshot tour fast-forwards with it).
#[derive(Resource)]
pub struct Speed(pub f32);

/// Player profile (saved by `ja_core::progress`).
#[derive(Resource)]
pub struct Prof(pub Profile);
impl Prof {
    pub fn lang(&self) -> Lang {
        self.0.lang
    }
    pub fn save(&self) {
        progress::save(&self.0);
    }
}

/// Music on/off (presentation setting, not part of the saved profile).
#[derive(Resource)]
pub struct Music(pub bool);

/// Space weather: the archive at start, replaced by whatever the background fetch returns.
#[derive(Resource)]
pub struct Sky {
    pub weather: Weather,
    pub loading: bool,
}
#[derive(Resource)]
struct SkyRx(Mutex<mpsc::Receiver<Weather>>);

/// Last mission for the debrief screen.
#[derive(Resource, Clone)]
pub struct LastResult {
    pub result: MissionResult,
    pub rewards: Rewards,
    pub reflected: bool,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Set {
    Input,
    Logic,
    View,
}

fn main() {
    let profile = progress::load();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "SPACE EXPLORERS — NASA Space Apps 2026".into(),
                        resolution: WindowResolution::new(1280.0, 760.0),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin { file_path: assets_dir(), meta_check: bevy::asset::AssetMetaCheck::Never, ..default() }),
        )
        .insert_resource(ClearColor(Color::srgb(0.03, 0.03, 0.09)))
        .insert_resource(Music(profile.music))
        .insert_resource(Prof(profile))
        .insert_resource(Sky { weather: Weather::archive(), loading: true })
        .insert_resource(Speed(1.0))
        .init_resource::<Setup>()
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .configure_sets(Update, (Set::Input, Set::Logic, Set::View).chain())
        .add_systems(Startup, (art::load, start_weather, spawn_camera))
        .add_systems(Update, poll_weather)
        .add_plugins((kit::KitPlugin, audio::AudioFx, screens::ScreensPlugin, touch::TouchPlugin))
        .add_plugins((world::WorldPlugin, player::PlayerPlugin, camera::CameraPlugin, build::BuildPlugin, hud::HudPlugin, windows::WindowsPlugin, tour::TourPlugin, save::SavePlugin, story::StoryPlugin, net::NetPlugin, landing::LandingPlugin, survival_ui::SurvivalUiPlugin))
        .add_plugins((ship::ShipPlugin, ship::ShipTestPlugin, encyclopedia::EncyclopediaPlugin))
        .add_systems(OnEnter(AppState::Playing), start_mission)
        .run();
}

/// Finds `assets/` next to the executable (release archive), inside a macOS .app bundle,
/// or at the workspace root (cargo run).
pub fn assets_dir() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for p in [dir.join("assets"), dir.join("../Resources/assets")] {
                if p.join("sprites").exists() {
                    return p.to_string_lossy().into_owned();
                }
            }
        }
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").to_string_lossy().into_owned()
}

#[derive(Component)]
pub struct MainCam;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCam, Msaa::Off));
}

fn start_weather(mut commands: Commands, prof: Res<Prof>) {
    let rx = weather::spawn_fetch(weather::api_key(&prof.0.nasa_key));
    commands.insert_resource(SkyRx(Mutex::new(rx)));
}

fn poll_weather(rx: Option<Res<SkyRx>>, mut sky: ResMut<Sky>) {
    let Some(rx) = rx else { return };
    let Ok(r) = rx.0.lock() else { return };
    if let Ok(w) = r.try_recv() {
        if !w.events.is_empty() {
            sky.weather = w;
        }
        sky.loading = false;
    }
}

fn start_mission(mut commands: Commands, mut setup: ResMut<Setup>, prof: Res<Prof>, sky: Res<Sky>, resume: Res<save::Resume>, net_start: Option<Res<net::NetStart>>, net: Res<net::Net>) {
    if let Some(ns) = net_start {
        // network guest: the host's mission; its state replaces ours as soon as it arrives
        let start = save::Start { players: ns.1, ..default() };
        commands.insert_resource(Sim::start_with(ns.0, start, &sky.weather, setup.plan));
        commands.remove_resource::<net::NetStart>();
        return;
    }
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
    let cfg = MissionConfig { map: setup.map, cadet: prof.0.cadet, coop: setup.coop, live: setup.live, seed };
    save::begin(&mut commands, &mut setup, resume.0, cfg, &sky.weather, net.crew());
}
