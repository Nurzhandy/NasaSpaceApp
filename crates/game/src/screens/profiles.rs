//! Profiles: choose who is flying, add a new one, delete one (press twice).
//! Names are callsigns from a list: no real names, nothing leaves this computer.

use super::*;
use ja_core::progress::{Book, MAX_PROFILES};

pub(super) fn profiles(p: &mut ChildSpawnerCommands, art: &Art, book: &Book, l: Lang, setup: &Setup) {
    body(p, |c| {
        kit::text(c, tr(l, "prof.title"), &art.head, 32.0, C_TEXT);
        c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, justify_content: JustifyContent::Center, column_gap: Val::Px(14.0), row_gap: Val::Px(14.0), max_width: Val::Px(1100.0), ..default() }).with_children(|g| {
            for (i, pr) in book.profiles.iter().enumerate() {
                let active = i == book.active;
                g.spawn((Node { width: Val::Px(200.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(kit::C_BG), BorderColor(if active { C_GOOD } else { C_LINE }), BorderRadius::all(Val::Px(10.0))))
                    .with_children(|card| {
                        let bg = kit::C_PANEL;
                        card.spawn((Button, kit::UiAction(Action::SelectProfile(i as u8)), kit::Focusable(i as u16 * 2), Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(8.0)), border: UiRect::all(Val::Px(3.0)), width: Val::Percent(100.0), ..default() }, BackgroundColor(bg), kit::Base(bg), BorderColor(C_LINE), BorderRadius::all(Val::Px(8.0))))
                            .with_children(|b| {
                                avatar(b, art, pr, 4.0);
                                kit::text(b, pr.name(l), &art.head, 20.0, C_TEXT);
                                b.spawn(kit::row(6.0)).with_children(|r| {
                                    kit::icon(r, art, "medal", 20.0);
                                    kit::text(r, trf(l, "prof.level", &[("n", &pr.level().to_string())]), &art.body, 16.0, C_MUTED);
                                    kit::icon(r, art, "star", 20.0);
                                    kit::text(r, pr.stars.to_string(), &art.body, 16.0, C_MUTED);
                                });
                                if active {
                                    kit::text(b, tr(l, "prof.playing"), &art.body, 15.0, C_GOOD);
                                }
                            });
                        let confirm = setup.confirm_delete == Some(i);
                        let label = if confirm { tr(l, "prof.confirm") } else { tr(l, "prof.delete") };
                        let dbg = if confirm { Color::srgb(0.6, 0.18, 0.22) } else { kit::C_BG };
                        card.spawn((Button, kit::UiAction(Action::DeleteProfile(i as u8)), kit::Focusable(i as u16 * 2 + 1), Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(dbg), kit::Base(dbg), BorderColor(C_LINE), BorderRadius::all(Val::Px(6.0))))
                            .with_children(|b| {
                                kit::icon(b, art, "cross", 18.0);
                                kit::wrap_text(b, label, &art.body, 14.0, if confirm { C_TEXT } else { C_MUTED }, 150.0);
                            });
                    });
            }
            if book.profiles.len() < MAX_PROFILES {
                kit::tile(g, art, Action::NewProfile, 100, "sparkles", tr(l, "prof.new"), 200.0, Color::srgb(0.2, 0.5, 0.4));
            }
        });
        kit::wrap_text(c, tr(l, "prof.privacy"), &art.body, 16.0, C_MUTED, 900.0);
    });
}
