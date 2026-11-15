//! B1 — the landing screen: choose a site near the lunar south pole, then fly the lander down
//! (side view). Rules and physics live in `ja_core::landing`; this file only draws and reads keys.
//!
//! Controls: W/S or ↑/↓ — engine lever, A/D or ←/→ — tilt, E — answer Mission Control (the 1202
//! alarm), R — try again after an abort. A green pad marks the flattest ground; a live checklist
//! shows each touchdown rule in green or red; after an abort a table shows what was wrong.
//! Helpers: "hold speed" (Junior: on) moves the lever by itself when you let go of W/S;
//! "guide" (offered after an abort) also steers towards the pad when you let go of A/D.

use crate::art::Art;
use crate::audio::Play;
use crate::kit::{self, Act, Action, Inputs, C_BAD, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use ja_core::landing::Touch;
use crate::{AppState, MainCam, Prof, Setup};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use ja_core::landing::{self, AbortReason, Lander, LanderInput, LandingEvent, Phase, SITES};
use ja_core::{Lang, MapId, Mode};

/// pixels per metre in the side view
const PPM: f32 = 1.5;
const LEVER_SPEED: f32 = 0.9;

#[derive(Resource, Default)]
pub struct Descent {
    pub lander: Option<Lander>,
    /// the screenshot tour flies with the autopilot
    pub autopilot: bool,
    lever: f32,
    idle: f32,
    ack: bool,
    /// helper: hold a safe sink speed when W/S is released
    hold: bool,
    /// helper: steer towards the pad when A/D is released
    guide: bool,
    /// big centre message and its time left
    call: Option<(String, f32)>,
    /// bump to rebuild the overlay (site cards / result)
    ui: u32,
    built: u32,
}

#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Scene;
#[derive(Component)]
struct LanderSprite;
#[derive(Component)]
struct Flame;
#[derive(Component)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Gauge {
    Alt,
    Hover,
    Vy,
    Vx,
    Angle,
    Ground,
    Pad,
}
/// The check / cross icon in front of a checklist line.
#[derive(Component)]
struct Mark(Gauge);
/// "Slow down!" / "All green — touch down!" under the call text.
#[derive(Component)]
struct Advice;
#[derive(Component)]
struct FuelFill;
#[derive(Component)]
struct LeverFill;
#[derive(Component)]
struct AlarmBox;
#[derive(Component)]
struct CallText;

pub struct LandingPlugin;
impl Plugin for LandingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Descent>()
            .add_systems(OnEnter(AppState::Landing), enter)
            .add_systems(OnExit(AppState::Landing), leave)
            .add_systems(Update, (acts, fly, draw, overlay, gauges).chain().after(crate::Set::Input).run_if(in_state(AppState::Landing)));
    }
}

fn enter(mut d: ResMut<Descent>, mut cam: Query<(&mut Transform, &mut Projection), With<MainCam>>) {
    let autopilot = d.autopilot;
    *d = Descent { ui: 1, autopilot, ..default() };
    for (mut t, mut p) in &mut cam {
        t.translation = Vec3::new(0.0, 0.0, t.translation.z);
        if let Projection::Orthographic(o) = p.as_mut() {
            o.scale = 1.0;
        }
    }
}

fn leave(mut commands: Commands, q: Query<Entity, Or<(With<Overlay>, With<Scene>)>>, mut cam: Query<&mut Transform, With<MainCam>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    for mut t in &mut cam {
        t.translation = Vec3::new(0.0, 0.0, t.translation.z);
    }
}

fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(9)
}

#[allow(clippy::too_many_arguments)]
fn acts(mut commands: Commands, mut acts: EventReader<Act>, mut d: ResMut<Descent>, prof: Res<Prof>, art: Res<Art>, mut setup: ResMut<Setup>, mut next: ResMut<NextState<AppState>>, mut play: EventWriter<Play>, inputs: Res<Inputs>, keys: Res<ButtonInput<KeyCode>>, scene: Query<Entity, With<Scene>>) {
    for Act(a) in acts.read() {
        match *a {
            Action::PickSite(i) => {
                let l = Lander::new(i, prof.0.cadet, seed()).with_pilot(setup.roles.contains(&ja_core::Role::Pilot));
                d.hold = l.junior;
                d.guide = false;
                for e in &scene {
                    commands.entity(e).despawn();
                }
                spawn_scene(&mut commands, &art, &l);
                d.lever = 0.45;
                d.lander = Some(l);
                d.ui += 1;
                play.write(Play("launch"));
            }
            Action::LandingRetry => retry(&mut d, false),
            Action::LandingRetryHelp => retry(&mut d, true),
            Action::LandingAck => d.ack = true,
            Action::LandingGo => {
                if let Some(l) = &d.lander {
                    setup.site = l.site;
                    setup.landing_coins = l.bonus_coins();
                    setup.map = MapId::Moon;
                    if setup.mode == Mode::Story {
                        setup.mode = Mode::Survival;
                    }
                    play.write(Play("launch"));
                    next.set(AppState::Playing);
                }
            }
            _ => {}
        }
    }
    // back to the expedition screen before choosing a site
    if inputs.0[0].back && d.lander.is_none() {
        next.set(AppState::Expedition);
    }
    // R: straight into the next approach
    if keys.just_pressed(KeyCode::KeyR) && d.lander.as_ref().map_or(false, |l| matches!(l.phase, Phase::Aborted(_))) {
        retry(&mut d, false);
    }
}

