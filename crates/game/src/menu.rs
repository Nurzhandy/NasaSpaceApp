//! B6 — menus: main menu over a pixel-art scene, map select with locks, briefing with the
//! if–then plan, skin shop, patches, "NASA data & sources", and the mission debrief with a chart.

use crate::art::{self, Art};
use crate::audio::Play;
use crate::kit::{self, Act, Action, C_ACCENT, C_BAD, C_BLUE, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use crate::save::{Resume, SavedMission};
use crate::{AppState, LastResult, Music, Prof, Setup, Sky, CELL, PX};
use ja_core::data::{map, skins, SkinSlot, BADGES, MAPS, SOURCES};
use ja_core::progress::ShopOutcome;
use ja_core::{i18n, Lang, MapId};
use bevy::prelude::*;
use bevy::sprite::Anchor;

#[derive(Component)]
struct Page;
#[derive(Component)]
struct Backdrop;
#[derive(Component)]
struct Walker {
    t: f32,
}

#[derive(Resource, Default)]
struct Built {
    key: Option<(AppState, u32, bool, bool, String)>,
}

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Built>()
            .add_systems(Update, (handle, backdrop, walker, build_page).chain().after(crate::Set::Input).run_if(not(in_state(AppState::Playing))).run_if(not(in_state(AppState::Ship))))
            .add_systems(Update, global_toggles.after(crate::Set::Input))
            .add_systems(OnEnter(AppState::Playing), clear_menus)
            .add_systems(OnEnter(AppState::Ship), clear_menus);
    }
}

fn clear_menus(mut commands: Commands, q: Query<Entity, Or<(With<Page>, With<Backdrop>)>>, mut built: ResMut<Built>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    built.key = None;
}

fn global_toggles(mut acts: EventReader<Act>, mut prof: ResMut<Prof>, mut music: ResMut<Music>, mut exit: EventWriter<AppExit>) {
    for Act(a) in acts.read() {
        match a {
            Action::ToggleLang => prof.0.lang = if prof.0.lang == Lang::Ru { Lang::En } else { Lang::Ru },
            Action::ToggleSound => prof.0.sound = !prof.0.sound,
            Action::ToggleMusic => music.0 = !music.0,
            Action::ToggleAge => prof.0.cadet = !prof.0.cadet,
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            _ => continue,
        }
        prof.save();
    }
}

