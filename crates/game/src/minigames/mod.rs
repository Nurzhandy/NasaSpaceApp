//! A3 — mini-game windows. The rules are in `ja_core::minigame`; here each of the six types is
//! drawn with big buttons (mouse, touch, keyboard focus, gamepad). Part B can reuse `build` on the Moon.

use crate::art::Art;
use crate::kit::{self, Act, Action, Inputs, UiAction, C_GOOD, C_MUTED, C_TEXT, C_WARN};
use crate::ship::{Camp, ShipUi};
use bevy::prelude::*;
use ja_core::minigame::{Mini, MiniInput, MiniState};
use ja_core::{CampaignCommand, Lang};

/// Colours for wires, bins and samples.
pub fn swatch(i: u8) -> Color {
    [Color::srgb(0.95, 0.3, 0.3), Color::srgb(0.3, 0.6, 1.0), Color::srgb(1.0, 0.85, 0.25), Color::srgb(0.35, 0.85, 0.45), Color::srgb(0.8, 0.45, 1.0)][i as usize % 5]
}

/// A plain coloured button that sends a mini-game input.
fn chip(p: &mut ChildSpawnerCommands, art: &Art, uid: u32, input: MiniInput, order: u16, color: Color, label: &str, w: f32) {
    p.spawn((
        Button,
        UiAction(Action::Mini(uid, input)),
        kit::Focusable(order),
        Node { width: Val::Px(w), height: Val::Px(54.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(3.0)), ..default() },
        BackgroundColor(color),
        kit::Base(color),
        BorderColor(kit::C_LINE),
        BorderRadius::all(Val::Px(8.0)),
    ))
    .with_children(|b| {
        if !label.is_empty() {
            kit::text(b, label, &art.head, 22.0, Color::srgb(0.05, 0.05, 0.1));
        }
    });
}

/// Hold to upload (the transmitter): sends `Hold(dt)` while pressed (mouse, touch, E or Space).
#[derive(Component)]
pub struct HoldButton(pub u32);

pub fn hold_buttons(time: Res<Time>, inputs: Res<Inputs>, keys: Res<ButtonInput<KeyCode>>, q: Query<(&Interaction, &HoldButton)>, mut camp: ResMut<Camp>, ui: Res<ShipUi>) {
    let Some((player, uid)) = ui.open else { return };
    for (i, h) in &q {
        let held = *i == Interaction::Pressed || inputs.0.get(player as usize).is_some_and(|p| p.act_held) || keys.pressed(KeyCode::Space);
        if held && h.0 == uid {
            let _ = camp.0.apply(player, CampaignCommand::Mini { uid, input: MiniInput::Hold(time.delta_secs()) });
        }
    }
}

fn bar(p: &mut ChildSpawnerCommands, frac: f32, color: Color, w: f32) {
    p.spawn((Node { width: Val::Px(w), height: Val::Px(14.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(7.0)))).with_children(|b| {
        b.spawn((Node { width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(color), BorderRadius::all(Val::Px(7.0))));
    });
}

/// A dial: [-] value [+] with a green zone.
fn dial(p: &mut ChildSpawnerCommands, art: &Art, uid: u32, value: i32, lo: i32, hi: i32, max: i32, label: &str) {
    p.spawn(kit::row(12.0)).with_children(|r| {
        kit::text(r, label, &art.body, 18.0, C_TEXT);
        chip(r, art, uid, MiniInput::Minus, 20, kit::C_PANEL, "-", 60.0);
        r.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(3.0), ..default() }).with_children(|s| {
            for i in 0..=max {
                let green = i >= lo && i <= hi;
                let c = if i == value { Color::WHITE } else if green { C_GOOD } else { Color::srgba(1.0, 1.0, 1.0, 0.15) };
                s.spawn((Node { width: Val::Px(16.0), height: Val::Px(if i == value { 34.0 } else { 22.0 }), ..default() }, BackgroundColor(c), BorderRadius::all(Val::Px(3.0))));
            }
        });
        chip(r, art, uid, MiniInput::Plus, 21, kit::C_PANEL, "+", 60.0);
    });
}

