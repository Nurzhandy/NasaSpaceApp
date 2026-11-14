//! In-game UI: HUD, prompts, alerts, Mission Control bubble, modal windows, and the shared
//! button dispatcher (mouse + keyboard/gamepad navigation). Icon-first, minimal text.

use crate::data::*;
use crate::i18n::Lang;
use crate::player::{Inputs, Player};
use crate::sim::{Sim, BATTERY_MAX, SHIELD_MAX};
use crate::{AppState, GameConfig};
use bevy::prelude::*;
use std::collections::HashMap;

// ------------------------------------------------------------------ assets

#[derive(Resource)]
pub struct Fonts {
    pub head: Handle<Font>,
    pub body: Handle<Font>,
}

#[derive(Resource, Default)]
pub struct Icons(pub HashMap<&'static str, Handle<Image>>);
impl Icons {
    pub fn get(&self, name: &str) -> Handle<Image> {
        self.0.get(name).cloned().unwrap_or_else(|| self.0.get("warning").cloned().unwrap_or_default())
    }
}

pub const ICON_NAMES: &[&str] = &[
    "alien", "antenna_bars", "astronaut", "back", "battery", "beacon", "book", "box", "bread", "broom", "cat", "check", "clipboard", "comet", "coop", "cross", "crown", "desert", "droplet", "earth", "exit",
    "f_angry", "f_happy", "f_lonely", "f_proud", "f_sad", "f_tired", "f_worried", "flag", "flash", "food", "frown", "gamepad", "gear", "gem", "globe", "goggles", "hammer", "heart", "hint", "home",
    "hourglass", "house", "ice", "info", "key", "lab", "lock", "map", "mars", "medal", "moon", "mute", "neutral", "night", "ok_hand", "oxygen", "paint", "pause", "planet", "plant", "plate", "play",
    "power", "radiation", "radio", "retry", "robot", "rock", "rocket", "rover", "run", "s_call", "s_hug", "s_music", "s_photo", "s_talk", "sad_no", "satellite", "science", "seedling", "shield",
    "shirt", "shop", "smile", "snow", "sos", "sound", "sparkles", "speech", "star", "stopwatch", "strong", "sun", "teacher", "test", "thumbs", "tornado", "trophy", "walk", "warning", "wind", "wrench",
];

pub fn load_ui_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut icons = Icons::default();
    for n in ICON_NAMES {
        icons.0.insert(n, asset_server.load(format!("icons/{n}.png")));
    }
    commands.insert_resource(icons);
    commands.insert_resource(Fonts { head: asset_server.load("fonts/RussoOne-Regular.ttf"), body: asset_server.load("fonts/PTSans-Bold.ttf") });
}

// ------------------------------------------------------------------ colors

pub const C_PANEL: Color = Color::srgba(0.08, 0.1, 0.26, 0.92);
pub const C_PANEL2: Color = Color::srgba(0.16, 0.2, 0.42, 0.95);
pub const C_LINE: Color = Color::srgba(0.63, 0.7, 1.0, 0.25);
pub const C_TEXT: Color = Color::srgb(0.93, 0.95, 1.0);
pub const C_MUTED: Color = Color::srgb(0.66, 0.7, 0.86);
pub const C_ACCENT: Color = Color::srgb(1.0, 0.71, 0.28);
pub const C_GOOD: Color = Color::srgb(0.3, 0.85, 0.65);
pub const C_BAD: Color = Color::srgb(1.0, 0.37, 0.5);
pub const C_WARN: Color = Color::srgb(1.0, 0.79, 0.29);
pub const C_BLUE: Color = Color::srgb(0.42, 0.55, 1.0);
pub const C_FOCUS: Color = Color::srgb(1.0, 0.85, 0.3);

// ------------------------------------------------------------------ actions & focus

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    GoMenu,
    GoMaps(bool),
    GoShop(u8),
    GoBadges,
    GoInfo,
    ToggleLang,
    ToggleSound,
    ToggleAge,
    PickMap(MapId),
    Launch,
    SetPlan(u8),
    ToggleLive,
    Buy(u8, usize),
    Wear(u8, usize),
    Board(u8, i8),
    Close,
    Choose(u8),
    Feel(usize),
    Support(usize),
    Pause,
    Resume,
    Beacon,
    StartBreath,
    ExitToMenu,
    PlayAgain,
    TutorialNext,
    Helped(u8),
    Quit,
}

#[derive(Component)]
pub struct UiAction(pub Action);
#[derive(Component)]
pub struct FocusOrder(pub u16);
#[derive(Resource, Default)]
pub struct Focus {
    pub idx: usize,
    pub dirty: bool,
}

// ------------------------------------------------------------------ in-game state resources

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ModalKind {
    #[default]
    None,
    Board,
    Harvest,
    Cargo,
    Emotion(usize),
    Pause,
    Beacon,
    Tutorial(u8),
}

#[derive(Resource, Default)]
pub struct Modal {
    pub kind: ModalKind,
    pub who: u8,
    pub picked: Option<usize>,
    pub breathing: bool,
    pub t: f32,
    pub version: u32,
}
impl Modal {
    pub fn open(&mut self, kind: ModalKind, who: u8) {
        self.kind = kind;
        self.who = who;
        self.picked = None;
        self.breathing = false;
        self.t = 0.0;
        self.version += 1;
    }
    pub fn close(&mut self) {
        self.open(ModalKind::None, 0);
    }
    pub fn touch(&mut self) {
        self.version += 1;
    }
    pub fn blocks(&self, idx: u8) -> bool {
        match self.kind {
            ModalKind::None => false,
            ModalKind::Pause | ModalKind::Beacon | ModalKind::Tutorial(_) => true,
            _ => self.who == idx,
        }
    }
}

pub struct Prompt {
    pub icon: &'static str,
    pub ru: &'static str,
    pub en: &'static str,
    pub progress: Option<f32>,
    pub warn: bool,
}
#[derive(Resource, Default)]
pub struct Prompts(pub [Option<Prompt>; 2]);

