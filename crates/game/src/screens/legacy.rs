//! The old map select and briefing (kept for the story maps; the campaign starts from «Начать»).

use super::*;

pub(super) fn maps(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, setup: &Setup) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(26.0)), row_gap: Val::Px(14.0), ..default() }).with_children(|c| {
        c.spawn(kit::row(10.0)).with_children(|r| {
            kit::icon(r, art, if setup.coop { "coop" } else { "astronaut" }, 38.0);
            kit::text(r, if setup.coop { l.pick("Вдвоём: выбери базу", "2 players: choose a base") } else { l.pick("Выбери базу", "Choose a base") }, &art.head, 30.0, C_TEXT);
            if setup.coop {
                kit::text(r, l.pick("игрок 2: стрелки + правый Shift, или второй геймпад", "player 2: arrows + right Shift, or a 2nd gamepad"), &art.body, 17.0, C_MUTED);
            }
        });
        c.spawn(kit::row(14.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(14.0), row_gap: Val::Px(14.0), ..default() }).with_children(|g| {
            for (i, m) in MAPS.iter().enumerate() {
                let open = prof.0.unlocked(m.id);
                let best = prof.0.best_stars(m.id);
                let bg = if open { kit::C_PANEL } else { Color::srgba(0.1, 0.1, 0.16, 0.92) };
                let mut e = g.spawn((Node { width: Val::Px(220.0), height: Val::Px(240.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(bg), kit::Base(bg), BorderColor(C_LINE), BorderRadius::all(Val::Px(10.0))));
                if open {
                    e.insert((Button, kit::UiAction(Action::PickMap(m.id)), kit::Focusable(i as u16)));
                }
                e.with_children(|card| {
                    card.spawn((ImageNode { image: art.sprite(art::tiles_for(m.id)), texture_atlas: Some(TextureAtlas { layout: art.tile_layout.clone(), index: 1 }), ..default() }, Node { width: Val::Px(96.0), height: Val::Px(96.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() })).with_children(|t| {
                        kit::icon(t, art, if open { m.icon } else { "lock" }, 60.0);
                    });
                    card.spawn((Text::new(l.pick(m.name_ru, m.name_en)), TextFont { font: art.body.clone(), font_size: 19.0, ..default() }, TextColor(if open { C_TEXT } else { C_MUTED }), TextLayout::new_with_justify(JustifyText::Center)));
                    if open {
                        card.spawn(kit::row(4.0)).with_children(|s| {
                            for k in 0..3 {
                                s.spawn((ImageNode { image: art.icon("star"), color: if k < best { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.2) }, ..default() }, Node { width: Val::Px(28.0), height: Val::Px(28.0), ..default() }));
                            }
                        });
                    } else {
                        card.spawn(kit::row(6.0)).with_children(|r| {
                            kit::icon(r, art, "medal", 22.0);
                            kit::text(r, format!("{} / {}", prof.0.total, m.need), &art.head, 18.0, C_MUTED);
                        });
                    }
                });
            }
        });
        kit::text(c, l.pick("Очки 🏅 за миссии открывают базы. Звёзды ⭐ тратятся в магазине — базы не закроются.", "Points 🏅 unlock bases. Stars ⭐ are spent in the shop — bases never relock."), &art.body, 17.0, C_MUTED);
    });
}

pub(super) fn briefing(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, setup: &Setup, sky: &Sky) {
    let m = map(setup.map);
    p.spawn(Node { flex_grow: 1.0, padding: UiRect::all(Val::Px(26.0)), ..default() }).with_children(|row| {
        row.spawn(kit::panel()).insert(Node { width: Val::Px(700.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(20.0)), row_gap: Val::Px(12.0), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::icon(r, art, m.icon, 60.0);
                kit::text(r, l.pick(m.name_ru, m.name_en), &art.head, 30.0, C_TEXT);
            });
            for (ic, label, value) in [
                ("radiation", l.pick("Радиация", "Radiation"), l.pick(m.dose_real_ru, m.dose_real_en).to_string()),
                ("radio", l.pick("Задержка связи", "Comms delay"), l.pick(m.delay_real_ru, m.delay_real_en).to_string()),
                ("run", l.pick("Гравитация", "Gravity"), format!("{:.0}%", m.gravity * 100.0)),
                ("hourglass", l.pick("Солов", "Sols"), m.sols.to_string()),
            ] {
                c.spawn(kit::row(10.0)).with_children(|r| {
                    kit::icon(r, art, ic, 28.0);
                    kit::text(r, label, &art.body, 19.0, C_MUTED);
                    kit::wrap_text(r, value, &art.body, 19.0, C_TEXT, 440.0);
                });
            }
            c.spawn(kit::row(14.0)).with_children(|r| {
                for (ic, ru, en) in [("check", "выжить", "survive"), ("science", "наука", "science"), ("radiation", "доза < 50%", "dose < 50%")] {
                    r.spawn(kit::row(4.0)).with_children(|s| {
                        kit::icon(s, art, "star", 24.0);
                        kit::icon(s, art, ic, 22.0);
                        kit::text(s, l.pick(ru, en), &art.body, 16.0, C_MUTED);
                    });
                }
            });
            kit::wrap_text(c, l.pick(m.fact_ru, m.fact_en), &art.body, 18.0, C_TEXT, 650.0);
            kit::text(c, l.pick("Мой план «если — то»:", "My if–then plan:"), &art.body, 18.0, C_MUTED);
            c.spawn(kit::row(8.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|r| {
                for (i, (ic, ru, en)) in i18n::IF_THEN.iter().enumerate() {
                    kit::button(r, art, Action::SetPlan(i as u8), i as u16, ic, l.pick(ru, en), setup.plan == i as u8);
                }
            });
            if m.id != MapId::Earth {
                let on = setup.live && sky.weather.live;
                c.spawn(kit::row(10.0)).with_children(|r| {
                    kit::button(r, art, Action::ToggleLive, 10, if on { "check" } else { "cross" }, l.pick("Реальная космическая погода", "Real space weather"), false);
                    kit::text(r, if sky.weather.live { "LIVE" } else { l.pick("АРХИВ (нет связи)", "ARCHIVE (offline)") }, &art.head, 15.0, if sky.weather.live { C_GOOD } else { C_WARN });
                });
            }
            c.spawn(Node { justify_content: JustifyContent::FlexEnd, ..default() }).with_children(|r| {
                kit::button(r, art, Action::Launch, 11, "rocket", l.pick("Старт миссии", "Launch mission"), true);
            });
        });
    });
}
