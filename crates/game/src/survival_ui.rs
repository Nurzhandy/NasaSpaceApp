//! B2–B7 on screen: the survival panel (heat, air tightness, pressure, module strength, each
//! astronaut's suit oxygen / fatigue / suit / heartbeat), tonight's threat banner with a
//! countdown, a screen tint during threats, the U key for upgrades, the report window after each
//! evening, and the safe return to the airlock when a suit tank runs dry.
//! Shown only in `Mode::Survival` (the creative mode has no drains to show).

use crate::art::Art;
use crate::kit::{self, C_BAD, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use crate::player::Player;
use crate::windows::{Modal, Win};
use crate::world::Happened;
use crate::{AppState, Prof, Set, Sim};
use bevy::prelude::*;
use ja_core::survival::{Threat, EVENING, T_EVACUATE};
use ja_core::{i18n, Forecast, GameEvent, Mode};

#[derive(Component)]
struct Panel;
#[derive(Component, Clone, Copy, PartialEq)]
enum Line {
    Temp,
    Seal,
    Strength,
    Coins,
}
#[derive(Component)]
struct BodyRow(u8, BodyStat);
#[derive(Clone, Copy, PartialEq)]
enum BodyStat {
    SuitO2,
    Fatigue,
    Suit,
    Heart,
}
#[derive(Component)]
struct BodyFill(u8, BodyStat);
#[derive(Component)]
struct Banner;
#[derive(Component)]
struct BannerText;
#[derive(Component)]
struct BannerIcon;
#[derive(Component)]
struct Tint;

/// a threat report waiting for the window (if another window was open)
#[derive(Resource, Default)]
struct PendingReport(bool);

/// red flash after a meteorite hit
#[derive(Resource, Default)]
struct Flash(f32);

pub struct SurvivalUiPlugin;
impl Plugin for SurvivalUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingReport>()
            .init_resource::<Flash>()
            .add_systems(Update, spawn.run_if(resource_added::<Sim>))
            .add_systems(Update, (events, keys).in_set(Set::Logic).run_if(in_state(AppState::Playing)))
            .add_systems(Update, (panel, banner, tint).in_set(Set::View).run_if(in_state(AppState::Playing)));
    }
}

fn survival(sim: &Sim) -> bool {
    sim.0.state().mode == Mode::Survival
}

fn spawn(mut commands: Commands, art: Res<Art>, prof: Res<Prof>, sim: Res<Sim>) {
    let st = sim.0.state();
    let sc = StateScoped(AppState::Playing);
    // the threat tint under the HUD
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::NONE), GlobalZIndex(1), Pickable::IGNORE, Tint, sc.clone()));
    if st.mode != Mode::Survival {
        return;
    }
    let l = prof.lang();
    // tonight's banner (top centre, under the sol clock)
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, top: Val::Px(84.0), left: Val::Percent(50.0), width: Val::Px(560.0), margin: UiRect::left(Val::Px(-280.0)), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(8.0)), border: UiRect::all(Val::Px(2.0)), ..default() },
            BackgroundColor(kit::C_BG),
            BorderColor(C_LINE),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Hidden,
            GlobalZIndex(11),
            Banner,
            sc.clone(),
        ))
        .with_children(|p| {
            p.spawn((ImageNode::new(art.icon("radio")), Node { width: Val::Px(36.0), height: Val::Px(36.0), ..default() }, BannerIcon));
            p.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 18.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(490.0), ..default() }, BannerText));
        });
    // the survival panel (left, under the resources)
    let players = st.players.len() as u8;
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, right: Val::Px(12.0), bottom: Val::Px(60.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(8.0)), border: UiRect::all(Val::Px(2.0)), width: Val::Px(300.0), ..default() },
            BackgroundColor(kit::C_BG),
            BorderColor(C_LINE),
            BorderRadius::all(Val::Px(8.0)),
            GlobalZIndex(10),
            Panel,
            sc,
        ))
        .with_children(|p| {
            for (line, ic) in [(Line::Temp, "snow"), (Line::Seal, "house"), (Line::Strength, "hammer"), (Line::Coins, "coin")] {
                p.spawn(kit::row(8.0)).with_children(|r| {
                    kit::icon(r, &art, ic, 26.0);
                    r.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 16.0, ..default() }, TextColor(C_TEXT), line));
                });
            }
            for i in 0..players {
                kit::text(p, if players > 1 { format!("{} {}", l.pick("Астронавт", "Astronaut"), i + 1) } else { l.pick("Астронавт", "Astronaut").to_string() }, &art.head, 16.0, kit::C_ACCENT);
                for (stat, ic) in [(BodyStat::SuitO2, "oxygen"), (BodyStat::Fatigue, "night"), (BodyStat::Suit, "shirt"), (BodyStat::Heart, "heart")] {
                    p.spawn(kit::row(6.0)).with_children(|r| {
                        kit::icon(r, &art, ic, 22.0);
                        r.spawn((Node { width: Val::Px(110.0), height: Val::Px(10.0), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(3.0)))).with_children(|b| {
                            b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOOD), BodyFill(i, stat)));
                        });
                        r.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(C_MUTED), BodyRow(i, stat)));
                    });
                }
            }
        });
}

