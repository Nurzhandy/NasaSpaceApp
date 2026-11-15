//! Shop: cosmetics for stars (never power) and a read-only list of base upgrades.

use super::*;

pub(super) fn shop(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, setup: &Setup) {
    let cat = match setup.shop_tab { 0 => SkinSlot::Suit, 1 => SkinSlot::Visor, _ => SkinSlot::Hat };
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(26.0)), column_gap: Val::Px(24.0), ..default() }).with_children(|row| {
        row.spawn(kit::panel()).insert(Node { width: Val::Px(250.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            avatar(c, art, &prof.0, 8.0);
            c.spawn(kit::row(6.0)).with_children(|r| {
                kit::icon(r, art, "star", 28.0);
                kit::text(r, prof.0.stars.to_string(), &art.head, 24.0, Color::srgb(1.0, 0.88, 0.5));
            });
            kit::wrap_text(c, tr(l, "shop.never_relock"), &art.body, 15.0, C_MUTED, 210.0);
        });
        row.spawn(kit::col(12.0)).with_children(|c| {
            c.spawn(kit::row(8.0)).with_children(|r| {
                for (i, (ic, key)) in [("shirt", "shop.suits"), ("goggles", "shop.visors"), ("crown", "shop.hats"), ("gear", "shop.upgrades")].into_iter().enumerate() {
                    kit::button(r, art, Action::ShopTab(i as u8), 100 + i as u16, ic, tr(l, key), setup.shop_tab == i as u8);
                }
            });
            if setup.shop_tab == 3 {
                upgrades(c, art, l);
                return;
            }
            c.spawn(kit::row(10.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(10.0), max_width: Val::Px(760.0), ..default() }).with_children(|g| {
                for (i, s) in skins(cat).iter().enumerate() {
                    let own = prof.0.owns(cat, s.id);
                    let wear = match cat { SkinSlot::Suit => &prof.0.suit, SkinSlot::Visor => &prof.0.visor, SkinSlot::Hat => &prof.0.hat } == s.id;
                    g.spawn((Button, kit::UiAction(Action::Skin(cat, i)), kit::Focusable(i as u16), Node { width: Val::Px(160.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(12.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(kit::C_PANEL), kit::Base(kit::C_PANEL), BorderColor(if wear { C_GOOD } else { C_LINE }), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
                        match cat {
                            SkinSlot::Suit => {
                                b.spawn((ImageNode { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: art::suit_color(s.id), ..default() }, Node { width: Val::Px(48.0), height: Val::Px(72.0), ..default() }));
                            }
                            SkinSlot::Visor => {
                                b.spawn((Node { width: Val::Px(56.0), height: Val::Px(40.0), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(Color::srgb(s.color[0], s.color[1], s.color[2])), BorderColor(Color::WHITE), BorderRadius::all(Val::Px(20.0))));
                            }
                            SkinSlot::Hat => match art::hat_index(s.id) {
                                Some(h) => {
                                    b.spawn((ImageNode { image: art.sprite("hats"), texture_atlas: Some(TextureAtlas { layout: art.hat_layout.clone(), index: h }), ..default() }, Node { width: Val::Px(56.0), height: Val::Px(56.0), ..default() }));
                                }
                                None => {
                                    kit::icon(b, art, "cross", 48.0);
                                }
                            },
                        }
                        b.spawn((Text::new(l.pick(s.name_ru, s.name_en)), TextFont { font: art.body.clone(), font_size: 16.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
                        b.spawn(kit::row(4.0)).with_children(|r| {
                            if wear {
                                kit::icon(r, art, "check", 20.0);
                                kit::text(r, l.pick("надето", "wearing"), &art.body, 15.0, C_GOOD);
                            } else if own {
                                kit::text(r, l.pick("надеть", "wear"), &art.body, 15.0, C_TEXT);
                            } else {
                                kit::icon(r, art, "star", 20.0);
                                kit::text(r, s.price.to_string(), &art.head, 17.0, if prof.0.stars >= s.price { C_ACCENT } else { C_MUTED });
                            }
                        });
                    });
                }
            });
        });
    });
}

/// Base upgrades are bought on the Moon (key U); here they are only explained.
fn upgrades(c: &mut ChildSpawnerCommands, art: &Art, l: Lang) {
    kit::wrap_text(c, tr(l, "shop.upgrades.note"), &art.body, 16.0, C_MUTED, 760.0);
    c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(10.0), max_width: Val::Px(780.0), ..default() }).with_children(|g| {
        for d in ja_core::survival::UPGRADE_DEFS.iter() {
            g.spawn((Node { width: Val::Px(380.0), flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(kit::C_BG), BorderColor(C_LINE), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
                kit::icon(b, art, d.icon, 40.0);
                b.spawn(kit::col(3.0)).with_children(|t| {
                    kit::text(t, l.pick(d.name_ru, d.name_en), &art.head, 18.0, C_TEXT);
                    kit::wrap_text(t, format!("+ {}", l.pick(d.plus_ru, d.plus_en)), &art.body, 14.0, C_GOOD, 300.0);
                    kit::wrap_text(t, format!("− {}", l.pick(d.minus_ru, d.minus_en)), &art.body, 14.0, C_WARN, 300.0);
                    t.spawn(kit::row(8.0)).with_children(|r| {
                        for (i, c) in d.cost.iter().enumerate() {
                            kit::icon(r, art, "coin", 16.0);
                            kit::text(r, format!("{} {c}", trf(l, "shop.level", &[("n", &(i + 1).to_string())])), &art.body, 14.0, C_MUTED);
                        }
                    });
                });
            });
        }
    });
}