pub fn build(p: &mut ChildSpawnerCommands, art: &Art, uid: u32, m: &Mini, l: Lang) {
    let hint = |p: &mut ChildSpawnerCommands, ru: &str, en: &str| {
        kit::text(p, l.pick(ru, en), &art.head, 18.0, C_WARN);
    };
    match &m.state {
        MiniState::Wires { left, right, links, picked } => {
            hint(p, "Нажми провод слева, потом гнездо того же цвета справа", "Press a wire on the left, then the socket of the same colour on the right");
            p.spawn(kit::row(80.0)).with_children(|r| {
                r.spawn(kit::col(8.0)).with_children(|c| {
                    for (i, col) in left.iter().enumerate() {
                        let done = links[i].is_some();
                        let label = if done { "OK" } else if *picked == Some(i) { "*" } else { "" };
                        chip(c, art, uid, MiniInput::Pick(i), 1 + i as u16, if done { swatch(*col).mix(&Color::BLACK, 0.5) } else { swatch(*col) }, label, 120.0);
                    }
                });
                r.spawn(kit::col(8.0)).with_children(|c| {
                    for (j, col) in right.iter().enumerate() {
                        let used = links.contains(&Some(j));
                        chip(c, art, uid, MiniInput::Link(j), 10 + j as u16, if used { swatch(*col).mix(&Color::BLACK, 0.5) } else { swatch(*col) }, if used { "OK" } else { "" }, 120.0);
                    }
                });
            });
        }
        MiniState::Sorting { items, bins, next } => {
            hint(p, "Положи предмет в корзину его цвета", "Drop the item into the bin of its colour");
            p.spawn(kit::row(14.0)).with_children(|r| {
                kit::text(r, format!("{} / {}", next, items.len()), &art.head, 20.0, C_MUTED);
                if let Some(it) = items.get(*next) {
                    r.spawn((Node { width: Val::Px(70.0), height: Val::Px(70.0), ..default() }, BackgroundColor(swatch(*it)), BorderRadius::MAX));
                }
            });
            p.spawn(kit::row(12.0)).with_children(|r| {
                for b in 0..*bins {
                    chip(r, art, uid, MiniInput::Bin(b), 1 + b as u16, swatch(b), "v", 110.0);
                }
            });
        }
        MiniState::Transmitter { upload, freq, target } => {
            if *upload < 1.0 {
                hint(p, "Держи кнопку (или E / пробел), пока полоса не заполнится", "Hold the button (or E / Space) until the bar is full");
                bar(p, *upload, kit::C_ACCENT, 600.0);
                p.spawn((
                    Button,
                    HoldButton(uid),
                    kit::Focusable(1),
                    UiAction(Action::Mini(uid, MiniInput::Hold(0.0))),
                    Node { width: Val::Px(260.0), height: Val::Px(64.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(3.0)), ..default() },
                    BackgroundColor(kit::C_PRIMARY),
                    kit::Base(kit::C_PRIMARY),
                    BorderColor(kit::C_LINE),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|b| {
                    kit::text(b, l.pick("Держать", "Hold"), &art.head, 24.0, Color::srgb(0.18, 0.08, 0.0));
                });
            } else {
                hint(p, "Поверни ручку: стрелка на зелёное деление", "Turn the dial: the needle onto the green mark");
                dial(p, art, uid, *freq, *target, *target, 20, l.pick("Частота", "Frequency"));
            }
        }
        MiniState::Coordinates { target, typed } => {
            hint(p, "Набери цифры по порядку (можно с клавиатуры)", "Type the digits in order (the keyboard works too)");
            let show: String = target.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" ");
            kit::text(p, show, &art.head, 34.0, C_TEXT);
            let typed_s: String = (0..target.len()).map(|i| typed.get(i).map(|d| d.to_string()).unwrap_or_else(|| "_".into())).collect::<Vec<_>>().join(" ");
            kit::text(p, typed_s, &art.head, 34.0, C_GOOD);
            p.spawn(kit::row(6.0)).insert(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(6.0), row_gap: Val::Px(6.0), max_width: Val::Px(700.0), ..default() }).with_children(|r| {
                for d in 0..10u8 {
                    chip(r, art, uid, MiniInput::Digit(d), 1 + d as u16, kit::C_PANEL.mix(&Color::WHITE, 0.6), &d.to_string(), 60.0);
                }
                chip(r, art, uid, MiniInput::Erase, 12, kit::C_PANEL, "<", 70.0);
            });
        }
        MiniState::FilterPressure { dirty, pressure, lo, hi } => {
            if dirty.iter().any(|d| *d) {
                hint(p, "Нажми на каждое грязное пятно фильтра", "Press every dirty spot on the filter");
                p.spawn(kit::row(10.0)).with_children(|r| {
                    for (i, d) in dirty.iter().enumerate() {
                        let c = if *d { Color::srgb(0.45, 0.35, 0.2) } else { Color::srgb(0.75, 0.9, 1.0) };
                        chip(r, art, uid, MiniInput::Pick(i), 1 + i as u16, c, if *d { "" } else { "OK" }, 80.0);
                    }
                });
            } else {
                hint(p, "Фильтр чистый. Выставь давление в зелёную зону", "The filter is clean. Set the pressure into the green zone");
                dial(p, art, uid, *pressure, *lo, *hi, 10, l.pick("Давление", "Pressure"));
            }
        }
        MiniState::PatchShield { holes, shield, target } => {
            if holes.iter().any(|h| *h) {
                hint(p, "Заклей каждую дырку (там, где пузырьки)", "Patch every hole (where the bubbles are)");
                p.spawn(kit::row(10.0)).with_children(|r| {
                    for (i, h) in holes.iter().enumerate() {
                        let c = if *h { Color::srgb(0.9, 0.3, 0.3) } else { Color::srgb(0.6, 0.65, 0.75) };
                        chip(r, art, uid, MiniInput::Pick(i), 1 + i as u16, c, if *h { "o" } else { "OK" }, 80.0);
                    }
                });
            } else {
                hint(p, "Дыры заделаны. Выставь щит на нужный уровень", "Holes patched. Set the shield to the right level");
                dial(p, art, uid, *shield, *target, *target, 6, l.pick("Щит", "Shield"));
            }
        }
    }
    p.spawn(kit::row(10.0)).with_children(|r| {
        bar(r, m.progress(), C_GOOD, 400.0);
        if m.mistakes > 0 {
            kit::text(r, format!("{} {}", l.pick("ошибок:", "mistakes:"), m.mistakes), &art.body, 15.0, C_MUTED);
        }
    });
}

/// Digits typed on the keyboard go into the coordinates mini-game.
pub fn digit_keys(mut keys: EventReader<bevy::input::keyboard::KeyboardInput>, ui: Res<ShipUi>, mut acts: EventWriter<Act>) {
    let Some((_, uid)) = ui.open else {
        keys.clear();
        return;
    };
    for k in keys.read() {
        if !k.state.is_pressed() {
            continue;
        }
        if let bevy::input::keyboard::Key::Character(c) = &k.logical_key {
            if let Some(d) = c.chars().next().and_then(|ch| ch.to_digit(10)) {
                acts.write(Act(Action::Mini(uid, MiniInput::Digit(d as u8))));
            }
        }
    }
}
