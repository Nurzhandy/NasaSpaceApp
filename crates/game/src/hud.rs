//! B5 — HUD: resource bars, sol clock, power, flare banner (LIVE/ARCHIVE), Mission Control radio
//! with comms delay, alerts for every `GameEvent`, refusal toasts, key hints.

use crate::art::Art;
use crate::kit::{self, Act, Action, C_BAD, C_BLUE, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use crate::world::{Happened, Refused};
use crate::{AppState, Prof, Set, Sim};
use astro_core::{i18n, GameApi, GameEvent};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Component)]
enum Stat {
    O2,
    Food,
    Dose,
    Battery,
    Science,
}
#[derive(Component)]
struct Fill(Stat);
#[derive(Component)]
struct Value(Stat);
#[derive(Component)]
struct SolText;
#[derive(Component)]
struct SolIcon;
#[derive(Component)]
struct SolBar;
#[derive(Component)]
struct PowerText;
#[derive(Component)]
struct MatText;
#[derive(Component)]
struct GemText;
#[derive(Component)]
struct FlareBanner;
#[derive(Component)]
struct FlareText;
#[derive(Component)]
struct FlareBar;
#[derive(Component)]
struct RadioBox;
#[derive(Component)]
struct RadioIcon;
#[derive(Component)]
struct RadioText;
#[derive(Component)]
struct AlertBox;
#[derive(Component)]
struct Toast(f32);
#[derive(Component)]
struct Mood;
#[derive(Component)]
struct ShelterNote;

struct Alert {
    icon: &'static str,
    text: String,
    color: Color,
    ttl: f32,
}
#[derive(Resource, Default)]
struct Alerts {
    list: Vec<Alert>,
    dirty: bool,
}

pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Alerts>()
            .add_systems(Update, spawn.run_if(resource_added::<Sim>))
            .add_systems(Update, (collect, bars, clock, flare, radio, alerts, toasts, shelter_note).in_set(Set::View).run_if(in_state(AppState::Playing)))
            .add_systems(OnExit(AppState::Playing), |mut a: ResMut<Alerts>| *a = Alerts::default());
    }
}

fn abs(left: Option<f32>, top: Option<f32>, right: Option<f32>, bottom: Option<f32>) -> Node {
    let v = |x: Option<f32>| x.map(Val::Px).unwrap_or(Val::Auto);
    Node { position_type: PositionType::Absolute, left: v(left), top: v(top), right: v(right), bottom: v(bottom), ..default() }
}