fn handle(mut acts: EventReader<Act>, mut resume: ResMut<Resume>, mut setup: ResMut<Setup>, mut prof: ResMut<Prof>, mut next: ResMut<NextState<AppState>>, mut last: Option<ResMut<LastResult>>, mut play: EventWriter<Play>) {
    for Act(a) in acts.read() {
        match *a {
            Action::Go(s) => next.set(s),
            Action::Continue => {
                resume.0 = true;
                play.write(Play("launch"));
                next.set(AppState::Playing);
            }
            Action::Play(coop) => {
                setup.coop = coop;
                next.set(AppState::Maps);
                setup.refresh += 1;
            }
            Action::PickMap(m) => {
                setup.map = m;
                next.set(AppState::Briefing);
            }
            Action::Launch => {
                play.write(Play("launch"));
                next.set(AppState::Playing);
            }
            Action::Again => next.set(AppState::Briefing),
            Action::SetPlan(p) => {
                setup.plan = p;
                setup.refresh += 1;
            }
            Action::ToggleLive => {
                setup.live = !setup.live;
                setup.refresh += 1;
            }
            Action::ShopTab(t) => {
                setup.shop_tab = t;
                setup.refresh += 1;
            }
            Action::Skin(slot, i) => {
                match prof.0.buy_or_equip(slot, i) {
                    Ok(outcome) => {
                        play.write(Play(if outcome == ShopOutcome::Bought { "badge" } else { "click" }));
                        prof.save();
                    }
                    Err(_) => {
                        play.write(Play("bad"));
                    }
                }
                setup.refresh += 1;
            }
            Action::Reflect(_) => {
                if let Some(l) = last.as_mut() {
                    // reflection is never scored (no right answers) — just a thank-you
                    if !l.reflected {
                        l.reflected = true;
                        play.write(Play("good"));
                        setup.refresh += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

fn backdrop(mut commands: Commands, art: Res<Art>, q: Query<(), With<Backdrop>>) {
    if !q.is_empty() {
        return;
    }
    let tiles = art.sprite("tiles_mars");
    let mut seed = 99u64;
    for y in -9..9 {
        for x in -15..15 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let v = ((seed >> 33) % 10) as usize;
            commands.spawn((
                Sprite { image: tiles.clone(), texture_atlas: Some(TextureAtlas { layout: art.tile_layout.clone(), index: if v < 6 { 0 } else { (v - 5).min(3) } }), custom_size: Some(Vec2::splat(CELL)), color: Color::srgb(0.75, 0.7, 0.75), anchor: Anchor::BottomLeft, ..default() },
                Transform::from_xyz(x as f32 * CELL, y as f32 * CELL, 0.0),
                Backdrop,
            ));
        }
    }
    let tint = art::rock_tint(MapId::Mars);
    for (name, x, y, s) in [("habitat", 140.0, 30.0, 48.0), ("solar", 330.0, 60.0, 32.0), ("greenhouse", 20.0, -110.0, 48.0), ("rock_big", -520.0, 120.0, 32.0), ("rock_big", 480.0, -240.0, 32.0), ("crater", -420.0, -260.0, 32.0), ("rover", 380.0, -110.0, 32.0)] {
        let h = if name == "rover" { 24.0 } else if name == "greenhouse" { 32.0 } else { s };
        let color = if name.starts_with("rock") || name == "crater" { tint } else { Color::WHITE };
        commands.spawn((Sprite { image: art.sprite(name), custom_size: Some(Vec2::new(s, h) * PX), color, anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(x, y, 5.0 - y * 0.001), Backdrop));
    }
    commands.spawn((
        Sprite { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 8 }), custom_size: Some(Vec2::new(16.0, 24.0) * PX), anchor: Anchor::BottomCenter, ..default() },
        Transform::from_xyz(-600.0, -170.0, 6.0),
        Walker { t: 0.0 },
        Backdrop,
    ));
}

fn walker(time: Res<Time>, mut q: Query<(&mut Walker, &mut Transform, &mut Sprite)>, prof: Res<Prof>) {
    for (mut w, mut tr, mut s) in &mut q {
        w.t += time.delta_secs();
        tr.translation.x += time.delta_secs() * 90.0;
        if tr.translation.x > 700.0 {
            tr.translation.x = -700.0;
        }
        s.color = art::suit_color(&prof.0.suit);
        if let Some(a) = s.texture_atlas.as_mut() {
            a.index = 8 + ((w.t * 8.0) as usize % 4);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_page(mut commands: Commands, state: Res<State<AppState>>, setup: Res<Setup>, prof: Res<Prof>, music: Res<Music>, saved: Res<SavedMission>, sky: Res<Sky>, art: Res<Art>, last: Option<Res<LastResult>>, mut built: ResMut<Built>, old: Query<Entity, With<Page>>, mut focus: ResMut<kit::Focus>, net: Res<crate::net::Net>, lobby: Res<crate::net::Lobby>) {
    let st = *state.get();
    let key = (st, setup.refresh, prof.0.lang == Lang::En, sky.loading, format!("{}{}{}{}{}{}", prof.0.stars, prof.0.suit, prof.0.visor, prof.0.hat, prof.0.sound as u8 + music.0 as u8 * 2, prof.0.cadet) + &format!("{:?}{}{}", saved.0, lobby.rev, crate::net::status_line(&net, prof.lang()).0));
    if built.key.as_ref() == Some(&key) {
        return;
    }
    let same_page = built.key.as_ref().map(|k| k.0 == st).unwrap_or(false);
    built.key = Some(key);
    for e in &old {
        commands.entity(e).despawn();
    }
    if !same_page {
        focus.idx = 0;
    }
    let l = prof.lang();
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, ..default() }, BackgroundColor(Color::srgba(0.02, 0.02, 0.09, if st == AppState::Menu { 0.25 } else { 0.55 })), GlobalZIndex(10), Page))
        .with_children(|p| {
            topbar(p, &art, &prof, l, st);
            match st {
                AppState::Menu => main_menu(p, &art, &prof, music.0, &saved, l, &sky),
                AppState::Maps => maps(p, &art, &prof, l, &setup),
                AppState::Briefing => briefing(p, &art, l, &setup, &sky),
                AppState::Shop => shop(p, &art, &prof, l, &setup),
                AppState::Badges => badges(p, &art, &prof, l),
                AppState::Info => info(p, &art, l, &sky),
                AppState::Lobby => crate::net::lobby_page(p, &art, l, &net, &lobby),
                AppState::Debrief => {
                    if let Some(r) = last.as_deref() {
                        debrief(p, &art, l, r);
                    }
                }
                AppState::Playing | AppState::Ship => {}
            }
        });
}

fn topbar(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, st: AppState) {
    p.spawn((Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)), ..default() }, BackgroundColor(Color::srgba(0.03, 0.04, 0.12, 0.85)))).with_children(|r| {
        r.spawn(kit::row(10.0)).with_children(|b| {
            if !matches!(st, AppState::Menu | AppState::Debrief) {
                let back = if st == AppState::Briefing { AppState::Maps } else { AppState::Menu };
                kit::button(b, art, Action::Go(back), 800, "back", "", false);
            }
            kit::icon(b, art, "rocket", 32.0);
            kit::text(b, l.pick("ЮНЫЙ АСТРОНАВТ: АВАНПОСТ", "JUNIOR ASTRONAUT: OUTPOST"), &art.head, 24.0, C_TEXT);
        });
        r.spawn(kit::row(8.0)).with_children(|b| {
            for (ic, v, c) in [("medal", prof.0.total.to_string(), C_TEXT), ("star", prof.0.stars.to_string(), Color::srgb(1.0, 0.88, 0.5))] {
                b.spawn((kit::row(6.0), BackgroundColor(kit::C_PANEL), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() }).with_children(|x| {
                    kit::icon(x, art, ic, 24.0);
                    kit::text(x, v, &art.head, 20.0, c);
                });
            }
        });
    });
}

/// Big skinned astronaut built from UI images (suit tinted, visor tinted, hat).
fn avatar(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, scale: f32) {
    let (w, h) = (16.0 * scale, 24.0 * scale);
    p.spawn(Node { width: Val::Px(w), height: Val::Px(h + 16.0 * scale * 0.6), justify_content: JustifyContent::Center, ..default() }).with_children(|c| {
        let atlas = |i| Some(TextureAtlas { layout: art.astro_layout.clone(), index: i });
        c.spawn((ImageNode { image: art.sprite("astronaut"), texture_atlas: atlas(0), color: art::suit_color(&prof.0.suit), ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(0.0), width: Val::Px(w), height: Val::Px(h), ..default() }));
        c.spawn((ImageNode { image: art.sprite("astronaut_visor"), texture_atlas: atlas(0), color: art::visor_color(&prof.0.visor), ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(0.0), width: Val::Px(w), height: Val::Px(h), ..default() }));
        if let Some(hi) = art::hat_index(&prof.0.hat) {
            c.spawn((ImageNode { image: art.sprite("hats"), texture_atlas: Some(TextureAtlas { layout: art.hat_layout.clone(), index: hi }), ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(h - 7.0 * scale), width: Val::Px(w), height: Val::Px(w), ..default() }));
        }
    });
}

fn main_menu(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, music: bool, saved: &SavedMission, l: Lang, sky: &Sky) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(26.0)), column_gap: Val::Px(30.0), ..default() }).with_children(|row| {
        row.spawn(kit::col(14.0)).insert(Node { width: Val::Px(620.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(14.0), ..default() }).with_children(|c| {
            kit::wrap_text(c, l.pick("Построй базу на Луне и Марсе", "Build a base on the Moon and Mars"), &art.head, 38.0, C_TEXT, 600.0);
            c.spawn(kit::row(8.0)).with_children(|r| {
                for ic in ["oxygen", "food", "shield", "science", "power"] {
                    kit::icon(r, art, ic, 34.0);
                }
                kit::text(r, l.pick("— энергии на всё не хватит", "— there’s not enough power for all"), &art.body, 19.0, C_MUTED);
            });
            if let Some((m, sol, sols, coop)) = saved.0 {
                // the saved mission comes first: one big green button
                let md = map(m);
                let bg = Color::srgb(0.18, 0.62, 0.42);
                c.spawn((
                    Button,
                    kit::UiAction(Action::Continue),
                    kit::Focusable(0),
                    Node { width: Val::Px(530.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), padding: UiRect::axes(Val::Px(18.0), Val::Px(12.0)), border: UiRect::all(Val::Px(4.0)), ..default() },
                    BackgroundColor(bg),
                    kit::Base(bg),
                    BorderColor(C_LINE),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|b| {
                    kit::icon(b, art, "play", 48.0);
                    b.spawn(kit::col(2.0)).with_children(|t| {
                        kit::text(t, l.pick("Продолжить миссию", "Continue mission"), &art.head, 26.0, C_TEXT);
                        t.spawn(kit::row(6.0)).with_children(|x| {
                            kit::icon(x, art, md.icon, 22.0);
                            kit::text(x, format!("{} · {} {}/{}", l.pick(md.name_ru, md.name_en), l.pick("сол", "sol"), sol, sols), &art.body, 18.0, C_TEXT);
                            if coop {
                                kit::icon(x, art, "coop", 22.0);
                            }
                        });
                    });
                });
            }
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::tile(r, art, Action::Play(false), 1, "astronaut", l.pick("Играть", "Play"), 150.0, Color::srgb(0.9, 0.42, 0.22));
                kit::tile(r, art, Action::Play(true), 2, "coop", l.pick("Вдвоём", "2 players"), 150.0, Color::srgb(0.26, 0.48, 0.92));
                kit::tile(r, art, Action::Go(AppState::Lobby), 3, "satellite", l.pick("По сети", "Network"), 150.0, Color::srgb(0.2, 0.6, 0.55));
                kit::tile(r, art, Action::Go(AppState::Shop), 4, "shop", l.pick("Магазин", "Shop"), 130.0, kit::C_PANEL);
            });
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::tile(r, art, Action::StartExpedition, 5, "rocket", l.pick("Экспедиция", "Expedition"), 140.0, Color::srgb(0.55, 0.3, 0.85));
                kit::tile(r, art, Action::Go(AppState::Badges), 6, "medal", l.pick("Нашивки", "Patches"), 140.0, kit::C_PANEL);
                kit::tile(r, art, Action::Go(AppState::Info), 7, "book", l.pick("Данные NASA", "NASA data"), 140.0, kit::C_PANEL);
                kit::tile(r, art, Action::ToggleAge, 8, if prof.0.cadet { "seedling" } else { "rocket" }, if prof.0.cadet { l.pick("Кадет 7–9", "Cadet 7–9") } else { l.pick("Пилот 10–12", "Pilot 10–12") }, 140.0, kit::C_PANEL);
            });
            c.spawn(kit::row(8.0)).with_children(|r| {
                kit::button(r, art, Action::ToggleLang, 9, "globe", if prof.0.lang == Lang::En { "EN" } else { "RU" }, false);
                kit::button(r, art, Action::ToggleSound, 10, if prof.0.sound { "sound" } else { "mute" }, "", false);
                kit::button(r, art, Action::ToggleMusic, 11, if music { "s_music" } else { "mute" }, "", false);
                kit::button(r, art, Action::Quit, 12, "exit", "", false);
            });
        });
        row.spawn(kit::col(14.0)).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(14.0), align_items: AlignItems::Center, ..default() }).with_children(|c| {
            avatar(c, art, prof, 9.0);
            weather_card(c, art, l, sky);
        });
    });
}