/// A fresh approach from orbit; `help` switches both helpers on.
fn retry(d: &mut Descent, help: bool) {
    if let Some(l) = d.lander.as_mut() {
        l.retry();
    }
    if help {
        d.hold = true;
        d.guide = true;
    }
    d.lever = 0.45;
    d.idle = 0.0;
    d.call = None;
    d.ui += 1;
}

fn fly(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, inputs: Res<Inputs>, mut d: ResMut<Descent>, mut prof: ResMut<Prof>, mut setup: ResMut<Setup>, mut play: EventWriter<Play>) {
    let dt = time.delta_secs();
    let l = prof.lang();
    if let Some((_, t)) = d.call.as_mut() {
        *t -= dt;
        if *t <= 0.0 {
            d.call = None;
        }
    }
    let Some(lander) = d.lander.clone() else { return };
    if lander.phase != Phase::Flying {
        return;
    }
    let pad = inputs.0[0];
    let up = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) || pad.mv.y > 0.4;
    let down = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) || pad.mv.y < -0.4;
    let mut turn = if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) || pad.mv.x < -0.4 {
        -1.0
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) || pad.mv.x > 0.4 {
        1.0
    } else {
        0.0
    };
    // the guide steers towards the pad while A/D is released
    if turn == 0.0 && d.guide {
        turn = landing::steer(&lander);
    }
    if up {
        d.lever += LEVER_SPEED * dt;
        d.idle = 0.0;
    } else if down {
        d.lever -= LEVER_SPEED * dt;
        d.idle = 0.0;
    } else {
        d.idle += dt;
        // the "hold speed" helper moves the lever to keep a safe sink speed
        if d.hold && d.idle > 0.4 {
            let target = landing::hold_throttle(&lander);
            d.lever += (target - d.lever).clamp(-LEVER_SPEED * 1.5 * dt, LEVER_SPEED * 1.5 * dt);
        }
    }
    d.lever = d.lever.clamp(0.0, 1.0);
    let ack = d.ack || pad.act;
    d.ack = false;
    let input = if d.autopilot {
        let i = landing::autopilot(&lander);
        d.lever = i.throttle;
        // keep the alarm on screen for a moment so the tour can show it
        LanderInput { ack_alarm: lander.alarm.map_or(false, |t| t < landing::ALARM_LEN - 1.5), ..i }
    } else {
        LanderInput { throttle: d.lever, turn, ack_alarm: ack }
    };
    let mut lander = lander;
    let events = lander.tick(dt, input);
    for e in events {
        match e {
            LandingEvent::Alarm1202 => {
                play.write(Play("alarm"));
                d.call = Some((l.pick("«1202!» Компьютер перегружен", "“1202!” The computer is overloaded").to_string(), 2.5));
            }
            LandingEvent::AlarmCleared => {
                play.write(Play("radio"));
                d.call = Some((l.pick("ЦУП: «Продолжаем!»", "Mission Control: “We’re go!”").to_string(), 1.8));
            }
            LandingEvent::FuelCall(s) => {
                play.write(Play("radio"));
                d.call = Some((format!("{} {s} {}", l.pick("ЦУП: «", "Mission Control: “"), l.pick("секунд!»", "seconds!”")), 2.5));
            }
            LandingEvent::Touchdown { stars, .. } => {
                // pilot achievements (shown later on the debrief)
                let tiers = prof.0.record_landing(stars as u32);
                prof.save();
                setup.new_tiers.extend(tiers);
                play.write(Play("badge"));
                d.call = None;
                d.ui += 1;
            }
            LandingEvent::Abort(_) => {
                play.write(Play("bad"));
                d.call = None;
                d.ui += 1;
            }
        }
    }
    d.lander = Some(lander);
}

