//! Achievements: six directions with bronze / silver / gold, then the mission patches.

use super::*;

pub(super) fn achievements(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, l: Lang) {
    body(p, |c| {
        kit::text(c, tr(l, "ach.title"), &art.head, 30.0, C_TEXT);
        c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, justify_content: JustifyContent::Center, column_gap: Val::Px(12.0), row_gap: Val::Px(12.0), max_width: Val::Px(1150.0), ..default() }).with_children(|g| {
            for t in TRACKS {
                let k = prof.tier(t);
                g.spawn((Node { width: Val::Px(360.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(kit::C_BG), BorderColor(tier_color(k)), BorderRadius::all(Val::Px(10.0))))
                    .with_children(|card| {
                        card.spawn((ImageNode { image: art.icon(t.icon()), color: if k > 0 { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.35) }, ..default() }, Node { width: Val::Px(54.0), height: Val::Px(54.0), flex_shrink: 0.0, ..default() }));
                        card.spawn(kit::col(4.0)).with_children(|x| {
                            kit::text(x, track_name(l, t), &art.head, 19.0, C_TEXT);
                            kit::wrap_text(x, track_what(l, t), &art.body, 14.0, C_MUTED, 260.0);
                            x.spawn(kit::row(4.0)).with_children(|r| {
                                for n in 1..=3u8 {
                                    r.spawn((Node { width: Val::Px(18.0), height: Val::Px(18.0), ..default() }, BackgroundColor(if k >= n { tier_color(n) } else { Color::srgba(1.0, 1.0, 1.0, 0.12) }), BorderRadius::all(Val::Px(9.0))));
                                }
                                if k > 0 {
                                    kit::text(r, tier_name(l, k), &art.body, 15.0, tier_color(k));
                                }
                            });
                            match t.next(&prof.stats) {
                                Some((have, need)) => {
                                    bar(x, have as f32 / need as f32, 240.0, tier_color(k + 1));
                                    kit::text(x, trf(l, "ach.next", &[("tier", tier_name(l, k + 1)), ("a", &have.to_string()), ("b", &need.to_string())]), &art.body, 14.0, C_MUTED);
                                }
                                None => {
                                    kit::text(x, tr(l, "ach.done"), &art.body, 14.0, C_GOOD);
                                }
                            }
                        });
                    });
            }
        });
        kit::text(c, tr(l, "ach.stars_note"), &art.body, 15.0, C_MUTED);
        kit::text(c, format!("{} {}/{}", tr(l, "ach.patches"), prof.badges.len(), BADGES.len()), &art.head, 20.0, C_TEXT);
        c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, justify_content: JustifyContent::Center, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), max_width: Val::Px(1150.0), ..default() }).with_children(|g| {
            for b in BADGES {
                let has = prof.has_badge(b.id);
                g.spawn((Node { width: Val::Px(120.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(6.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(if has { Color::srgba(0.36, 0.3, 0.1, 0.92) } else { kit::C_BG }), BorderColor(if has { C_ACCENT } else { C_LINE }), BorderRadius::all(Val::Px(8.0)))).with_children(|x| {
                    x.spawn((ImageNode { image: art.icon(b.icon), color: if has { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.25) }, ..default() }, Node { width: Val::Px(36.0), height: Val::Px(36.0), ..default() }));
                    x.spawn((Text::new(l.pick(b.name_ru, b.name_en)), TextFont { font: art.body.clone(), font_size: 13.0, ..default() }, TextColor(if has { C_TEXT } else { C_MUTED }), TextLayout::new_with_justify(JustifyText::Center)));
                });
            }
        });
    });
}
