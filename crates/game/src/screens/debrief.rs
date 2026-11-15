//! Mission debrief: result, cause, rewards, new achievements, a chart and a reflection.

use super::*;

pub(super) fn debrief(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, last: &LastResult, landing_tiers: &[(Track, u8)]) {
    let r = &last.result;
    let m = map(r.map);
    p.spawn(Node { flex_grow: 1.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|o| {
        o.spawn(kit::panel()).insert((Node { width: Val::Px(760.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BorderColor(if r.won { C_GOOD } else { C_WARN }))).with_children(|d| {
            d.spawn(kit::row(14.0)).with_children(|s| {
                kit::icon(s, art, if r.won { "trophy" } else { "sos" }, 56.0);
                kit::text(s, if r.won { l.pick("Миссия выполнена!", "Mission complete!") } else { l.pick("Экипаж безопасно вернулся домой", "The crew returned home safely") }, &art.head, 28.0, C_TEXT);
                for k in 0..3 {
                    s.spawn((ImageNode { image: art.icon("star"), color: if k < r.stars { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.2) }, ..default() }, Node { width: Val::Px(40.0), height: Val::Px(40.0), ..default() }));
                }
            });
            d.spawn(kit::row(20.0)).with_children(|s| {
                for (ic, v) in [(m.icon, l.pick(m.name_ru, m.name_en).to_string()), ("hourglass", format!("{}/{}", r.sol, r.sols)), ("science", format!("{:.0}/{:.0}", r.science, r.goal)), ("radiation", format!("{:.0}%", r.dose))] {
                    s.spawn(kit::row(6.0)).with_children(|x| {
                        kit::icon(x, art, ic, 26.0);
                        kit::text(x, v, &art.body, 19.0, C_TEXT);
                    });
                }
            });
            d.spawn(kit::row(8.0)).with_children(|s| {
                kit::text(s, format!("+{}", r.points), &art.head, 36.0, C_ACCENT);
                kit::icon(s, art, "star", 32.0);
                kit::icon(s, art, "medal", 32.0);
            });
            if let Some(nm) = last.rewards.new_map {
                let nd = map(nm);
                d.spawn(kit::row(8.0)).with_children(|s| {
                    kit::icon(s, art, "key", 26.0);
                    kit::icon(s, art, nd.icon, 26.0);
                    kit::text(s, format!("{}: {}", l.pick("Открыта новая база", "New base unlocked"), l.pick(nd.name_ru, nd.name_en)), &art.body, 19.0, C_GOOD);
                });
            }
            if !last.rewards.badges.is_empty() {
                d.spawn(kit::row(12.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(12.0), justify_content: JustifyContent::Center, ..default() }).with_children(|s| {
                    kit::text(s, l.pick("Новые нашивки:", "New patches:"), &art.body, 17.0, C_MUTED);
                    for b in &last.rewards.badges {
                        if let Some(bd) = BADGES.iter().find(|x| x.id == *b) {
                            s.spawn(kit::row(4.0)).with_children(|x| {
                                kit::icon(x, art, bd.icon, 24.0);
                                kit::text(x, l.pick(bd.name_ru, bd.name_en), &art.body, 16.0, C_ACCENT);
                            });
                        }
                    }
                });
            }
            let tiers: Vec<(Track, u8)> = landing_tiers.iter().chain(last.rewards.tiers.iter()).copied().collect();
            if !tiers.is_empty() {
                d.spawn(kit::row(10.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|s| {
                    kit::text(s, tr(l, "ach.new"), &art.body, 17.0, C_MUTED);
                    for (t, k) in tiers {
                        s.spawn((kit::row(4.0), BackgroundColor(kit::C_PANEL), BorderColor(tier_color(k)), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(4.0), padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)), border: UiRect::all(Val::Px(2.0)), ..default() }).with_children(|x| {
                            kit::icon(x, art, t.icon(), 22.0);
                            kit::text(x, format!("{} · {}", track_name(l, t), tier_name(l, k)), &art.body, 15.0, tier_color(k));
                        });
                    }
                });
            }
            if let Some(c) = r.cause {
                d.spawn((kit::row(10.0), BackgroundColor(kit::C_PANEL), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), ..default() }).with_children(|s| {
                    kit::icon(s, art, match c { ja_core::Cause::Oxygen => "oxygen", ja_core::Cause::Food => "food", ja_core::Cause::Cold => "snow", ja_core::Cause::Dose => "radiation" }, 30.0);
                    kit::wrap_text(s, i18n::cause_msg(c).text(l), &art.body, 17.0, C_TEXT, 620.0);
                });
                kit::text(d, l.pick("Ошибка — это данные. Теперь ты знаешь, что изменить.", "A mistake is data. Now you know what to change."), &art.body, 17.0, C_GOOD);
            }
            chart(d, art, l, r);
            kit::text(d, if last.reflected { l.pick("Спасибо, что подумал!", "Thanks for thinking about it!") } else { l.pick("Что тебе помогло? Где это пригодится?", "What helped you? Where else is it useful?") }, &art.body, 17.0, C_MUTED);
            d.spawn(kit::row(8.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), justify_content: JustifyContent::Center, ..default() }).with_children(|s| {
                for (i, (ic, ru, en)) in i18n::REFLECT_HELPED.iter().chain(i18n::REFLECT_WHERE.iter()).enumerate() {
                    s.spawn((Button, kit::UiAction(Action::Reflect(i as u8)), kit::Focusable(10 + i as u16), Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(kit::C_PANEL), kit::Base(kit::C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(6.0)))).with_children(|b| {
                        kit::icon(b, art, ic, 22.0);
                        kit::text(b, l.pick(ru, en), &art.body, 16.0, C_TEXT);
                    });
                }
            });
            d.spawn(kit::row(10.0)).with_children(|s| {
                kit::button(s, art, Action::Again, 0, "retry", l.pick("Ещё раз", "Again"), true);
                kit::button(s, art, Action::Go(AppState::Achievements), 1, "trophy", tr(l, "menu.achievements"), false);
                kit::button(s, art, Action::Go(AppState::Menu), 2, "home", l.pick("Меню", "Menu"), false);
            });
        });
    });
}

/// Small bar chart: oxygen, food and dose at the end of each sol.
fn chart(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, r: &ja_core::MissionResult) {
    if r.history.is_empty() {
        return;
    }
    p.spawn(kit::col(4.0)).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() }).with_children(|c| {
        c.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexEnd, column_gap: Val::Px(10.0), height: Val::Px(70.0), padding: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)))).with_children(|g| {
            for h in &r.history {
                g.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexEnd, column_gap: Val::Px(2.0), ..default() }).with_children(|b| {
                    for (v, col) in [(h.o2, Color::srgb(0.35, 0.75, 1.0)), (h.food, Color::srgb(1.0, 0.7, 0.3)), (h.dose, Color::srgb(1.0, 0.4, 0.5))] {
                        b.spawn((Node { width: Val::Px(8.0), height: Val::Px(v.clamp(2.0, 100.0) * 0.6), ..default() }, BackgroundColor(col)));
                    }
                });
            }
        });
        c.spawn(kit::row(12.0)).with_children(|x| {
            for (ic, ru, en) in [("oxygen", "кислород", "oxygen"), ("food", "еда", "food"), ("radiation", "доза", "dose")] {
                x.spawn(kit::row(4.0)).with_children(|y| {
                    kit::icon(y, art, ic, 18.0);
                    kit::text(y, l.pick(ru, en), &art.body, 14.0, C_MUTED);
                });
            }
            kit::text(x, l.pick("по солам", "by sol"), &art.body, 14.0, C_MUTED);
        });
    });
}