fn spawn_scene(commands: &mut Commands, art: &Art, l: &Lander) {
    let sc = StateScoped(AppState::Landing);
    let ground = Color::srgb(0.46, 0.48, 0.54);
    let half = landing::GROUND_HALF * PPM;
    // stars
    let mut x: u64 = 0x9e3779b97f4a7c15;
    let mut rnd = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % 10_000) as f32 / 10_000.0
    };
    for _ in 0..260 {
        let (sx, sy) = (rnd() * (half * 2.0 + 1600.0) - half - 800.0, rnd() * 900.0 + 20.0);
        let s = 1.0 + (rnd() * 2.5).floor();
        commands.spawn((Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.4 + rnd() * 0.6), Vec2::splat(s)), Transform::from_xyz(sx, sy, -5.0), Scene, sc.clone()));
    }
    // Earth low over the horizon, as seen from the south pole
    commands.spawn((ImageNode::new(art.icon("earth")), Node { position_type: PositionType::Absolute, left: Val::Px(60.0), top: Val::Px(50.0), width: Val::Px(64.0), height: Val::Px(64.0), ..default() }, GlobalZIndex(2), Scene, sc.clone()));
    // ground and boulder fields
    commands.spawn((Sprite::from_color(ground, Vec2::new(half * 2.0 + 2400.0, 600.0)), Anchor::TopCenter, Transform::from_xyz(0.0, 0.0, -2.0), Scene, sc.clone()));
    commands.spawn((Sprite::from_color(Color::srgb(0.6, 0.62, 0.68), Vec2::new(half * 2.0 + 2400.0, 4.0)), Anchor::TopCenter, Transform::from_xyz(0.0, 0.0, -1.9), Scene, sc.clone()));
    for edge in [-1.0, 1.0] {
        // the end of the flat ground: tall crater walls
        commands.spawn((Sprite::from_color(Color::srgb(0.34, 0.35, 0.41), Vec2::new(900.0, 520.0)), Anchor::BottomCenter, Transform::from_xyz(edge * (half + 450.0), -2.0, -1.8), Scene, sc.clone()));
    }
    for b in &l.boulders {
        let (x0, x1) = (b.from * PPM, b.to * PPM);
        commands.spawn((Sprite::from_color(Color::srgb(0.36, 0.37, 0.42), Vec2::new(x1 - x0, 8.0)), Anchor::BottomLeft, Transform::from_xyz(x0, -2.0, -1.5), Scene, sc.clone()));
        let mut bx = x0 + 4.0;
        while bx < x1 - 6.0 {
            let big = rnd() > 0.6;
            commands.spawn((
                Sprite { image: art.sprite(if big { "rock_big" } else { "rock_small" }), custom_size: Some(Vec2::splat(if big { 30.0 } else { 18.0 })), color: Color::srgb(0.75, 0.76, 0.82), ..default() },
                Anchor::BottomCenter,
                Transform::from_xyz(bx, -1.0, -1.4),
                Scene,
                sc.clone(),
            ));
            bx += 10.0 + rnd() * 14.0;
        }
    }
    // the landing pad: green lights on flat ground and a faint beam to find it from high up
    let (p0, p1) = (l.pad.0 * PPM, l.pad.1 * PPM);
    let green = Color::srgb(0.3, 0.9, 0.55);
    commands.spawn((Sprite::from_color(Color::srgba(0.3, 0.9, 0.55, 0.35), Vec2::new(p1 - p0, 6.0)), Anchor::BottomLeft, Transform::from_xyz(p0, -2.0, -1.3), Scene, sc.clone()));
    let mid = (p0 + p1) * 0.5;
    for k in 1..40 {
        commands.spawn((Sprite::from_color(Color::srgba(0.3, 0.9, 0.55, 0.45), Vec2::new(3.0, 10.0)), Anchor::BottomCenter, Transform::from_xyz(mid, 30.0 + k as f32 * 24.0, -3.0), Scene, sc.clone()));
    }
    for x in [p0, p1] {
        commands.spawn((Sprite::from_color(green, Vec2::new(5.0, 16.0)), Anchor::BottomCenter, Transform::from_xyz(x, 0.0, -1.2), Scene, sc.clone()));
    }
    commands.spawn((Sprite { image: art.icon("flag"), custom_size: Some(Vec2::splat(30.0)), ..default() }, Anchor::BottomCenter, Transform::from_xyz((p0 + p1) * 0.5, 0.0, -1.2), Scene, sc.clone()));
    // the lander and its flame
    commands
        .spawn((Sprite { image: art.sprite("lander"), custom_size: Some(Vec2::splat(72.0)), ..default() }, Anchor::Custom(Vec2::new(0.0, -0.4)), Transform::from_xyz(l.x * PPM, l.alt * PPM, 2.0), LanderSprite, Scene, sc.clone()))
        .with_children(|p| {
            p.spawn((Sprite { image: art.sprite("flame"), custom_size: Some(Vec2::new(22.0, 30.0)), ..default() }, Anchor::TopCenter, Transform::from_xyz(0.0, -20.0, -0.1), Flame));
        });
}

