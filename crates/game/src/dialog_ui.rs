//! A6 — dialog window: NPC portrait and name, the line (Junior / Advanced), choices or "next", skip.

use crate::art::Art;
use crate::kit::{self, Action, C_ACCENT, C_MUTED, C_TEXT};
use bevy::prelude::*;
use ja_core::content::Content;
use ja_core::dialog::DialogState;
use ja_core::{Lang, Level};

pub fn build(p: &mut ChildSpawnerCommands, art: &Art, c: &Content, d: &DialogState, level: Level, l: Lang) {
    let Some(node) = c.dialog(&d.id).and_then(|x| x.nodes.iter().find(|n| n.id == d.node)) else { return };
    let npc = c.npc(&node.npc);
    p.spawn(kit::panel()).insert(Node { width: Val::Px(900.0), flex_direction: FlexDirection::Row, column_gap: Val::Px(18.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), margin: UiRect::bottom(Val::Px(40.0)), ..default() }).with_children(|b| {
        b.spawn(kit::col(4.0)).insert(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), min_width: Val::Px(110.0), ..default() }).with_children(|x| {
            kit::icon(x, art, npc.map(|n| n.icon.as_str()).unwrap_or("speech"), 64.0);
            kit::text(x, npc.map(|n| n.name.get(l)).unwrap_or(""), &art.head, 18.0, C_ACCENT);
            kit::text(x, npc.map(|n| n.role.get(l)).unwrap_or(""), &art.body, 12.0, C_MUTED);
        });
        b.spawn(kit::col(10.0)).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), flex_grow: 1.0, ..default() }).with_children(|x| {
            kit::wrap_text(x, node.text.get(level, l), &art.body, 20.0, C_TEXT, 720.0);
            x.spawn(kit::row(10.0)).insert(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(8.0), ..default() }).with_children(|r| {
                if node.choices.is_empty() {
                    kit::button(r, art, Action::DialogNext, 1, "play", l.pick("Дальше", "Next"), true);
                } else {
                    for (i, ch) in node.choices.iter().enumerate() {
                        kit::button(r, art, Action::DialogChoose(i), 1 + i as u16, "speech", ch.text.get(l), i == 0);
                    }
                }
                kit::button(r, art, Action::DialogSkip, 50, "", l.pick("Пропустить", "Skip"), false);
            });
        });
    });
}
