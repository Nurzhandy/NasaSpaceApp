//! Part A — the ship "Aurora": a top-down interior with 9 rooms, walking crew, consoles with
//! tasks, the ship HUD and the windows (mini-games, Touch Panel, physics cards, dialogs, game over).
//! Rules live in `ja_core::campaign`; this module only draws them and sends commands.

pub mod card;
pub mod decor;
pub mod npc;
pub mod pet;

use crate::art::{self, Art};
use crate::audio::Play;
use crate::kit::{self, Act, Action, Inputs, UiBusy, C_BAD, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use crate::{AppState, MainCam, Prof, Set, Setup, Speed};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use ja_core::campaign::{CampaignRefusal, REACH};
use ja_core::content::{Content, TaskKind};
use ja_core::{Campaign, CampaignCommand, CampaignEvent, ExpeditionConfig, Lang, Level, Phase};

/// pixels per ship cell
pub const SC: f32 = 44.0;
/// ship size in cells (3×3 rooms of 10×7)
pub const SHIP_W: f32 = 30.0;
pub const SHIP_H: f32 = 21.0;

/// Loaded content (`assets/content`). F5 reloads it.
#[derive(Resource, Default)]
pub struct ContentRes(pub Option<Content>);

/// The running expedition.
#[derive(Resource)]
pub struct Camp(pub Campaign);

/// What this computer shows on top of the ship.
#[derive(Resource, Default)]
pub struct ShipUi {
    /// (player, task uid) of the open mini-game / Touch Panel
    pub open: Option<(u8, u32)>,
    pub paused: bool,
    pub toasts: Vec<(String, Color, f32)>,
}

#[derive(Component)]
pub struct Crew {
    pub idx: u8,
    pub pos: Vec2,
    pub dir: usize,
    pub anim: f32,
}
#[derive(Component)]
struct CrewPrompt(u8);
#[derive(Component)]
struct Marker;
#[derive(Component)]
struct LeakMark;
#[derive(Component)]
pub struct ShipWindow;
#[derive(Component)]
struct ShipHud;
#[derive(Component)]
struct ToastBox;

pub fn to_px(x: f32, y: f32) -> Vec2 {
    Vec2::new((x - SHIP_W / 2.0) * SC, (y - SHIP_H / 2.0) * SC)
}

/// Part B's screenshot tour (JA_SHOTS without JA_SHIP_TEST) skips the flight and lands right away.
pub fn skip_flight() -> bool {
    std::env::var_os("JA_SHOTS").is_some() && std::env::var_os("JA_SHIP_TEST").is_none()
}

pub fn content_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(crate::assets_dir()).join("content")
}

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ContentRes>()
            .init_resource::<ShipUi>()
            .add_systems(Startup, load_content)
            .add_systems(Update, (start_expedition, reload_f5).after(Set::Input))
            .add_systems(OnEnter(AppState::Ship), (spawn_ship, crate::ship::reset_camera))
            .add_systems(Update, (decor::animate, decor::animate_doors).run_if(in_state(AppState::Ship)))
            .init_resource::<npc::Chatter>()
            .add_systems(OnEnter(AppState::Ship), npc::spawn.after(spawn_ship))
            .add_systems(Update, (npc::walk, npc::visor, npc::chatter, pet::sync, pet::animate, pet::hunger).chain().in_set(Set::View).run_if(in_state(AppState::Ship)).run_if(resource_exists::<Camp>))
            .add_systems(OnExit(AppState::Ship), |mut ui: ResMut<ShipUi>, mut busy: ResMut<UiBusy>| {
                *ui = ShipUi::default();
                busy.0 = false;
            })
            .add_systems(
                Update,
                (walk, interact, pet::click, crate::minigames::pointer, ship_actions, tick_campaign)
                    .chain()
                    .in_set(Set::Logic)
                    .run_if(in_state(AppState::Ship))
                    .run_if(resource_exists::<Camp>),
            )
            .add_systems(
                Update,
                (draw_crew, markers, hud, windows, crate::minigames::draw, crate::minigames::progress_bar, crate::touch_panel::update_rocks, toasts, task_timer, follow_camera)
                    .chain()
                    .in_set(Set::View)
                    .run_if(in_state(AppState::Ship))
                    .run_if(resource_exists::<Camp>),
            );
    }
}

fn load_content(mut res: ResMut<ContentRes>) {
    match ja_core::content::load(&content_dir()) {
        Ok(c) => {
            info!("content: {} tasks, {} cards, {} dialogs, {} texts without Kazakh", c.tasks.len(), c.cards.len(), c.dialogs.len(), ja_core::content::missing_kk(&c).len());
            res.0 = Some(c);
        }
        Err(errs) => {
            for e in &errs {
                error!("content: {e}");
            }
        }
    }
}