fn panel(sim: Res<Sim>, prof: Res<Prof>, mut lines: Query<(&Line, &mut Text, &mut TextColor), Without<BodyRow>>, mut rows: Query<(&BodyRow, &mut Text), Without<Line>>, mut fills: Query<(&BodyFill, &mut Node, &mut BackgroundColor)>) {
    if !survival(&sim) {
        return;
    }
    let st = sim.0.state();
    let l = prof.lang();
    let sv = &st.surv;
    for (line, mut t, mut c) in &mut lines {
        let (s, col) = match line {
            Line::Temp => (
                format!("{} {:.0}°C · {} {:.0}°C", l.pick("Купол", "Dome"), sv.temp, l.pick("снаружи", "outside"), sv.outside),
                if sv.temp < 0.0 { C_BAD } else if sv.temp < 10.0 { C_WARN } else { C_TEXT },
            ),
            Line::Seal => (
                format!("{} {:.0}% · {:.0} {}", l.pick("Герметичн.", "Tightness"), sv.seal, sv.pressure, l.pick("кПа", "kPa")),
                if sv.seal < 60.0 { C_BAD } else if sv.seal < 100.0 { C_WARN } else { C_TEXT },
            ),
            Line::Strength => {
                let hp = st.modules.iter().map(|m| m.hp).sum::<f32>() / st.modules.len().max(1) as f32;
                let broken = st.modules.iter().filter(|m| m.broken).count();
                (format!("{} {:.0}% · {} {broken}", l.pick("Прочность", "Strength"), hp, l.pick("сломано", "broken")), if broken > 0 { C_WARN } else { C_TEXT })
            }
            Line::Coins => (format!("{} {}", st.coins, l.pick("монет · U — улучшения", "coins · U — upgrades")), kit::C_ACCENT),
        };
        t.0 = s;
        c.0 = col;
    }
    for (r, mut t) in &mut rows {
        let Some(p) = st.players.get(r.0 as usize) else { continue };
        let b = &p.body;
        t.0 = match r.1 {
            BodyStat::SuitO2 => format!("{} {:.0}%", l.pick("баллон", "tank"), b.suit_o2),
            BodyStat::Fatigue => format!("{} {:.0}%", l.pick("усталость", "fatigue"), b.fatigue),
            BodyStat::Suit => format!("{} {:.0}%", l.pick("скафандр", "suit"), b.suit),
            BodyStat::Heart => format!("{:.0} {}", b.heart, l.pick("уд/мин", "bpm")),
        };
    }
    for (f, mut n, mut bg) in &mut fills {
        let Some(p) = st.players.get(f.0 as usize) else { continue };
        let b = &p.body;
        let (v, bad) = match f.1 {
            BodyStat::SuitO2 => (b.suit_o2, b.suit_o2 < 25.0),
            BodyStat::Fatigue => (b.fatigue, b.fatigue > 75.0),
            BodyStat::Suit => (b.suit, b.suit < 40.0),
            BodyStat::Heart => ((b.heart - 50.0) / 1.0, b.heart > 120.0),
        };
        n.width = Val::Percent(v.clamp(0.0, 100.0));
        bg.0 = if bad { C_BAD } else if f.1 == BodyStat::Heart || f.1 == BodyStat::Fatigue { C_WARN } else { C_GOOD };
    }
}