fn weather_card(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, sky: &Sky) {
    let w = &sky.weather;
    p.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(12.0)), border: UiRect::all(Val::Px(3.0)), max_width: Val::Px(470.0), ..default() }).with_children(|c| {
        c.spawn(kit::row(8.0)).with_children(|r| {
            kit::icon(r, art, "sun", 28.0);
            kit::text(r, l.pick("Космическая погода", "Space weather"), &art.head, 19.0, C_TEXT);
            let (tag, col) = if sky.loading { (l.pick("загрузка…", "loading…"), C_MUTED) } else if w.live { ("LIVE · NASA DONKI", C_GOOD) } else { (l.pick("АРХИВ · NASA DONKI", "ARCHIVE · NASA DONKI"), C_WARN) };
            kit::text(r, tag, &art.head, 14.0, col);
        });
        for e in w.events.iter().take(3) {
            c.spawn(kit::row(6.0)).with_children(|r| {
                let col = match e.kind.as_str() { "FLR" => C_ACCENT, "GST" => Color::srgb(0.65, 0.5, 1.0), "SEP" => C_BAD, _ => C_BLUE };
                r.spawn((Node { padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)), ..default() }, BackgroundColor(col), BorderRadius::all(Val::Px(3.0)))).with_children(|t| {
                    kit::text(t, &e.kind, &art.head, 12.0, Color::BLACK);
                });
                kit::wrap_text(r, format!("{} · {}", l.pick(&e.label_ru, &e.label_en), e.date), &art.body, 14.0, C_MUTED, 380.0);
            });
        }
        kit::text(c, l.pick("Вспышка в игре — настоящая вспышка на Солнце.", "A flare in the game is a real flare from the Sun."), &art.body, 13.0, C_MUTED);
    });
}

