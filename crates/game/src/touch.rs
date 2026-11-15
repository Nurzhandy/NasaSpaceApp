//! B11 — touch controls: a virtual joystick on the left, an action button (E), build (B) and
//! pause on the right. They show up in play and landing after the first touch ("auto"),
//! always, or never (settings). The joystick also works with a mouse drag, for demos on a PC.
//! Everything ends up in player 1's `kit::Pad`, so the rest of the game needs no changes.

use crate::art::Art;
use crate::{AppState, Prof, Set};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// This frame's touch input (merged into player 1 by `kit::read_inputs`).
#[derive(Resource, Default)]
pub struct TouchPad {
    pub mv: Vec2,
    pub act: bool,
    pub act_held: bool,
    pub build: bool,
    pub pause: bool,
    /// a finger touched the screen at least once (for "auto")
    seen: bool,
    shown: bool,
    stick: Option<u64>,
    held: [bool; 3],
}

#[derive(Component)]
struct TouchUi;
#[derive(Component)]
struct StickBase;
#[derive(Component)]
struct Knob;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Btn {
    Act,
    Build,
    Pause,
}

const STICK_R: f32 = 80.0;
const KNOB_R: f32 = 34.0;

pub struct TouchPlugin;
impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchPad>().add_systems(Update, (spawn, read.in_set(Set::Input).before(crate::kit::read_inputs), view.in_set(Set::View), make_room.in_set(Set::View)));
    }
}

/// Where everything sits for a window of `size` (logical pixels, top-left origin).
fn layout(size: Vec2) -> (Vec2, [(Btn, Vec2, f32); 3]) {
    let stick = Vec2::new(40.0 + STICK_R, size.y - 50.0 - STICK_R);
    let act = Vec2::new(size.x - 110.0, size.y - 120.0);
    (stick, [(Btn::Act, act, 58.0), (Btn::Build, act + Vec2::new(-120.0, 40.0), 38.0), (Btn::Pause, act + Vec2::new(-90.0, -95.0), 30.0)])
}

fn spawn(mut commands: Commands, art: Option<Res<Art>>, q: Query<(), With<TouchUi>>) {
    let Some(art) = art else { return };
    if !q.is_empty() {
        return;
    }
    let ring = |r: f32| (Node { position_type: PositionType::Absolute, width: Val::Px(r * 2.0), height: Val::Px(r * 2.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(3.0)), ..default() }, BorderRadius::all(Val::Px(r)), Visibility::Hidden, GlobalZIndex(40), TouchUi);
    commands.spawn((ring(STICK_R), Button, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.10)), BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)), StickBase));
    commands.spawn((ring(KNOB_R), BackgroundColor(Color::srgba(1.0, 0.85, 0.5, 0.55)), BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.7)), Pickable::IGNORE, Knob));
    for (b, icon, r) in [(Btn::Act, "ok_hand", 58.0), (Btn::Build, "hammer", 38.0), (Btn::Pause, "pause", 30.0)] {
        commands.spawn((ring(r), Button, BackgroundColor(Color::srgba(1.0, 0.62, 0.22, 0.45)), BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.6)), b)).with_children(|c| {
            c.spawn((ImageNode::new(art.icon(icon)), Node { width: Val::Px(r * 1.1), height: Val::Px(r * 1.1), ..default() }, Pickable::IGNORE));
        });
    }
}

fn stick_vec(p: Vec2, center: Vec2) -> Vec2 {
    let d = (p - center) / STICK_R;
    let v = Vec2::new(d.x, -d.y).clamp_length_max(1.0);
    if v.length() < 0.15 {
        Vec2::ZERO
    } else {
        v
    }
}

