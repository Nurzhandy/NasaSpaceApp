//! B9 — every screen outside the mission: main menu, profiles, profile, character and role,
//! new expedition, inventory, shop, achievements, settings, NASA data, lobby and the debrief.
//! One file per screen; this module owns the shared page builder, actions and backdrop.

mod achievements;
mod character;
mod debrief;
mod expedition;
mod info;
mod inventory;
mod legacy;
mod main_menu;
mod profile;
mod profiles;
mod settings;
mod shop;

use crate::art::{self, Art};
use crate::audio::Play;
use crate::kit::{self, tr, trf, Act, Action, C_ACCENT, C_BAD, C_BLUE, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use crate::save::{Resume, SavedMission};
use crate::{AppState, LastResult, Music, Prof, Setup, Sky, CELL, PX};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use ja_core::data::{map, skins, SkinSlot, BADGES, MAPS, SOURCES};
use ja_core::progress::{self, ShopOutcome};
use ja_core::{i18n, Lang, MapId, Profile, Role, Track, TRACKS};

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
    key: Option<(AppState, u32, Lang, bool, String)>,
}

pub struct ScreensPlugin;
impl Plugin for ScreensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Built>()
            .add_systems(Startup, apply_window)
            .add_systems(Update, (handle, backdrop, walker, build_page).chain().after(crate::Set::Input).run_if(not(in_state(AppState::Playing)).and(not(in_state(AppState::Landing)))))
            .add_systems(Update, (global_toggles.after(crate::Set::Input), art::sync_head_font))
            .add_systems(OnEnter(AppState::Playing), clear_menus)
            .add_systems(OnEnter(AppState::Landing), clear_menus);
    }
}

fn clear_menus(mut commands: Commands, q: Query<Entity, Or<(With<Page>, With<Backdrop>)>>, mut built: ResMut<Built>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    built.key = None;
}

fn set_fullscreen(w: &mut Window, on: bool) {
    w.mode = if on { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
}

fn apply_window(prof: Res<Prof>, mut win: Query<&mut Window, With<PrimaryWindow>>) {
    if prof.0.fullscreen {
        if let Ok(mut w) = win.single_mut() {
            set_fullscreen(&mut w, true);
        }
    }
}

/// Settings that work on every screen (and in the in-game pause window).
fn global_toggles(mut acts: EventReader<Act>, mut prof: ResMut<Prof>, mut music: ResMut<Music>, mut setup: ResMut<Setup>, mut win: Query<&mut Window, With<PrimaryWindow>>, mut exit: EventWriter<AppExit>) {
    for Act(a) in acts.read() {
        match *a {
            Action::ToggleLang => prof.0.lang = prof.0.lang.next(),
            Action::SetLang(l) => prof.0.lang = l,
            Action::ToggleSound => prof.0.sound = !prof.0.sound,
            Action::ToggleMusic => {
                music.0 = !music.0;
                prof.0.music = music.0;
            }
            Action::ToggleFullscreen => {
                prof.0.fullscreen = !prof.0.fullscreen;
                if let Ok(mut w) = win.single_mut() {
                    set_fullscreen(&mut w, prof.0.fullscreen);
                }
            }
            Action::SetTouch(t) => prof.0.touch = t.min(2),
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            _ => continue,
        }
        setup.refresh += 1;
        prof.save();
    }
}

#[allow(clippy::too_many_arguments)]
fn handle(mut acts: EventReader<Act>, mut resume: ResMut<Resume>, mut setup: ResMut<Setup>, mut prof: ResMut<Prof>, mut music: ResMut<Music>, mut next: ResMut<NextState<AppState>>, mut last: Option<ResMut<LastResult>>, mut play: EventWriter<Play>) {
    for Act(a) in acts.read() {
        // any press other than "delete" again cancels a pending delete
        if !matches!(a, Action::DeleteProfile(_)) {
            setup.confirm_delete = None;
        }
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
            }
            Action::PickMap(m) => {
                setup.map = m;
                next.set(AppState::Briefing);
            }
            Action::Launch => {
                play.write(Play("launch"));
                next.set(AppState::Playing);
            }
            Action::Again => next.set(AppState::Expedition),
            Action::SetCreative(c) => setup.mode = if c { ja_core::Mode::Sandbox } else { ja_core::Mode::Survival },
            Action::SetJunior(j) => {
                prof.0.cadet = j;
                prof.save();
            }
            Action::SetPlayers(two) => setup.coop = two,
            Action::CycleRole2 => setup.roles[1] = setup.roles[1].next(),
            Action::StartExpedition => {
                // the campaign: land on the Moon, then survive (or build freely in creative mode)
                setup.map = MapId::Moon;
                setup.roles = [prof.0.role, if setup.coop { setup.roles[1] } else { Role::None }];
                setup.new_tiers.clear();
                resume.0 = false;
                play.write(Play("launch"));
                next.set(AppState::Landing);
            }
            Action::SetPlan(p) => setup.plan = p,
            Action::ToggleLive => setup.live = !setup.live,
            Action::ShopTab(t) => setup.shop_tab = t,
            Action::Skin(slot, i) => match prof.0.buy_or_equip(slot, i) {
                Ok(outcome) => {
                    play.write(Play(if outcome == ShopOutcome::Bought { "badge" } else { "click" }));
                    prof.save();
                }
                Err(_) => {
                    play.write(Play("bad"));
                }
            },
            Action::Reflect(_) => {
                if let Some(l) = last.as_mut() {
                    // reflection is never scored (no right answers) — just a thank-you
                    if !l.reflected {
                        l.reflected = true;
                        play.write(Play("good"));
                    }
                }
            }
            // ---- profiles: the active one is always saved before another is loaded
            Action::SelectProfile(i) => {
                prof.save();
                let mut b = progress::load_book();
                b.select(i as usize);
                progress::save_book(&b);
                prof.0 = b.current().clone();
                music.0 = prof.0.music;
                play.write(Play("click"));
                next.set(AppState::Menu);
            }
            Action::NewProfile => {
                prof.save();
                let mut b = progress::load_book();
                if b.add().is_some() {
                    progress::save_book(&b);
                    prof.0 = b.current().clone();
                    play.write(Play("good"));
                    next.set(AppState::Character);
                } else {
                    play.write(Play("bad"));
                }
            }
            Action::DeleteProfile(i) => {
                if setup.confirm_delete == Some(i as usize) {
                    prof.save();
                    let mut b = progress::load_book();
                    b.remove(i as usize);
                    progress::save_book(&b);
                    prof.0 = b.current().clone();
                    music.0 = prof.0.music;
                    setup.confirm_delete = None;
                    play.write(Play("bad"));
                } else {
                    setup.confirm_delete = Some(i as usize);
                    play.write(Play("alarm"));
                }
            }
            Action::Callsign(d) => {
                prof.0.next_callsign(d as i32);
                prof.save();
            }
            Action::Tag(d) => {
                prof.0.next_tag(d as i32);
                prof.save();
            }
            Action::SetRole(r) => {
                prof.0.role = r;
                prof.save();
                play.write(Play("click"));
            }
            _ => continue,
        }
        setup.refresh += 1;
    }
}

