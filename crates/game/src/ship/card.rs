//! A5 — the physics card after an episode, and the calm "breather" after a solved problem.

use crate::art::Art;
use crate::kit::{self, Action, C_ACCENT, C_MUTED, C_TEXT};
use bevy::prelude::*;
use ja_core::campaign::Breather;
use ja_core::content::{CardKind, Content};
use ja_core::{Lang, Level};

pub fn build(p: &mut ChildSpawnerCommands, art: &Art, c: &Content, id: &str, level: Level, l: Lang) {
    let Some(card) = c.cards.iter().find(|k| k.id == id) else { return };
    p.spawn(kit::panel()).insert(Node { width: Val::Px(680.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(22.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|b| {
        let (kind_ru, kind_en, icon) = match card.kind {
            CardKind::Quantity => ("величина", "quantity", "test"),
            CardKind::Phenomenon => ("явление", "phenomenon", "sparkles"),
            CardKind::Concept => ("понятие", "concept", "map"),
        };
        b.spawn(kit::row(12.0)).with_children(|r| {
            kit::icon(r, art, icon, 48.0);
            r.spawn(kit::col(2.0)).with_children(|t| {
                kit::text(t, l.pick("Физика полёта", "Flight physics"), &art.body, 15.0, C_MUTED);
                kit::text(t, card.title.get(l), &art.head, 30.0, C_ACCENT);
                kit::text(t, l.pick(kind_ru, kind_en), &art.body, 15.0, C_MUTED);
            });
        });
        // symbol and unit only for quantities; formulas as plain text
        if level == Level::Advanced {
            let mut line = vec![];
            if let Some(s) = &card.symbol {
                line.push(format!("{} {s}", l.pick("обозначение", "symbol")));
            }
            if let Some(u) = &card.unit {
                line.push(format!("{} {u}", l.pick("единицы:", "unit:")));
            }
            if !line.is_empty() {
                kit::text(b, line.join(" · "), &art.body, 17.0, C_TEXT);
            }
            if let Some(f) = &card.formula {
                b.spawn((Node { padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)), BorderRadius::all(Val::Px(6.0)))).with_children(|x| {
                    kit::text(x, f.clone(), &art.head, 26.0, C_TEXT);
                });
            }
        }
        let text = match (level, &card.advanced) {
            (Level::Advanced, Some(a)) => a.get(l),
            _ => card.junior.get(l),
        };
        kit::wrap_text(b, text, &art.body, 19.0, C_TEXT, 630.0);
        if card.draft {
            kit::text(b, l.pick("черновик текста — методисты заменят", "draft text — to be replaced by the team"), &art.body, 12.0, C_MUTED);
        }
        kit::button(b, art, Action::CloseCard, 1, "check", l.pick("Понятно!", "Got it!"), true);
    });
}

pub fn breather(p: &mut ChildSpawnerCommands, art: &Art, b: Breather, l: Lang) {
    let (icon, ru, en) = match b {
        Breather::PhotoEarth => ("s_photo", "Передышка: в иллюминаторе — Земля. Экипаж делает снимок на память.", "A breather: Earth in the window. The crew takes a photo to remember."),
        Breather::CallHome => ("s_call", "Передышка: короткий звонок домой. Близкие гордятся вами!", "A breather: a short call home. Your families are proud of you!"),
        Breather::CrewLine => ("thumbs", "Тимур: «Хорошо сработали! Минутка тишины — и дальше».", "Timur: “Nice work! A minute of quiet, then on we go.”"),
    };
    p.spawn(kit::panel()).insert(Node { width: Val::Px(560.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(20.0)), border: UiRect::all(Val::Px(3.0)), align_items: AlignItems::Center, ..default() }).with_children(|x| {
        kit::icon(x, art, icon, 56.0);
        kit::wrap_text(x, l.pick(ru, en), &art.body, 19.0, C_TEXT, 500.0);
        kit::button(x, art, Action::CloseBreather, 1, "play", l.pick("Дальше", "Continue"), true);
    });
}