/// Developer key: reload `assets/content` without restarting.
fn reload_f5(keys: Res<ButtonInput<KeyCode>>, mut res: ResMut<ContentRes>, camp: Option<ResMut<Camp>>, mut ui: ResMut<ShipUi>) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }
    match ja_core::content::load(&content_dir()) {
        Ok(c) => {
            if let Some(mut camp) = camp {
                camp.0.reload_content(&c);
            }
            ui.toasts.push(("Content reloaded (F5)".into(), C_GOOD, 3.0));
            res.0 = Some(c);
        }
        Err(errs) => {
            let first = errs.first().map(|e| e.to_string()).unwrap_or_default();
            error!("content reload: {} errors, first: {first}", errs.len());
            ui.toasts.push((format!("Content error: {first}"), C_BAD, 6.0));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn start_expedition(mut commands: Commands, mut acts: EventReader<Act>, content: Res<ContentRes>, setup: Res<Setup>, prof: Res<Prof>, mut next: ResMut<NextState<AppState>>, mut play: EventWriter<Play>, net: Res<crate::net::Net>) {
    for Act(a) in acts.read() {
        if *a != Action::StartExpedition {
            continue;
        }
        let Some(c) = &content.0 else {
            error!("no content: can't start the expedition");
            continue;
        };
        if skip_flight() {
            continue;
        }
        // the choices from part B's "new expedition" screen
        let level = if prof.0.cadet { Level::Junior } else { Level::Advanced };
        let crew = match net.crew() {
            0 if setup.coop => 2,
            0 => 1,
            n => n,
        };
        let mut cfg = ExpeditionConfig::new(crew, level).with_auto_roles();
        if setup.mode == ja_core::Mode::Sandbox {
            cfg.mode = ja_core::PlayMode::Creative;
        }
        commands.insert_resource(Camp(Campaign::new(cfg, c)));
        let _ = (&mut next, &mut play); // part B's menu switches the state and plays the sound
    }
}

pub fn reset_camera(mut cam: Query<(&mut Transform, &mut Projection), With<MainCam>>) {
    if let Ok((mut t, mut p)) = cam.single_mut() {
        // the camera starts on the crew in the living quarters and then follows (follow_camera)
        let at = to_px(5.0, 10.5);
        t.translation = Vec3::new(at.x, at.y, t.translation.z);
        if let Projection::Orthographic(o) = &mut *p {
            o.scale = 1.0;
        }
    }
}

// ---------------------------------------------------------------- the ship interior

fn spawn_ship(mut commands: Commands, camp: Res<Camp>, art: Res<Art>, prof: Res<Prof>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<ColorMaterial>>) {
    let sc = StateScoped(AppState::Ship);
    let c = camp.0.content();
    let Some(layout) = &c.ship else { return };
    let l = prof.lang();
    // space flying past, the hull, the rooms with furniture, the walls
    decor::spawn(&mut commands, layout, &mut meshes, &mut mats, &art, l);
    // consoles: a bright frame, a dark screen and a big clear picture of the job
    for (i, r) in layout.rooms.iter().enumerate() {
        for k in &r.consoles {
            let p = to_px(k.x as f32 + 0.5, k.y as f32 + 0.5);
            commands.spawn((Sprite { color: Color::srgba(1.0, 0.85, 0.3, 0.25), custom_size: Some(Vec2::splat(SC * 1.55)), ..default() }, Transform::from_xyz(p.x, p.y, -0.95), decor::Blink { speed: 1.2, phase: i as f32, base: 0.3 }, sc.clone()));
            commands.spawn((Sprite { color: Color::srgb(1.0, 0.82, 0.3), custom_size: Some(Vec2::splat(SC * 1.32)), ..default() }, Transform::from_xyz(p.x, p.y, -0.9), sc.clone()));
            commands.spawn((Sprite { color: Color::srgb(0.07, 0.09, 0.2), custom_size: Some(Vec2::splat(SC * 1.2)), ..default() }, Transform::from_xyz(p.x, p.y, -0.89), sc.clone()));
            commands.spawn((Sprite { image: art.icon(&k.icon), custom_size: Some(Vec2::splat(SC * 1.08)), ..default() }, Transform::from_xyz(p.x, p.y, -0.8), sc.clone()));
        }
    }
    // the crew
    let st = camp.0.state();
    for (i, m) in st.crew.iter().enumerate() {
        let (suit, visor) = if i == 0 { (art::suit_color(&prof.0.suit), art::visor_color(&prof.0.visor)) } else { ([Color::srgb(0.5, 0.75, 1.0), Color::srgb(0.6, 1.0, 0.6), Color::srgb(1.0, 0.7, 0.5)][(i - 1) % 3], Color::srgb(1.0, 0.8, 0.3)) };
        let p = to_px(m.pos.0, m.pos.1);
        let size = Vec2::new(16.0, 24.0) * 2.6;
        commands
            .spawn((
                Sprite { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: suit, custom_size: Some(size), anchor: Anchor::BottomCenter, ..default() },
                Transform::from_xyz(p.x, p.y, 1.0),
                Crew { idx: i as u8, pos: Vec2::new(m.pos.0, m.pos.1), dir: 0, anim: 0.0 },
                sc.clone(),
            ))
            .with_children(|c| {
                c.spawn((Sprite { image: art.sprite("astronaut_visor"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: visor, custom_size: Some(size), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(0.0, 0.0, 0.01)));
            });
        commands.spawn((
            Text2d::new(""),
            TextFont { font: art.body.clone(), font_size: 17.0, ..default() },
            TextColor(Color::WHITE),
            Transform::from_xyz(p.x, p.y + 64.0, 5.0),
            CrewPrompt(i as u8),
            sc.clone(),
        ));
    }
    // HUD and toasts
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, ShipHud, GlobalZIndex(10), sc.clone()));
    commands.spawn((Node { position_type: PositionType::Absolute, right: Val::Px(12.0), bottom: Val::Px(60.0), width: Val::Px(380.0), flex_direction: FlexDirection::ColumnReverse, row_gap: Val::Px(6.0), ..default() }, ToastBox, GlobalZIndex(30), sc));
}

// ---------------------------------------------------------------- walking and consoles

fn walk(time: Res<Time>, inputs: Res<Inputs>, ui: Res<ShipUi>, mut camp: ResMut<Camp>, mut q: Query<&mut Crew>, net: Res<crate::net::Net>) {
    let dt = time.delta_secs().min(0.05);
    let Some(layout) = camp.0.content().ship.clone() else { return };
    for mut c in &mut q {
        if net.is_remote(c.idx) {
            // another computer walks this one: glide to where the network says
            if let Some(t) = camp.0.state().crew.get(c.idx as usize).map(|k| Vec2::new(k.pos.0, k.pos.1)) {
                let d = t - c.pos;
                if d.length() > 0.03 {
                    c.dir = if d.x.abs() > d.y.abs() { if d.x < 0.0 { 1 } else { 2 } } else if d.y > 0.0 { 3 } else { 0 };
                    c.anim += dt * 9.0;
                    c.pos += d * (12.0 * dt).min(1.0);
                } else {
                    c.anim = 0.0;
                }
            }
            continue;
        }
        let Some(pad) = inputs.0.get(c.idx as usize).copied() else { continue };
        let busy = ui.paused || ui.open.is_some_and(|o| o.0 == c.idx) || camp.0.frozen();
        if busy || pad.mv.length() < 0.1 {
            c.anim = 0.0;
            continue;
        }
        let mv = pad.mv;
        c.dir = if mv.x.abs() > mv.y.abs() { if mv.x < 0.0 { 1 } else { 2 } } else if mv.y > 0.0 { 3 } else { 0 };
        c.anim += dt * 9.0;
        let to = c.pos + mv * 4.2 * dt;
        let p = ja_core::ship::step(&layout, (c.pos.x, c.pos.y), (to.x, to.y));
        c.pos = Vec2::new(p.0, p.1);
        camp.0.set_crew_pos(c.idx, p.0, p.1);
    }
}

/// The task nearest to this crew member that they can work on.
fn nearest_task(camp: &Campaign, pos: Vec2) -> Option<(u32, f32)> {
    let st = camp.state();
    st.tasks
        .iter()
        .filter_map(|t| camp.console_pos(&t.room, &t.console).map(|p| (t.uid, (pos.x - p.0).hypot(pos.y - p.1))))
        .filter(|(_, d)| *d <= REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

#[allow(clippy::too_many_arguments)]
fn interact(inputs: Res<Inputs>, mut ui: ResMut<ShipUi>, mut camp: ResMut<Camp>, q: Query<&Crew>, prof: Res<Prof>, mut play: EventWriter<Play>, npcs: Query<&npc::Npc>, pets: Query<&pet::PetBody>) {
    if ui.paused || camp.0.frozen() {
        return;
    }
    for c in &q {
        let Some(pad) = inputs.0.get(c.idx as usize) else { continue };
        if !pad.act || ui.open.is_some() {
            continue;
        }
        let uid = match what_e(&camp.0, c.pos, &npcs, &pets, prof.lang()) {
            Some((EAct::Task(uid), _)) => uid,
            Some((act, _)) => {
                let l = prof.lang();
                let (cmd, msg) = match act {
                    EAct::Talk(n) => (CampaignCommand::Talk { npc: n }, None),
                    EAct::Feed => (CampaignCommand::FeedPet, None),
                    EAct::Adopt(k) => (CampaignCommand::AdoptPet { kind: k }, Some(l.pick("Теперь у тебя есть питомец! Он ходит за тобой. E рядом с ним — «сидеть».", "You have a pet now! It follows you. Press E next to it to make it stay."))),
                    _ => (CampaignCommand::PetToggle, None),
                };
                if let Ok(ev) = camp.0.apply(c.idx, cmd) {
                    play.write(Play("click"));
                    if let Some(m) = msg {
                        ui.toasts.push((m.into(), C_GOOD, 4.0));
                    }
                    if act == EAct::Toggle {
                        let follow = camp.0.state().pet.as_ref().is_some_and(|p| p.follow);
                        ui.toasts.push((if follow { l.pick("Питомец идёт за тобой", "Your pet follows you") } else { l.pick("Питомец сидит и ждёт", "Your pet stays here") }.into(), C_GOOD, 2.5));
                    }
                    // "feed the pet": open the job right away
                    for e in ev {
                        if let CampaignEvent::TaskOffered { uid, .. } = e {
                            if camp.0.apply(c.idx, CampaignCommand::OpenTask { uid }).is_ok() {
                                ui.open = Some((c.idx, uid));
                            }
                        }
                    }
                }
                continue;
            }
            None => continue,
        };
        match camp.0.apply(c.idx, CampaignCommand::OpenTask { uid }) {
            Ok(ev) => {
                ui.open = Some((c.idx, uid));
                play.write(Play("click"));
                for e in ev {
                    if let CampaignEvent::WaveStarted(n) = e {
                        let l = prof.lang();
                        ui.toasts.push((format!("{} {n}", l.pick("Волна камней", "Rock wave")), C_WARN, 3.0));
                    }
                }
            }
            Err(r) => {
                let l = prof.lang();
                let s = match r {
                    CampaignRefusal::Busy => l.pick("Здесь уже работает другой астронавт", "Another astronaut is working here"),
                    CampaignRefusal::TooFar => l.pick("Подойди ближе", "Walk closer"),
                    _ => l.pick("Сейчас нельзя", "Not now"),
                };
                ui.toasts.push((s.into(), C_WARN, 2.5));
                play.write(Play("bad"));
            }
        }
    }
}

/// Buttons in the ship windows.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn ship_actions(mut acts: EventReader<Act>, mut camp: ResMut<Camp>, mut ui: ResMut<ShipUi>, mut next: ResMut<NextState<AppState>>, mut setup: ResMut<Setup>, prof: Res<Prof>, mut play: EventWriter<Play>, mut busy: ResMut<UiBusy>, inputs: Res<Inputs>, net: Res<crate::net::Net>) {
    let player = ui.open.map(|o| o.0).unwrap_or(0);
    let mut evs = vec![];
    // Esc: close the open window, or pause
    let back = inputs.0.iter().any(|p| p.back || p.pause);
    if back {
        if let Some((p, uid)) = ui.open.take() {
            let _ = camp.0.apply(p, CampaignCommand::CloseTask { uid });
        } else if camp.0.state().dialog.is_some() {
            let _ = camp.0.apply(0, CampaignCommand::DialogSkip);
        } else {
            ui.paused = !ui.paused;
            camp.0.set_paused(ui.paused);
        }
    }
    for Act(a) in acts.read() {
        let r = match *a {
            Action::Mini(uid, input) => camp.0.apply(player, CampaignCommand::Mini { uid, input }),
            Action::CloseTask => {
                if let Some((p, uid)) = ui.open.take() {
                    let _ = camp.0.apply(p, CampaignCommand::CloseTask { uid });
                }
                continue;
            }
            Action::DialogNext => camp.0.apply(0, CampaignCommand::DialogNext),
            Action::DialogChoose(i) => camp.0.apply(0, CampaignCommand::DialogChoose(i)),
            Action::DialogSkip => camp.0.apply(0, CampaignCommand::DialogSkip),
            Action::CloseCard => camp.0.apply(0, CampaignCommand::CloseCard),
            Action::CloseBreather => camp.0.apply(0, CampaignCommand::CloseBreather),
            Action::CloseLesson => camp.0.apply(0, CampaignCommand::CloseLesson),
            Action::RestartDay => camp.0.apply(0, CampaignCommand::RestartDay),
            Action::NextDay => camp.0.apply(0, CampaignCommand::NextDay),
            Action::ShipResume => {
                ui.paused = false;
                camp.0.set_paused(false);
                continue;
            }
            Action::ShipQuit => {
                next.set(AppState::Menu);
                continue;
            }
            Action::ToMoon if camp.0.is_mirror() || net.local_idx().is_some_and(|i| i != 0) => {
                // only the host flies the lander; the others follow when the base starts
                play.write(Play("bad"));
                continue;
            }
            Action::ToMoon => {
                // into the lander: part B's landing, then survival on the Moon
                let _ = camp.0.apply(0, CampaignCommand::Landed);
                setup.map = ja_core::MapId::Moon;
                setup.coop = camp.0.state().cfg.players > 1;
                setup.landing_coins = 0;
                play.write(Play("launch"));
                next.set(AppState::Landing);
                continue;
            }
            _ => continue,
        };
        match r {
            Ok(e) => evs.extend(e),
            Err(_) => {
                play.write(Play("bad"));
            }
        }
    }
    report(&evs, &mut ui, &camp.0, prof.lang(), &mut play);
    busy.0 = ui.open.is_some() || ui.paused || camp.0.frozen() || !matches!(camp.0.state().phase, Phase::Ship { .. });
}

fn tick_campaign(time: Res<Time>, speed: Res<Speed>, mut camp: ResMut<Camp>, mut ui: ResMut<ShipUi>, prof: Res<Prof>, mut play: EventWriter<Play>) {
    let dt = time.delta_secs().min(0.1) * speed.0;
    let steps = (dt / 0.1).ceil().max(1.0) as u32;
    let mut evs = vec![];
    for _ in 0..steps {
        evs.extend(camp.0.tick(dt / steps as f32));
    }
    // a finished or failed task closes its window
    if let Some((_, uid)) = ui.open {
        if !camp.0.state().tasks.iter().any(|t| t.uid == uid) {
            ui.open = None;
        }
    }
    report(&evs, &mut ui, &camp.0, prof.lang(), &mut play);
}

/// Sounds and short messages for what happened.
fn report(evs: &[CampaignEvent], ui: &mut ShipUi, camp: &Campaign, l: Lang, play: &mut EventWriter<Play>) {
    let c = camp.content();
    let title = |id: &str| c.task(id).map(|t| t.title.get(l).to_string()).unwrap_or_default();
    for e in evs {
        let (msg, col, snd): (Option<String>, Color, &'static str) = match e {
            CampaignEvent::DayStarted(d) => (c.day(*d).map(|x| x.title.get(l).to_string()), kit::C_ACCENT, "radio"),
            CampaignEvent::TaskOffered { id, .. } => (Some(format!("{} {}", l.pick("Новое задание:", "New task:"), title(id))), C_TEXT, "radio"),
            CampaignEvent::ProblemStarted { id, .. } => (Some(format!("{} {}", l.pick("СРОЧНО:", "URGENT:"), title(id))), C_BAD, "alarm"),
            CampaignEvent::TaskDone { id, coins, .. } => {
                info!("TASK done {id}");
                (Some(format!("{} · +{coins}", title(id))), C_GOOD, "coin")
            }
            CampaignEvent::TaskFailed { id, .. } => (Some(c.task(id).and_then(|t| t.advice.as_ref()).map(|a| a.get(l).to_string()).unwrap_or_else(|| format!("{} {}", l.pick("Не успели:", "Missed:"), title(id)))), C_WARN, "bad"),
            CampaignEvent::AsteroidHit { room } => (Some(match room {
                Some(r) => format!("{} {}", l.pick("Удар! Пробоина:", "Hit! Hole in:"), c.room(r).map(|x| x.name.get(l)).unwrap_or("")),
                None => l.pick("Удар по корпусу!", "Hull hit!").into(),
            }), C_BAD, "bad"),
            CampaignEvent::WaveOver { hits } => (Some(format!("{} {hits}", l.pick("Волна пройдена. Попаданий:", "Wave over. Hits:"))), C_GOOD, "good"),
            CampaignEvent::Mistake => {
                info!("MINI mistake");
                (None, C_TEXT, "bad")
            }
            CampaignEvent::GameOver(_) => (None, C_TEXT, "launch"),
            CampaignEvent::Card(_) | CampaignEvent::Breather(_) => (None, C_TEXT, "badge"),
            CampaignEvent::Landing => (None, C_TEXT, "launch"),
            _ => (None, C_TEXT, ""),
        };
        if let Some(m) = msg {
            ui.toasts.push((m, col, 4.0));
        }
        if !snd.is_empty() {
            play.write(Play(snd));
        }
    }
}

// ---------------------------------------------------------------- drawing

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_crew(camp: Res<Camp>, mut q: Query<(&Crew, &mut Transform, &mut Sprite, &Children), (Without<CrewPrompt>, Without<npc::Npc>)>, mut layers: Query<&mut Sprite, (Without<Crew>, Without<CrewPrompt>, Without<npc::Npc>)>, mut prompts: Query<(&CrewPrompt, &mut Transform, &mut Text2d), (Without<Crew>, Without<npc::Npc>)>, ui: Res<ShipUi>, prof: Res<Prof>, npcs: Query<&npc::Npc>, pets: Query<&pet::PetBody>) {
    let l = prof.lang();
    for (c, mut tr, mut s, children) in &mut q {
        let frame = if c.anim > 0.0 { (c.anim as usize) % 4 } else { 0 };
        let idx = c.dir * 4 + frame;
        if let Some(a) = s.texture_atlas.as_mut() {
            a.index = idx;
        }
        for ch in children.iter() {
            if let Ok(mut v) = layers.get_mut(ch) {
                if let Some(a) = v.texture_atlas.as_mut() {
                    a.index = idx;
                }
            }
        }
        let p = to_px(c.pos.x, c.pos.y - 0.4);
        tr.translation = Vec3::new(p.x, p.y, 1.0 - c.pos.y * 0.001);
        for (cp, mut ptr, mut text) in &mut prompts {
            if cp.0 != c.idx {
                continue;
            }
            ptr.translation = Vec3::new(p.x, p.y + 82.0, 5.0);
            let show = !ui.open.is_some_and(|o| o.0 == c.idx);
            text.0 = if show { what_e(&camp.0, c.pos, &npcs, &pets, l).map(|x| x.1).unwrap_or_default() } else { String::new() };
            if c.idx == 1 && !text.0.is_empty() {
                text.0 = text.0.replacen("[E]", "[Shift]", 1);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum EAct {
    Task(u32),
    Talk(u8),
    Feed,
    Adopt(u8),
    Toggle,
}

/// What E does here, and the prompt for it: a job first, then talking to the crew, feeding the
/// pet at its corner, adopting a pet, telling your pet to stay / follow.
fn what_e(camp: &Campaign, pos: Vec2, npcs: &Query<&npc::Npc>, pets: &Query<&pet::PetBody>, l: Lang) -> Option<(EAct, String)> {
    let st = camp.state();
    let c = camp.content();
    if let Some((uid, _)) = nearest_task(camp, pos) {
        let name = st.tasks.iter().find(|t| t.uid == uid).and_then(|t| c.task(&t.id)).map(|d| d.title.get(l).to_string()).unwrap_or_default();
        return Some((EAct::Task(uid), format!("[E] {name}")));
    }
    if let Some(n) = npc::near(npcs, pos) {
        let name = c.npcs.get(n as usize).map(|x| x.name.get(l).to_string()).unwrap_or_default();
        return Some((EAct::Talk(n), format!("[E] {} {name}", l.pick("Поговорить:", "Talk:"))));
    }
    let reach = ja_core::campaign::PET_REACH;
    match &st.pet {
        None => camp.pet_spots().iter().position(|s| pos.distance(Vec2::new(s.0, s.1)) < reach).map(|k| {
            let (_, ru, en) = ja_core::minigame::PETS[k];
            (EAct::Adopt(k as u8), format!("[E] {} {}", l.pick("Взять питомца:", "Adopt:"), l.pick(ru, en)))
        }),
        Some(p) => {
            let bowl = camp.console_pos("habitat", "petbed").map(|b| pos.distance(Vec2::new(b.0, b.1)) < ja_core::campaign::REACH).unwrap_or(false);
            if bowl && !st.tasks.iter().any(|t| t.id == "pet_feed") {
                return Some((EAct::Feed, format!("[E] {}", l.pick("Покормить питомца", "Feed the pet"))));
            }
            let at = pet::pet_pos(pets, p.kind).unwrap_or(Vec2::new(p.at.0, p.at.1));
            (pos.distance(at) < reach + 0.4).then(|| (EAct::Toggle, if p.follow { format!("[E] {}", l.pick("Питомец: сидеть", "Pet: stay")) } else { format!("[E] {}", l.pick("Питомец: за мной!", "Pet: follow me!")) }))
        }
    }
}

/// Task markers over consoles and holes in leaking rooms (rebuilt when they change).
fn markers(mut commands: Commands, camp: Res<Camp>, art: Res<Art>, old: Query<Entity, Or<(With<Marker>, With<LeakMark>)>>, time: Res<Time>, mut key: Local<String>, mut bob: Query<&mut Transform, With<Marker>>) {
    let st = camp.0.state();
    let k = format!("{:?}{:?}", st.tasks.iter().map(|t| (t.uid, &t.room)).collect::<Vec<_>>(), st.ship.leaks);
    let t = time.elapsed_secs();
    if *key == k {
        for mut tr in &mut bob {
            tr.translation.y += (t * 4.0).sin() * 0.3;
        }
        return;
    }
    *key = k;
    for e in &old {
        commands.entity(e).despawn();
    }
    for task in &st.tasks {
        let Some(p) = camp.0.console_pos(&task.room, &task.console) else { continue };
        let p = to_px(p.0, p.1 + 0.9);
        let (icon, col) = match task.kind {
            TaskKind::Problem => ("warning", Color::srgb(1.0, 0.45, 0.4)),
            TaskKind::Asteroids => ("comet", Color::srgb(1.0, 0.8, 0.4)),
            TaskKind::Story => ("star", Color::srgb(1.0, 0.9, 0.4)),
            TaskKind::Routine => ("hint", Color::WHITE),
        };
        commands.spawn((Sprite { image: art.icon(icon), color: col, custom_size: Some(Vec2::splat(38.0)), ..default() }, Transform::from_xyz(p.x, p.y + 6.0, 3.0), Marker, StateScoped(AppState::Ship)));
    }
    for room in &st.ship.leaks {
        if let Some(r) = camp.0.content().room(room) {
            let p = to_px(r.x as f32 + r.w as f32 - 1.0, r.y as f32 + 1.0);
            commands.spawn((Sprite { image: art.sprite("leak"), custom_size: Some(Vec2::splat(40.0)), ..default() }, Transform::from_xyz(p.x, p.y, 2.0), LeakMark, StateScoped(AppState::Ship)));
        }
    }
}

fn bar(p: &mut ChildSpawnerCommands, art: &Art, icon: &str, label: &str, frac: f32, value: String, color: Color) {
    p.spawn(kit::row(8.0)).with_children(|r| {
        kit::icon(r, art, icon, 26.0);
        r.spawn(kit::col(2.0)).with_children(|c| {
            kit::text(c, label, &art.body, 13.0, C_MUTED);
            c.spawn((Node { width: Val::Px(140.0), height: Val::Px(10.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)))).with_children(|b| {
                b.spawn((Node { width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(color)));
            });
        });
        kit::text(r, value, &art.head, 15.0, C_TEXT);
    });
}

/// Ship systems (left), the day and coins (top), the task list (right). Rebuilt twice a second.
fn hud(mut commands: Commands, camp: Res<Camp>, art: Res<Art>, prof: Res<Prof>, root: Query<Entity, With<ShipHud>>, time: Res<Time>, mut t: Local<f32>) {
    *t += time.delta_secs();
    if *t < 0.5 {
        return;
    }
    *t = 0.0;
    let Ok(root) = root.single() else { return };
    let l = prof.lang();
    let st = camp.0.state();
    let c = camp.0.content();
    let adv = st.cfg.level == Level::Advanced;
    let s = &st.ship;
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|p| {
        // systems
        p.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(10.0), top: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() }, BackgroundColor(kit::C_BG), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
            kit::text(b, c.ship.as_ref().map(|x| x.name.get(l).split(" (").next().unwrap_or("").to_string()).unwrap_or_default(), &art.head, 16.0, kit::C_ACCENT);
            let col = |v: f32| if v < 30.0 { C_BAD } else if v < 60.0 { C_WARN } else { C_GOOD };
            bar(b, &art, "shield", l.pick("Прочность корпуса", "Hull strength"), s.hull / 100.0, format!("{:.0}%", s.hull), col(s.hull));
            bar(b, &art, "house", l.pick("Герметичность", "Airtightness"), s.seal / 100.0, format!("{:.0}%", s.seal), col(s.seal));
            if adv {
                bar(b, &art, "wind", l.pick("Давление", "Pressure"), s.pressure / 101.3, format!("{:.1} кПа", s.pressure), col(s.pressure / 1.013));
            }
            bar(b, &art, "oxygen", l.pick("Кислород", "Oxygen"), s.o2 / 100.0, format!("{:.0}%", s.o2), col(s.o2));
            bar(b, &art, "warning", "CO₂", s.co2 / 100.0, format!("{:.0}%", s.co2), col(100.0 - s.co2));
            if adv {
                // energy (stored, kWh) and power (made / used, kW) are different quantities
                bar(b, &art, "battery", l.pick("Энергия (запас)", "Energy (stored)"), s.energy / ja_core::ship::ENERGY_CAP, format!("{:.0} кВт·ч", s.energy), col(s.energy / ja_core::ship::ENERGY_CAP * 100.0));
                kit::text(b, format!("{} {:.1} / {:.1} кВт", l.pick("Мощность: даём / тратим", "Power: made / used"), s.power_in, s.power_out), &art.body, 13.0, C_MUTED);
            } else {
                bar(b, &art, "battery", l.pick("Энергия", "Energy"), s.energy / ja_core::ship::ENERGY_CAP, format!("{:.0}%", s.energy / ja_core::ship::ENERGY_CAP * 100.0), col(s.energy / ja_core::ship::ENERGY_CAP * 100.0));
            }
            // crew roles: each gives the whole team a small bonus
            for (i, r) in st.cfg.roles.iter().enumerate().take(st.cfg.players as usize) {
                let Some(r) = r else { continue };
                let (icon, ru, en, bru, ben) = match r {
                    ja_core::Role::Pilot => ("rocket", "пилот", "pilot", "удары астероидов слабее", "asteroid hits are softer"),
                    ja_core::Role::Engineer => ("wrench", "инженер", "engineer", "больше времени на ремонт", "more time for repairs"),
                    ja_core::Role::Researcher => ("science", "исследователь", "researcher", "+50% монет в лаборатории", "+50% coins in the lab"),
                    ja_core::Role::LifeSupport => ("oxygen", "жизнеобеспечение", "life support", "воздух портится медленнее", "air goes bad slower"),
                };
                b.spawn(kit::row(6.0)).with_children(|x| {
                    kit::icon(x, &art, icon, 18.0);
                    kit::text(x, format!("{} {} — {}:", l.pick("Игрок", "Player"), i + 1, l.pick(ru, en)), &art.head, 12.0, crate::net::seat_color(i as u8));
                    kit::text(x, l.pick(bru, ben), &art.body, 12.0, C_MUTED);
                });
            }
        });
        // the crew's log of laws and the pet
        p.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(10.0), top: Val::Px(470.0), padding: UiRect::all(Val::Px(10.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() }, BackgroundColor(kit::C_BG), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
            b.spawn(kit::row(6.0)).with_children(|r| {
                kit::icon(r, &art, "book", 22.0);
                let total = c.tasks.iter().filter(|t| t.law.is_some()).count();
                kit::text(r, format!("{} {} / {}", l.pick("Журнал законов:", "Law log:"), st.laws.len(), total), &art.head, 14.0, kit::C_ACCENT);
            });
            if let Some(pet) = &st.pet {
                let (icon, ru, en) = ja_core::minigame::PETS[pet.kind as usize % 3];
                b.spawn(kit::row(6.0)).with_children(|r| {
                    kit::icon(r, &art, icon, 24.0);
                    kit::text(r, l.pick(ru, en), &art.body, 14.0, C_TEXT);
                    let col = if pet.mood >= 50.0 { C_GOOD } else { C_WARN };
                    kit::icon(r, &art, "heart", 16.0);
                    kit::text(r, format!("{:.0}%", pet.mood), &art.head, 14.0, col);
                });
                if pet.mood < 50.0 {
                    kit::text(b, l.pick("Питомец голоден — бонус не работает", "The pet is hungry — no bonus"), &art.body, 12.0, C_WARN);
                } else {
                    kit::text(b, l.pick("Питомец рад: +2 монеты за задание", "Happy pet: +2 coins per task"), &art.body, 12.0, C_MUTED);
                }
            }
        });
        // day + coins
        let day = match st.phase {
            Phase::Ship { day } => c.day(day).map(|d| d.title.get(l).to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        p.spawn((Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), margin: UiRect::left(Val::Px(-200.0)), width: Val::Px(400.0), top: Val::Px(8.0), justify_content: JustifyContent::Center, column_gap: Val::Px(14.0), padding: UiRect::all(Val::Px(8.0)), ..default() }, BackgroundColor(kit::C_BG), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
            kit::text(b, day, &art.head, 18.0, C_TEXT);
            kit::icon(b, &art, "medal", 24.0);
            kit::text(b, st.wallet.team.to_string(), &art.head, 18.0, kit::C_ACCENT);
        });
        // tasks
        p.spawn((Node { position_type: PositionType::Absolute, right: Val::Px(10.0), top: Val::Px(10.0), width: Val::Px(300.0), padding: UiRect::all(Val::Px(10.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() }, BackgroundColor(kit::C_BG), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
            kit::text(b, l.pick("Задания", "Tasks"), &art.head, 16.0, kit::C_ACCENT);
            if st.tasks.is_empty() {
                kit::text(b, l.pick("Пока всё спокойно", "All calm for now"), &art.body, 14.0, C_MUTED);
            }
            for task in &st.tasks {
                let Some(d) = c.task(&task.id) else { continue };
                let room = c.room(&task.room).map(|r| r.name.get(l)).unwrap_or("");
                let col = if task.kind == TaskKind::Problem { C_BAD } else { C_TEXT };
                b.spawn(kit::col(2.0)).with_children(|r| {
                    kit::wrap_text(r, d.title.get(l), &art.body, 15.0, col, 280.0);
                    let timer = task.t_left.map(|x| format!(" · {:.0} {}", x.max(0.0), l.pick("с", "s"))).unwrap_or_default();
                    kit::text(r, format!("{room}{timer}"), &art.body, 12.0, C_MUTED);
                    if let (Some(a), Some(b2)) = (task.t_left, task.t_total) {
                        r.spawn((Node { width: Val::Px(270.0), height: Val::Px(5.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)))).with_children(|x| {
                            x.spawn((Node { width: Val::Percent((a / b2).clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_BAD)));
                        });
                    }
                });
            }
        });
        // keys
        p.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(10.0), bottom: Val::Px(10.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() }, BackgroundColor(Color::srgba(0.05, 0.06, 0.16, 0.75)), BorderRadius::all(Val::Px(6.0)))).with_children(|b| {
            kit::text(b, l.pick("WASD — идти · E — работать · Esc — пауза · F5 — обновить контент", "WASD — walk · E — work · Esc — pause · F5 — reload content"), &art.body, 14.0, C_MUTED);
        });
    });
}

#[derive(Component)]
struct TaskTimer;

/// The open problem's countdown, updated in place.
fn task_timer(camp: Res<Camp>, ui: Res<ShipUi>, mut q: Query<&mut Text, With<TaskTimer>>) {
    let Some(x) = ui.open.and_then(|(_, uid)| camp.0.state().tasks.iter().find(|t| t.uid == uid)).and_then(|t| t.t_left) else { return };
    for mut t in &mut q {
        let s = format!("{:.0}", x.max(0.0));
        if t.0 != s {
            t.0 = s;
        }
    }
}

/// The camera follows this computer's astronaut through the ship.
fn follow_camera(time: Res<Time>, crew: Query<&Crew>, net: Res<crate::net::Net>, mut cam: Query<&mut Transform, With<MainCam>>, ui: Res<ShipUi>) {
    let me = net.local_idx().unwrap_or(0);
    let Some(c) = crew.iter().find(|c| c.idx == me) else { return };
    let Ok(mut t) = cam.single_mut() else { return };
    let mut want = to_px(c.pos.x, c.pos.y);
    // keep the view inside the ship (with the wings and the nose a little visible)
    let (hx, hy) = (SHIP_W * SC / 2.0 - 300.0, SHIP_H * SC / 2.0 - 160.0);
    want.x = want.x.clamp(-hx, hx);
    want.y = want.y.clamp(-hy - 60.0, hy + 60.0);
    let speed = if ui.open.is_some() { 2.0 } else { 6.0 };
    let cur = t.translation.truncate();
    let n = cur + (want - cur) * (speed * time.delta_secs()).min(1.0);
    t.translation.x = n.x;
    t.translation.y = n.y;
}

fn toasts(mut commands: Commands, time: Res<Time>, mut ui: ResMut<ShipUi>, art: Res<Art>, root: Query<Entity, With<ToastBox>>, mut shown: Local<usize>) {
    let dt = time.delta_secs();
    let before = ui.toasts.len();
    for t in ui.toasts.iter_mut() {
        t.2 -= dt;
    }
    ui.toasts.retain(|t| t.2 > 0.0);
    while ui.toasts.len() > 5 {
        ui.toasts.remove(0);
    }
    if ui.toasts.len() == before && before == *shown {
        return;
    }
    *shown = ui.toasts.len();
    let Ok(root) = root.single() else { return };
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|p| {
        for (s, c, _) in ui.toasts.iter().rev() {
            p.spawn((Node { padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)), border: UiRect::left(Val::Px(4.0)), ..default() }, BackgroundColor(kit::C_BG), BorderColor(*c), BorderRadius::all(Val::Px(6.0)))).with_children(|b| {
                kit::wrap_text(b, s.clone(), &art.body, 16.0, C_TEXT, 350.0);
            });
        }
    });
}

// ---------------------------------------------------------------- windows

/// One window at a time, by priority: game over, landing, dialog, card, breather, pause, task.
#[allow(clippy::too_many_arguments)]
fn windows(mut commands: Commands, camp: Res<Camp>, ui: Res<ShipUi>, art: Res<Art>, prof: Res<Prof>, old: Query<Entity, With<ShipWindow>>, mut key: Local<String>, mut focus: ResMut<kit::Focus>) {
    let st = camp.0.state();
    let l = prof.lang();
    let open_task = ui.open.and_then(|(_, uid)| st.tasks.iter().find(|t| t.uid == uid));
    let k = format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{}|{:?}|{:?}|{}",
        st.phase,
        st.dialog,
        st.card,
        st.breather,
        st.lesson,
        ui.paused,
        open_task.map(|t| (t.uid, t.mini.as_ref().map(|m| m.mistakes))),
        open_task.map(|t| t.t_left.is_some()),
        l == Lang::En
    );
    if *key == k {
        return;
    }
    let same = key.split('|').next() == k.split('|').next() && open_task.is_some();
    *key = k;
    for e in &old {
        commands.entity(e).despawn();
    }
    if !same {
        focus.idx = 0;
    }
    let level = st.cfg.level;
    let c = camp.0.content();
    let overlay = |commands: &mut Commands, dim: f32, bottom: bool| -> Entity {
        commands
            .spawn((
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: if bottom { AlignItems::FlexEnd } else { AlignItems::Center }, padding: UiRect::all(Val::Px(20.0)), ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.05, dim)),
                GlobalZIndex(20),
                ShipWindow,
                StateScoped(AppState::Ship),
            ))
            .id()
    };
    if let Phase::GameOver(f) = st.phase {
        let root = overlay(&mut commands, 0.75, false);
        commands.entity(root).with_children(|p| {
            p.spawn(kit::panel()).insert(Node { width: Val::Px(620.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(14.0), padding: UiRect::all(Val::Px(22.0)), border: UiRect::all(Val::Px(3.0)), align_items: AlignItems::Center, ..default() }).with_children(|b| {
                kit::icon(b, &art, "sos", 64.0);
                kit::text(b, l.pick("Корабль не выдержал", "The ship couldn't hold"), &art.head, 30.0, C_BAD);
                kit::text(b, l.pick("Экипаж эвакуирован — все целы.", "The crew was evacuated — everyone is safe."), &art.body, 20.0, C_TEXT);
                let why = match f {
                    ja_core::ship::ShipFail::Hull => l.pick("Корпус разрушен ударами камней. Уводи корабль на карте и чини пробоины.", "The hull broke from rock hits. Steer on the map and patch the holes."),
                    ja_core::ship::ShipFail::Pressure => l.pick("Из пробоин вышел весь воздух. Заделывай дыры сразу.", "All the air leaked out. Patch holes right away."),
                    ja_core::ship::ShipFail::Oxygen => l.pick("Кончился кислород. Следи за системами жизнеобеспечения.", "Oxygen ran out. Watch life support."),
                };
                kit::wrap_text(b, why, &art.body, 18.0, C_WARN, 560.0);
                b.spawn(kit::row(10.0)).with_children(|r| {
                    kit::button(r, &art, Action::RestartDay, 1, "retry", l.pick("Начать день заново", "Start the day again"), true);
                    kit::button(r, &art, Action::ShipQuit, 2, "home", l.pick("В меню", "Menu"), false);
                });
            });
        });
        return;
    }
    if st.phase == Phase::Landing {
        let root = overlay(&mut commands, 0.7, false);
        commands.entity(root).with_children(|p| {
            p.spawn(kit::panel()).insert(Node { width: Val::Px(640.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(14.0), padding: UiRect::all(Val::Px(22.0)), border: UiRect::all(Val::Px(3.0)), align_items: AlignItems::Center, ..default() }).with_children(|b| {
                kit::icon(b, &art, "moon", 64.0);
                kit::text(b, l.pick("Мы на орбите Луны!", "We're in lunar orbit!"), &art.head, 30.0, C_GOOD);
                kit::text(b, format!("{} {}", l.pick("Бюджет экспедиции:", "Expedition budget:"), st.wallet.team), &art.body, 20.0, C_TEXT);
                kit::wrap_text(b, l.pick("Экипаж переходит в посадочный модуль. Впереди — посадка и база на Луне.", "The crew moves into the lander. Next: landing and a base on the Moon."), &art.body, 18.0, C_MUTED, 580.0);
                kit::button(b, &art, Action::ToMoon, 1, "rocket", l.pick("На Луну!", "To the Moon!"), true);
            });
        });
        return;
    }
    if let Some(d) = &st.dialog {
        let root = overlay(&mut commands, 0.25, true);
        commands.entity(root).with_children(|p| crate::dialog_ui::build(p, &art, c, d, level, l));
        return;
    }
    if let Some(ls) = &st.lesson {
        let root = overlay(&mut commands, 0.6, false);
        commands.entity(root).with_children(|p| card::lesson(p, &art, c, ls, st.laws.len(), level, l));
        return;
    }
    if let Some(id) = &st.card {
        let root = overlay(&mut commands, 0.6, false);
        commands.entity(root).with_children(|p| card::build(p, &art, c, id, level, l));
        return;
    }
    if let Some(b) = st.breather {
        let root = overlay(&mut commands, 0.4, false);
        commands.entity(root).with_children(|p| card::breather(p, &art, b, l));
        return;
    }
    if ui.paused {
        let root = overlay(&mut commands, 0.6, false);
        commands.entity(root).with_children(|p| {
            p.spawn(kit::panel()).insert(Node { width: Val::Px(460.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(22.0)), border: UiRect::all(Val::Px(3.0)), align_items: AlignItems::Center, ..default() }).with_children(|b| {
                kit::text(b, l.pick("Пауза", "Paused"), &art.head, 30.0, C_TEXT);
                kit::button(b, &art, Action::ShipResume, 1, "play", l.pick("Продолжить", "Resume"), true);
                if st.cfg.mode == ja_core::PlayMode::Creative {
                    kit::button(b, &art, Action::NextDay, 2, "night", l.pick("Следующий день", "Next day"), false);
                }
                kit::button(b, &art, Action::ShipQuit, 3, "home", l.pick("Выйти в меню", "Back to menu"), false);
            });
        });
        return;
    }
    if let Some(t) = open_task {
        let root = overlay(&mut commands, 0.35, false);
        let def = c.task(&t.id);
        commands.entity(root).with_children(|p| {
            p.spawn(kit::panel()).insert(Node { width: Val::Px(760.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|b| {
                b.spawn(kit::row(10.0)).with_children(|r| {
                    kit::text(r, def.map(|d| d.title.get(l)).unwrap_or(""), &art.head, 24.0, if t.kind == TaskKind::Problem { C_BAD } else { kit::C_ACCENT });
                    if let Some(x) = t.t_left {
                        kit::icon(r, &art, "stopwatch", 26.0);
                        let e = kit::text(r, format!("{:.0}", x.max(0.0)), &art.head, 22.0, C_WARN);
                        r.commands().entity(e).insert(TaskTimer);
                    }
                });
                if let Some(d) = def {
                    kit::wrap_text(b, d.brief.get(level, l), &art.body, 17.0, C_TEXT, 720.0);
                    if let Some(h) = &d.history {
                        kit::text(b, h.get(l), &art.body, 14.0, C_MUTED);
                    }
                }
                b.spawn((Node { height: Val::Px(2.0), width: Val::Percent(100.0), ..default() }, BackgroundColor(C_LINE)));
                if t.kind == TaskKind::Asteroids {
                    crate::touch_panel::build(b, &art, l);
                } else if let Some(m) = &t.mini {
                    crate::minigames::build(b, t.uid);
                    b.spawn(kit::row(12.0)).with_children(|r| {
                        r.spawn((Node { width: Val::Px(420.0), height: Val::Px(12.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(6.0)))).with_children(|x| {
                            x.spawn((Node { width: Val::Percent(m.progress() * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOOD), BorderRadius::all(Val::Px(6.0)), crate::minigames::MiniProgress));
                        });
                        if m.mistakes > 0 {
                            kit::text(r, format!("{} {}", l.pick("Ошибок:", "Mistakes:"), m.mistakes), &art.body, 15.0, C_WARN);
                        }
                    });
                }
                kit::button(b, &art, Action::CloseTask, 900, "back", l.pick("Отойти (Esc)", "Step away (Esc)"), false);
            });
        });
    }
}

// ---------------------------------------------------------------- automated walk-through

/// `JA_SHIP_TEST=1 JA_SHOTS=dir`: starts an expedition, plays it like a quick player and takes
/// screenshots of every window type, then quits at the Moon.
pub struct ShipTestPlugin;
impl Plugin for ShipTestPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("JA_SHIP_TEST").is_none() {
            return;
        }
        app.add_systems(Update, ship_test.after(Set::View));
        if std::env::var_os("JA_MOUSE_TEST").is_some() {
            app.add_systems(Update, mouse_drive.in_set(Set::Input).before(crate::minigames::pointer));
        }
    }
}

#[derive(Default)]
struct TestRun {
    t: f32,
    step: u32,
    shots: Vec<&'static str>,
}

#[allow(clippy::too_many_arguments)]
fn ship_test(
    mut commands: Commands,
    time: Res<Time>,
    mut run: Local<TestRun>,
    mut acts: EventWriter<Act>,
    state: Res<State<AppState>>,
    camp: Option<ResMut<Camp>>,
    ui: Option<ResMut<ShipUi>>,
    mut speed: ResMut<Speed>,
    mut crew: Query<&mut Crew>,
    mut exit: EventWriter<AppExit>,
) {
    run.t += time.delta_secs();
    let dir = std::env::var_os("JA_SHOTS").map(std::path::PathBuf::from);
    let shot = |commands: &mut Commands, run: &mut TestRun, name: &'static str| {
        if run.shots.contains(&name) {
            return;
        }
        run.shots.push(name);
        info!("SHIP-TEST shot {name}");
        if let Some(d) = &dir {
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(d.join(format!("{name}.png"))));
        }
    };
    if run.step == 0 {
        if run.t > 1.0 {
            acts.write(Act(Action::StartExpedition));
            run.step = 1;
            run.t = 0.0;
        }
        return;
    }
    if run.step == 101 && *state.get() == AppState::Landing && run.t > 2.5 {
        // the flight handed over to part B's landing
        shot(&mut commands, &mut run, "27_ship_to_landing");
        info!("SHIP-TEST reached part B's landing");
        run.step = 102;
        run.t = 0.0;
        return;
    }
    if run.step == 102 && run.t > 1.5 {
        exit.write(AppExit::Success);
        return;
    }
    if *state.get() != AppState::Ship || run.t < 1.2 {
        return;
    }
    let Some(mut camp) = camp else { return };
    let Some(mut ui) = ui else { return };
    let st = camp.0.state().clone();
    if run.t > 600.0 {
        error!("SHIP-TEST timeout");
        exit.write(AppExit::error());
        return;
    }
    if st.phase == Phase::Landing {
        if run.step < 99 {
            speed.0 = 1.0;
            run.step = 99;
            run.t = 0.0;
            return;
        }
        if run.t > 1.5 {
            shot(&mut commands, &mut run, "26_ship_landing");
            info!("SHIP-TEST landing: coins {} hull {:.0} cards {} solved {} failed {}", st.wallet.team, st.ship.hull, st.cards_seen.len(), st.score.solved, st.score.failed);
            run.step = 100;
        }
        if run.step == 100 && run.t > 3.0 {
            acts.write(Act(Action::ToMoon));
            run.step = 101;
            run.t = 0.0;
        }
        return;
    }
    if let Some(d) = &st.dialog {
        if d.id.starts_with("talk_") {
            shot(&mut commands, &mut run, "29_ship_talk");
        }
        shot(&mut commands, &mut run, "20_ship_dialog");
        if run.t > 1.5 {
            acts.write(Act(Action::DialogChoose(0)));
            acts.write(Act(Action::DialogNext));
            run.t = 1.2;
        }
        return;
    }
    if st.card.is_some() {
        shot(&mut commands, &mut run, "23_ship_card");
        if run.t > 1.5 {
            acts.write(Act(Action::CloseCard));
            run.t = 1.2;
        }
        return;
    }
    if st.breather.is_some() {
        acts.write(Act(Action::CloseBreather));
        return;
    }
    if st.lesson.is_some() {
        shot(&mut commands, &mut run, "28_ship_lesson");
        if run.t > 1.2 {
            acts.write(Act(Action::CloseLesson));
            run.t = 0.0;
        }
        return;
    }
    // wait for an urgent problem once, to show it in the HUD
    let problem = st.tasks.iter().find(|t| t.kind == TaskKind::Problem);
    if run.step == 1 {
        shot(&mut commands, &mut run, "21_ship_scene");
        run.step = 2;
    }
    if run.step == 3 && problem.is_none() {
        // fast-forward while waiting — but not while the real mouse plays a timed game
        speed.0 = if std::env::var_os("JA_MOUSE_TEST").is_some() && ui.open.is_some() { 1.0 } else { 4.0 };
        if speed.0 > 1.0 || ui.open.is_none() {
            return;
        }
    }
    if run.step == 3 && problem.is_some() {
        speed.0 = 1.0;
        shot(&mut commands, &mut run, "24_ship_problem");
        run.step = 4;
        run.t = 0.0;
        return;
    }
    // the pet and the crew: adopt the cat after a few jobs, then talk to Aya, then feed the cat
    if ui.open.is_none() && st.done.len() >= 3 && st.pet.is_none() {
        let spot = camp.0.pet_spots()[1];
        for mut c in &mut crew {
            if c.idx == 0 {
                c.pos = Vec2::new(spot.0 + 0.6, spot.1);
            }
        }
        camp.0.set_crew_pos(0, spot.0 + 0.6, spot.1);
        if camp.0.apply(0, CampaignCommand::AdoptPet { kind: 1 }).is_ok() {
            info!("SHIP-TEST adopted the cat");
        }
        run.t = 0.0;
        return;
    }
    if st.pet.is_some() && run.t > 1.5 && !run.shots.contains(&"30_ship_pet") {
        shot(&mut commands, &mut run, "30_ship_pet");
    }
    if ui.open.is_none() && st.done.len() >= 5 && !st.seen_dialogs.iter().any(|d| d.starts_with("talk_")) {
        let aya = camp.0.content().npcs.iter().position(|n| n.id == "aya").unwrap_or(1) as u8;
        if camp.0.apply(0, CampaignCommand::Talk { npc: aya }).is_ok() {
            info!("SHIP-TEST talking to Aya");
        }
        return;
    }
    if ui.open.is_none() && st.done.len() >= 7 && st.pet.is_some() && !st.done.iter().any(|d| d == "pet_feed") && !st.tasks.iter().any(|t| t.id == "pet_feed") {
        if camp.0.apply(0, CampaignCommand::FeedPet).is_ok() {
            info!("SHIP-TEST feeding the pet");
        }
    }
    let Some(task) = problem.or(st.tasks.iter().find(|t| t.id == "pet_feed")).or(st.tasks.first()).cloned() else { return };
    // walk (teleport) to the console and work there
    if ui.open.is_none() {
        if let Some((x, y)) = camp.0.console_pos(&task.room, &task.console).map(|(x, y)| {
            // stand on the floor in front of the console (consoles are solid, along the walls)
            let layout = camp.0.content().ship.clone();
            [(x, y - 1.0), (x, y + 1.0), (x - 1.0, y), (x + 1.0, y)].into_iter().find(|p| layout.as_ref().is_some_and(|l| ja_core::ship::walkable(l, p.0, p.1))).unwrap_or((x, y - 1.0))
        }) {
            for mut c in &mut crew {
                if c.idx == 0 {
                    c.pos = Vec2::new(x, y);
                }
            }
            camp.0.set_crew_pos(0, x, y);
        }
        if camp.0.apply(0, CampaignCommand::OpenTask { uid: task.uid }).is_ok() {
            ui.open = Some((0, task.uid));
        }
        run.t = 0.0;
        return;
    }
    if task.kind == TaskKind::Asteroids {
        if run.t > 4.0 {
            shot(&mut commands, &mut run, "25_ship_touch_panel");
        }
        speed.0 = if run.t > 4.5 { 3.0 } else { 1.0 };
        return;
    }
    if run.t < 0.8 {
        return;
    }
    if let Some(m) = &task.mini {
        shot(&mut commands, &mut run, "22_ship_minigame");
        let name: &'static str = Box::leak(format!("mg_{}", task.id).into_boxed_str());
        shot(&mut commands, &mut run, name);
        // JA_MOUSE_TEST: the first jobs are played with the real mouse cursor (mouse_drive)
        if !(std::env::var_os("JA_MOUSE_TEST").is_some() && mouse_task(&task.id)) {
            for i in m.solution() {
                acts.write(Act(Action::Mini(task.uid, i)));
            }
        }
    }
    if run.step == 2 && st.done.len() >= 2 {
        run.step = 3;
    }
    run.t = 0.0;
}

const MOUSE_TASKS: [&str; 3] = ["go_nogo", "systems_check", "tli_burn"];

/// `JA_MOUSE_TEST=all`: every job is played with the real cursor.
fn mouse_task(id: &str) -> bool {
    std::env::var("JA_MOUSE_TEST").is_ok_and(|v| v == "all") || MOUSE_TASKS.contains(&id)
}

/// `JA_MOUSE_TEST=1` (with `JA_SHIP_TEST`): plays the first mini-games by moving the real cursor
/// and pressing the real mouse button, so the canvas → `MiniInput` path is checked end to end.
#[allow(clippy::too_many_arguments)]
fn mouse_drive(
    camp: Option<Res<Camp>>,
    ui: Option<Res<ShipUi>>,
    canvas: Query<(&crate::minigames::Canvas, &ComputedNode, &GlobalTransform)>,
    mut windows: Query<&mut Window>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    time: Res<Time>,
    speed: Res<Speed>,
    mut q: Local<(u32, Vec<ja_core::MiniInput>, u8, f32)>,
) {
    use ja_core::MiniInput as M;
    let (Some(camp), Some(ui)) = (camp, ui) else { return };
    let Some((_, uid)) = ui.open else { return };
    let Some(task) = camp.0.state().tasks.iter().find(|t| t.uid == uid) else { return };
    if !mouse_task(&task.id) {
        return;
    }
    let Ok((_, node, gt)) = canvas.single() else { return };
    let Ok(mut win) = windows.single_mut() else { return };
    if q.0 != uid {
        let Some(m) = &task.mini else { return };
        // merge back-to-back waits: frame-by-frame waiting would add a frame of delay to each
        let mut moves: Vec<ja_core::MiniInput> = vec![];
        for i in m.solution() {
            match (moves.last_mut(), i) {
                (Some(M::Wait(a)), M::Wait(b)) => *a += b,
                _ => moves.push(i),
            }
        }
        *q = (uid, moves, 0, 0.0);
        info!("MOUSE-TEST start {} ({} moves)", task.id, q.1.len());
    }
    // canvas pixels → physical window pixels
    let size = node.size();
    let top_left = gt.translation().truncate() - size / 2.0;
    let to_win = |x: f32, y: f32| bevy::math::DVec2::new((top_left.x + x / ja_core::minigame::CW * size.x) as f64, (top_left.y + y / ja_core::minigame::CH * size.y) as f64);
    if q.3 > 0.0 {
        q.3 -= time.delta_secs().min(0.1) * speed.0;
        return;
    }
    let Some(step) = q.1.first().copied() else {
        // all moves sent but the job is still open (a slip, like a real player's): look and try again
        q.3 -= time.delta_secs();
        if q.3 < -1.0 {
            info!("MOUSE-TEST retry {}", task.id);
            q.0 = 0;
            q.3 = 0.0;
        }
        return;
    };
    // each pointer move takes two frames: move the cursor, then (next frame) press / release
    match (step, q.2) {
        (M::Wait(t), _) => {
            q.3 = t;
            q.1.remove(0);
        }
        (M::Press(x, y) | M::Release(x, y) | M::Drag(x, y), 0) => {
            win.set_physical_cursor_position(Some(to_win(x, y)));
            q.2 = 1;
            if matches!(step, M::Drag(..)) {
                q.1.remove(0);
                q.2 = 0;
            }
        }
        (M::Press(..), _) => {
            mouse.press(MouseButton::Left);
            q.1.remove(0);
            q.2 = 0;
        }
        (M::Release(..), _) => {
            mouse.release(MouseButton::Left);
            q.1.remove(0);
            q.2 = 0;
        }
        (M::Space(on), _) => {
            info!("MOUSE-TEST space {on}");
            q.1.remove(0);
        }
        _ => {
            q.1.remove(0);
        }
    }
    if q.1.is_empty() {
        info!("MOUSE-TEST moves sent for {}", task.id);
    }
}
