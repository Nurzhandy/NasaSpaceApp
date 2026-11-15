//! Inventory: everything the astronaut owns — suits, visors, accessories (click to wear),
//! mission patches, and what they collected over all expeditions.

use super::*;

pub(super) fn inventory(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, l: Lang) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(22.0)), column_gap: Val::Px(22.0), ..default() }).with_children(|row| {
        row.spawn(kit::panel()).insert(Node { width: Val::Px(260.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            avatar(c, art, prof, 6.0);
            kit::text(c, prof.name(l), &art.head, 22.0, C_TEXT);
            kit::text(c, tr(l, "inv.counts"), &art.body, 16.0, C_MUTED);
            for (ic, v, key) in [("science", prof.stats.samples, "inv.samples"), ("rock", prof.stats.collected, "inv.collected"), ("rocket", prof.stats.landings, "inv.landings")] {
                c.spawn(kit::row(6.0)).with_children(|r| {
                    kit::icon(r, art, ic, 24.0);
                    kit::text(r, v.to_string(), &art.head, 18.0, C_TEXT);
                    kit::wrap_text(r, tr(l, key), &art.body, 15.0, C_MUTED, 150.0);
                });
            }
        });
        row.spawn(kit::col(10.0)).insert(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() }).with_children(|c| {
            kit::text(c, tr(l, "inv.title"), &art.head, 28.0, C_TEXT);
            kit::text(c, tr(l, "inv.wear_hint"), &art.body, 15.0, C_MUTED);
            let mut order = 0u16;
            for (slot, key) in [(SkinSlot::Suit, "inv.suits"), (SkinSlot::Visor, "inv.visors"), (SkinSlot::Hat, "inv.hats")] {
                kit::text(c, tr(l, key), &art.head, 18.0, C_TEXT);
                c.spawn(kit::row(8.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|g| {
                    let wearing = match slot { SkinSlot::Suit => &prof.suit, SkinSlot::Visor => &prof.visor, SkinSlot::Hat => &prof.hat };
                    for (i, s) in skins(slot).iter().enumerate().filter(|(_, s)| prof.owns(slot, s.id)) {
                        let on = wearing == s.id;
                        g.spawn((Button, kit::UiAction(Action::Skin(slot, i)), kit::Focusable(order), Node { width: Val::Px(110.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(6.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(kit::C_PANEL), kit::Base(kit::C_PANEL), BorderColor(if on { C_GOOD } else { C_LINE }), BorderRadius::all(Val::Px(8.0))))
                            .with_children(|b| {
                                match slot {
                                    SkinSlot::Suit => {
                                        b.spawn((ImageNode { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: art::suit_color(s.id), ..default() }, Node { width: Val::Px(32.0), height: Val::Px(48.0), ..default() }));
                                    }
                                    SkinSlot::Visor => {
                                        b.spawn((Node { width: Val::Px(40.0), height: Val::Px(28.0), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(Color::srgb(s.color[0], s.color[1], s.color[2])), BorderColor(Color::WHITE), BorderRadius::all(Val::Px(14.0))));
                                    }
                                    SkinSlot::Hat => match art::hat_index(s.id) {
                                        Some(h) => {
                                            b.spawn((ImageNode { image: art.sprite("hats"), texture_atlas: Some(TextureAtlas { layout: art.hat_layout.clone(), index: h }), ..default() }, Node { width: Val::Px(40.0), height: Val::Px(40.0), ..default() }));
                                        }
                                        None => {
                                            kit::icon(b, art, "cross", 36.0);
                                        }
                                    },
                                }
                                b.spawn((Text::new(l.pick(s.name_ru, s.name_en)), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
                                if on {
                                    kit::text(b, tr(l, "inv.wearing"), &art.body, 13.0, C_GOOD);
                                }
                            });
                        order += 1;
                    }
                });
            }
            kit::text(c, format!("{} {}/{}", tr(l, "inv.patches"), prof.badges.len(), BADGES.len()), &art.head, 18.0, C_TEXT);
            c.spawn(kit::row(6.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(6.0), row_gap: Val::Px(6.0), ..default() }).with_children(|g| {
                if prof.badges.is_empty() {
                    kit::text(g, tr(l, "inv.empty"), &art.body, 15.0, C_MUTED);
                }
                for b in BADGES.iter().filter(|b| prof.has_badge(b.id)) {
                    g.spawn((kit::row(4.0), BackgroundColor(Color::srgba(0.36, 0.3, 0.1, 0.92)), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(4.0), padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)), ..default() }).with_children(|x| {
                        kit::icon(x, art, b.icon, 22.0);
                        kit::text(x, l.pick(b.name_ru, b.name_en), &art.body, 14.0, C_TEXT);
                    });
                }
            });
        });
    });
}