fn banner(sim: Res<Sim>, prof: Res<Prof>, art: Res<Art>, mut b: Query<(&mut Visibility, &mut BorderColor), With<Banner>>, mut t: Query<&mut Text, With<BannerText>>, mut ic: Query<&mut ImageNode, With<BannerIcon>>) {
    if !survival(&sim) {
        return;
    }
    let st = sim.0.state();
    let l = prof.lang();
    // the flare banner has the same place — it wins
    let (text, icon, color) = if st.flare.is_some() {
        (None, "sun", C_BAD)
    } else if let Some(a) = st.surv.active {
        let (icon, ru, en) = i18n::threat_name(a.tonight.threat);
        let (_, todo_ru, todo_en) = i18n::threat_todo(a.tonight.threat);
        (Some(format!("{} {} · {}", l.pick("Сейчас:", "Now:"), l.pick(ru, en), l.pick(todo_ru, todo_en))), icon, C_BAD)
    } else if let Some(Forecast::Tonight(t)) = st.forecast.filter(|_| st.t < EVENING && st.sol <= st.sols) {
        let secs = ((EVENING - st.t) * ja_core::game::SOL_LEN).max(0.0);
        let m = i18n::tonight_msg(t);
        (Some(format!("{} · {} {:.0} {}", m.text(l), l.pick("через", "in"), secs, l.pick("с", "s"))), m.icon, C_WARN)
    } else if st.mc.is_some() && st.t < EVENING {
        (Some(l.pick("Ждём прогноз ЦУП на вечер…", "Waiting for Mission Control’s evening forecast…").to_string()), "antenna_bars", C_LINE)
    } else {
        (None, "radio", C_LINE)
    };
    for (mut v, mut bc) in &mut b {
        *v = if text.is_some() { Visibility::Visible } else { Visibility::Hidden };
        bc.0 = color;
    }
    if let Some(s) = text {
        for mut x in &mut t {
            x.0 = s.clone();
        }
        for mut i in &mut ic {
            i.image = art.icon(icon);
        }
    }
}

fn tint(sim: Res<Sim>, time: Res<Time>, mut flash: ResMut<Flash>, mut q: Query<&mut BackgroundColor, With<Tint>>) {
    let st = sim.0.state();
    flash.0 = (flash.0 - time.delta_secs() * 2.5).max(0.0);
    let mut c = Color::NONE;
    if st.mode == Mode::Survival {
        if let Some(a) = st.surv.active {
            c = match a.tonight.threat {
                Threat::Cold => Color::srgba(0.35, 0.6, 1.0, 0.16),
                Threat::Dust => Color::srgba(0.55, 0.45, 0.3, 0.22),
                Threat::EnergyCrisis => Color::srgba(0.0, 0.0, 0.05, 0.3),
                Threat::Meteors => Color::srgba(0.4, 0.1, 0.05, 0.1),
                Threat::Failure => Color::srgba(0.3, 0.25, 0.0, 0.08),
                Threat::Flare => Color::NONE,
            };
        }
        // the colder the dome, the bluer the screen
        let cold = ((5.0 - st.surv.temp) / (5.0 - T_EVACUATE)).clamp(0.0, 1.0);
        if cold > 0.0 {
            c = Color::srgba(0.4, 0.65, 1.0, 0.1 + 0.25 * cold);
        }
    }
    if flash.0 > 0.0 {
        c = Color::srgba(1.0, 0.35, 0.2, 0.35 * flash.0);
    }
    for mut bg in &mut q {
        bg.0 = c;
    }
}

fn events(mut ev: EventReader<Happened>, mut modal: ResMut<Modal>, mut pending: ResMut<PendingReport>, mut flash: ResMut<Flash>, mut players: Query<&mut Player>, sim: Res<Sim>) {
    for Happened(e) in ev.read() {
        match e {
            GameEvent::ThreatOver(_) => pending.0 = true,
            GameEvent::MeteorHit { .. } => flash.0 = 1.0,
            GameEvent::SuitEmpty { player } => {
                // the rules moved the astronaut to the airlock — move the sprite too
                let to = sim.0.state().players.get(*player as usize).map(|p| p.pos).unwrap_or(ja_core::game::DOOR);
                for mut p in &mut players {
                    if p.idx == *player {
                        p.pos = Vec2::new(to.0, to.1);
                    }
                }
            }
            _ => {}
        }
    }
    if pending.0 && !modal.is_open() && sim.0.state().surv.report.is_some() {
        pending.0 = false;
        modal.open(Win::Report, 0);
    }
}

fn keys(keys: Res<ButtonInput<KeyCode>>, mut modal: ResMut<Modal>, sim: Res<Sim>, build: Res<crate::build::BuildMode>) {
    if !matches!(sim.0.state().mode, Mode::Survival | Mode::Sandbox) || build.active {
        return;
    }
    if keys.just_pressed(KeyCode::KeyU) {
        if modal.win == Win::Upgrades {
            modal.close();
        } else if !modal.is_open() {
            modal.open(Win::Upgrades, 0);
        }
    }
}