#[derive(Clone)]
pub struct Alert {
    pub icon: &'static str,
    pub ru: String,
    pub en: String,
    pub ttl: f32,
    pub color: Color,
}
impl Alert {
    pub fn info(icon: &'static str, ru: &str, en: &str) -> Self { Self { icon, ru: ru.into(), en: en.into(), ttl: 5.0, color: C_BLUE } }
    pub fn good(icon: &'static str, ru: &str, en: &str) -> Self { Self { icon, ru: ru.into(), en: en.into(), ttl: 5.0, color: C_GOOD } }
    pub fn warn(icon: &'static str, ru: &str, en: &str) -> Self { Self { icon, ru: ru.into(), en: en.into(), ttl: 7.0, color: C_WARN } }
    pub fn danger_owned(icon: &'static str, ru: String, en: String) -> Self { Self { icon, ru, en, ttl: 9.0, color: C_BAD } }
    pub fn badge(icon: &'static str, ru: &str, en: &str) -> Self { Self { icon, ru: ru.into(), en: en.into(), ttl: 6.0, color: C_ACCENT } }
}
#[derive(Resource, Default)]
pub struct Alerts {
    pub items: Vec<Alert>,
    pub dirty: bool,
}
impl Alerts {
    pub fn push(&mut self, a: Alert) {
        self.items.push(a);
        if self.items.len() > 4 {
            self.items.remove(0);
        }
        self.dirty = true;
    }
}

#[derive(Resource, Default)]
pub struct McMsg {
    pub icon: &'static str,
    pub ru: &'static str,
    pub en: &'static str,
    pub sent_at: f32,
    pub arrive_at: f32,
    pub active: bool,
    pub played: bool,
}

// ------------------------------------------------------------------ widgets

pub fn lang(p: &crate::save::Profile) -> Lang {
    if p.lang_en { Lang::En } else { Lang::Ru }
}

pub fn text(p: &mut ChildSpawnerCommands, s: impl Into<String>, font: &Handle<Font>, size: f32, color: Color) -> Entity {
    p.spawn((Text::new(s), TextFont { font: font.clone(), font_size: size, ..default() }, TextColor(color))).id()
}

pub fn icon(p: &mut ChildSpawnerCommands, icons: &Icons, name: &str, size: f32) -> Entity {
    p.spawn((ImageNode::new(icons.get(name)), Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() })).id()
}

pub fn panel_node() -> Node {
    Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(22.0)), row_gap: Val::Px(12.0), border: UiRect::all(Val::Px(2.0)), ..default() }
}

/// A big button: icon + label. `order` enables keyboard/gamepad focus.
pub fn icon_button(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, action: Action, order: u16, icon_name: &str, label: &str, primary: bool) -> Entity {
    let bg = if primary { Color::srgb(1.0, 0.65, 0.2) } else { C_PANEL2 };
    let fg = if primary { Color::srgb(0.16, 0.08, 0.0) } else { C_TEXT };
    p.spawn((
        Button,
        UiAction(action),
        FocusOrder(order),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(18.0), Val::Px(12.0)),
            border: UiRect::all(Val::Px(3.0)),
            min_height: Val::Px(60.0),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor(C_LINE),
        BorderRadius::all(Val::Px(18.0)),
    ))
    .with_children(|b| {
        if !icon_name.is_empty() {
            icon(b, icons, icon_name, 36.0);
        }
        if !label.is_empty() {
            text(b, label, &fonts.body, 24.0, fg);
        }
    })
    .id()
}

/// Square icon-only button (tiles in menus).
pub fn tile_button(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, action: Action, order: u16, icon_name: &str, label: &str, w: f32, bg: Color) -> Entity {
    p.spawn((
        Button,
        UiAction(action),
        FocusOrder(order),
        Node {
            width: Val::Px(w),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(16.0)),
            border: UiRect::all(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor(C_LINE),
        BorderRadius::all(Val::Px(22.0)),
    ))
    .with_children(|b| {
        icon(b, icons, icon_name, 64.0);
        if !label.is_empty() {
            b.spawn((Text::new(label), TextFont { font: fonts.body.clone(), font_size: 22.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
        }
    })
    .id()
}

pub fn overlay_root(commands: &mut Commands, marker: impl Bundle, dim: bool) -> Entity {
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            BackgroundColor(if dim { Color::srgba(0.0, 0.0, 0.05, 0.6) } else { Color::NONE }),
            GlobalZIndex(50),
            marker,
        ))
        .id()
}

// ------------------------------------------------------------------ HUD

#[derive(Component)]
pub struct HudRoot;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Meter {
    O2,
    Food,
    Dose,
    Battery,
    Science,
}
#[derive(Component)]
pub struct MeterFill(pub Meter);
#[derive(Component)]
pub struct MeterText(pub Meter);
#[derive(Component)]
pub struct MeterRow(pub Meter);
#[derive(Component)]
pub struct SolText;
#[derive(Component)]
pub struct DayFill;
#[derive(Component)]
pub struct DayIcon;
#[derive(Component)]
pub struct GemText;
#[derive(Component)]
pub struct PromptRoot(pub u8);
#[derive(Component)]
pub struct PromptIcon(pub u8);
#[derive(Component)]
pub struct PromptText(pub u8);
#[derive(Component)]
pub struct PromptFill(pub u8);
#[derive(Component)]
pub struct AlertRoot;
#[derive(Component)]
pub struct McRoot;
#[derive(Component)]
pub struct McIcon;
#[derive(Component)]
pub struct McText;
#[derive(Component)]
pub struct ModalRoot;
#[derive(Component)]
pub struct BeaconOrb;
#[derive(Component)]
pub struct BeaconText;
#[derive(Component)]
pub struct ShelterOverlay;
#[derive(Component)]
pub struct FlareVignette;
#[derive(Component)]
pub struct CrewFace;

pub fn spawn_hud(mut commands: Commands, icons: Res<Icons>, fonts: Res<Fonts>, profile: Res<crate::save::Profile>, cfg: Res<GameConfig>) {
    let l = lang(&profile);
    let cadet = profile.cadet;
    // top-left meters
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Px(14.0), top: Val::Px(12.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(12.0)), ..default() }, BackgroundColor(C_PANEL), BorderRadius::all(Val::Px(18.0)), HudRoot, StateScoped(AppState::Playing)))
        .with_children(|p| {
            for (m, ic) in [(Meter::O2, "oxygen"), (Meter::Food, "food"), (Meter::Dose, "radiation"), (Meter::Battery, "battery"), (Meter::Science, "science")] {
                if m == Meter::Food && cadet {
                    continue;
                }
                p.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }, MeterRow(m))).with_children(|r| {
                    icon(r, &icons, ic, 38.0);
                    r.spawn((Node { width: Val::Px(190.0), height: Val::Px(20.0), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(10.0))))
                        .with_children(|bar| {
                            bar.spawn((Node { width: Val::Percent(50.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOOD), BorderRadius::all(Val::Px(10.0)), MeterFill(m)));
                        });
                    r.spawn((Text::new("--"), TextFont { font: fonts.body.clone(), font_size: 20.0, ..default() }, TextColor(C_TEXT), Node { min_width: Val::Px(64.0), ..default() }, MeterText(m)));
                });
            }
        });
    // top-center sol clock
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), top: Val::Px(12.0), margin: UiRect::left(Val::Px(-130.0)), width: Val::Px(260.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(10.0)), ..default() }, BackgroundColor(C_PANEL), BorderRadius::all(Val::Px(18.0)), StateScoped(AppState::Playing)))
        .with_children(|p| {
            p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                r.spawn((ImageNode::new(icons.get("sun")), Node { width: Val::Px(34.0), height: Val::Px(34.0), ..default() }, DayIcon));
                r.spawn((Text::new("SOL 1"), TextFont { font: fonts.head.clone(), font_size: 26.0, ..default() }, TextColor(C_TEXT), SolText));
            });
            p.spawn((Node { width: Val::Percent(100.0), height: Val::Px(8.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(4.0)))).with_children(|bar| {
                bar.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_ACCENT), BorderRadius::all(Val::Px(4.0)), DayFill));
            });
        });
    // top-right: gems + pause
    commands
        .spawn((Node { position_type: PositionType::Absolute, right: Val::Px(14.0), top: Val::Px(12.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }, StateScoped(AppState::Playing)))
        .with_children(|p| {
            p.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)), ..default() }, BackgroundColor(C_PANEL), BorderRadius::all(Val::Px(16.0)))).with_children(|r| {
                icon(r, &icons, "gem", 28.0);
                r.spawn((Text::new("0"), TextFont { font: fonts.head.clone(), font_size: 22.0, ..default() }, TextColor(C_TEXT), GemText));
            });
            p.spawn((Button, UiAction(Action::Pause), FocusOrder(900), Node { width: Val::Px(52.0), height: Val::Px(52.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(16.0))))
                .with_children(|b| {
                    icon(b, &icons, "pause", 30.0);
                });
        });
    // crew mood face (right, under pause)
    commands
        .spawn((Node { position_type: PositionType::Absolute, right: Val::Px(14.0), top: Val::Px(76.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)), ..default() }, BackgroundColor(C_PANEL), BorderRadius::all(Val::Px(16.0)), StateScoped(AppState::Playing)))
        .with_children(|r| {
            icon(r, &icons, "coop", 26.0);
            r.spawn((ImageNode::new(icons.get("smile")), Node { width: Val::Px(30.0), height: Val::Px(30.0), ..default() }, CrewFace));
        });
    // alerts (right side)
    commands.spawn((Node { position_type: PositionType::Absolute, right: Val::Px(14.0), top: Val::Px(130.0), width: Val::Px(360.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() }, AlertRoot, StateScoped(AppState::Playing)));
    // Mission Control bubble (bottom-left)
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Px(14.0), bottom: Val::Px(14.0), max_width: Val::Px(420.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(12.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(18.0)), McRoot, StateScoped(AppState::Playing)))
        .with_children(|p| {
            icon(p, &icons, "radio", 40.0);
            p.spawn((ImageNode::new(icons.get("check")), Node { width: Val::Px(34.0), height: Val::Px(34.0), ..default() }, McIcon));
            p.spawn((Text::new(""), TextFont { font: fonts.body.clone(), font_size: 20.0, ..default() }, TextColor(C_TEXT), McText));
        });
    // prompts (bottom center; one per player in co-op)
    let n = if cfg.coop { 2 } else { 1 };
    for i in 0..n {
        let (left, ml) = if cfg.coop { (Val::Percent(25.0 + 50.0 * i as f32), -170.0) } else { (Val::Percent(50.0), -170.0) };
        commands
            .spawn((Node { position_type: PositionType::Absolute, left, bottom: Val::Px(90.0), margin: UiRect::left(Val::Px(ml)), width: Val::Px(340.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(C_PANEL), BorderColor(C_ACCENT), BorderRadius::all(Val::Px(20.0)), Visibility::Hidden, PromptRoot(i as u8), StateScoped(AppState::Playing)))
            .with_children(|p| {
                p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
                    r.spawn((ImageNode::new(icons.get("check")), Node { width: Val::Px(40.0), height: Val::Px(40.0), ..default() }, PromptIcon(i as u8)));
                    let key = if i == 0 { "F" } else { "⇧" };
                    r.spawn((Node { padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(Color::srgb(0.95, 0.95, 1.0)), BorderColor(C_LINE), BorderRadius::all(Val::Px(8.0)))).with_children(|k| {
                        text(k, key, &fonts.head, 22.0, Color::srgb(0.1, 0.1, 0.2));
                    });
                    r.spawn((Text::new(""), TextFont { font: fonts.body.clone(), font_size: 22.0, ..default() }, TextColor(C_TEXT), PromptText(i as u8)));
                });
                p.spawn((Node { width: Val::Percent(100.0), height: Val::Px(8.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(4.0)))).with_children(|bar| {
                    bar.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOOD), BorderRadius::all(Val::Px(4.0)), PromptFill(i as u8)));
                });
            });
    }
    // flare vignette + shelter overlay
    commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), border: UiRect::all(Val::Px(18.0)), ..default() }, BorderColor(Color::srgba(1.0, 0.4, 0.2, 0.0)), FlareVignette, GlobalZIndex(5), StateScoped(AppState::Playing)));
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() }, BackgroundColor(Color::srgba(0.02, 0.03, 0.08, 0.7)), Visibility::Hidden, ShelterOverlay, GlobalZIndex(4), StateScoped(AppState::Playing)))
        .with_children(|p| {
            icon(p, &icons, "house", 96.0);
            text(p, l.t("Ты в убежище. Радиация снижена.", "You are in the shelter. Radiation reduced."), &fonts.body, 26.0, C_TEXT);
        });
    let _ = l;
}