fn spawn(mut commands: Commands, art: Res<Art>, prof: Res<Prof>, sim: Res<Sim>) {
    let l = prof.lang();
    let cadet = sim.0.state().cadet;
    let sc = StateScoped(AppState::Playing);
    let box_style = (BackgroundColor(kit::C_BG), BorderColor(C_LINE), BorderRadius::all(Val::Px(8.0)));
    // resources
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(2.0)), ..abs(Some(12.0), Some(12.0), None, None) }, box_style, GlobalZIndex(10), sc.clone()))
        .with_children(|p| {
            for (r, ic) in [(Stat::O2, "oxygen"), (Stat::Food, "food"), (Stat::Dose, "radiation"), (Stat::Battery, "battery"), (Stat::Science, "science")] {
                if r == Stat::Food && cadet {
                    continue;
                }
                p.spawn(kit::row(8.0)).with_children(|row| {
                    kit::icon(row, &art, ic, 32.0);
                    row.spawn((Node { width: Val::Px(150.0), height: Val::Px(16.0), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(3.0)))).with_children(|b| {
                        b.spawn((Node { width: Val::Percent(50.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOOD), Fill(r)));
                    });
                    row.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 18.0, ..default() }, TextColor(C_TEXT), Node { min_width: Val::Px(58.0), ..default() }, Value(r)));
                });
            }
            p.spawn(kit::row(8.0)).with_children(|row| {
                kit::icon(row, &art, "wrench", 28.0);
                row.spawn((Text::new("0"), TextFont { font: art.head.clone(), font_size: 20.0, ..default() }, TextColor(kit::C_ACCENT), MatText));
                kit::icon(row, &art, "gem", 28.0);
                row.spawn((Text::new("0"), TextFont { font: art.head.clone(), font_size: 20.0, ..default() }, TextColor(Color::srgb(0.6, 0.95, 1.0)), GemText));
            });
        });
    // sol clock + power
    commands
        .spawn((Node { width: Val::Px(300.0), margin: UiRect::left(Val::Px(-150.0)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(8.0)), border: UiRect::all(Val::Px(2.0)), ..abs(None, Some(12.0), None, None) }, box_style, GlobalZIndex(10), sc.clone()))
        .insert(Node { left: Val::Percent(50.0), width: Val::Px(300.0), margin: UiRect::left(Val::Px(-150.0)), position_type: PositionType::Absolute, top: Val::Px(12.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(8.0)), border: UiRect::all(Val::Px(2.0)), ..default() })
        .with_children(|p| {
            p.spawn(kit::row(10.0)).with_children(|r| {
                r.spawn((ImageNode::new(art.icon("sun")), Node { width: Val::Px(30.0), height: Val::Px(30.0), ..default() }, SolIcon));
                r.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 22.0, ..default() }, TextColor(C_TEXT), SolText));
                kit::icon(r, &art, "power", 26.0);
                r.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 20.0, ..default() }, TextColor(C_WARN), PowerText));
            });
            p.spawn((Node { width: Val::Percent(100.0), height: Val::Px(6.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)))).with_children(|b| {
                b.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(kit::C_ACCENT), SolBar));
            });
        });
    // flare banner
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, top: Val::Px(84.0), left: Val::Percent(50.0), width: Val::Px(560.0), margin: UiRect::left(Val::Px(-280.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(3.0)), ..default() },
            BackgroundColor(Color::srgba(0.45, 0.07, 0.1, 0.94)),
            BorderColor(Color::srgb(1.0, 0.5, 0.3)),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Hidden,
            GlobalZIndex(12),
            FlareBanner,
            sc.clone(),
        ))
        .with_children(|p| {
            p.spawn(kit::row(10.0)).with_children(|r| {
                kit::icon(r, &art, "sun", 40.0);
                kit::icon(r, &art, "house", 32.0);
                r.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 19.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(450.0), ..default() }, FlareText));
            });
            p.spawn((Node { width: Val::Percent(100.0), height: Val::Px(6.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)))).with_children(|b| {
                b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(1.0, 0.6, 0.3)), FlareBar));
            });
        });
    // top-right: pause + mood
    commands.spawn((kit::row(8.0), GlobalZIndex(10), sc.clone())).insert(Node { position_type: PositionType::Absolute, right: Val::Px(12.0), top: Val::Px(12.0), flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), align_items: AlignItems::Center, ..default() }).with_children(|r| {
        r.spawn((Node { padding: UiRect::all(Val::Px(6.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, box_style)).with_children(|m| {
            m.spawn((ImageNode::new(art.icon("smile")), Node { width: Val::Px(34.0), height: Val::Px(34.0), ..default() }, Mood));
        });
        kit::button(r, &art, Action::Pause, 900, "pause", "", false);
    });
    // alerts
    commands.spawn((Node { position_type: PositionType::Absolute, right: Val::Px(12.0), top: Val::Px(84.0), width: Val::Px(340.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() }, GlobalZIndex(10), AlertBox, sc.clone()));
    // radio
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Px(12.0), bottom: Val::Px(12.0), max_width: Val::Px(430.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, box_style, GlobalZIndex(10), RadioBox, sc.clone()))
        .with_children(|p| {
            kit::icon(p, &art, "radio", 36.0);
            p.spawn((ImageNode::new(art.icon("antenna_bars")), Node { width: Val::Px(30.0), height: Val::Px(30.0), ..default() }, RadioIcon));
            p.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 18.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(330.0), ..default() }, RadioText));
        });
    // key hints
    commands
        .spawn((Node { position_type: PositionType::Absolute, right: Val::Px(12.0), bottom: Val::Px(12.0), flex_direction: FlexDirection::Row, column_gap: Val::Px(14.0), padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)), ..default() }, BackgroundColor(Color::srgba(0.05, 0.06, 0.16, 0.75)), BorderRadius::all(Val::Px(6.0)), GlobalZIndex(9), sc.clone()))
        .with_children(|p| {
            for (ic, k, ru, en) in [("hammer", "B", "стройка", "build"), ("power", "Tab", "энергия", "power"), ("ok_hand", "F", "действие", "action"), ("pause", "Esc", "пауза", "pause")] {
                p.spawn(kit::row(5.0)).with_children(|r| {
                    kit::icon(r, &art, ic, 22.0);
                    kit::text(r, k, &art.head, 15.0, kit::C_ACCENT);
                    kit::text(r, l.t(ru, en), &art.body, 15.0, C_MUTED);
                });
            }
        });
    // shelter note
    commands
        .spawn((Node { position_type: PositionType::Absolute, top: Val::Percent(30.0), left: Val::Percent(50.0), margin: UiRect::left(Val::Px(-220.0)), width: Val::Px(440.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(14.0)), ..default() }, BackgroundColor(Color::srgba(0.04, 0.05, 0.14, 0.85)), BorderRadius::all(Val::Px(8.0)), Visibility::Hidden, GlobalZIndex(8), ShelterNote, sc))
        .with_children(|p| {
            kit::icon(p, &art, "house", 64.0);
            p.spawn((Text::new(l.t("Ты в жилом модуле. Радиация в 7 раз меньше.", "You’re in the habitat. Radiation is 7× lower.")), TextFont { font: art.body.clone(), font_size: 20.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
        });
}

fn collect(mut ev: EventReader<Happened>, mut refused: EventReader<Refused>, mut alerts: ResMut<Alerts>, prof: Res<Prof>, mut commands: Commands, art: Res<Art>, toasts: Query<Entity, With<Toast>>) {
    let l = prof.lang();
    for Happened(e) in ev.read() {
        if matches!(e, GameEvent::ForecastArrived(_) | GameEvent::Flare { .. }) {
            continue;
        }
        if let Some((icon, text)) = i18n::event(e, l) {
            let color = match e {
                GameEvent::Flare { .. } => C_BAD,
                GameEvent::ModuleBroken(_) | GameEvent::Leak(_) | GameEvent::DustStorm => C_WARN,
                GameEvent::Built(_) | GameEvent::Repaired(_) | GameEvent::SamplesDelivered { .. } | GameEvent::Supported | GameEvent::HarvestReady => C_GOOD,
                _ => C_BLUE,
            };
            alerts.list.push(Alert { icon, text, color, ttl: if matches!(e, GameEvent::Flare { .. }) { 9.0 } else { 5.0 } });
            if alerts.list.len() > 4 {
                alerts.list.remove(0);
            }
            alerts.dirty = true;
        }
    }
    for Refused(why) in refused.read() {
        for e in &toasts {
            commands.entity(e).despawn();
        }
        let msg = i18n::refusal(why, l);
        commands
            .spawn((Node { position_type: PositionType::Absolute, bottom: Val::Px(210.0), left: Val::Percent(50.0), width: Val::Px(520.0), margin: UiRect::left(Val::Px(-260.0)), justify_content: JustifyContent::Center, ..default() }, GlobalZIndex(25), Toast(2.5), StateScoped(AppState::Playing)))
            .with_children(|p| {
                p.spawn((Node { flex_direction: FlexDirection::Row, padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)), column_gap: Val::Px(10.0), align_items: AlignItems::Center, ..default() }, BackgroundColor(Color::srgba(0.45, 0.08, 0.12, 0.95)), BorderRadius::all(Val::Px(8.0)))).with_children(|row| {
                    kit::icon(row, &art, "hint", 30.0);
                    kit::text(row, msg.clone(), &art.body, 20.0, C_TEXT);
                });
            });
    }
}

