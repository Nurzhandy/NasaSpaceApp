//! Game flow while playing: setup, simulation ticking, events → alerts, mission end.

use crate::data::*;
use crate::nasa::Weather;
use crate::player::Player;
use crate::sim::{Forecast, Sim, Status};
use crate::ui::{Alert, Alerts, McMsg, Modal, ModalKind};
use crate::world::{CargoCapsule, StationKind, WorldAssets};
use crate::{AppState, GameConfig};
use bevy::prelude::*;

/// Snapshot shown on the debrief screen.
#[derive(Resource, Clone)]
pub struct MissionResult {
    pub map: MapId,
    pub won: bool,
    pub cause: Option<crate::sim::Cause>,
    pub stars: u32,
    pub points: u32,
    pub sol: u32,
    pub sols: u32,
    pub science: f32,
    pub goal: f32,
    pub dose: f32,
    pub new_map: Option<MapId>,
    pub history: Vec<crate::sim::SolRecord>,
    pub coop: bool,
    pub cadet: bool,
    pub badges: Vec<&'static str>,
    pub helped: bool,
    pub sheltered: bool,
}

pub fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<GameConfig>,
    profile: Res<crate::save::Profile>,
    weather: Res<Weather>,
    mut modal: ResMut<Modal>,
    mut alerts: ResMut<Alerts>,
    mut mc: ResMut<McMsg>,
) {
    let live = cfg.live && weather.live && cfg.map != MapId::Earth;
    let mut sim = Sim::new(cfg.map, profile.cadet, cfg.coop, live, &weather);
    crate::player::set_map(cfg.map);
    let samples = 4;
    crate::world::spawn_world(&mut commands, &mut meshes, &mut materials, cfg.map, &mut sim.rng, samples);
    crate::player::spawn_players(&mut commands, &mut meshes, &mut materials, &profile, cfg.coop, cfg.map);
    sim.samples_total = samples;
    alerts.items.clear();
    alerts.dirty = true;
    *mc = McMsg::default();
    send_forecast(&sim, &mut mc, 0.0);
    *modal = Modal::default();
    if profile.missions == 0 && cfg.map == MapId::Earth {
        modal.open(ModalKind::Tutorial(0), 0);
    }
    commands.insert_resource(sim);
    commands.insert_resource(crate::player::Inputs::default());
}

fn send_forecast(sim: &Sim, mc: &mut McMsg, now: f32) {
    let delay = sim.def().delay_s;
    let (icon, ru, en) = match sim.forecast {
        Some(Forecast::Sure(EventKind::Flare)) => ("sun", "Завтра вспышка! Готовь щит и убежище", "Flare tomorrow! Prepare shield and shelter"),
        Some(Forecast::Maybe(EventKind::Flare)) => ("sun", "Возможно, завтра вспышка (~70%)", "Maybe a flare tomorrow (~70%)"),
        Some(Forecast::Sure(EventKind::Dust)) => ("tornado", "Завтра пылевая буря: энергии будет мало", "Dust storm tomorrow: low power"),
        Some(Forecast::Sure(EventKind::Failure)) => ("wrench", "Завтра генератор может сломаться", "The generator may fail tomorrow"),
        Some(Forecast::Sure(EventKind::Leak)) => ("comet", "Завтра поток микрометеоритов", "Micrometeorite shower tomorrow"),
        Some(Forecast::Night) => ("night", "Завтра лунная ночь: солнца не будет", "Lunar night tomorrow: no sun"),
        Some(_) => ("radio", "Завтра всё спокойно", "Tomorrow looks calm"),
        None if sim.silence_left > 0 => ("antenna_bars", "…связи нет: Солнце глушит сигнал", "…no signal: the Sun blocks radio"),
        None => ("check", "Завтра всё спокойно", "Tomorrow looks calm"),
    };
    mc.icon = icon;
    mc.ru = ru;
    mc.en = en;
    mc.sent_at = now;
    mc.arrive_at = now + delay;
    mc.active = true;
}

