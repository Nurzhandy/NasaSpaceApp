//! Junior Astronaut: Outpost — a 3D game for kids 7–12 about running a Moon/Mars base
//! with real NASA data. NASA Space Apps Challenge 2026 · "Junior Astronaut Mission Trainer".

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
mod tour;
mod ui;
mod world;

use bevy::prelude::*;
use data::MapId;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Playing,
    Debrief,
}

/// What the next mission will be (chosen in the menu).
#[derive(Resource, Clone, Debug)]
pub struct GameConfig {
    pub map: MapId,
    pub coop: bool,
    /// use live NASA DONKI events when they are available
    pub live: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { map: MapId::Earth, coop: false, live: true }
    }
}

/// Frame order: read input → move the world & simulate → UI reacts.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    Input,
    World,
    Ui,
}

fn main() {
    let profile = save::load();
    let lang = if profile.lang_en { i18n::Lang::En } else { i18n::Lang::Ru };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Юный астронавт: Аванпост · Junior Astronaut: Outpost".into(),
                resolution: (1280.0_f32, 760.0_f32).into(),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.015, 0.02, 0.06)))
        .insert_resource(profile)
        .insert_resource(lang)
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .init_resource::<GameConfig>()
        .init_resource::<player::Inputs>()
        .configure_sets(Update, (GameSet::Input, GameSet::World, GameSet::Ui).chain())
        .add_plugins((nasa::NasaPlugin, ui::UiPlugin, menu::MenuPlugin, tour::TourPlugin))
        .add_systems(Startup, (audio::load_sounds, spawn_ui_camera))
        .add_systems(OnEnter(AppState::Playing), (game::setup_game, audio::start_ambient))
        .add_systems(OnExit(AppState::Playing), game::teardown_game)
        .add_systems(Update, player::read_inputs.in_set(GameSet::Input))
        .add_systems(
            Update,
            (
                player::split_viewports,
                player::move_players,
                player::follow_camera,
                interact::interact_system,
                game::sim_tick,
                game::cargo_landing,
                world::environment_system,
                world::crewmate_walk,
            )
                .chain()
                .in_set(GameSet::World)
                .run_if(in_state(AppState::Playing))
                .run_if(resource_exists::<sim::Sim>),
        )
        .add_systems(Update, (world::animate_props, world::animate_limbs))
        .run();
}

/// One 2D camera draws all UI on top of the 3D view(s), including both halves in split screen.
fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Camera { order: 10, clear_color: ClearColorConfig::None, ..default() }, IsDefaultUiCamera));
}