pub fn update_hud(
    sim: Option<Res<Sim>>,
    time: Res<Time>,
    profile: Res<crate::save::Profile>,
    icons: Res<Icons>,
    mut fills: Query<(&MeterFill, &mut Node, &mut BackgroundColor), (Without<DayFill>, Without<PromptFill>)>,
    mut texts: Query<(&MeterText, &mut Text), (Without<SolText>, Without<GemText>, Without<McText>, Without<PromptText>)>,
    mut sol: Query<&mut Text, (With<SolText>, Without<MeterText>, Without<GemText>, Without<McText>, Without<PromptText>)>,
    mut day: Query<&mut Node, (With<DayFill>, Without<MeterFill>, Without<PromptFill>)>,
    mut dicon: Query<&mut ImageNode, (With<DayIcon>, Without<CrewFace>)>,
    mut face: Query<&mut ImageNode, (With<CrewFace>, Without<DayIcon>)>,
    mut gems: Query<&mut Text, (With<GemText>, Without<MeterText>, Without<SolText>, Without<McText>, Without<PromptText>)>,
    players: Query<&Player>,
    mut shelter: Query<&mut Visibility, With<ShelterOverlay>>,
    mut vignette: Query<&mut BorderColor, With<FlareVignette>>,
) {
    let Some(sim) = sim else { return };
    let l = lang(&profile);
    for (f, mut node, mut bg) in &mut fills {
        let (v, inverse, txt) = match f.0 {
            Meter::O2 => (sim.o2, false, format!("{:.0}%", sim.o2)),
            Meter::Food => (sim.food, false, format!("{:.0}%", sim.food)),
            Meter::Dose => (sim.dose, true, format!("{:.0}%", sim.dose)),
            Meter::Battery => (sim.battery / BATTERY_MAX * 100.0, false, format!("{:.0}/{}", sim.battery, BATTERY_MAX as u32)),
            Meter::Science => ((sim.science / sim.goal * 100.0).min(100.0), false, format!("{:.0}/{}", sim.science, sim.goal)),
        };
        node.width = Val::Percent(v.clamp(0.0, 100.0));
        let bad = if inverse { v >= 70.0 } else { v <= 25.0 };
        let warn = if inverse { v >= 45.0 } else { v <= 45.0 };
        let neutral = matches!(f.0, Meter::Science | Meter::Battery);
        let pulse = 0.6 + 0.4 * (time.elapsed_secs() * 6.0).sin();
        bg.0 = if neutral {
            if f.0 == Meter::Science { C_BLUE } else { C_WARN }
        } else if bad {
            C_BAD.with_alpha(pulse)
        } else if warn {
            C_WARN
        } else {
            C_GOOD
        };
        let _ = txt;
    }
    for (t, mut text) in &mut texts {
        text.0 = match t.0 {
            Meter::O2 => format!("{:.0}%", sim.o2),
            Meter::Food => format!("{:.0}%", sim.food),
            Meter::Dose => format!("{:.0}%", sim.dose),
            Meter::Battery => format!("{:.0}/{}", sim.battery, BATTERY_MAX as u32),
            Meter::Science => format!("{:.0}/{}", sim.science, sim.goal),
        };
    }
    for mut t in &mut sol {
        t.0 = format!("{} {} / {}", l.t("СОЛ", "SOL"), sim.sol, sim.sols);
    }
    for mut n in &mut day {
        n.width = Val::Percent(sim.t * 100.0);
    }
    for mut i in &mut dicon {
        let name = if sim.is_night() { "night" } else if sim.dust_left > 0 { "tornado" } else if sim.flare.is_some() { "flash" } else { "sun" };
        i.image = icons.get(name);
    }
    for mut i in &mut face {
        let name = if sim.morale >= 70.0 { "f_happy" } else if sim.morale >= 40.0 { "smile" } else if sim.morale >= 25.0 { "neutral" } else { "frown" };
        i.image = icons.get(name);
    }
    let carried: u32 = players.iter().map(|p| p.carried).sum();
    for mut t in &mut gems {
        t.0 = format!("{carried}");
    }
    let any_shelter = !sim.coop && players.iter().any(|p| p.in_shelter);
    for mut v in &mut shelter {
        *v = if any_shelter { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut b in &mut vignette {
        let a = if sim.flare.is_some() { 0.35 + 0.35 * (time.elapsed_secs() * 5.0).sin().abs() } else { 0.0 };
        b.0 = Color::srgba(1.0, 0.45, 0.2, a);
    }
}

pub fn update_prompts(
    prompts: Res<Prompts>,
    profile: Res<crate::save::Profile>,
    icons: Res<Icons>,
    mut roots: Query<(&PromptRoot, &mut Visibility, &mut BorderColor)>,
    mut picons: Query<(&PromptIcon, &mut ImageNode)>,
    mut ptexts: Query<(&PromptText, &mut Text)>,
    mut pfills: Query<(&PromptFill, &mut Node)>,
) {
    let l = lang(&profile);
    for (r, mut vis, mut border) in &mut roots {
        let idx = r.0 as usize;
        match &prompts.0[idx] {
            Some(p) => {
                *vis = Visibility::Visible;
                border.0 = if p.warn { C_BAD } else { C_ACCENT };
                for (pi, mut img) in &mut picons {
                    if pi.0 == r.0 {
                        img.image = icons.get(p.icon);
                    }
                }
                for (pt, mut t) in &mut ptexts {
                    if pt.0 == r.0 {
                        t.0 = l.t(p.ru, p.en).to_string();
                    }
                }
                for (pf, mut n) in &mut pfills {
                    if pf.0 == r.0 {
                        n.width = Val::Percent(p.progress.unwrap_or(0.0).clamp(0.0, 1.0) * 100.0);
                    }
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

pub fn update_alerts(mut commands: Commands, time: Res<Time>, mut alerts: ResMut<Alerts>, icons: Res<Icons>, fonts: Res<Fonts>, profile: Res<crate::save::Profile>, root: Query<Entity, With<AlertRoot>>) {
    let dt = time.delta_secs();
    let before = alerts.items.len();
    for a in &mut alerts.items {
        a.ttl -= dt;
    }
    alerts.items.retain(|a| a.ttl > 0.0);
    if alerts.items.len() != before {
        alerts.dirty = true;
    }
    if !alerts.dirty {
        return;
    }
    alerts.dirty = false;
    let Ok(root) = root.get_single() else { return };
    let l = lang(&profile);
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|p| {
        for a in &alerts.items {
            p.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::left(Val::Px(6.0)), ..default() }, BackgroundColor(C_PANEL), BorderColor(a.color), BorderRadius::all(Val::Px(14.0))))
                .with_children(|r| {
                    icon(r, &icons, a.icon, 36.0);
                    r.spawn((Text::new(l.t(&a.ru, &a.en).to_string()), TextFont { font: fonts.body.clone(), font_size: 19.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(290.0), ..default() }));
                });
        }
    });
}

pub fn update_mc(
    time: Res<Time>,
    mut mc: ResMut<McMsg>,
    sim: Option<Res<Sim>>,
    profile: Res<crate::save::Profile>,
    icons: Res<Icons>,
    mut root: Query<&mut Visibility, With<McRoot>>,
    mut mi: Query<&mut ImageNode, With<McIcon>>,
    mut mt: Query<&mut Text, With<McText>>,
    mut sfx: crate::audio::Sfx,
) {
    let Some(sim) = sim else { return };
    let l = lang(&profile);
    let now = time.elapsed_secs();
    for mut v in &mut root {
        *v = if mc.active { Visibility::Visible } else { Visibility::Hidden };
    }
    if !mc.active {
        return;
    }
    let arrived = now >= mc.arrive_at;
    if arrived && !mc.played {
        mc.played = true;
        sfx.play("radio");
    }
    for mut i in &mut mi {
        i.image = icons.get(if arrived { mc.icon } else { "antenna_bars" });
    }
    for mut t in &mut mt {
        t.0 = if arrived {
            l.t(mc.ru, mc.en).to_string()
        } else {
            let left = (mc.arrive_at - now).max(0.0);
            format!("{} {:.1} {}", l.t("Сигнал летит с Земли…", "Signal travelling from Earth…"), left, l.t("с", "s"))
        };
    }
    if !arrived {
        mc.played = false;
    }
    let _ = sim;
}

// ------------------------------------------------------------------ modals

pub fn modal_render(
    mut commands: Commands,
    modal: Res<Modal>,
    mut last: Local<u32>,
    sim: Option<Res<Sim>>,
    icons: Res<Icons>,
    fonts: Res<Fonts>,
    profile: Res<crate::save::Profile>,
    weather: Res<crate::nasa::Weather>,
    old: Query<Entity, With<ModalRoot>>,
    mut focus: ResMut<Focus>,
) {
    if modal.version == *last {
        return;
    }
    *last = modal.version;
    for e in &old {
        commands.entity(e).despawn();
    }
    focus.idx = 0;
    focus.dirty = true;
    let Some(sim) = sim else { return };
    if modal.kind == ModalKind::None {
        return;
    }
    let l = lang(&profile);
    let root = overlay_root(&mut commands, (ModalRoot, StateScoped(AppState::Playing)), true);
    commands.entity(root).with_children(|o| {
        o.spawn((Node { min_width: Val::Px(560.0), max_width: Val::Px(760.0), ..panel_node() }, BackgroundColor(C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(26.0)))).with_children(|p| match modal.kind {
            ModalKind::Board => board(p, &sim, &icons, &fonts, l),
            ModalKind::Harvest => choice(p, &icons, &fonts, l, "plant", l.t("Урожай готов!", "Harvest is ready!"),
                [("plate", l.t("Съесть", "Eat"), "food", "+12", "heart", "+10", "science", "0"), ("test", l.t("В лабораторию", "To the lab"), "science", "+2", "food", "+4", "plate", "−8")]),
            ModalKind::Cargo => choice(p, &icons, &fonts, l, "box", l.t("Грузовой корабль", "Cargo ship"),
                [("strong", l.t("Разгрузить всё", "Unload all"), "food", "+20", "oxygen", "+8", "power", "−2"), ("ok_hand", l.t("Только нужное", "Essentials only"), "food", "+8", "power", "0", "box", "−12")]),
            ModalKind::Emotion(m) => emotion(p, &modal, m, &icons, &fonts, l),
            ModalKind::Pause => pause(p, &icons, &fonts, &profile, l),
            ModalKind::Beacon => beacon(p, &modal, &icons, &fonts, l),
            ModalKind::Tutorial(step) => tutorial(p, step, &icons, &fonts, l, &weather),
            ModalKind::None => {}
        });
    });
}

fn title_row(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, ic: &str, title: &str) {
    p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
        icon(r, icons, ic, 48.0);
        text(r, title, &fonts.head, 30.0, C_TEXT);
    });
}

fn board(p: &mut ChildSpawnerCommands, sim: &Sim, icons: &Icons, fonts: &Fonts, l: Lang) {
    let free = sim.available() as i32 - sim.alloc.used() as i32;
    title_row(p, icons, fonts, "power", l.t("Энергия на сол", "Power for this sol"));
    // free / gen / battery row
    p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(22.0), ..default() }).with_children(|r| {
        r.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() }, BackgroundColor(Color::srgba(1.0, 0.8, 0.3, 0.18)), BorderRadius::all(Val::Px(14.0)))).with_children(|f| {
            icon(f, icons, "flash", 30.0);
            text(f, format!("{free}"), &fonts.head, 34.0, if free < 0 { C_BAD } else { C_ACCENT });
            text(f, l.t("свободно", "free"), &fonts.body, 20.0, C_MUTED);
        });
        r.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|f| {
            icon(f, icons, if sim.is_night() { "night" } else { "sun" }, 28.0);
            text(f, format!("{}", sim.gen_now as i32), &fonts.head, 24.0, C_TEXT);
        });
        r.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|f| {
            icon(f, icons, "battery", 28.0);
            text(f, format!("{:.0}", sim.battery), &fonts.head, 24.0, C_TEXT);
        });
    });
    // power strip
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), flex_wrap: FlexWrap::Wrap, row_gap: Val::Px(4.0), ..default() }).with_children(|s| {
        let total = sim.available();
        let used = sim.alloc.used();
        let gen = sim.gen_now as u32;
        for i in 0..total {
            let c = if i < used { Color::srgba(1.0, 1.0, 1.0, 0.15) } else if i >= gen { Color::srgb(0.45, 0.7, 1.0) } else { C_WARN };
            s.spawn((Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() }, BackgroundColor(c), BorderRadius::all(Val::Px(6.0))));
        }
    });
    let rows: [(u8, &str, &str, &str, &str, &str, u32); 4] = [
        (0, "oxygen", l.t("Кислород", "Oxygen"), "oxygen", "+6%", "", sim.alloc.ls),
        (1, "plant", l.t("Оранжерея", "Greenhouse"), "food", "+5%", "", sim.alloc.green),
        (2, "shield", l.t("Щит", "Shield"), "radiation", "−30%", "", sim.alloc.shield),
        (3, "science", l.t("Наука", "Science"), "science", "+1", "", sim.alloc.sci),
    ];
    let mut order = 0u16;
    for (row, ic, name, eff_ic, eff, _, val) in rows {
        if row == 1 && sim.cadet {
            continue;
        }
        p.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), padding: UiRect::axes(Val::Px(6.0), Val::Px(8.0)), border: UiRect::top(Val::Px(1.0)), ..default() }, BorderColor(C_LINE))).with_children(|r| {
            icon(r, icons, ic, 40.0);
            r.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(170.0), ..default() }).with_children(|c| {
                text(c, name, &fonts.body, 22.0, C_TEXT);
                c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|e| {
                    text(e, "1", &fonts.body, 16.0, C_MUTED);
                    icon(e, icons, "flash", 16.0);
                    text(e, "=", &fonts.body, 16.0, C_MUTED);
                    icon(e, icons, eff_ic, 16.0);
                    text(e, eff, &fonts.body, 16.0, C_MUTED);
                });
            });
            step_button(r, fonts, Action::Board(row, -1), order, "−", val > 0);
            r.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(3.0), min_width: Val::Px(96.0), justify_content: JustifyContent::Center, ..default() }).with_children(|b| {
                if val == 0 {
                    text(b, "0", &fonts.head, 22.0, C_MUTED);
                }
                for _ in 0..val {
                    b.spawn((Node { width: Val::Px(16.0), height: Val::Px(24.0), ..default() }, BackgroundColor(C_WARN), BorderRadius::all(Val::Px(4.0))));
                }
            });
            let can_add = free > 0 && !(row == 2 && val >= SHIELD_MAX);
            step_button(r, fonts, Action::Board(row, 1), order + 1, "+", can_add);
        });
        order += 2;
    }
    p.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::FlexEnd, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
        icon_button(r, icons, fonts, Action::Close, 20, "check", l.t("Готово", "Done"), true);
    });
}

