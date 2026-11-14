//! Shared UI kit: colors, widgets (icon-first buttons), actions, keyboard/gamepad input,
//! and focus navigation so every screen works with mouse, keyboard and gamepad.

use crate::art::Art;
use crate::{AppState, Set};
use ja_core::data::SkinSlot;
use ja_core::{MapId, MiniInput, ModuleId};
use bevy::prelude::*;

pub const C_BG: Color = Color::srgba(0.07, 0.08, 0.2, 0.94);
pub const C_PANEL: Color = Color::srgba(0.15, 0.18, 0.4, 0.96);
pub const C_LINE: Color = Color::srgba(0.63, 0.7, 1.0, 0.3);
pub const C_TEXT: Color = Color::srgb(0.95, 0.96, 1.0);
pub const C_MUTED: Color = Color::srgb(0.68, 0.72, 0.88);
pub const C_ACCENT: Color = Color::srgb(1.0, 0.72, 0.3);
pub const C_GOOD: Color = Color::srgb(0.3, 0.86, 0.6);
pub const C_BAD: Color = Color::srgb(1.0, 0.38, 0.45);
pub const C_WARN: Color = Color::srgb(1.0, 0.8, 0.3);
pub const C_BLUE: Color = Color::srgb(0.45, 0.6, 1.0);
pub const C_FOCUS: Color = Color::srgb(1.0, 0.9, 0.35);
pub const C_PRIMARY: Color = Color::srgb(1.0, 0.62, 0.22);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    Go(AppState),
    Play(bool),
    /// continue the saved mission
    Continue,
    PickMap(MapId),
    Launch,
    SetPlan(u8),
    ToggleLive,
    ToggleLang,
    ToggleSound,
    ToggleMusic,
    ShopTab(u8),
    Skin(SkinSlot, usize),
    Power(ModuleId, i8),
    Close,
    Harvest(bool),
    Cargo(bool),
    Feel(usize),
    Support(usize),
    Pause,
    Beacon,
    Breathe,
    ExitMission,
    Again,
    TutorialNext,
    Reflect(u8),
    BuildPick(usize),
    Quit,
    /// ship expedition (part A, ship/); the flight starts with `StartExpedition` (new expedition screen)
    Mini(u32, MiniInput),
    /// encyclopedia (encyclopedia.rs): tab, item
    EncTab(u8),
    EncPick(u8),
    CloseTask,
    DialogNext,
    DialogChoose(usize),
    DialogSkip,
    CloseCard,
    CloseBreather,
    RestartDay,
    NextDay,
    ToMoon,
    ShipResume,
    ShipQuit,
    /// network game (net.rs)
    NetHost,
    NetJoin(u8),
    NetJoinTyped,
    NetStop,
    /// new expedition screen
    SetCreative(bool),
    SetJunior(bool),
    SetPlayers(bool),
    StartExpedition,
    /// landing (landing.rs)
    PickSite(u8),
    LandingRetry,
    LandingGo,
    LandingAck,
    /// survival upgrades window
    Upgrades,
    BuyUpgrade(u8),
}

#[derive(Event, Clone, Copy, Debug)]
pub struct Act(pub Action);

#[derive(Component)]
pub struct UiAction(pub Action);
#[derive(Component)]
pub struct Focusable(pub u16);
#[derive(Resource, Default)]
pub struct Focus {
    pub idx: usize,
}

/// Per-player input snapshot for this frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Pad {
    pub mv: Vec2,
    pub act: bool,
    pub act_held: bool,
    pub build: bool,
    pub demolish: bool,
    pub power: bool,
    pub pause: bool,
    pub back: bool,
    pub prev: bool,
    pub next: bool,
    pub nav: Vec2,
}
#[derive(Resource, Default)]
pub struct Inputs(pub [Pad; 2]);

/// True while an in-game window needs the focus (set by `windows`).
#[derive(Resource, Default)]
pub struct UiBusy(pub bool);

pub struct KitPlugin;
impl Plugin for KitPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Act>()
            .init_resource::<Focus>()
            .init_resource::<Inputs>()
            .init_resource::<UiBusy>()
            .add_systems(Update, (read_inputs, buttons, navigate).chain().in_set(Set::Input));
    }
}