fn draw(d: Res<Descent>, time: Res<Time>, mut q: Query<&mut Transform, (With<LanderSprite>, Without<MainCam>, Without<Flame>)>, mut flame: Query<(&mut Transform, &mut Visibility), (With<Flame>, Without<LanderSprite>, Without<MainCam>)>, mut cam: Query<&mut Transform, (With<MainCam>, Without<LanderSprite>, Without<Flame>)>) {
    let Some(l) = &d.lander else { return };
    let (x, y) = (l.x * PPM, l.alt * PPM);
    for mut t in &mut q {
        t.translation.x = x;
        t.translation.y = y;
        t.rotation = Quat::from_rotation_z(-l.angle.to_radians());
    }
    for (mut t, mut v) in &mut flame {
        let on = l.throttle > 0.02 && l.phase == Phase::Flying;
        *v = if on { Visibility::Inherited } else { Visibility::Hidden };
        let flicker = 0.85 + 0.15 * (time.elapsed_secs() * 40.0).sin();
        t.scale = Vec3::new(0.6 + 0.4 * l.throttle, (0.4 + 1.2 * l.throttle) * flicker, 1.0);
    }
    for mut t in &mut cam {
        let want = Vec3::new(x, (y * 0.55).max(300.0), t.translation.z);
        t.translation = t.translation.lerp(want, (time.delta_secs() * 4.0).min(1.0));
    }
}