fn step_button(p: &mut ChildSpawnerCommands, fonts: &Fonts, action: Action, order: u16, label: &str, enabled: bool) {
    p.spawn((
        Button,
        UiAction(action),
        FocusOrder(order),
        Node { width: Val::Px(50.0), height: Val::Px(50.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(3.0)), ..default() },
        BackgroundColor(if enabled { C_PANEL2 } else { Color::srgba(0.2, 0.2, 0.3, 0.4) }),
        BorderColor(C_LINE),
        BorderRadius::all(Val::Px(14.0)),
    ))
    .with_children(|b| {
        text(b, label, &fonts.head, 30.0, if enabled { C_TEXT } else { C_MUTED });
    });
}

type ChoiceDef<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str, &'a str, &'a str);
fn choice(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, l: Lang, ic: &str, title: &str, opts: [ChoiceDef; 2]) {
    title_row(p, icons, fonts, ic, title);
    text(p, l.t("Бесплатного выбора нет: у каждого варианта есть цена.", "No choice is free: every option has a cost."), &fonts.body, 18.0, C_MUTED);
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
        for (i, (oic, label, g1i, g1, g2i, g2, ci, c)) in opts.iter().enumerate() {
            r.spawn((Button, UiAction(Action::Choose(i as u8)), FocusOrder(i as u16), Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), width: Val::Px(250.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(C_PANEL2), BorderColor(C_LINE), BorderRadius::all(Val::Px(20.0))))
                .with_children(|b| {
                    icon(b, icons, oic, 64.0);
                    text(b, *label, &fonts.body, 24.0, C_TEXT);
                    for (ii, val, col) in [(g1i, g1, C_GOOD), (g2i, g2, C_GOOD), (ci, c, C_BAD)] {
                        if val.is_empty() {
                            continue;
                        }
                        b.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|e| {
                            icon(e, icons, ii, 24.0);
                            text(e, *val, &fonts.head, 20.0, if val.starts_with('−') { C_BAD } else if *val == "0" { C_MUTED } else { col });
                        });
                    }
                });
        }
    });
    let _ = l;
}