fn read_inputs(keys: Res<ButtonInput<KeyCode>>, pads: Query<(Entity, &Gamepad)>, mut inputs: ResMut<Inputs>, setup: Res<crate::Setup>, state: Res<State<AppState>>, net: Res<crate::net::Net>) {
    let ax = |n: bool, p: bool| (p as i32 - n as i32) as f32;
    let k = |c: KeyCode| keys.pressed(c);
    let j = |c: KeyCode| keys.just_pressed(c);
    let playing = matches!(*state.get(), AppState::Playing | AppState::Ship);
    // network co-op: one astronaut per computer, so the whole keyboard is this player's
    let coop = setup.coop && playing && !net.active();
    let mut p1 = Pad {
        mv: Vec2::new(ax(k(KeyCode::KeyA), k(KeyCode::KeyD)), ax(k(KeyCode::KeyS), k(KeyCode::KeyW))),
        act: j(KeyCode::KeyF) || j(KeyCode::KeyE) || (!coop && j(KeyCode::Enter)),
        act_held: k(KeyCode::KeyF) || k(KeyCode::KeyE) || (!coop && k(KeyCode::Enter)),
        build: j(KeyCode::KeyB),
        demolish: j(KeyCode::KeyX),
        power: j(KeyCode::Tab),
        pause: j(KeyCode::Escape) || j(KeyCode::KeyP),
        back: j(KeyCode::Escape) || j(KeyCode::Backspace),
        prev: j(KeyCode::KeyQ),
        next: j(KeyCode::KeyR),
        nav: Vec2::new(ax(j(KeyCode::KeyA), j(KeyCode::KeyD)), ax(j(KeyCode::KeyS), j(KeyCode::KeyW))),
    };
    let arrows = Vec2::new(ax(k(KeyCode::ArrowLeft), k(KeyCode::ArrowRight)), ax(k(KeyCode::ArrowDown), k(KeyCode::ArrowUp)));
    let arrows_j = Vec2::new(ax(j(KeyCode::ArrowLeft), j(KeyCode::ArrowRight)), ax(j(KeyCode::ArrowDown), j(KeyCode::ArrowUp)));
    let mut p2 = Pad::default();
    if coop {
        p2.mv = arrows;
        p2.nav = arrows_j;
        p2.act = j(KeyCode::ShiftRight) || j(KeyCode::Enter) || j(KeyCode::NumpadEnter);
        p2.act_held = k(KeyCode::ShiftRight) || k(KeyCode::Enter) || k(KeyCode::NumpadEnter);
        p2.build = j(KeyCode::ControlRight) || j(KeyCode::Numpad0);
        p2.demolish = j(KeyCode::Delete) || j(KeyCode::NumpadDecimal);
    } else {
        p1.mv += arrows;
        p1.nav += arrows_j;
    }
    let mut list: Vec<(Entity, &Gamepad)> = pads.iter().collect();
    list.sort_by_key(|(e, _)| *e);
    for (i, (_, g)) in list.iter().enumerate().take(2) {
        let p = if i == 0 { &mut p1 } else { &mut p2 };
        let ls = g.left_stick();
        if ls.length() > 0.25 {
            p.mv += ls;
        }
        p.act |= g.just_pressed(GamepadButton::South);
        p.act_held |= g.pressed(GamepadButton::South);
        p.build |= g.just_pressed(GamepadButton::North);
        p.demolish |= g.just_pressed(GamepadButton::West);
        p.power |= g.just_pressed(GamepadButton::Select);
        p.pause |= g.just_pressed(GamepadButton::Start);
        p.back |= g.just_pressed(GamepadButton::East);
        p.prev |= g.just_pressed(GamepadButton::LeftTrigger);
        p.next |= g.just_pressed(GamepadButton::RightTrigger);
        p.nav += Vec2::new(ax(g.just_pressed(GamepadButton::DPadLeft), g.just_pressed(GamepadButton::DPadRight)), ax(g.just_pressed(GamepadButton::DPadDown), g.just_pressed(GamepadButton::DPadUp)));
    }
    p1.mv = p1.mv.clamp_length_max(1.0);
    p2.mv = p2.mv.clamp_length_max(1.0);
    inputs.0 = match net.remote_idx() {
        // the guest plays astronaut 1, the host astronaut 0; the other one is moved by the network
        Some(0) if playing => [Pad::default(), p1],
        Some(1) if playing => [p1, Pad::default()],
        _ => [p1, p2],
    };
}

fn buttons(mut q: Query<(&Interaction, &UiAction, &mut BackgroundColor, Option<&Base>), Changed<Interaction>>, mut out: EventWriter<Act>) {
    for (i, a, mut bg, base) in &mut q {
        let b = base.map(|b| b.0).unwrap_or(bg.0);
        match i {
            Interaction::Pressed => {
                out.write(Act(a.0));
            }
            Interaction::Hovered => bg.0 = b.mix(&Color::WHITE, 0.12),
            Interaction::None => bg.0 = b,
        }
    }
}

/// Remembers the resting background so hover can be undone.
#[derive(Component)]
pub struct Base(pub Color);