fn site_card(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, i: usize) {
    let s = &SITES[i];
    p.spawn((Button, kit::UiAction(Action::PickSite(i as u8)), kit::Focusable(i as u16), Node { width: Val::Px(330.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(kit::C_PANEL), kit::Base(kit::C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(10.0))))
        .with_children(|b| {
            kit::icon(b, art, s.icon, 64.0);
            kit::text(b, l.pick(s.name_ru, s.name_en), &art.head, 24.0, C_TEXT);
            kit::wrap_text(b, l.pick(s.desc_ru, s.desc_en), &art.body, 16.0, C_MUTED, 300.0);
            for (ic, ru, en, v, good) in [("sun", "Солнце", "Sun", s.sun, true), ("ice", "Лёд рядом", "Ice nearby", s.ice, true), ("rock", "Камни", "Boulders", s.rough, false)] {
                b.spawn(kit::row(8.0)).with_children(|r| {
                    kit::icon(r, art, ic, 26.0);
                    r.spawn(Node { width: Val::Px(120.0), ..default() }).with_children(|t| {
                        kit::text(t, l.pick(ru, en), &art.body, 17.0, C_TEXT);
                    });
                    kit::pips(r, v as u32, 3, if good { C_GOOD } else { C_WARN }, 20.0);
                });
            }
        });
}

fn abort_text(r: AbortReason, l: Lang) -> &'static str {
    match r {
        AbortReason::TooFast => l.pick("Слишком быстро! Метров за 50 до грунта прибавь тягу (W), чтобы «Вниз» стало зелёным.", "Too fast! About 50 m above the ground add thrust (W) until “Down” turns green."),
        AbortReason::Tilted => l.pick("Модуль наклонён. Перед касанием выровняй его (A/D).", "The lander is tilted. Level it before touchdown (A/D)."),
        AbortReason::Boulders => l.pick("Под модулем камни! Армстронг тоже увёл «Орёл» от поля валунов — ищи ровное место.", "Boulders below! Armstrong also flew the Eagle past a boulder field — find flat ground."),
        AbortReason::OffCourse => l.pick("Модуль улетел с района посадки. Наклони его против сноса (A или D) — и снос погаснет.", "The lander left the landing area. Tilt against the drift (A or D) to stop it."),
        AbortReason::NoFuel => l.pick("Топливо кончилось. Береги его: не держи тягу на полную долго.", "Out of fuel. Save it: don’t keep full thrust for long."),
    }
}

fn overlay(mut commands: Commands, mut d: ResMut<Descent>, art: Res<Art>, prof: Res<Prof>, old: Query<Entity, With<Overlay>>, mut focus: ResMut<kit::Focus>) {
    if d.ui == d.built {
        return;
    }
    d.built = d.ui;
    for e in &old {
        commands.entity(e).despawn();
    }
    focus.idx = 0;
    let l = prof.lang();
    let sc = StateScoped(AppState::Landing);
    match &d.lander {
        None => {
            commands.spawn((kit::overlay(true, 20), Overlay, sc)).with_children(|o| {
                o.spawn(kit::col(16.0)).insert(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(16.0), ..default() }).with_children(|c| {
                    kit::text(c, l.pick("Где садимся? Южный полюс Луны", "Where do we land? The lunar south pole"), &art.head, 34.0, C_TEXT);
                    kit::wrap_text(c, l.pick("Это настоящие районы, которые NASA рассматривает для посадок у южного полюса. У каждого места свои плюсы и минусы.", "These are real regions NASA considers for landings near the south pole. Every site has pros and cons."), &art.body, 18.0, C_MUTED, 900.0);
                    c.spawn(kit::row(16.0)).with_children(|r| {
                        for i in 0..SITES.len() {
                            site_card(r, &art, l, i);
                        }
                    });
                });
            });
        }
        Some(lander) => {
            // flight instruments and the touchdown checklist (always), plus the result panel
            let hold = d.hold;
            let guide = d.guide;
            commands.spawn((Node { position_type: PositionType::Absolute, right: Val::Px(14.0), top: Val::Px(14.0), width: Val::Px(330.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), padding: UiRect::all(Val::Px(12.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(kit::C_BG), BorderColor(C_LINE), BorderRadius::all(Val::Px(8.0)), GlobalZIndex(15), Overlay, sc.clone())).with_children(|p| {
                kit::text(p, l.pick(lander.site_def().name_ru, lander.site_def().name_en), &art.head, 20.0, kit::C_ACCENT);
                for (g, ic) in [(Gauge::Alt, "flag"), (Gauge::Hover, "hourglass")] {
                    p.spawn(kit::row(8.0)).with_children(|r| {
                        kit::icon(r, &art, ic, 22.0);
                        r.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 17.0, ..default() }, TextColor(C_TEXT), g));
                    });
                }
                for (label_ru, label_en, fuel) in [("Топливо", "Fuel", true), ("Тяга", "Thrust", false)] {
                    p.spawn(kit::row(8.0)).with_children(|r| {
                        r.spawn(Node { width: Val::Px(80.0), ..default() }).with_children(|t| {
                            kit::text(t, l.pick(label_ru, label_en), &art.body, 15.0, C_MUTED);
                        });
                        r.spawn((Node { width: Val::Px(200.0), height: Val::Px(12.0), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(3.0)))).with_children(|b| {
                            if fuel {
                                b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOOD), FuelFill));
                            } else {
                                b.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(kit::C_ACCENT), LeverFill));
                            }
                        });
                    });
                }
                kit::text(p, l.pick("Чек-лист касания", "Touchdown checklist"), &art.head, 17.0, C_TEXT);
                for g in [Gauge::Vy, Gauge::Vx, Gauge::Angle, Gauge::Ground, Gauge::Pad] {
                    p.spawn(kit::row(8.0)).with_children(|r| {
                        r.spawn((ImageNode::new(art.icon("check")), Node { width: Val::Px(22.0), height: Val::Px(22.0), flex_shrink: 0.0, ..default() }, Mark(g)));
                        r.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 16.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(280.0), ..default() }, g));
                    });
                }
                if hold || guide {
                    let s = match (hold, guide) {
                        (true, true) => l.pick("Помощник: держит скорость и ведёт к площадке, пока ты не жмёшь клавиши", "Helper: holds the speed and steers to the pad while you don’t press keys"),
                        _ => l.pick("Помощник держит скорость, когда отпускаешь W/S", "The helper holds the speed when you let go of W/S"),
                    };
                    kit::wrap_text(p, s, &art.body, 14.0, C_GOOD, 300.0);
                }
            });
            // controls hint
            commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(14.0), bottom: Val::Px(14.0), padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() }, BackgroundColor(Color::srgba(0.05, 0.06, 0.16, 0.8)), BorderRadius::all(Val::Px(6.0)), GlobalZIndex(15), Overlay, sc.clone())).with_children(|p| {
                kit::text(p, l.pick("W/S — тяга · A/D — наклон · E — ответить ЦУП", "W/S — thrust · A/D — tilt · E — answer Mission Control"), &art.body, 16.0, C_TEXT);
                kit::text(p, l.pick("Садись на зелёную площадку: там ровно. Отпустишь A/D — модуль выровняется сам.", "Land on the green pad: it’s flat. Let go of A/D and the lander levels itself."), &art.body, 15.0, C_GOOD);
                kit::text(p, l.pick("Луна притягивает в 6 раз слабее Земли: g = 1,62 м/с²", "The Moon pulls 6 times weaker than Earth: g = 1.62 m/s²"), &art.body, 15.0, C_MUTED);
            });
            // the 1202 alarm
            commands.spawn((Node { position_type: PositionType::Absolute, top: Val::Px(120.0), left: Val::Percent(50.0), width: Val::Px(440.0), margin: UiRect::left(Val::Px(-220.0)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(Color::srgba(0.45, 0.07, 0.1, 0.95)), BorderColor(Color::srgb(1.0, 0.5, 0.3)), BorderRadius::all(Val::Px(8.0)), Visibility::Hidden, GlobalZIndex(18), AlarmBox, Overlay, sc.clone())).with_children(|p| {
                kit::text(p, "1202", &art.head, 46.0, C_TEXT);
                kit::wrap_text(p, l.pick("Компьютер перегружен и не показывает приборы. Так было на Аполлоне‑11 (1969). ЦУП разрешил продолжать.", "The computer is overloaded and hides the instruments. This happened on Apollo 11 (1969). Mission Control said go on."), &art.body, 17.0, C_TEXT, 400.0);
                kit::button(p, &art, Action::LandingAck, 0, "radio", l.pick("Продолжаем! (E)", "We’re go! (E)"), true);
            });
            commands.spawn((Node { position_type: PositionType::Absolute, top: Val::Px(70.0), left: Val::Percent(50.0), width: Val::Px(600.0), margin: UiRect::left(Val::Px(-300.0)), justify_content: JustifyContent::Center, ..default() }, GlobalZIndex(16), Overlay, sc.clone())).with_children(|p| {
                p.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 28.0, ..default() }, TextColor(kit::C_ACCENT), TextLayout::new_with_justify(JustifyText::Center), CallText));
            });
            commands.spawn((Node { position_type: PositionType::Absolute, top: Val::Px(116.0), left: Val::Percent(50.0), width: Val::Px(600.0), margin: UiRect::left(Val::Px(-300.0)), justify_content: JustifyContent::Center, ..default() }, GlobalZIndex(16), Overlay, sc.clone())).with_children(|p| {
                p.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 26.0, ..default() }, TextColor(C_GOOD), TextLayout::new_with_justify(JustifyText::Center), Advice));
            });
            match lander.phase {
                Phase::Flying => {}
                Phase::Aborted(r) => {
                    commands.spawn((kit::overlay(true, 30), Overlay, sc)).with_children(|o| {
                        o.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(22.0)), border: UiRect::all(Val::Px(3.0)), max_width: Val::Px(680.0), ..default() }).with_children(|p| {
                            kit::text(p, l.pick("Прерываем посадку!", "Abort the landing!"), &art.head, 30.0, C_BAD);
                            kit::wrap_text(p, abort_text(r, l), &art.body, 19.0, C_TEXT, 600.0);
                            if let Some(t) = lander.touch.filter(|_| r != AbortReason::OffCourse) {
                                touch_table(p, &art, l, lander, t);
                            }
                            kit::wrap_text(p, l.pick("Никто не пострадал: автопилот вернул модуль на орбиту. Площадка и камни останутся на тех же местах.", "Nobody is hurt: the autopilot took the lander back to orbit. The pad and the boulders stay where they are."), &art.body, 16.0, C_MUTED, 600.0);
                            p.spawn(kit::row(12.0)).with_children(|r| {
                                kit::button(r, &art, Action::LandingRetry, 0, "retry", l.pick("Ещё заход (R)", "Try again (R)"), true);
                                if !(d.hold && d.guide) {
                                    kit::button(r, &art, Action::LandingRetryHelp, 1, "robot", l.pick("Заход с помощником", "Try with the helper"), false);
                                }
                            });
                        });
                    });
                }
                Phase::Landed => {
                    let stars = match lander.attempts {
                        1 => 3,
                        2 => 2,
                        _ => 1,
                    };
                    commands.spawn((kit::overlay(true, 30), Overlay, sc)).with_children(|o| {
                        o.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(22.0)), border: UiRect::all(Val::Px(3.0)), max_width: Val::Px(640.0), ..default() }).with_children(|p| {
                            kit::icon(p, &art, "flag", 64.0);
                            kit::text(p, l.pick("Модуль на Луне!", "The lander is on the Moon!"), &art.head, 32.0, C_GOOD);
                            kit::pips(p, stars, 3, kit::C_ACCENT, 26.0);
                            p.spawn(kit::row(16.0)).with_children(|r| {
                                for (ic, v) in [("retry", format!("{} {}", l.pick("заходов:", "approaches:"), lander.attempts)), ("droplet", format!("{} {:.0}%", l.pick("топливо:", "fuel:"), lander.fuel / lander.fuel_max * 100.0)), ("coin", format!("+{}", lander.bonus_coins()))] {
                                    r.spawn(kit::row(6.0)).with_children(|x| {
                                        kit::icon(x, &art, ic, 26.0);
                                        kit::text(x, v, &art.head, 20.0, C_TEXT);
                                    });
                                }
                            });
                            kit::wrap_text(p, l.pick("История: Армстронг посадил «Орёл», когда топлива оставалось примерно на 20 секунд.", "History: Armstrong landed the Eagle with about 20 seconds of fuel left."), &art.body, 17.0, C_MUTED, 580.0);
                            kit::button(p, &art, Action::LandingGo, 0, "walk", l.pick("Выйти на Луну", "Step onto the Moon"), true);
                        });
                    });
                }
            }
        }
    }
}