fn maps(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, setup: &Setup) {
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

fn briefing(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, setup: &Setup, sky: &Sky) {
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

fn shop(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, setup: &Setup) {
    let cat = match setup.shop_tab { 0 => SkinSlot::Suit, 1 => SkinSlot::Visor, _ => SkinSlot::Hat };
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(26.0)), column_gap: Val::Px(24.0), ..default() }).with_children(|row| {
        row.spawn(kit::panel()).insert(Node { width: Val::Px(250.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            avatar(c, art, prof, 8.0);
            c.spawn(kit::row(6.0)).with_children(|r| {
                kit::icon(r, art, "star", 28.0);
                kit::text(r, prof.0.stars.to_string(), &art.head, 24.0, Color::srgb(1.0, 0.88, 0.5));
            });
            kit::wrap_text(c, l.pick("Покупки не закрывают базы.", "Buying never relocks bases."), &art.body, 15.0, C_MUTED, 210.0);
        });
        row.spawn(kit::col(12.0)).with_children(|c| {
            c.spawn(kit::row(8.0)).with_children(|r| {
                for (i, (ic, ru, en)) in [("shirt", "Скафандр", "Suit"), ("goggles", "Визор", "Visor"), ("crown", "Украшение", "Accessory")].iter().enumerate() {
                    kit::button(r, art, Action::ShopTab(i as u8), 100 + i as u16, ic, l.pick(ru, en), setup.shop_tab == i as u8);
                }
            });
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

fn badges(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(26.0)), row_gap: Val::Px(12.0), ..default() }).with_children(|c| {
        kit::text(c, format!("{} {}/{}", l.pick("Нашивки миссий", "Mission patches"), prof.0.badges.len(), BADGES.len()), &art.head, 28.0, C_TEXT);
        c.spawn(kit::row(10.0)).insert(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(10.0), ..default() }).with_children(|g| {
            for b in BADGES {
                let has = prof.0.badges.iter().any(|x| x == b.id);
                g.spawn((Node { width: Val::Px(165.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(12.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(if has { Color::srgba(0.36, 0.3, 0.1, 0.92) } else { kit::C_BG }), BorderColor(if has { C_ACCENT } else { C_LINE }), BorderRadius::all(Val::Px(8.0)))).with_children(|x| {
                    x.spawn((ImageNode { image: art.icon(b.icon), color: if has { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.25) }, ..default() }, Node { width: Val::Px(54.0), height: Val::Px(54.0), ..default() }));
                    x.spawn((Text::new(l.pick(b.name_ru, b.name_en)), TextFont { font: art.body.clone(), font_size: 15.0, ..default() }, TextColor(if has { C_TEXT } else { C_MUTED }), TextLayout::new_with_justify(JustifyText::Center)));
                });
            }
        });
    });
}

fn info(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, sky: &Sky) {
    p.spawn(Node { flex_grow: 1.0, padding: UiRect::all(Val::Px(22.0)), ..default() }).with_children(|o| {
        o.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), max_width: Val::Px(1000.0), ..default() }).with_children(|c| {
            c.spawn(kit::row(10.0)).with_children(|r| {
                kit::icon(r, art, "teacher", 38.0);
                kit::text(r, l.pick("Для взрослых, учителей и судей", "For adults, teachers & judges"), &art.head, 24.0, C_TEXT);
            });
            for (ic, ru, en) in [
                ("hammer", "Ребёнок сам строит базу: панели, батареи, кислород, оранжерея, щит, лаборатория. Энергии всегда меньше, чем нужно — каждое решение чего-то стоит.", "The child builds the base: panels, batteries, oxygen, greenhouse, shield, lab. There’s never enough power — every decision costs something."),
                ("radiation", "Дозы пропорциональны реальным измерениям: Луна 1,369 мЗв/сут (Chang’e‑4), Марс 0,64 (Curiosity RAD), глубокий космос 1,84.", "Doses follow real measurements: Moon 1.369 mSv/day (Chang’e‑4), Mars 0.64 (Curiosity RAD), deep space 1.84."),
                ("sun", "Вспышки — реальные события NASA DONKI. Без интернета используется архив, и на экране честно написано «АРХИВ».", "Flares are real NASA DONKI events. Offline, the archive is used and the screen honestly says “ARCHIVE”."),
                ("radio", "Сообщения ЦУП приходят с задержкой: Луна 1,3 с (реально), Марс 4–24 мин (в игре ускорено и подписано).", "Mission Control messages arrive late: Moon 1.3 s (real), Mars 4–24 min (sped up and labeled)."),
                ("heart", "Экипаж не погибает: эвакуация и разбор причины. Подсказки бесплатны, дыхательный маяк добровольный, рейтинга нет.", "No crew deaths: evacuation and a debrief. Hints are free, the breathing beacon is voluntary, no ranking."),
                ("key", "Живые данные: запустите игру с NASA_API_KEY=… (бесплатный ключ на api.nasa.gov).", "Live data: run the game with NASA_API_KEY=… (free key at api.nasa.gov)."),
            ] {
                c.spawn(kit::row(10.0)).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                    kit::icon(r, art, ic, 26.0);
                    kit::wrap_text(r, l.pick(ru, en), &art.body, 16.0, C_TEXT, 900.0);
                });
            }
            kit::text(c, if sky.weather.live { format!("DONKI LIVE · {} · {}", sky.weather.range, sky.weather.events.len()) } else { l.pick("DONKI: архив (нет сети или ключа)", "DONKI: archive (offline or no key)").to_string() }, &art.head, 15.0, if sky.weather.live { C_GOOD } else { C_WARN });
            kit::text(c, l.pick("Источники:", "Sources:"), &art.head, 17.0, C_TEXT);
            for s in SOURCES {
                kit::wrap_text(c, format!("• {} — {}", s.title, s.url), &art.body, 12.0, C_MUTED, 940.0);
            }
        });
    });
}

fn debrief(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, last: &LastResult) {
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
            if let Some(c) = r.cause {
                d.spawn((kit::row(10.0), BackgroundColor(kit::C_PANEL), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), ..default() }).with_children(|s| {
                    kit::icon(s, art, match c { ja_core::Cause::Oxygen => "oxygen", ja_core::Cause::Food => "food", ja_core::Cause::Dose => "radiation" }, 30.0);
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
                kit::button(s, art, Action::Play(r.coop), 1, "map", l.pick("К базам", "Bases"), false);
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
