//! B9 — screenshot tour for the README and judges.
//!
//! `JA_SHOTS=docs/screenshots_2d JA_PROFILE=/tmp/ja_tour.json cargo run -p game --release`
//! walks through menus, a Moon mission (power, build mode, a flare, pause, beacon), the debrief
//! and co-op, saving PNGs, then quits. `JA_TOUR=survival` shows the campaign instead: new
//! expedition, landing (autopilot), survival on the Moon, upgrades, an evening threat and its report.

use crate::build::BuildMode;
use crate::kit::{Act, Action};
use crate::windows::{Modal, Win};
use crate::{AppState, Prof, Setup, Speed};
use ja_core::MapId;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use std::path::PathBuf;

#[derive(Resource)]
struct Tour {
    dir: PathBuf,
    t: f32,
    step: usize,
}

enum Do {
    Wait(f32),
    Shot(&'static str),
    Act(Action),
    Open(Win),
    Close,
    Build(bool),
    Speed(f32),
    UntilDebrief,
    /// fast-forward until a flare (or any finale wave) is on screen; gives up after 40 s
    UntilFlare,
    Coop,
    /// survival tour: fly the landing with the autopilot, wait for touchdown / the alarm
    AutoLand,
    UntilLanded,
    UntilAlarm,
    /// survival: wait for tonight's forecast, the evening threat, the report window
    UntilForecast,
    UntilThreat,
    UntilReport,
    Quit,
}

const STEPS: &[Do] = &[
    Do::Wait(2.5),
    Do::Shot("01_menu"),
    Do::Act(Action::Go(AppState::Shop)),
    Do::Wait(1.2),
    Do::Shot("02_shop"),
    Do::Act(Action::Play(false)),
    Do::Wait(1.2),
    Do::Shot("03_maps"),
    Do::Act(Action::PickMap(MapId::Moon)),
    Do::Wait(1.2),
    Do::Shot("04_briefing"),
    Do::Act(Action::Launch),
    Do::Wait(1.5),
    Do::Shot("05_tutorial"),
    Do::Close,
    Do::Wait(1.5),
    Do::Shot("06_base"),
    Do::Open(Win::Power),
    Do::Wait(1.0),
    Do::Shot("07_power_console"),
    Do::Close,
    Do::Build(true),
    Do::Wait(1.0),
    Do::Shot("08_build_mode"),
    Do::Build(false),
    Do::Speed(40.0),
    Do::UntilFlare,
    Do::Speed(1.0),
    Do::Wait(1.0),
    Do::Shot("09_flare"),
    Do::Open(Win::Pause),
    Do::Wait(0.8),
    Do::Shot("10_pause"),
    Do::Act(Action::Beacon),
    Do::Act(Action::Breathe),
    Do::Wait(2.5),
    Do::Shot("11_beacon"),
    Do::Close,
    Do::Speed(400.0),
    Do::UntilDebrief,
    Do::Speed(1.0),
    Do::Wait(1.5),
    Do::Shot("12_debrief"),
    Do::Coop,
    Do::Wait(1.0),
    Do::Act(Action::PickMap(MapId::Mars)),
    Do::Wait(0.5),
    Do::Act(Action::Launch),
    Do::Wait(1.5),
    Do::Close,
    Do::Wait(1.5),
    Do::Shot("13_coop"),
    Do::Wait(1.0),
    Do::Quit,
];

/// `JA_TOUR=save`: play a bit, leave to the menu, press "Continue" (checks saving end-to-end).
const SAVE_STEPS: &[Do] = &[
    Do::Wait(2.0),
    Do::Act(Action::Play(false)),
    Do::Wait(0.6),
    Do::Act(Action::PickMap(MapId::Moon)),
    Do::Wait(0.6),
    Do::Act(Action::Launch),
    Do::Wait(1.0),
    Do::Close,
    Do::Speed(30.0),
    Do::Wait(3.0),
    Do::Speed(1.0),
    Do::Build(true),
    Do::Wait(0.5),
    Do::Build(false),
    Do::Wait(1.0),
    Do::Shot("s1_before_exit"),
    Do::Act(Action::ExitMission),
    Do::Wait(1.5),
    Do::Shot("s2_menu_continue"),
    Do::Act(Action::Continue),
    Do::Wait(2.0),
    Do::Shot("s3_after_continue"),
    Do::Wait(1.0),
    Do::Quit,
];

/// `JA_TOUR=survival`: the campaign — new expedition, landing, survival on the Moon.
const SURVIVAL_STEPS: &[Do] = &[
    Do::Wait(2.5),
    Do::Shot("20_menu"),
    Do::Act(Action::Go(AppState::Expedition)),
    Do::Wait(1.2),
    Do::Shot("21_expedition"),
    Do::Act(Action::StartExpedition),
    Do::Wait(1.5),
    Do::Shot("22_landing_sites"),
    Do::AutoLand,
    Do::Act(Action::PickSite(1)),
    Do::Wait(4.0),
    Do::Shot("23_descent"),
    Do::UntilAlarm,
    Do::Wait(0.4),
    Do::Shot("24_alarm_1202"),
    Do::UntilLanded,
    Do::Wait(1.2),
    Do::Shot("25_touchdown"),
    Do::Act(Action::LandingGo),
    Do::Wait(1.5),
    Do::Close,
    Do::UntilForecast,
    Do::Wait(0.5),
    Do::Shot("26_base_forecast"),
    Do::Open(Win::Upgrades),
    Do::Wait(1.0),
    Do::Shot("27_upgrades"),
    Do::Close,
    Do::Speed(12.0),
    Do::UntilThreat,
    Do::Speed(1.0),
    Do::Wait(1.0),
    Do::Shot("28_evening_threat"),
    Do::Speed(12.0),
    Do::UntilReport,
    Do::Speed(1.0),
    Do::Wait(1.0),
    Do::Shot("29_report"),
    Do::Close,
    Do::Wait(0.5),
    Do::Quit,
];

fn steps() -> &'static [Do] {
    match std::env::var("JA_TOUR").as_deref() {
        Ok("save") => SAVE_STEPS,
        Ok("survival") => SURVIVAL_STEPS,
        _ => STEPS,
    }
}