/// "What it was at touchdown / what it had to be" — one line per rule.
fn touch_table(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, lander: &Lander, t: Touch) {
    let (lv, lh, la) = lander.limits();
    let ms = l.pick("м/с", "m/s");
    let rows = [
        (l.pick("Скорость вниз", "Down speed"), format!("{:.1} {ms}", t.down.max(0.0)), format!("≤ {lv:.1} {ms}"), t.down <= lv),
        (l.pick("Скорость вбок", "Sideways speed"), format!("{:.1} {ms}", t.side.abs()), format!("≤ {lh:.1} {ms}"), t.side.abs() <= lh),
        (l.pick("Наклон", "Tilt"), format!("{:.0}°", t.tilt.abs()), format!("≤ {la:.0}°"), t.tilt.abs() <= la),
        (l.pick("Грунт", "Ground"), if t.boulders { l.pick("камни", "boulders") } else { l.pick("ровно", "flat") }.to_string(), l.pick("ровно", "flat").to_string(), !t.boulders),
    ];
    p.spawn((kit::col(4.0), BackgroundColor(kit::C_PANEL), BorderRadius::all(Val::Px(8.0)))).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(10.0)), ..default() }).with_children(|c| {
        c.spawn(kit::row(8.0)).with_children(|r| {
            for (w, h) in [(220.0, l.pick("В момент касания", "At touchdown")), (110.0, l.pick("было", "was")), (110.0, l.pick("нужно", "needed"))] {
                r.spawn(Node { width: Val::Px(w), ..default() }).with_children(|x| {
                    kit::text(x, h, &art.body, 15.0, C_MUTED);
                });
            }
        });
        for (name, was, need, ok) in rows {
            c.spawn(kit::row(8.0)).with_children(|r| {
                r.spawn(kit::row(6.0)).insert(Node { width: Val::Px(220.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|x| {
                    kit::icon(x, art, if ok { "check" } else { "cross" }, 22.0);
                    kit::text(x, name, &art.body, 17.0, C_TEXT);
                });
                for (w, v, col) in [(110.0, was, if ok { C_GOOD } else { C_BAD }), (110.0, need, C_TEXT)] {
                    r.spawn(Node { width: Val::Px(w), ..default() }).with_children(|x| {
                        kit::text(x, v, &art.head, 17.0, col);
                    });
                }
            });
        }
    });
}

#[allow(clippy::type_complexity)]
fn gauges(d: Res<Descent>, prof: Res<Prof>, art: Res<Art>, mut texts: Query<(&Gauge, &mut Text, &mut TextColor), (Without<CallText>, Without<Advice>)>, mut marks: Query<(&Mark, &mut ImageNode, &mut Visibility), Without<AlarmBox>>, mut fuel: Query<(&mut Node, &mut BackgroundColor), (With<FuelFill>, Without<LeverFill>)>, mut lever: Query<&mut Node, (With<LeverFill>, Without<FuelFill>)>, mut alarm: Query<&mut Visibility, (With<AlarmBox>, Without<Mark>)>, mut call: Query<&mut Text, (With<CallText>, Without<Gauge>, Without<Advice>)>, mut advice: Query<(&mut Text, &mut TextColor), (With<Advice>, Without<Gauge>, Without<CallText>)>) {
    let Some(lnd) = &d.lander else { return };
    let l = prof.lang();
    let ok = lnd.instruments_ok();
    let (lv, lh, la) = lnd.limits();
    let ms = l.pick("м/с", "m/s");
    // during the descent "down" is compared with a good speed for the altitude; near the ground with the limit
    let norm = lnd.safe_down().max(lv);
    let [_, side_ok, tilt_ok, flat] = lnd.checklist();
    let down_ok = -lnd.vy <= norm;
    let pad_ok = lnd.over_pad();
    let state = |g: Gauge| -> bool {
        match g {
            Gauge::Vy => down_ok,
            Gauge::Vx => side_ok,
            Gauge::Angle => tilt_ok,
            Gauge::Ground => flat,
            Gauge::Pad => pad_ok,
            _ => true,
        }
    };
    for (g, mut t, mut c) in &mut texts {
        let s = match g {
            Gauge::Alt => format!("{} {:.0} {}", l.pick("Высота", "Altitude"), lnd.alt, l.pick("м", "m")),
            Gauge::Hover => format!("{} {:.0} {}", l.pick("Зависнуть можно", "Hover left"), lnd.hover_left(), l.pick("с", "s")),
            Gauge::Vy => format!("{} {:.1} {ms} · {} ≤ {norm:.0}", l.pick("Вниз", "Down"), (-lnd.vy).max(0.0), l.pick("норма", "aim")),
            Gauge::Vx => format!("{} {:.1} {ms} · {} ≤ {lh:.1}", l.pick("Вбок", "Sideways"), lnd.vx.abs(), l.pick("норма", "aim")),
            Gauge::Angle => format!("{} {:.0}° · {} ≤ {la:.0}°", l.pick("Наклон", "Tilt"), lnd.angle.abs(), l.pick("норма", "aim")),
            Gauge::Ground => if flat { l.pick("Внизу ровно", "Flat ground below") } else { l.pick("Внизу камни!", "Boulders below!") }.to_string(),
            Gauge::Pad => {
                let dx = lnd.to_pad();
                if pad_ok {
                    l.pick("Ты над площадкой", "You’re over the pad").to_string()
                } else {
                    format!("{} {} {:.0} {}", l.pick("Площадка", "Pad"), if dx > 0.0 { l.pick("справа", "to the right") } else { l.pick("слева", "to the left") }, dx.abs(), l.pick("м", "m"))
                }
            }
        };
        let readable = ok || matches!(g, Gauge::Ground | Gauge::Pad);
        t.0 = if readable { s } else { "— — —".into() };
        c.0 = match g {
            Gauge::Alt | Gauge::Hover => C_TEXT,
            _ if !readable => C_MUTED,
            _ if state(*g) => C_GOOD,
            Gauge::Pad => C_WARN,
            _ => C_BAD,
        };
    }
    for (m, mut img, mut v) in &mut marks {
        let want = art.icon(if state(m.0) { "check" } else { "cross" });
        if img.image != want {
            img.image = want;
        }
        v.set_if_neq(if ok || matches!(m.0, Gauge::Ground | Gauge::Pad) { Visibility::Inherited } else { Visibility::Hidden });
    }
    // one clear tip near the ground: the first rule that is red, or "all green"
    for (mut t, mut c) in &mut advice {
        let tip = if lnd.phase != Phase::Flying || lnd.alt > 80.0 || !ok {
            None
        } else if !flat {
            Some((l.pick("Уйди с камней — к зелёной площадке!", "Move off the boulders — to the green pad!"), C_BAD))
        } else if -lnd.vy > norm {
            Some((l.pick("Медленнее! Прибавь тягу (W)", "Slow down! More thrust (W)"), C_BAD))
        } else if !side_ok {
            Some((l.pick("Гаси снос вбок: наклон против движения", "Stop the drift: tilt against it"), C_WARN))
        } else if !tilt_ok {
            Some((l.pick("Выровняй модуль: отпусти A/D", "Level the lander: let go of A/D"), C_WARN))
        } else {
            Some((l.pick("Всё зелёное — садись!", "All green — touch down!"), C_GOOD))
        };
        let (s, col) = tip.map(|(s, c)| (s.to_string(), c)).unwrap_or_default();
        t.0 = s;
        c.0 = col;
    }
    for (mut n, mut bg) in &mut fuel {
        let f = lnd.fuel / lnd.fuel_max;
        n.width = Val::Percent(f * 100.0);
        bg.0 = if lnd.hover_left() <= 30.0 { C_BAD } else if lnd.hover_left() <= 60.0 { C_WARN } else { C_GOOD };
    }
    for mut n in &mut lever {
        n.width = Val::Percent(lnd.throttle * 100.0);
    }
    for mut v in &mut alarm {
        *v = if lnd.alarm.is_some() && lnd.phase == Phase::Flying { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut t in &mut call {
        t.0 = d.call.as_ref().map(|c| c.0.clone()).unwrap_or_default();
    }
}