fn emotion(p: &mut ChildSpawnerCommands, modal: &Modal, m: usize, icons: &Icons, fonts: &Fonts, l: Lang) {
    let mood = &MOODS[m];
    title_row(p, icons, fonts, "speech", mood.who);
    p.spawn((Text::new(l.t(mood.text_ru, mood.text_en)), TextFont { font: fonts.body.clone(), font_size: 22.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(620.0), ..default() }));
    match modal.picked {
        None => {
            text(p, l.t("Что он(а) чувствует?", "What are they feeling?"), &fonts.body, 20.0, C_MUTED);
            p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                for (i, f) in FEELINGS.iter().enumerate() {
                    icon_button(r, icons, fonts, Action::Feel(i), i as u16, f.icon, l.t(f.name_ru, f.name_en), false);
                }
            });
        }
        Some(pick) => {
            let f = &FEELINGS[pick];
            let ok = mood.ok.contains(&f.id);
            p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                icon(r, icons, f.icon, 34.0);
                let note = if ok {
                    l.t("Да, похоже на то. Чувства бывают разными — это нормально.", "Yes, it seems so. Feelings differ — that’s OK.")
                } else {
                    l.t("Может быть! Подсказки в истории говорят о другом. Люди чувствуют по-разному.", "Maybe! The story hints at something else. People feel different things.")
                };
                r.spawn((Text::new(note), TextFont { font: fonts.body.clone(), font_size: 20.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(560.0), ..default() }));
            });
            text(p, l.t("Как поддержать? (−1 ⚡ на связь с Землёй)", "How to help? (−1 ⚡ for the call to Earth)"), &fonts.body, 20.0, C_MUTED);
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                for (i, (ic, ru, en)) in mood.support.iter().enumerate() {
                    icon_button(r, icons, fonts, Action::Support(i), i as u16, ic, l.t(ru, en), false);
                }
            });
        }
    }
}

