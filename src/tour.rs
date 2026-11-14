//! Screenshot tour for the README and the presentation.
//!
//! `JA_SHOTS=shots JA_PROFILE=shots/profile.json cargo run --release`
//! walks through the menu, a Moon mission (power console, a real-archive flare, crew support),
//! the debrief and split-screen co-op, saves PNGs into the folder and quits.

use crate::data::MapId;
use crate::menu::MenuPage;
use crate::sim::{Sim, Status};
use crate::ui::{Action, Block, Modal, ModalKind, UiAction};
use crate::{AppState, GameConfig};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use std::path::PathBuf;

#[derive(Resource)]
struct Tour {
    dir: PathBuf,
    t: f32,
    step: usize,
}

pub struct TourPlugin;

impl Plugin for TourPlugin {
    fn build(&self, app: &mut App) {
        let Some(dir) = std::env::var_os("JA_SHOTS") else { return };
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        app.insert_resource(Tour { dir, t: 0.0, step: 0 }).add_systems(Update, tour.after(crate::GameSet::Ui));
    }
}

enum Do {
    Shot(&'static str),
    Act(Action),
    Unlock,
    Open(ModalKind),
    Flare,
    Crew,
    Win,
    CoopAgain,
    Quit,
}

const STEPS: &[(f32, Do)] = &[
    (0.5, Do::Unlock),
    (3.0, Do::Shot("01_menu")),
    (3.5, Do::Act(Action::Page(MenuPage::Shop(0)))),
    (5.0, Do::Shot("02_shop")),
    (5.5, Do::Act(Action::Solo)),
    (7.0, Do::Shot("03_maps")),
    (7.5, Do::Act(Action::PickMap(MapId::Moon))),
    (9.0, Do::Shot("04_briefing")),
    (9.5, Do::Act(Action::Start)),
    (15.0, Do::Shot("05_moon_base")),
    (15.5, Do::Open(ModalKind::Board)),
    (16.0, Do::Act(Action::Alloc(Block::Shield, 1))),
    (16.3, Do::Act(Action::Alloc(Block::Shield, 1))),
    (17.5, Do::Shot("06_power_console")),
    (18.0, Do::Act(Action::Close)),
    (18.2, Do::Flare),
    (21.0, Do::Shot("07_flare")),
    (22.0, Do::Crew),
    (24.0, Do::Shot("08_crew_support")),
    (24.5, Do::Act(Action::Close)),
    (25.0, Do::Act(Action::Beacon)),
    (27.0, Do::Shot("09_space_beacon")),
    (27.5, Do::Act(Action::Close)),
    (28.0, Do::Win),
    (31.0, Do::Shot("10_debrief")),
    (31.5, Do::Act(Action::Reflect(0, 1))),
    (32.0, Do::Act(Action::Reflect(1, 0))),
    (33.0, Do::Shot("11_debrief_reflection")),
    (33.5, Do::CoopAgain),
    (39.0, Do::Shot("12_coop_split_screen")),
    (40.0, Do::Quit),
];

#[allow(clippy::too_many_arguments)]
fn tour(
    mut commands: Commands,
    time: Res<Time>,
    mut tour: ResMut<Tour>,
    mut ev: EventWriter<UiAction>,
    mut cfg: ResMut<GameConfig>,
    mut profile: ResMut<crate::save::Profile>,
    sim: Option<ResMut<Sim>>,
    mut modal: ResMut<Modal>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    tour.t += time.delta_secs();
    let mut sim = sim;
    while let Some((at, what)) = STEPS.get(tour.step) {
        if tour.t < *at {
            break;
        }
        tour.step += 1;
        match what {
            Do::Shot(name) => {
                let path = tour.dir.join(format!("{name}.png"));
                info!("tour: screenshot {}", path.display());
                commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            }
            Do::Act(a) => {
                ev.write(UiAction(*a));
            }
            Do::Unlock => {
                profile.total = profile.total.max(240);
                profile.stars = profile.stars.max(150);
                profile.missions = profile.missions.max(1);
            }
            Do::Open(k) => modal.open(*k, 0),
            Do::Flare => {
                if let Some(s) = sim.as_mut() {
                    s.pending.push(crate::data::EventKind::Flare);
                    s.t = s.t.max(0.1);
                }
            }
            Do::Crew => {
                if let Some(s) = sim.as_mut() {
                    s.emotion = Some(1);
                }
                modal.open(ModalKind::Emotion(1), 0);
            }
            Do::Win => {
                if let Some(s) = sim.as_mut() {
                    s.science = s.goal + 1.0;
                    s.sol = s.sols;
                    s.status = Status::Won;
                }
            }
            Do::CoopAgain => {
                cfg.coop = true;
                next.set(AppState::Playing);
            }
            Do::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
