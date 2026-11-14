//! Junior Astronaut: Outpost — a 3D game for kids 7–12 about running a Moon/Mars base
//! with real NASA data. NASA Space Apps Challenge 2026.

mod audio;
mod data;
mod game;
mod i18n;
mod interact;
mod menu;
mod nasa;
mod player;
mod save;
mod sim;
mod ui;
mod world;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

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

/// What the next mission will be (set from the menus).
#[derive(Resource)]
pub struct GameConfig {
    pub map: data::MapId,
    pub coop: bool,
    pub live: bool,
    pub plan: u8,
    pub shop_tab: u8,
    pub refresh: u32,
    pub reflected: bool,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self { map: data::MapId::Earth, coop: false, live: true, plan: 0, shop_tab: 0, refresh: 0, reflected: false }
    }
}

fn main() {
    let mut profile = save::load();
    if let Ok(k) = std::env::var("NASA_API_KEY") {
        if !k.trim().is_empty() {
            profile.nasa_key = k;
        }
    }
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Junior Astronaut: Outpost — NASA Space Apps 2026".into(),
                        resolution: WindowResolution::new(1280.0, 760.0),
                        present_mode: PresentMode::AutoVsync,
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin { meta_check: bevy::asset::AssetMetaCheck::Never, ..default() }),
        )
        .insert_resource(profile)
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.07)))
        .insert_resource(AmbientLight { color: Color::srgb(0.8, 0.85, 1.0), brightness: 250.0, ..default() })
        .init_resource::<GameConfig>()
        .init_resource::<ui::Modal>()
        .init_resource::<ui::Prompts>()
        .init_resource::<ui::Alerts>()
        .init_resource::<ui::McMsg>()
        .init_resource::<ui::Focus>()
        .init_resource::<menu::MenuTrack>()
        .init_resource::<player::Inputs>()
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .add_plugins(nasa::NasaPlugin)
        .add_systems(Startup, (ui::load_ui_assets, audio::load_sounds, spawn_ui_camera))
        // menus
        .add_systems(Update, (menu::ensure_preview, menu::clear_old_astro, menu::menu_render).chain().run_if(not(in_state(AppState::Playing))))
        .add_systems(OnEnter(AppState::Playing), (menu::despawn_preview, despawn_menu_ui, game::setup_game, ui::spawn_hud, audio::start_ambient).chain())
        .add_systems(OnExit(AppState::Playing), game::teardown_game)
        .add_systems(OnEnter(AppState::Debrief), reset_reflection)
        // gameplay
        .add_systems(
            Update,
            (
                player::read_inputs,
                ui::nav_system,
                interact::interact_system,
                game::sim_tick,
                player::move_players,
                player::follow_camera,
                player::split_viewports,
                world::environment_system,
                world::crewmate_walk,
                game::cargo_landing,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(Update, (ui::update_hud, ui::update_prompts, ui::update_alerts, ui::update_mc, ui::modal_render, ui::beacon_anim).run_if(in_state(AppState::Playing)))
        .add_systems(Update, (ui::nav_system).run_if(not(in_state(AppState::Playing))))
        .add_systems(Update, (ui::button_system, world::animate_props, world::animate_limbs, add_hold_to_players))
        .run();
}

#[derive(Component)]
struct UiCam;

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Camera { order: 10, clear_color: ClearColorConfig::None, ..default() }, bevy::ui::IsDefaultUiCamera, UiCam));
}

fn despawn_menu_ui(mut commands: Commands, q: Query<Entity, With<menu::MenuRoot>>, mut track: ResMut<menu::MenuTrack>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    track.state = None;
}

fn reset_reflection(mut cfg: ResMut<GameConfig>) {
    cfg.reflected = false;
    cfg.refresh += 1;
}

fn add_hold_to_players(mut commands: Commands, q: Query<Entity, (With<player::Player>, Without<interact::Hold>)>) {
    for e in &q {
        commands.entity(e).insert(interact::Hold::default());
    }
}
