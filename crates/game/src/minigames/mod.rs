//! Mini-game window: a canvas that draws `Mini::scene` (rules and pictures come from
//! `ja_core::minigame`) and turns the mouse / touchpad / Space key into `MiniInput`s.

use crate::art::Art;
use crate::kit::{Act, Action};
use crate::ship::{Camp, ShipUi};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use ja_core::minigame::{MiniInput, Shape, CH, CW};

/// The drawing area of the open task's mini-game.
#[derive(Component)]
pub struct Canvas(pub u32);

pub fn build(p: &mut ChildSpawnerCommands, uid: u32) {
    p.spawn((
        Node { width: Val::Px(CW), height: Val::Px(CH), position_type: PositionType::Relative, overflow: Overflow::clip(), ..default() },
        BackgroundColor(Color::srgb(0.08, 0.1, 0.2)),
        BorderRadius::all(Val::Px(8.0)),
        RelativeCursorPosition::default(),
        Canvas(uid),
    ));
}

fn rgb(c: u32, a: f32) -> Color {
    Color::srgba_u8((c >> 16) as u8, (c >> 8) as u8, c as u8, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

fn abs(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node { position_type: PositionType::Absolute, left: Val::Px(x), top: Val::Px(y), width: Val::Px(w), height: Val::Px(h), ..default() }
}

/// Redraw the canvas whenever the picture changes.
pub fn draw(mut commands: Commands, camp: Res<Camp>, ui: Res<ShipUi>, art: Res<Art>, prof: Res<crate::Prof>, canvas: Query<(Entity, &Canvas)>, mut last: Local<(Option<Entity>, Vec<Shape>)>) {
    let Ok((e, c)) = canvas.single() else {
        last.0 = None;
        return;
    };
    let Some(m) = ui.open.and_then(|(_, uid)| camp.0.state().tasks.iter().find(|t| t.uid == uid)).and_then(|t| t.mini.as_ref()).filter(|_| ui.open.is_some_and(|o| o.1 == c.0)) else { return };
    let scene = m.scene(prof.lang());
    if last.0 == Some(e) && last.1 == scene {
        return;
    }
    commands.entity(e).despawn_related::<Children>();
    commands.entity(e).with_children(|p| {
        for s in &scene {
            match s {
                Shape::Rect { x, y, w, h, c, a, r } => {
                    p.spawn((abs(*x, *y, *w, *h), BackgroundColor(rgb(*c, *a)), BorderRadius::all(Val::Px(*r))));
                }
                Shape::Circle { x, y, r, c, a } => {
                    p.spawn((abs(x - r, y - r, r * 2.0, r * 2.0), BackgroundColor(rgb(*c, *a)), BorderRadius::MAX));
                }
                Shape::Ring { x, y, r, c, w } => {
                    let mut n = abs(x - r, y - r, r * 2.0, r * 2.0);
                    n.border = UiRect::all(Val::Px(*w));
                    p.spawn((n, BorderColor(rgb(*c, 1.0)), BorderRadius::MAX));
                }
                Shape::Icon { n, x, y, s } => {
                    if *s > 0.0 {
                        p.spawn((abs(x - s / 2.0, y - s / 2.0, *s, *s), ImageNode::new(art.icon(n))));
                    }
                }
                Shape::Text { x, y, t, s, c, center } => {
                    let mut n = if *center { abs(x - 360.0, *y, 720.0, s * 1.4) } else { abs(*x, *y, CW - x, s * 1.4) };
                    if *center {
                        n.justify_content = JustifyContent::Center;
                    } else {
                        // a column parent gives the text its width, so long lines wrap
                        n.flex_direction = FlexDirection::Column;
                    }
                    p.spawn((n)).with_children(|t2| {
                        t2.spawn((Text::new(t.clone()), TextFont { font: art.body.clone(), font_size: *s, ..default() }, TextColor(rgb(*c, 1.0))));
                    });
                }
                Shape::Line { x0, y0, x1, y1, w, c } => {
                    let len = (x1 - x0).hypot(y1 - y0);
                    let step = (w * 0.9).max(3.0);
                    let n = (len / step).ceil().max(1.0) as usize;
                    for k in 0..=n {
                        let f = k as f32 / n as f32;
                        let (x, y) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                        p.spawn((abs(x - w / 2.0, y - w / 2.0, *w, *w), BackgroundColor(rgb(*c, 1.0)), BorderRadius::MAX));
                    }
                }
            }
        }
    });
    *last = (Some(e), scene);
}

/// The progress bar under the canvas (updated in place: rebuilding the window mid-drag would lose input).
#[derive(Component)]
pub struct MiniProgress;

pub fn progress_bar(camp: Res<Camp>, ui: Res<ShipUi>, mut q: Query<&mut Node, With<MiniProgress>>) {
    let Some(m) = ui.open.and_then(|(_, uid)| camp.0.state().tasks.iter().find(|t| t.uid == uid)).and_then(|t| t.mini.as_ref()) else { return };
    for mut n in &mut q {
        n.width = Val::Percent(m.progress() * 100.0);
    }
}

/// Mouse / touchpad on the canvas → Press, Drag, Release (canvas pixels); Space → the big button.
pub fn pointer(mouse: Res<ButtonInput<MouseButton>>, keys: Res<ButtonInput<KeyCode>>, canvas: Query<(&Canvas, &RelativeCursorPosition)>, ui: Res<ShipUi>, mut acts: EventWriter<Act>, mut state: Local<(bool, Vec2)>) {
    let Ok((c, rel)) = canvas.single() else {
        state.0 = false;
        return;
    };
    if ui.open.map(|o| o.1) != Some(c.0) {
        return;
    }
    let uid = c.0;
    if keys.just_pressed(KeyCode::Space) {
        acts.write(Act(Action::Mini(uid, MiniInput::Space(true))));
    }
    if keys.just_released(KeyCode::Space) {
        acts.write(Act(Action::Mini(uid, MiniInput::Space(false))));
    }
    let Some(n) = rel.normalized else {
        // a just-built canvas doesn't know the cursor yet: never lose a release
        if state.0 && mouse.just_released(MouseButton::Left) {
            state.0 = false;
            acts.write(Act(Action::Mini(uid, MiniInput::Release(state.1.x, state.1.y))));
        }
        return;
    };
    let at = Vec2::new(n.x * CW, n.y * CH);
    let inside = (0.0..=1.0).contains(&n.x) && (0.0..=1.0).contains(&n.y);
    if mouse.just_pressed(MouseButton::Left) && inside {
        state.0 = true;
        state.1 = at;
        acts.write(Act(Action::Mini(uid, MiniInput::Press(at.x, at.y))));
        return;
    }
    if state.0 && mouse.pressed(MouseButton::Left) && at.distance(state.1) > 1.5 {
        let p = at.clamp(Vec2::ZERO, Vec2::new(CW, CH));
        state.1 = p;
        acts.write(Act(Action::Mini(uid, MiniInput::Drag(p.x, p.y))));
    }
    if state.0 && mouse.just_released(MouseButton::Left) {
        state.0 = false;
        let p = at.clamp(Vec2::ZERO, Vec2::new(CW, CH));
        acts.write(Act(Action::Mini(uid, MiniInput::Release(p.x, p.y))));
    }
}