fn backdrop(mut commands: Commands, art: Res<Art>, q: Query<(), With<Backdrop>>) {
    if !q.is_empty() {
        return;
    }
    let tiles = art.sprite("tiles_moon");
    let mut seed = 99u64;
    for y in -9..9 {
        for x in -15..15 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let v = ((seed >> 33) % 10) as usize;
            commands.spawn((
                Sprite { image: tiles.clone(), texture_atlas: Some(TextureAtlas { layout: art.tile_layout.clone(), index: if v < 6 { 0 } else { (v - 5).min(3) } }), custom_size: Some(Vec2::splat(CELL)), color: Color::srgb(0.75, 0.75, 0.8), anchor: Anchor::BottomLeft, ..default() },
                Transform::from_xyz(x as f32 * CELL, y as f32 * CELL, 0.0),
                Backdrop,
            ));
        }
    }
    let tint = art::rock_tint(MapId::Moon);
    for (name, x, y, s) in [("habitat", 140.0, 30.0, 48.0), ("solar", 330.0, 60.0, 32.0), ("greenhouse", 20.0, -110.0, 48.0), ("rock_big", -520.0, 120.0, 32.0), ("rock_big", 480.0, -240.0, 32.0), ("crater", -420.0, -260.0, 32.0), ("lander", 390.0, -130.0, 24.0)] {
        let h = if name == "greenhouse" { 32.0 } else { s };
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
    let p0 = &prof.0;
    let key = (st, setup.refresh, p0.lang, sky.loading, format!("{}{}{}{}{}{}{}", p0.stars, p0.suit, p0.visor, p0.hat, p0.sound as u8 + music.0 as u8 * 2, p0.cadet, art.head == art.body) + &format!("{:?}{}{}", saved.0, lobby.rev, crate::net::status_line(&net, prof.lang()).0));
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
        .spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, ..default() }, BackgroundColor(Color::srgba(0.02, 0.02, 0.09, if st == AppState::Menu { 0.25 } else { 0.6 })), GlobalZIndex(10), Page))
        .with_children(|p| {
            topbar(p, &art, &prof, l, st);
            match st {
                AppState::Menu => main_menu::main_menu(p, &art, &prof, music.0, &saved, l, &sky),
                AppState::Maps => legacy::maps(p, &art, &prof, l, &setup),
                AppState::Briefing => legacy::briefing(p, &art, l, &setup, &sky),
                AppState::Expedition => expedition::expedition(p, &art, &prof, l, &setup),
                AppState::Shop => shop::shop(p, &art, &prof, l, &setup),
                AppState::Achievements => achievements::achievements(p, &art, &prof.0, l),
                AppState::Info => info::info(p, &art, l, &sky),
                AppState::Profiles => profiles::profiles(p, &art, &progress::load_book(), l, &setup),
                AppState::ProfilePage => profile::profile(p, &art, &prof.0, l),
                AppState::Character => character::character(p, &art, &prof.0, l),
                AppState::Inventory => inventory::inventory(p, &art, &prof.0, l),
                AppState::Settings => settings::settings(p, &art, &prof.0, music.0, l),
                AppState::Lobby => crate::net::lobby_page(p, &art, l, &net, &lobby),
                AppState::Debrief => {
                    if let Some(r) = last.as_deref() {
                        debrief::debrief(p, &art, l, r, &setup.new_tiers);
                    }
                }
                AppState::Playing | AppState::Landing => {}
            }
        });
}