pub fn sim_tick(
    mut commands: Commands,
    time: Res<Time>,
    weather: Res<Weather>,
    mut sim: ResMut<Sim>,
    mut alerts: ResMut<Alerts>,
    mut mc: ResMut<McMsg>,
    modal: Res<Modal>,
    assets: Res<WorldAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    players: Query<&Player>,
    mut profile: ResMut<crate::save::Profile>,
    mut next: ResMut<NextState<AppState>>,
    mut sfx: crate::audio::Sfx,
    cfg: Res<GameConfig>,
) {
    // modals pause the world in solo play
    sim.paused = !sim.coop && modal.kind != ModalKind::None;
    let n = players.iter().count().max(1) as f32;
    sim.players_sheltered = players.iter().filter(|p| p.in_shelter).count() as f32 / n;
    sim.players_out = 1.0 - sim.players_sheltered;
    let now = time.elapsed_secs();

    let new_sol = sim.tick(time.delta_secs(), &weather);
    if new_sol {
        sfx.play("radio");
        send_forecast(&sim, &mut mc, now);
        if sim.is_night() {
            alerts.push(Alert::warn("night", "Лунная ночь: панели почти не работают", "Lunar night: panels barely work"));
        }
        alerts.push(Alert::info("hourglass", "Новый сол", "New sol"));
    }
    if let Some(kind) = sim.fire_pending(&weather) {
        match kind {
            EventKind::Flare => {
                sfx.play("alarm");
                let (ru, en) = if let Some(f) = &sim.flare {
                    (
                        format!("ВСПЫШКА! {} ({}){} — беги в убежище", f.event.label_ru, f.event.date, if f.live { " · LIVE" } else { " · архив" }),
                        format!("FLARE! {} ({}){} — run to the shelter", f.event.label_en, f.event.date, if f.live { " · LIVE" } else { " · archive" }),
                    )
                } else {
                    ("Вспышка!".into(), "Flare!".into())
                };
                alerts.push(Alert::danger_owned("sun", ru, en));
            }
            EventKind::Dust => {
                sfx.play("alarm");
                alerts.push(Alert::warn("tornado", "Пылевая буря! Панели в пыли — можно почистить", "Dust storm! Panels are dusty — you can clean them"));
            }
            EventKind::Failure => {
                sfx.play("bad");
                alerts.push(Alert::warn("wrench", "Генератор кислорода сломался", "Oxygen generator broke down"));
            }
            EventKind::Leak => {
                sfx.play("bad");
                alerts.push(Alert::warn("comet", "Микрометеорит! Утечка воздуха", "Micrometeorite! Air leak"));
            }
            EventKind::Harvest => {
                sfx.play("good");
                alerts.push(Alert::good("plant", "Урожай готов в оранжерее", "Harvest ready in the greenhouse"));
            }
            EventKind::Discovery => {
                sfx.play("good");
                for _ in 0..3 {
                    crate::world::spawn_sample(&mut commands, &assets.gem_mesh, &assets.gem_mat, sim.map, &mut sim.rng);
                }
                alerts.push(Alert::info("gem", "БИП нашёл новые образцы вокруг базы", "BEEP found new samples around the base"));
            }
            EventKind::Supply => {
                sfx.play("launch");
                spawn_cargo(&mut commands, &mut meshes, &mut materials);
                alerts.push(Alert::info("box", "Грузовой корабль садится", "Cargo ship is landing"));
            }
            EventKind::Emotion => {
                sfx.play("radio");
                alerts.push(Alert::info("speech", "Члену экипажа нужна поддержка", "A crewmate needs support"));
            }
            EventKind::Conjunction => {
                sfx.play("radio");
                alerts.push(Alert::warn("antenna_bars", "Солнце глушит связь на 2 сола", "The Sun blocks radio for 2 sols"));
            }
        }
    }
    if sim.flare.is_none() && sim.flags.flares > 0 && sim.flags.sheltered_in_flare && profile.award("shield") {
        alerts.push(Alert::badge("shield", "Нашивка: Успел в убежище", "Patch: Made it to shelter"));
        sfx.play("badge");
    }

    if sim.status != Status::Playing {
        finish(&sim, &mut profile, &mut next, &mut commands, &cfg, &mut sfx);
    }
}

