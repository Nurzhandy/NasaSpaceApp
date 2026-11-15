//! Character: the callsign (picked, never typed) and the expedition role.

use super::*;
use ja_core::ROLES;

pub(super) fn character(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, l: Lang) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(22.0)), column_gap: Val::Px(26.0), ..default() }).with_children(|row| {
        row.spawn(kit::panel()).insert(Node { width: Val::Px(340.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            kit::text(c, tr(l, "char.title"), &art.head, 26.0, C_TEXT);
            avatar(c, art, prof, 7.0);
            kit::text(c, tr(l, "prof.callsign"), &art.body, 17.0, C_MUTED);
            c.spawn(kit::row(8.0)).with_children(|r| {
                kit::button(r, art, Action::Callsign(-1), 0, "back", "", false);
                kit::text(r, prof.name(l), &art.head, 24.0, C_ACCENT);
                kit::button(r, art, Action::Callsign(1), 1, "play", "", false);
            });
            c.spawn(kit::row(8.0)).with_children(|r| {
                kit::button(r, art, Action::Tag(-1), 2, "", "−", false);
                kit::text(r, format!("№ {}", prof.tag), &art.head, 20.0, C_TEXT);
                kit::button(r, art, Action::Tag(1), 3, "", "+", false);
            });
            kit::wrap_text(c, tr(l, "char.look"), &art.body, 15.0, C_MUTED, 300.0);
            c.spawn(kit::row(8.0)).with_children(|r| {
                kit::button(r, art, Action::Go(AppState::Inventory), 4, "box", "", false);
                kit::button(r, art, Action::Go(AppState::Shop), 5, "shop", "", false);
            });
        });
        row.spawn(kit::col(10.0)).with_children(|c| {
            kit::text(c, tr(l, "char.role"), &art.head, 24.0, C_TEXT);
            for (i, r) in ROLES.iter().enumerate() {
                card(c, art, Action::SetRole(*r), 10 + i as u16, r.icon(), role_name(l, *r), role_bonus(l, *r), prof.role == *r, 520.0);
            }
            kit::wrap_text(c, tr(l, "role.note"), &art.body, 15.0, C_MUTED, 520.0);
        });
    });
}