fn topbar(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, st: AppState) {
    p.spawn((Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)), ..default() }, BackgroundColor(Color::srgba(0.03, 0.04, 0.12, 0.85)))).with_children(|r| {
        r.spawn(kit::row(10.0)).with_children(|b| {
            if !matches!(st, AppState::Menu | AppState::Debrief) {
                let back = match st {
                    AppState::Briefing => AppState::Maps,
                    AppState::Character => AppState::ProfilePage,
                    _ => AppState::Menu,
                };
                kit::button(b, art, Action::Go(back), 800, "back", "", false);
            }
            kit::icon(b, art, "rocket", 32.0);
            kit::text(b, "SPACE EXPLORERS", &art.head, 24.0, C_TEXT);
        });
        r.spawn(kit::row(8.0)).with_children(|b| {
            let chips = [
                (prof.0.role.icon(), prof.0.name(l), C_TEXT),
                ("medal", trf(l, "prof.level", &[("n", &prof.0.level().to_string())]), C_TEXT),
                ("star", prof.0.stars.to_string(), Color::srgb(1.0, 0.88, 0.5)),
            ];
            for (ic, v, c) in chips {
                b.spawn((kit::row(6.0), BackgroundColor(kit::C_PANEL), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() }).with_children(|x| {
                    kit::icon(x, art, ic, 24.0);
                    kit::text(x, v, &art.head, 18.0, c);
                });
            }
        });
    });
}

/// Big skinned astronaut built from UI images (suit tinted, visor tinted, hat).
fn avatar(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, scale: f32) {
    let (w, h) = (16.0 * scale, 24.0 * scale);
    p.spawn(Node { width: Val::Px(w), height: Val::Px(h + 16.0 * scale * 0.6), justify_content: JustifyContent::Center, flex_shrink: 0.0, ..default() }).with_children(|c| {
        let atlas = |i| Some(TextureAtlas { layout: art.astro_layout.clone(), index: i });
        c.spawn((ImageNode { image: art.sprite("astronaut"), texture_atlas: atlas(0), color: art::suit_color(&prof.suit), ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(0.0), width: Val::Px(w), height: Val::Px(h), ..default() }));
        c.spawn((ImageNode { image: art.sprite("astronaut_visor"), texture_atlas: atlas(0), color: art::visor_color(&prof.visor), ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(0.0), width: Val::Px(w), height: Val::Px(h), ..default() }));
        if let Some(hi) = art::hat_index(&prof.hat) {
            c.spawn((ImageNode { image: art.sprite("hats"), texture_atlas: Some(TextureAtlas { layout: art.hat_layout.clone(), index: hi }), ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(h - 7.0 * scale), width: Val::Px(w), height: Val::Px(w), ..default() }));
        }
    });
}

/// Page body: a padded column that fills the screen under the top bar.
fn body(p: &mut ChildSpawnerCommands, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(14.0), padding: UiRect::all(Val::Px(20.0)), overflow: Overflow::clip(), ..default() }).with_children(f);
}