fn spawn_cargo(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let body = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.9, 0.95), metallic: 0.4, ..default() });
    let flame = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.6, 0.2), emissive: LinearRgba::rgb(6.0, 2.5, 0.5), ..default() });
    commands
        .spawn((
            Transform::from_xyz(0.0, 60.0, 13.0),
            Visibility::default(),
            CargoCapsule { landing: 0.0 },
            crate::world::Station { kind: StationKind::Cargo, radius: 2.4 },
            crate::world::Obstacle { radius: 1.6 },
            StateScoped(AppState::Playing),
        ))
        .with_children(|p| {
            p.spawn((Mesh3d(meshes.add(Cylinder::new(1.2, 2.2))), MeshMaterial3d(body.clone()), Transform::from_xyz(0.0, 1.1, 0.0)));
            p.spawn((Mesh3d(meshes.add(Mesh::from(Cone { radius: 1.2, height: 1.2 }))), MeshMaterial3d(body), Transform::from_xyz(0.0, 2.8, 0.0)));
            p.spawn((Mesh3d(meshes.add(Mesh::from(Cone { radius: 0.6, height: 1.4 }))), MeshMaterial3d(flame), Transform::from_xyz(0.0, -0.6, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)), crate::world::Spin(8.0)));
        });
}

pub fn cargo_landing(time: Res<Time>, sim: Option<Res<Sim>>, mut q: Query<(Entity, &mut CargoCapsule, &mut Transform)>, mut commands: Commands) {
    let Some(sim) = sim else { return };
    for (e, mut c, mut tr) in &mut q {
        if tr.translation.y > 0.0 {
            tr.translation.y = (tr.translation.y - time.delta_secs() * 18.0).max(0.0);
            c.landing += time.delta_secs();
        }
        if !sim.cargo {
            commands.entity(e).despawn();
        }
    }
}

fn finish(sim: &Sim, profile: &mut crate::save::Profile, next: &mut NextState<AppState>, commands: &mut Commands, cfg: &GameConfig, sfx: &mut crate::audio::Sfx) {
    let won = sim.status == Status::Won;
    let stars = sim.stars();
    let points = sim.points();
    let before: Vec<MapId> = MAPS.iter().filter(|m| profile.unlocked(m.need)).map(|m| m.id).collect();
    let mut badges = vec![];
    if profile.last_failed && profile.award("comeback") {
        badges.push("comeback");
    }
    profile.last_failed = !won;
    profile.missions += 1;
    let key = format!("{:?}", sim.map);
    let best = profile.best.get(&key).copied().unwrap_or(0);
    profile.best.insert(key, best.max(stars));
    profile.add_points(points);
    if profile.award("first") {
        badges.push("first");
    }
    if won {
        if sim.science >= sim.goal && profile.award("scientist") { badges.push("scientist"); }
        if sim.map == MapId::Moon && profile.award("moon") { badges.push("moon"); }
        if sim.map == MapId::Mars && profile.award("mars") { badges.push("mars"); }
        if sim.map == MapId::Europa && profile.award("europa") { badges.push("europa"); }
        if sim.flags.storm && profile.award("storm") { badges.push("storm"); }
        if sim.flags.night && profile.award("night") { badges.push("night"); }
        if cfg.coop && profile.award("coop") { badges.push("coop"); }
    }
    if sim.flags.helped && profile.award("helper") { badges.push("helper"); }
    if sim.live && profile.award("live") { badges.push("live"); }
    crate::save::save(profile);
    let new_map = MAPS.iter().filter(|m| profile.unlocked(m.need)).map(|m| m.id).find(|id| !before.contains(id));
    sfx.play(if won { "badge" } else { "launch" });
    commands.insert_resource(MissionResult {
        map: sim.map,
        won,
        cause: if let Status::Lost(c) = sim.status { Some(c) } else { None },
        stars,
        points,
        sol: sim.sol,
        sols: sim.sols,
        science: sim.science,
        goal: sim.goal,
        dose: sim.dose,
        new_map,
        history: sim.history.clone(),
        coop: sim.coop,
        cadet: sim.cadet,
        badges,
        helped: sim.flags.helped,
        sheltered: sim.flags.sheltered_in_flare,
    });
    next.set(AppState::Debrief);
}

pub fn teardown_game(mut commands: Commands, mut modal: ResMut<Modal>) {
    commands.remove_resource::<Sim>();
    *modal = Modal::default();
}