pub struct TourPlugin;
impl Plugin for TourPlugin {
    fn build(&self, app: &mut App) {
        let Some(dir) = std::env::var_os("JA_SHOTS") else { return };
        if std::env::var_os("JA_NET_TEST").is_some() {
            return; // the network test drives the game itself
        }
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        app.insert_resource(Tour { dir, t: 0.0, step: 0 }).add_systems(Startup, unlock).add_systems(Update, run.after(crate::Set::View));
    }
}

fn unlock(mut prof: ResMut<Prof>) {
    prof.0.total = prof.0.total.max(450);
    prof.0.stars = prof.0.stars.max(260);
    prof.0.missions = 0;
}

fn run(
    mut commands: Commands,
    time: Res<Time>,
    mut tour: ResMut<Tour>,
    state: Res<State<AppState>>,
    mut acts: EventWriter<Act>,
    mut modal: ResMut<Modal>,
    mut build: ResMut<BuildMode>,
    mut speed: ResMut<Speed>,
    mut setup: ResMut<Setup>,
    mut exit: EventWriter<AppExit>,
    sim: Option<Res<crate::Sim>>,
    mut descent: ResMut<crate::landing::Descent>,
) {
    let Some(step) = steps().get(tour.step) else { return };
    tour.t += time.delta_secs();
    let advance = match step {
        Do::Wait(s) => tour.t >= *s,
        Do::Shot(name) => {
            let path = tour.dir.join(format!("{name}.png"));
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            true
        }
        Do::Act(a) => {
            acts.write(Act(*a));
            true
        }
        Do::Open(w) => {
            modal.open(*w, 0);
            true
        }
        Do::Close => {
            modal.close();
            true
        }
        Do::Build(on) => {
            build.active = *on;
            build.player = 0;
            build.sel = 4; // shield
            true
        }
        Do::Speed(s) => {
            speed.0 = *s;
            true
        }
        Do::UntilDebrief => *state.get() == AppState::Debrief,
        Do::UntilFlare => sim.as_ref().map_or(true, |s| s.0.state().flare.is_some() || s.0.state().boss.is_some()) || tour.t > 40.0,
        Do::Coop => {
            setup.coop = true;
            acts.write(Act(Action::Play(true)));
            true
        }
        Do::AutoLand => {
            descent.autopilot = true;
            true
        }
        Do::UntilLanded => descent.lander.as_ref().map_or(false, |l| l.phase == ja_core::landing::Phase::Landed) || tour.t > 60.0,
        Do::UntilAlarm => descent.lander.as_ref().map_or(false, |l| l.alarm.is_some()) || tour.t > 30.0,
        Do::UntilForecast => sim.as_ref().map_or(false, |s| matches!(s.0.state().forecast, Some(ja_core::Forecast::Tonight(_)))) || tour.t > 20.0,
        Do::UntilThreat => sim.as_ref().map_or(false, |s| s.0.state().surv.active.is_some()) || tour.t > 40.0,
        Do::UntilReport => modal.win == Win::Report || tour.t > 40.0,
        Do::Quit => {
            exit.write(AppExit::Success);
            true
        }
    };
    if advance {
        tour.step += 1;
        tour.t = 0.0;
    }
}
