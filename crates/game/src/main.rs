//! Junior Astronaut: Outpost — 2D pixel-art desktop game (part B of docs/PLAN_2D.md).
//! Everything visible and audible lives here; the rules come from `astro_core`.

mod art;
mod audio;
mod build;
mod camera;
mod hud;
mod kit;
mod menu;
mod player;
mod tour;
mod windows;
mod world;

use astro_core::{GameApi, MapId, Profile, Weather};
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
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
    Shop,
    Badges,
    Info,
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
}
impl Default for Setup {
    fn default() -> Self {
        Self { map: MapId::Earth, coop: false, live: true, plan: 0, shop_tab: 0, refresh: 0 }
    }
}

/// The running mission (rules from `astro_core`).
#[derive(Resource)]
pub struct Sim(pub astro_core::Game);

/// Time multiplier (the screenshot tour fast-forwards with it).
#[derive(Resource)]
pub struct Speed(pub f32);

/// Profile wrapper so systems can mark it dirty and save.
#[derive(Resource)]
pub struct Prof(pub Profile);
impl Prof {
    pub fn lang(&self) -> astro_core::Lang {
        if self.0.lang_en { astro_core::Lang::En } else { astro_core::Lang::Ru }
    }
}

/// Space weather: archive at start, replaced if the live fetch succeeds.
#[derive(Resource)]
pub struct Sky {
    pub weather: Weather,
    pub loading: bool,
}
#[derive(Resource)]
struct SkyRx(Mutex<mpsc::Receiver<Option<Weather>>>);

/// Last mission result for the debrief screen.
#[derive(Resource, Clone)]
pub struct LastResult {
    pub result: astro_core::MissionResult,
    pub new_badges: Vec<&'static str>,
    pub new_map: Option<MapId>,
    pub reflected: bool,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Set {
    Input,
    Logic,
    View,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Junior Astronaut: Outpost — NASA Space Apps 2026".into(),
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
        .insert_resource(Prof(Profile::load()))
        .insert_resource(Sky { weather: Weather::archive(), loading: true })
        .insert_resource(Speed(1.0))
        .init_resource::<Setup>()
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .configure_sets(Update, (Set::Input, Set::Logic, Set::View).chain())
        .add_systems(Startup, (art::load, start_weather, spawn_camera))
        .add_systems(Update, poll_weather)
        .add_plugins((kit::KitPlugin, audio::AudioFx, menu::MenuPlugin, world::WorldPlugin, player::PlayerPlugin, camera::CameraPlugin, build::BuildPlugin, hud::HudPlugin, windows::WindowsPlugin, tour::TourPlugin))
        .add_systems(OnEnter(AppState::Playing), start_mission)
        .add_systems(OnExit(AppState::Playing), |mut c: Commands| c.remove_resource::<Sim>())
        .run();
}

/// Finds `assets/` next to the executable (release archive) or at the workspace root (cargo run).
fn assets_dir() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("assets");
            if p.join("sprites").exists() {
                return p.to_string_lossy().into_owned();
            }
            // macOS .app bundle: Contents/MacOS/game → Contents/Resources/assets
            let p = dir.join("../Resources/assets");
            if p.join("sprites").exists() {
                return p.to_string_lossy().into_owned();
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    root.to_string_lossy().into_owned()
}

#[derive(Component)]
pub struct MainCam;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCam, Msaa::Off));
}

fn start_weather(mut commands: Commands) {
    let (tx, rx) = mpsc::channel();
    commands.insert_resource(SkyRx(Mutex::new(rx)));
    let key = std::env::var("NASA_API_KEY").ok().filter(|k| !k.trim().is_empty()).unwrap_or_else(|| "DEMO_KEY".into());
    std::thread::spawn(move || {
        let _ = tx.send(astro_core::weather::fetch(&key).ok());
    });
}

fn poll_weather(rx: Option<Res<SkyRx>>, mut sky: ResMut<Sky>) {
    let Some(rx) = rx else { return };
    let Ok(r) = rx.0.lock() else { return };
    if let Ok(w) = r.try_recv() {
        if let Some(w) = w {
            if w.live && !w.events.is_empty() {
                sky.weather = w;
            }
        }
        sky.loading = false;
    }
}

fn start_mission(mut commands: Commands, setup: Res<Setup>, prof: Res<Prof>, sky: Res<Sky>) {
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
    let cfg = astro_core::MissionConfig { map: setup.map, cadet: prof.0.cadet, coop: setup.coop, live: setup.live && sky.weather.live, seed };
    commands.insert_resource(Sim(astro_core::Game::new(cfg, &sky.weather)));
}
