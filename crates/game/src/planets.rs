//! A10 — the Solar System map (planets from `assets/content/planets/*.json`). Only the Moon can be
//! flown to now; the others are for reading.

use crate::art::Art;
use crate::kit::{self, Action, UiAction, C_GOOD, C_MUTED, C_TEXT};
use bevy::prelude::*;
use ja_core::content::{Content, Planet};
use ja_core::{Lang, Level};

/// Order from the Sun (the Moon sits next to Earth).
const ORDER: &[&str] = &["mercury", "venus", "earth", "moon", "mars", "jupiter", "saturn", "uranus", "neptune"];

pub fn ordered(c: &Content) -> Vec<&Planet> {
    let mut v: Vec<&Planet> = c.planets.iter().collect();
    v.sort_by_key(|p| ORDER.iter().position(|x| *x == p.id).unwrap_or(99));
    v
}

pub fn map(p: &mut ChildSpawnerCommands, art: &Art, c: &Content, pick: usize, level: Level, l: Lang) {
    let list = ordered(c);
    p.spawn(kit::row(10.0)).insert(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), align_items: AlignItems::FlexEnd, ..default() }).with_children(|r| {
        r.spawn(kit::col(2.0)).insert(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }).with_children(|x| {
            kit::icon(x, art, "sun", 70.0);
            kit::text(x, l.pick("Солнце", "Sun"), &art.body, 14.0, C_MUTED);
        });
        for (i, pl) in list.iter().enumerate() {
            let on = i == pick;
            let bg = if on { kit::C_PRIMARY } else { kit::C_PANEL };
            r.spawn((
                Button,
                UiAction(Action::EncPick(i as u8)),
                kit::Focusable(10 + i as u16),
                Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(6.0)), border: UiRect::all(Val::Px(3.0)), row_gap: Val::Px(2.0), ..default() },
                BackgroundColor(bg),
                kit::Base(bg),
                BorderColor(kit::C_LINE),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|b| {
                kit::icon(b, art, &pl.icon, if pl.id == "jupiter" || pl.id == "saturn" { 56.0 } else { 44.0 });
                kit::text(b, pl.name.get(l), &art.body, 14.0, C_TEXT);
                if pl.playable {
                    kit::text(b, l.pick("можно лететь", "can fly"), &art.body, 11.0, if on { Color::srgb(0.1, 0.3, 0.05) } else { C_GOOD });
                }
            });
        }
    });
    let Some(pl) = list.get(pick) else { return };
    p.spawn(kit::panel()).insert(Node { width: Val::Px(900.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|b| {
        b.spawn(kit::row(10.0)).with_children(|r| {
            kit::icon(r, art, &pl.icon, 48.0);
            kit::text(r, pl.name.get(l), &art.head, 28.0, kit::C_ACCENT);
        });
        for (icon, ru, en, t) in [("walk", "Тяжесть:", "Gravity:", &pl.gravity), ("sun", "Температура:", "Temperature:", &pl.temperature), ("wind", "Атмосфера:", "Atmosphere:", &pl.atmosphere)] {
            b.spawn(kit::row(8.0)).with_children(|r| {
                kit::icon(r, art, icon, 24.0);
                kit::text(r, l.pick(ru, en), &art.head, 16.0, C_MUTED);
                kit::wrap_text(r, t.get(l), &art.body, 16.0, C_TEXT, 700.0);
            });
        }
        kit::wrap_text(b, pl.surface.get(level, l), &art.body, 18.0, C_TEXT, 860.0);
        kit::wrap_text(b, pl.history.get(level, l), &art.body, 16.0, C_MUTED, 860.0);
    });
}