fn pause(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang) {
    title_row(p, icons, fonts, "pause", l.t("Пауза", "Paused"));
    p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() }).with_children(|r| {
        icon_button(r, icons, fonts, Action::Resume, 0, "play", l.t("Продолжить", "Resume"), true);
        icon_button(r, icons, fonts, Action::Beacon, 1, "beacon", l.t("Космический маяк — подышать", "Space Beacon — breathe"), false);
        icon_button(r, icons, fonts, Action::ToggleSound, 2, if profile.sound { "sound" } else { "mute" }, l.t("Звук", "Sound"), false);
        icon_button(r, icons, fonts, Action::ToggleLang, 3, "globe", if profile.lang_en { "English" } else { "Русский" }, false);
        icon_button(r, icons, fonts, Action::ExitToMenu, 4, "exit", l.t("Выйти в меню", "Exit to menu"), false);
    });
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(16.0), flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|r| {
        for (ic, ru, en) in [("walk", "WASD / стрелки", "WASD / arrows"), ("run", "Space — прыжок", "Space — jump"), ("ok_hand", "F — действие", "F — action"), ("gamepad", "геймпад: A — действие", "gamepad: A — action")] {
            r.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|e| {
                icon(e, icons, ic, 22.0);
                text(e, l.t(ru, en), &fonts.body, 16.0, C_MUTED);
            });
        }
    });
}

fn beacon(p: &mut ChildSpawnerCommands, modal: &Modal, icons: &Icons, fonts: &Fonts, l: Lang) {
    title_row(p, icons, fonts, "beacon", l.t("Космический маяк", "Space Beacon"));
    text(p, l.t("Подышим вместе с маяком. Это по желанию — остановись в любой момент.", "Breathe with the beacon. It’s optional — stop any time."), &fonts.body, 20.0, C_MUTED);
    p.spawn(Node { width: Val::Percent(100.0), height: Val::Px(220.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|c| {
        c.spawn((Node { width: Val::Px(90.0), height: Val::Px(90.0), ..default() }, BackgroundColor(Color::srgb(1.0, 0.85, 0.35)), BorderRadius::MAX, BeaconOrb));
    });
    p.spawn((Text::new(if modal.breathing { "" } else { l.t("Нажми «Начать»", "Press “Start”") }), TextFont { font: fonts.head.clone(), font_size: 26.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center), BeaconText));
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), justify_content: JustifyContent::Center, ..default() }).with_children(|r| {
        if !modal.breathing {
            icon_button(r, icons, fonts, Action::StartBreath, 0, "play", l.t("Начать", "Start"), true);
        }
        icon_button(r, icons, fonts, Action::Resume, 1, "back", l.t("К миссии", "Back to mission"), false);
    });
}