#[allow(clippy::too_many_arguments)]
fn read(touches: Res<Touches>, win: Query<&Window, With<PrimaryWindow>>, prof: Res<Prof>, state: Res<State<AppState>>, mut pad: ResMut<TouchPad>, btns: Query<(&Interaction, &Btn)>, base: Query<&Interaction, With<StickBase>>) {
    pad.act = false;
    pad.build = false;
    pad.pause = false;
    if touches.iter_just_pressed().next().is_some() {
        pad.seen = true;
    }
    let in_game = matches!(state.get(), AppState::Playing | AppState::Landing);
    pad.shown = in_game && match prof.0.touch {
        1 => true,
        2 => false,
        _ => pad.seen,
    };
    let Ok(w) = win.single() else { return };
    if !pad.shown {
        pad.mv = Vec2::ZERO;
        pad.act_held = false;
        pad.stick = None;
        pad.held = [false; 3];
        return;
    }
    let size = Vec2::new(w.width(), w.height());
    let (center, _) = layout(size);
    // a finger that lands on the lower-left part of the screen drives the joystick
    for t in touches.iter_just_pressed() {
        let p = t.position();
        if p.x < size.x * 0.45 && p.y > size.y * 0.35 {
            pad.stick = Some(t.id());
        }
    }
    let mut mv = None;
    if let Some(id) = pad.stick {
        match touches.get_pressed(id) {
            Some(t) => mv = Some(stick_vec(t.position(), center)),
            None => pad.stick = None,
        }
    }
    // mouse drag on the joystick (desktop demo)
    if mv.is_none() && base.iter().any(|i| *i == Interaction::Pressed) {
        mv = w.cursor_position().map(|c| stick_vec(c, center));
    }
    pad.mv = mv.unwrap_or(Vec2::ZERO);
    // buttons: UI interaction handles both fingers and the mouse
    let mut now = [false; 3];
    for (i, b) in &btns {
        if *i == Interaction::Pressed {
            now[*b as usize] = true;
        }
    }
    pad.act = now[0] && !pad.held[0];
    pad.build = now[1] && !pad.held[1] && *state.get() == AppState::Playing;
    pad.pause = now[2] && !pad.held[2];
    pad.act_held = now[0];
    pad.held = now;
}

#[allow(clippy::type_complexity)]
fn view(pad: Res<TouchPad>, state: Res<State<AppState>>, win: Query<&Window, With<PrimaryWindow>>, mut base: Query<(&mut Node, &mut Visibility), (With<StickBase>, Without<Knob>, Without<Btn>)>, mut knob: Query<(&mut Node, &mut Visibility), (With<Knob>, Without<StickBase>, Without<Btn>)>, mut btns: Query<(&Btn, &mut Node, &mut Visibility), (Without<StickBase>, Without<Knob>)>) {
    let Ok(w) = win.single() else { return };
    let (center, buttons) = layout(Vec2::new(w.width(), w.height()));
    let vis = |on: bool| if on { Visibility::Visible } else { Visibility::Hidden };
    let place = |n: &mut Node, c: Vec2, r: f32| {
        n.left = Val::Px(c.x - r);
        n.top = Val::Px(c.y - r);
    };
    for (mut n, mut v) in &mut base {
        place(&mut n, center, STICK_R);
        v.set_if_neq(vis(pad.shown));
    }
    for (mut n, mut v) in &mut knob {
        place(&mut n, center + Vec2::new(pad.mv.x, -pad.mv.y) * (STICK_R - KNOB_R * 0.5), KNOB_R);
        v.set_if_neq(vis(pad.shown));
    }
    for (b, mut n, mut v) in &mut btns {
        if let Some((_, c, r)) = buttons.iter().find(|(k, _, _)| k == b) {
            place(&mut n, *c, *r);
        }
        let on = pad.shown && (*b != Btn::Build || *state.get() == AppState::Playing);
        v.set_if_neq(vis(on));
    }
}

/// While the touch controls are on, the HUD steps aside: the message box moves above the
/// joystick, the survival panel moves up out of the buttons' way, the key hints hide.
#[allow(clippy::type_complexity)]
fn make_room(pad: Res<TouchPad>, mut radio: Query<&mut Node, (With<crate::hud::RadioBox>, Without<crate::survival_ui::Panel>)>, mut panel: Query<&mut Node, (With<crate::survival_ui::Panel>, Without<crate::hud::RadioBox>)>, mut hints: Query<&mut Visibility, With<crate::hud::KeyHints>>) {
    let on = pad.shown;
    for mut n in &mut radio {
        let want = Val::Px(if on { 2.0 * STICK_R + 64.0 } else { 12.0 });
        if n.bottom != want {
            n.bottom = want;
        }
    }
    for mut n in &mut panel {
        let (top, bottom) = if on { (Val::Px(120.0), Val::Auto) } else { (Val::Auto, Val::Px(60.0)) };
        if n.top != top || n.bottom != bottom {
            n.top = top;
            n.bottom = bottom;
        }
    }
    for mut v in &mut hints {
        v.set_if_neq(if on { Visibility::Hidden } else { Visibility::Inherited });
    }
}