fn bars(sim: Res<Sim>, time: Res<Time>, mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor)>, mut vals: Query<(&Value, &mut Text), (Without<MatText>, Without<GemText>)>, mut mats: Query<&mut Text, (With<MatText>, Without<Value>, Without<GemText>)>, mut gems: Query<&mut Text, (With<GemText>, Without<Value>, Without<MatText>)>, mut mood: Query<&mut ImageNode, With<Mood>>, art: Res<Art>) {
    let st = sim.0.state();
    let pulse = 0.6 + 0.4 * (time.elapsed_secs() * 6.0).sin();
    for (f, mut n, mut bg) in &mut fills {
        let (v, inverse, neutral) = match f.0 {
            Stat::O2 => (st.o2, false, false),
            Stat::Food => (st.food, false, false),
            Stat::Dose => (st.dose, true, false),
            Stat::Battery => (st.battery / st.battery_max.max(1.0) * 100.0, false, true),
            Stat::Science => ((st.science / st.goal.max(1.0)) * 100.0, false, true),
        };
        n.width = Val::Percent(v.clamp(0.0, 100.0));
        bg.0 = if neutral {
            if f.0 == Stat::Science { C_BLUE } else { C_WARN }
        } else if (inverse && v >= 70.0) || (!inverse && v <= 25.0) {
            C_BAD.with_alpha(pulse)
        } else if (inverse && v >= 45.0) || (!inverse && v <= 45.0) {
            C_WARN
        } else {
            C_GOOD
        };
    }
    for (v, mut t) in &mut vals {
        t.0 = match v.0 {
            Stat::O2 => format!("{:.0}%", st.o2),
            Stat::Food => format!("{:.0}%", st.food),
            Stat::Dose => format!("{:.0}%", st.dose),
            Stat::Battery => format!("{:.0}/{:.0}", st.battery, st.battery_max),
            Stat::Science => format!("{:.0}/{:.0}", st.science, st.goal),
        };
    }
    for mut t in &mut mats {
        t.0 = st.materials.to_string();
    }
    let carried: u32 = st.players.iter().map(|p| p.carried).sum();
    for mut t in &mut gems {
        t.0 = carried.to_string();
    }
    for mut i in &mut mood {
        i.image = art.icon(if st.crew_mood.is_some() { "f_worried" } else if st.morale >= 70.0 { "f_happy" } else if st.morale >= 40.0 { "smile" } else if st.morale >= 25.0 { "neutral" } else { "frown" });
    }
}