fn tutorial(p: &mut ChildSpawnerCommands, step: u8, icons: &Icons, fonts: &Fonts, l: Lang, weather: &crate::nasa::Weather) {
    title_row(p, icons, fonts, "teacher", l.t("Инструктор", "Instructor"));
    let steps: [(&str, &str, &str, &str); 4] = [
        ("walk", "ok_hand", "WASD — ходить, Space — прыгать, F — действие.", "WASD — walk, Space — jump, F — action."),
        ("power", "flash", "Подойди к пульту ⚡ у двери и раздай энергию: кислород, оранжерея, щит, наука. На всё не хватит!", "Go to the ⚡ board by the door and split power: oxygen, greenhouse, shield, science. There isn’t enough for everything!"),
        ("radiation", "house", "Следи за дозой радиации. Когда ЦУП объявит вспышку — беги в убежище 🏠.", "Watch the radiation dose. When Mission Control calls a flare — run to the shelter 🏠."),
        ("gem", "science", "Собирай образцы 💎 вокруг базы и неси в лабораторию. Удачи, астронавт!", "Collect 💎 samples around the base and bring them to the lab. Good luck, astronaut!"),
    ];
    let (i1, i2, ru, en) = steps[(step as usize).min(3)];
    p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(16.0), ..default() }).with_children(|r| {
        icon(r, icons, i1, 72.0);
        icon(r, icons, i2, 72.0);
        r.spawn((Text::new(l.t(ru, en)), TextFont { font: fonts.body.clone(), font_size: 24.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(480.0), ..default() }));
    });
    p.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::FlexEnd, align_items: AlignItems::Center, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
        text(r, format!("{}/4", step + 1), &fonts.body, 18.0, C_MUTED);
        icon_button(r, icons, fonts, Action::TutorialNext, 0, if step >= 3 { "rocket" } else { "play" }, if step >= 3 { l.t("Поехали!", "Let’s go!") } else { l.t("Дальше", "Next") }, true);
    });
    let _ = weather;
}

pub fn beacon_anim(time: Res<Time>, mut modal: ResMut<Modal>, profile: Res<crate::save::Profile>, mut orb: Query<&mut Node, With<BeaconOrb>>, mut txt: Query<&mut Text, With<BeaconText>>) {
    if modal.kind != ModalKind::Beacon || !modal.breathing {
        return;
    }
    let l = lang(&profile);
    modal.t += time.delta_secs();
    let cycle = 9.0; // 4 s in, 5 s out
    let n = (modal.t / cycle) as u32;
    let ph = modal.t % cycle;
    let (size, label) = if n >= 4 {
        (90.0, l.t("Молодец, что попробовал!", "Nice job trying it!").to_string())
    } else if ph < 4.0 {
        (90.0 + 110.0 * (ph / 4.0), format!("{} {}", l.t("Вдох…", "Breathe in…"), (ph as u32) + 1))
    } else {
        (200.0 - 110.0 * ((ph - 4.0) / 5.0), format!("{} {}", l.t("Выдох…", "Breathe out…"), ((ph - 4.0) as u32) + 1))
    };
    for mut n in &mut orb {
        n.width = Val::Px(size);
        n.height = Val::Px(size);
    }
    for mut t in &mut txt {
        t.0 = label.clone();
    }
}

// ------------------------------------------------------------------ dispatcher

#[derive(bevy::ecs::system::SystemParam)]
pub struct Ctx<'w> {
    pub modal: ResMut<'w, Modal>,
    pub sim: Option<ResMut<'w, Sim>>,
    pub profile: ResMut<'w, crate::save::Profile>,
    pub next: ResMut<'w, NextState<AppState>>,
    pub cfg: ResMut<'w, GameConfig>,
    pub alerts: ResMut<'w, Alerts>,
    pub focus: ResMut<'w, Focus>,
    pub exit: EventWriter<'w, AppExit>,
}