/// A selectable card button (bigger than `kit::button`, with a highlighted "on" state).
#[allow(clippy::too_many_arguments)]
fn card(p: &mut ChildSpawnerCommands, art: &Art, act: Action, order: u16, icon: &str, name: &str, desc: &str, on: bool, w: f32) {
    let bg = if on { Color::srgb(0.18, 0.42, 0.62) } else { kit::C_PANEL };
    p.spawn((Button, kit::UiAction(act), kit::Focusable(order), Node { width: Val::Px(w), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(4.0)), ..default() }, BackgroundColor(bg), kit::Base(bg), BorderColor(if on { C_ACCENT } else { C_LINE }), BorderRadius::all(Val::Px(10.0))))
        .with_children(|b| {
            kit::icon(b, art, icon, 44.0);
            b.spawn(kit::col(2.0)).with_children(|t| {
                kit::text(t, name, &art.head, 20.0, C_TEXT);
                if !desc.is_empty() {
                    kit::wrap_text(t, desc, &art.body, 15.0, C_MUTED, w - 80.0);
                }
            });
        });
}

/// Horizontal progress bar.
fn bar(p: &mut ChildSpawnerCommands, frac: f32, w: f32, color: Color) {
    p.spawn((Node { width: Val::Px(w), height: Val::Px(10.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)), BorderRadius::all(Val::Px(5.0)))).with_children(|b| {
        b.spawn((Node { width: Val::Px(w * frac.clamp(0.0, 1.0)), height: Val::Px(10.0), ..default() }, BackgroundColor(color), BorderRadius::all(Val::Px(5.0))));
    });
}

/// Bronze, silver, gold.
fn tier_color(k: u8) -> Color {
    match k {
        1 => Color::srgb(0.85, 0.55, 0.3),
        2 => Color::srgb(0.8, 0.84, 0.92),
        3 => Color::srgb(1.0, 0.82, 0.3),
        _ => Color::srgba(1.0, 1.0, 1.0, 0.15),
    }
}

fn tier_name(l: Lang, k: u8) -> &'static str {
    match k {
        1 => tr(l, "ach.tier1"),
        2 => tr(l, "ach.tier2"),
        _ => tr(l, "ach.tier3"),
    }
}

fn track_name(l: Lang, t: Track) -> &'static str {
    match t {
        Track::Explorer => tr(l, "ach.explorer"),
        Track::Engineer => tr(l, "ach.engineer"),
        Track::Pilot => tr(l, "ach.pilot"),
        Track::Survivor => tr(l, "ach.survivor"),
        Track::Scientist => tr(l, "ach.scientist"),
        Track::Team => tr(l, "ach.team"),
    }
}

fn track_what(l: Lang, t: Track) -> &'static str {
    match t {
        Track::Explorer => tr(l, "ach.explorer.what"),
        Track::Engineer => tr(l, "ach.engineer.what"),
        Track::Pilot => tr(l, "ach.pilot.what"),
        Track::Survivor => tr(l, "ach.survivor.what"),
        Track::Scientist => tr(l, "ach.scientist.what"),
        Track::Team => tr(l, "ach.team.what"),
    }
}

fn role_name(l: Lang, r: Role) -> &'static str {
    match r {
        Role::None => tr(l, "role.none"),
        Role::Pilot => tr(l, "role.pilot"),
        Role::Engineer => tr(l, "role.engineer"),
        Role::Researcher => tr(l, "role.researcher"),
        Role::LifeSupport => tr(l, "role.life_support"),
    }
}

fn role_bonus(l: Lang, r: Role) -> &'static str {
    match r {
        Role::None => tr(l, "role.none.bonus"),
        Role::Pilot => tr(l, "role.pilot.bonus"),
        Role::Engineer => tr(l, "role.engineer.bonus"),
        Role::Researcher => tr(l, "role.researcher.bonus"),
        Role::LifeSupport => tr(l, "role.life_support.bonus"),
    }
}

/// A medal row for the six directions (profile page, debrief).
fn track_medals(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, size: f32) {
    p.spawn(kit::row(8.0)).with_children(|r| {
        for t in TRACKS {
            let k = prof.tier(t);
            r.spawn((Node { width: Val::Px(size + 12.0), height: Val::Px(size + 12.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(kit::C_PANEL), BorderColor(tier_color(k)), BorderRadius::all(Val::Px(size))))
                .with_children(|m| {
                    m.spawn((ImageNode { image: art.icon(t.icon()), color: if k > 0 { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.3) }, ..default() }, Node { width: Val::Px(size), height: Val::Px(size), ..default() }));
                });
        }
    });
}