fn clock(sim: Res<Sim>, prof: Res<Prof>, art: Res<Art>, mut t: Query<&mut Text, (With<SolText>, Without<PowerText>)>, mut p: Query<(&mut Text, &mut TextColor), (With<PowerText>, Without<SolText>)>, mut ic: Query<&mut ImageNode, With<SolIcon>>, mut bar: Query<&mut Node, With<SolBar>>) {
    let st = sim.0.state();
    let l = prof.lang();
    for mut x in &mut t {
        x.0 = format!("{} {}/{}", l.t("СОЛ", "SOL"), st.sol, st.sols);
    }
    for (mut x, mut c) in &mut p {
        x.0 = format!("{}/{}", st.power_used, st.power_gen);
        c.0 = if st.power_used > st.power_gen { if st.battery > 0.0 { C_WARN } else { C_BAD } } else { C_GOOD };
    }
    for mut i in &mut ic {
        i.image = art.icon(if st.flare.is_some() { "flash" } else if st.night || st.daylight < 0.05 { "night" } else if st.dust_storm { "tornado" } else { "sun" });
    }
    for mut n in &mut bar {
        n.width = Val::Percent(st.day_t.clamp(0.0, 1.0) * 100.0);
    }
}

fn flare(sim: Res<Sim>, prof: Res<Prof>, mut b: Query<&mut Visibility, With<FlareBanner>>, mut t: Query<&mut Text, With<FlareText>>, mut bar: Query<&mut Node, With<FlareBar>>) {
    let st = sim.0.state();
    let l = prof.lang();
    for mut v in &mut b {
        *v = if st.flare.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Some(f) = &st.flare {
        for mut x in &mut t {
            let tag = if f.live { "LIVE · NASA DONKI".to_string() } else { l.t("АРХИВ · NASA DONKI", "ARCHIVE · NASA DONKI").to_string() };
            x.0 = format!("{}\n{} ({}) · {}", l.t("ВСПЫШКА! Беги в жилой модуль или под щит", "FLARE! Get into the habitat or under the shield"), l.t(&f.label_ru, &f.label_en), f.date, tag);
        }
        for mut n in &mut bar {
            n.width = Val::Percent(f.left.clamp(0.0, 1.0) * 100.0);
        }
    }
}

fn radio(sim: Res<Sim>, prof: Res<Prof>, art: Res<Art>, mut b: Query<&mut Visibility, With<RadioBox>>, mut i: Query<&mut ImageNode, With<RadioIcon>>, mut t: Query<&mut Text, With<RadioText>>) {
    let st = sim.0.state();
    let l = prof.lang();
    for mut v in &mut b {
        *v = if st.radio.is_some() || st.silence { Visibility::Visible } else { Visibility::Hidden };
    }
    let (icon, text) = match &st.radio {
        _ if st.silence => ("antenna_bars", l.t("…тишина: Солнце глушит связь", "…silence: the Sun blocks the radio").to_string()),
        Some(r) if r.arrives_in > 0.0 => ("antenna_bars", format!("{} {:.1} {}", l.t("Сигнал летит с Земли…", "Signal travelling from Earth…"), r.arrives_in, l.t("с", "s"))),
        Some(r) => (r.icon, format!("{}: {}", l.t("ЦУП", "Mission Control"), l.t(&r.text_ru, &r.text_en))),
        None => ("radio", String::new()),
    };
    for mut x in &mut i {
        x.image = art.icon(icon);
    }
    for mut x in &mut t {
        x.0 = text.clone();
    }
}

fn alerts(mut commands: Commands, time: Res<Time>, mut a: ResMut<Alerts>, art: Res<Art>, root: Query<Entity, With<AlertBox>>) {
    let dt = time.delta_secs();
    let n = a.list.len();
    for x in &mut a.list {
        x.ttl -= dt;
    }
    a.list.retain(|x| x.ttl > 0.0);
    if a.list.len() != n {
        a.dirty = true;
    }
    if !a.dirty {
        return;
    }
    a.dirty = false;
    let Ok(root) = root.single() else { return };
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|p| {
        for x in &a.list {
            p.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(8.0)), border: UiRect::left(Val::Px(6.0)), ..default() }, BackgroundColor(kit::C_BG), BorderColor(x.color), BorderRadius::all(Val::Px(6.0)))).with_children(|r| {
                kit::icon(r, &art, x.icon, 30.0);
                kit::wrap_text(r, x.text.clone(), &art.body, 17.0, C_TEXT, 270.0);
            });
        }
    });
}

fn toasts(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Toast)>) {
    for (e, mut t) in &mut q {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn shelter_note(sim: Res<Sim>, mut q: Query<&mut Visibility, With<ShelterNote>>) {
    let st = sim.0.state();
    let show = !st.coop && st.players.first().map(|p| p.in_shelter).unwrap_or(false);
    for mut v in &mut q {
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
    }
}

#[allow(dead_code)]
fn unused(_: Act) {}