pub fn dispatch(a: Action, c: &mut Ctx, sfx: &mut crate::audio::Sfx) {
    sfx.play("click");
    match a {
        Action::GoMenu => c.next.set(AppState::Menu),
        Action::GoMaps(coop) => {
            c.cfg.coop = coop;
            c.next.set(AppState::Maps);
        }
        Action::GoShop(tab) => {
            c.cfg.shop_tab = tab;
            c.next.set(AppState::Shop);
            c.cfg.refresh += 1;
        }
        Action::GoBadges => c.next.set(AppState::Badges),
        Action::GoInfo => c.next.set(AppState::Info),
        Action::ToggleLang => {
            c.profile.lang_en = !c.profile.lang_en;
            crate::save::save(&c.profile);
            c.cfg.refresh += 1;
            c.modal.touch();
        }
        Action::ToggleSound => {
            c.profile.sound = !c.profile.sound;
            crate::save::save(&c.profile);
            c.cfg.refresh += 1;
            c.modal.touch();
        }
        Action::ToggleAge => {
            c.profile.cadet = !c.profile.cadet;
            crate::save::save(&c.profile);
            c.cfg.refresh += 1;
        }
        Action::PickMap(m) => {
            c.cfg.map = m;
            c.next.set(AppState::Briefing);
        }
        Action::SetPlan(p) => {
            c.cfg.plan = p;
            c.cfg.refresh += 1;
        }
        Action::ToggleLive => {
            c.cfg.live = !c.cfg.live;
            c.cfg.refresh += 1;
        }
        Action::Launch => {
            sfx.play("launch");
            c.next.set(AppState::Playing);
        }
        Action::Buy(cat, i) => {
            let (list, owned) = match cat { 0 => (SUITS, &mut c.profile.owned_suits), 1 => (VISORS, &mut c.profile.owned_visors), _ => (HATS, &mut c.profile.owned_hats) };
            let item = &list[i];
            if c.profile.stars >= item.price && !owned.iter().any(|o| o == item.id) {
                owned.push(item.id.to_string());
                c.profile.stars -= item.price;
                match cat { 0 => c.profile.suit = item.id.into(), 1 => c.profile.visor = item.id.into(), _ => c.profile.hat = item.id.into() }
                crate::save::save(&c.profile);
                sfx.play("badge");
                c.cfg.refresh += 1;
            } else {
                sfx.play("bad");
            }
        }
        Action::Wear(cat, i) => {
            let list = match cat { 0 => SUITS, 1 => VISORS, _ => HATS };
            let id = list[i].id.to_string();
            match cat { 0 => c.profile.suit = id, 1 => c.profile.visor = id, _ => c.profile.hat = id }
            crate::save::save(&c.profile);
            c.cfg.refresh += 1;
        }
        Action::Board(row, d) => {
            if let Some(sim) = c.sim.as_mut() {
                let free = sim.available() as i32 - sim.alloc.used() as i32;
                let v = match row { 0 => &mut sim.alloc.ls, 1 => &mut sim.alloc.green, 2 => &mut sim.alloc.shield, _ => &mut sim.alloc.sci };
                if d > 0 && free > 0 && !(row == 2 && *v >= SHIELD_MAX) {
                    *v += 1;
                } else if d < 0 && *v > 0 {
                    *v -= 1;
                } else {
                    sfx.play("bad");
                }
                c.modal.touch();
            }
        }
        Action::Close | Action::Resume => c.modal.close(),
        Action::Choose(i) => {
            if let Some(sim) = c.sim.as_mut() {
                match c.modal.kind {
                    ModalKind::Harvest => {
                        if i == 0 { sim.harvest_eat(); } else { sim.harvest_lab(); }
                        sfx.play("good");
                    }
                    ModalKind::Cargo => {
                        if i == 0 { sim.cargo_all(); } else { sim.cargo_some(); }
                        sfx.play("good");
                    }
                    _ => {}
                }
            }
            c.modal.close();
        }
        Action::Feel(i) => {
            c.modal.picked = Some(i);
            c.modal.touch();
        }
        Action::Support(_) => {
            if let Some(sim) = c.sim.as_mut() {
                sim.support_crew();
            }
            sfx.play("good");
            let l = lang(&c.profile);
            c.alerts.push(Alert::good("heart", l.t("Экипажу стало лучше · −1 ⚡", "The crew feels better · −1 ⚡"), l.t("Экипажу стало лучше · −1 ⚡", "The crew feels better · −1 ⚡")));
            c.modal.close();
        }
        Action::Pause => {
            if c.modal.kind == ModalKind::None {
                c.modal.open(ModalKind::Pause, 0);
            } else {
                c.modal.close();
            }
        }
        Action::Beacon => c.modal.open(ModalKind::Beacon, 0),
        Action::StartBreath => {
            c.modal.breathing = true;
            c.modal.t = 0.0;
            c.modal.touch();
            if c.profile.award("beacon") {
                crate::save::save(&c.profile);
                c.alerts.push(Alert::badge("beacon", "Нашивка: Знаю, как успокоиться", "Patch: I know how to calm down"));
            }
        }
        Action::ExitToMenu => {
            c.modal.close();
            c.next.set(AppState::Menu);
        }
        Action::PlayAgain => c.next.set(AppState::Briefing),
        Action::TutorialNext => {
            if let ModalKind::Tutorial(s) = c.modal.kind {
                if s >= 3 {
                    c.modal.close();
                } else {
                    c.modal.open(ModalKind::Tutorial(s + 1), 0);
                }
            }
        }
        Action::Helped(_) => {
            if !c.cfg.reflected {
                c.cfg.reflected = true;
                c.profile.add_points(5);
                crate::save::save(&c.profile);
                sfx.play("coin");
                c.cfg.refresh += 1;
            }
        }
        Action::Quit => {
            c.exit.write(AppExit::Success);
        }
    }
}

pub fn button_system(mut q: Query<(&Interaction, &UiAction, &mut BackgroundColor), Changed<Interaction>>, mut ctx: Ctx, mut sfx: crate::audio::Sfx) {
    let mut fire = None;
    for (i, a, mut bg) in &mut q {
        match i {
            Interaction::Pressed => fire = Some(a.0),
            Interaction::Hovered => bg.0 = bg.0.mix(&Color::WHITE, 0.08),
            Interaction::None => bg.0 = bg.0.mix(&Color::BLACK, 0.0),
        }
    }
    if let Some(a) = fire {
        dispatch(a, &mut ctx, &mut sfx);
    }
}

/// Keyboard / gamepad navigation over focusable buttons; Esc / Start opens pause while playing.
pub fn nav_system(
    inputs: Option<Res<Inputs>>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut buttons: Query<(&FocusOrder, &UiAction, &mut BorderColor)>,
    mut ctx: Ctx,
    mut sfx: crate::audio::Sfx,
) {
    let playing = *state.get() == AppState::Playing;
    let (mut nav, mut act, mut back, mut pause) = (Vec2::ZERO, false, false, false);
    if let Some(inp) = inputs.as_ref() {
        for i in inp.0 {
            nav += i.nav;
            act |= i.act;
            back |= i.back;
            pause |= i.pause;
        }
    } else {
        act = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
        back = keys.just_pressed(KeyCode::Escape);
    }
    if !playing {
        nav += Vec2::new(
            (keys.just_pressed(KeyCode::ArrowRight) as i32 - keys.just_pressed(KeyCode::ArrowLeft) as i32) as f32,
            (keys.just_pressed(KeyCode::ArrowUp) as i32 - keys.just_pressed(KeyCode::ArrowDown) as i32) as f32,
        );
        act |= keys.just_pressed(KeyCode::Enter);
    }
    let modal_open = ctx.modal.kind != ModalKind::None;
    if playing && pause {
        dispatch(Action::Pause, &mut ctx, &mut sfx);
        return;
    }
    if playing && back && modal_open && !matches!(ctx.modal.kind, ModalKind::Tutorial(_)) {
        dispatch(Action::Close, &mut ctx, &mut sfx);
        return;
    }
    if playing && !modal_open {
        return;
    }
    let mut list: Vec<(u16, Action)> = buttons.iter().filter(|(o, _, _)| o.0 < 900).map(|(o, a, _)| (o.0, a.0)).collect();
    if list.is_empty() {
        return;
    }
    list.sort_by_key(|x| x.0);
    let moved = nav.x.abs() > 0.5 || nav.y.abs() > 0.5;
    if moved {
        let step: i32 = if nav.x > 0.5 || nav.y < -0.5 { 1 } else { -1 };
        let n = list.len() as i32;
        ctx.focus.idx = ((ctx.focus.idx as i32 + step).rem_euclid(n)) as usize;
        ctx.focus.dirty = true;
        sfx.play_vol("click", 0.3);
    }
    if ctx.focus.dirty || moved {
        ctx.focus.dirty = false;
        let cur = list.get(ctx.focus.idx).map(|x| x.0);
        for (o, _, mut bc) in &mut buttons {
            bc.0 = if Some(o.0) == cur && (!playing || modal_open) { C_FOCUS } else { C_LINE };
        }
    }
    if act && !playing {
        // in menus, Enter/A activates the focused button (mouse users just click)
        if let Some((_, a)) = list.get(ctx.focus.idx) {
            dispatch(*a, &mut ctx, &mut sfx);
        }
    } else if act && modal_open {
        if let Some((_, a)) = list.get(ctx.focus.idx) {
            dispatch(*a, &mut ctx, &mut sfx);
        }
    }
}