fn navigate(inputs: Res<Inputs>, keys: Res<ButtonInput<KeyCode>>, state: Res<State<AppState>>, busy: Res<UiBusy>, mut focus: ResMut<Focus>, mut q: Query<(&Focusable, &UiAction, &mut BorderColor)>, mut out: EventWriter<Act>) {
    let playing = matches!(*state.get(), AppState::Playing | AppState::Ship);
    if playing && !busy.0 {
        return;
    }
    let mut list: Vec<(u16, Action)> = q.iter().map(|(f, a, _)| (f.0, a.0)).collect();
    if list.is_empty() {
        return;
    }
    list.sort_by_key(|x| x.0);
    let nav = inputs.0[0].nav + inputs.0[1].nav;
    let n = list.len();
    if focus.idx >= n {
        focus.idx = 0;
    }
    if nav.x > 0.5 || nav.y < -0.5 {
        focus.idx = (focus.idx + 1) % n;
    } else if nav.x < -0.5 || nav.y > 0.5 {
        focus.idx = (focus.idx + n - 1) % n;
    }
    let cur = list[focus.idx].0;
    for (f, _, mut b) in &mut q {
        let want = if f.0 == cur { C_FOCUS } else { C_LINE };
        if b.0 != want {
            b.0 = want;
        }
    }
    let confirm = inputs.0[0].act || inputs.0[1].act || keys.just_pressed(KeyCode::Enter) || (!playing && keys.just_pressed(KeyCode::Space));
    if confirm {
        out.write(Act(list[focus.idx].1));
    }
}

// ------------------------------------------------------------------ widgets

pub fn text(p: &mut ChildSpawnerCommands, s: impl Into<String>, font: &Handle<Font>, size: f32, color: Color) -> Entity {
    p.spawn((Text::new(s), TextFont { font: font.clone(), font_size: size, ..default() }, TextColor(color))).id()
}

pub fn wrap_text(p: &mut ChildSpawnerCommands, s: impl Into<String>, font: &Handle<Font>, size: f32, color: Color, max_w: f32) -> Entity {
    p.spawn((Text::new(s), TextFont { font: font.clone(), font_size: size, ..default() }, TextColor(color), Node { max_width: Val::Px(max_w), ..default() })).id()
}

pub fn icon(p: &mut ChildSpawnerCommands, art: &Art, name: &str, size: f32) -> Entity {
    p.spawn((ImageNode::new(art.icon(name)), Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() })).id()
}

pub fn row(gap: f32) -> Node {
    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(gap), ..default() }
}
pub fn col(gap: f32) -> Node {
    Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(gap), ..default() }
}

pub fn panel() -> (Node, BackgroundColor, BorderColor, BorderRadius) {
    (
        Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(20.0)), row_gap: Val::Px(12.0), border: UiRect::all(Val::Px(3.0)), ..default() },
        BackgroundColor(C_BG),
        BorderColor(C_LINE),
        BorderRadius::all(Val::Px(8.0)),
    )
}

/// Icon + short label button.
pub fn button(p: &mut ChildSpawnerCommands, art: &Art, action: Action, order: u16, icon_name: &str, label: &str, primary: bool) -> Entity {
    let bg = if primary { C_PRIMARY } else { C_PANEL };
    let fg = if primary { Color::srgb(0.18, 0.08, 0.0) } else { C_TEXT };
    p.spawn((
        Button,
        UiAction(action),
        Focusable(order),
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)), border: UiRect::all(Val::Px(3.0)), min_height: Val::Px(56.0), ..default() },
        BackgroundColor(bg),
        Base(bg),
        BorderColor(C_LINE),
        BorderRadius::all(Val::Px(8.0)),
    ))
    .with_children(|b| {
        if !icon_name.is_empty() {
            icon(b, art, icon_name, 34.0);
        }
        if !label.is_empty() {
            text(b, label, &art.body, 22.0, fg);
        }
    })
    .id()
}

/// Big square tile: large icon over a label (menus).
pub fn tile(p: &mut ChildSpawnerCommands, art: &Art, action: Action, order: u16, icon_name: &str, label: &str, w: f32, bg: Color) -> Entity {
    p.spawn((
        Button,
        UiAction(action),
        Focusable(order),
        Node { width: Val::Px(w), height: Val::Px(w * 0.9), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(10.0), border: UiRect::all(Val::Px(4.0)), ..default() },
        BackgroundColor(bg),
        Base(bg),
        BorderColor(C_LINE),
        BorderRadius::all(Val::Px(10.0)),
    ))
    .with_children(|b| {
        icon(b, art, icon_name, w * 0.42);
        b.spawn((Text::new(label), TextFont { font: art.head.clone(), font_size: 22.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
    })
    .id()
}

/// Full-screen absolute container.
pub fn overlay(dim: bool, z: i32) -> (Node, BackgroundColor, GlobalZIndex) {
    (
        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
        BackgroundColor(if dim { Color::srgba(0.0, 0.0, 0.05, 0.62) } else { Color::NONE }),
        GlobalZIndex(z),
    )
}

/// Row of "pips" (power blocks, stars).
pub fn pips(p: &mut ChildSpawnerCommands, on: u32, total: u32, color: Color, size: f32) {
    p.spawn(row(3.0)).with_children(|r| {
        for i in 0..total {
            r.spawn((Node { width: Val::Px(size * 0.7), height: Val::Px(size), ..default() }, BackgroundColor(if i < on { color } else { Color::srgba(1.0, 1.0, 1.0, 0.15) }), BorderRadius::all(Val::Px(2.0))));
        }
    });
}
