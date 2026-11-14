//! Main menu with a 3D astronaut preview, map select, briefing, skin shop, patches,
//! the "NASA data & sources" page, and the post-mission debrief.

use crate::audio::Sfx;
use crate::data::*;
use crate::game::MissionResult;
use crate::i18n::Lang;
use crate::nasa::Weather;
use crate::player::Inputs;
use crate::save::Profile;
use crate::sim::Cause;
use crate::ui::*;
use crate::world::{spawn_astronaut, WalkAnim};
use crate::{AppState, GameConfig};
use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuPage {
    #[default]
    Main,
    Maps,
    Brief(MapId),
    Shop(u8),
    Badges,
    Sources,
}

/// Debrief reflection answers. Never scored.
#[derive(Resource, Default)]
pub struct Reflection {
    pub helped: Option<u8>,
    pub where_: Option<u8>,
}

#[derive(Component)]
struct ScreenRoot;
#[derive(Component)]
struct PreviewAstro {
    skin: (String, String, String),
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuPage>()
            .init_resource::<Reflection>()
            .add_systems(OnEnter(AppState::Menu), spawn_preview_menu)
            .add_systems(OnEnter(AppState::Debrief), (spawn_preview_debrief, reset_reflection))
            .add_systems(
                Update,
                (
                    menu_back.run_if(in_state(AppState::Menu).or(in_state(AppState::Debrief))),
                    handle_menu_actions.run_if(in_state(AppState::Menu).or(in_state(AppState::Debrief))),
                    build_menu.run_if(in_state(AppState::Menu)),
                    build_debrief.run_if(in_state(AppState::Debrief)),
                    preview_astro.run_if(in_state(AppState::Menu)),
                )
                    .chain()
                    .in_set(crate::GameSet::Ui)
                    .after(crate::ui::click_system),
            );
    }
}

// ---------------------------------------------------------------- 3D preview

fn spawn_preview(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, state: AppState, pedestal: bool) {
    let scoped = StateScoped(state);
    commands.spawn((
        Camera3d::default(),
        Camera { order: 0, clear_color: ClearColorConfig::Custom(Color::srgb(0.015, 0.02, 0.06)), ..default() },
        Transform::from_xyz(0.0, 1.6, 6.2).looking_at(Vec3::new(0.0, 1.15, 0.0), Vec3::Y),
        scoped.clone(),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadows_enabled: true, ..default() },
        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        scoped.clone(),
    ));
    commands.spawn((PointLight { intensity: 300_000.0, range: 20.0, color: Color::srgb(0.4, 0.7, 1.0), ..default() }, Transform::from_xyz(-1.0, 3.0, -2.0), scoped.clone()));
    // a little patch of lunar ground under the astronaut
    if pedestal {
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.6, 0.3))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.57, 0.63), perceptual_roughness: 1.0, ..default() })),
        Transform::from_xyz(2.1, -0.15, 0.0),
        scoped.clone(),
    ));
    }
    // Mars hanging in the background
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(4.0).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.42, 0.22), emissive: LinearRgba::rgb(0.25, 0.08, 0.03), perceptual_roughness: 0.9, ..default() })),
        Transform::from_xyz(7.0, 4.5, -18.0),
        scoped.clone(),
    ));
    let star_mesh = meshes.add(Sphere::new(0.07).mesh().ico(1).unwrap());
    let star_mat = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::rgb(3.0, 3.0, 3.5), unlit: true, ..default() });
    let mut rng = crate::sim::Rng(0x5EED_1234_ABCD_0001);
    for _ in 0..260 {
        let p = Vec3::new(rng.range(-28.0, 28.0), rng.range(-6.0, 18.0), rng.range(-30.0, -12.0));
        commands.spawn((Mesh3d(star_mesh.clone()), MeshMaterial3d(star_mat.clone()), Transform::from_translation(p).with_scale(Vec3::splat(rng.range(0.6, 1.8))), scoped.clone()));
    }
}

fn spawn_preview_menu(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    spawn_preview(&mut commands, &mut meshes, &mut materials, AppState::Menu, true);
}

fn spawn_preview_debrief(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    spawn_preview(&mut commands, &mut meshes, &mut materials, AppState::Debrief, false);
}

fn reset_reflection(mut r: ResMut<Reflection>) {
    *r = Reflection::default();
}

/// Spins the preview astronaut and rebuilds it when the skin changes.
fn preview_astro(
    mut commands: Commands,
    time: Res<Time>,
    profile: Res<Profile>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &PreviewAstro, &mut Transform)>,
) {
    let skin = (profile.suit.clone(), profile.visor.clone(), profile.hat.clone());
    let mut found = false;
    for (e, pa, mut tr) in &mut q {
        if pa.skin != skin {
            commands.entity(e).despawn();
            continue;
        }
        found = true;
        let t = time.elapsed_secs();
        tr.rotation = Quat::from_rotation_y(0.5 + (t * 0.5).sin() * 0.7);
        tr.translation.y = (t * 1.6).sin().abs() * 0.08;
    }
    if found {
        return;
    }
    let suit = SUITS.iter().find(|s| s.id == profile.suit).map(|s| s.color).unwrap_or(Color::WHITE);
    let visor = VISORS.iter().find(|s| s.id == profile.visor).map(|s| s.color).unwrap_or(Color::srgb(0.5, 0.8, 1.0));
    let root = spawn_astronaut(&mut commands, &mut meshes, &mut materials, suit, visor, &profile.hat);
    commands.entity(root).insert((Transform::from_xyz(2.1, 0.0, 0.0).with_scale(Vec3::splat(1.25)), PreviewAstro { skin }, WalkAnim::default(), StateScoped(AppState::Menu)));
}

// ---------------------------------------------------------------- actions

fn menu_back(inputs: Res<Inputs>, mut page: ResMut<MenuPage>, state: Res<State<AppState>>, mut next: ResMut<NextState<AppState>>) {
    if !(inputs.0[0].back || inputs.0[1].back) {
        return;
    }
    match state.get() {
        AppState::Menu => {
            let to = match *page {
                MenuPage::Main => return,
                MenuPage::Brief(_) => MenuPage::Maps,
                _ => MenuPage::Main,
            };
            *page = to;
        }
        AppState::Debrief => {
            *page = MenuPage::Main;
            next.set(AppState::Menu);
        }
        AppState::Playing => {}
    }
}

fn buy(p: &mut Profile, tab: u8, i: usize, sfx: &mut Sfx) {
    let list = [SUITS, VISORS, HATS][tab.min(2) as usize];
    let Some(item) = list.get(i) else { return };
    let id = item.id.to_string();
    let (owned, cur) = match tab {
        0 => (&mut p.owned_suits, &mut p.suit),
        1 => (&mut p.owned_visors, &mut p.visor),
        _ => (&mut p.owned_hats, &mut p.hat),
    };
    if owned.contains(&id) {
        *cur = id;
    } else if p.stars >= item.price {
        // stars are a separate balance: spending them never relocks a map
        p.stars -= item.price;
        owned.push(id.clone());
        *cur = id;
        sfx.play("coin");
    } else {
        sfx.play("bad");
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_menu_actions(
    mut ev: EventReader<UiAction>,
    state: Res<State<AppState>>,
    mut page: ResMut<MenuPage>,
    mut cfg: ResMut<GameConfig>,
    mut profile: ResMut<Profile>,
    mut next: ResMut<NextState<AppState>>,
    mut refl: ResMut<Reflection>,
    mut exit: EventWriter<AppExit>,
    mut sfx: Sfx,
) {
    for UiAction(a) in ev.read() {
        match *a {
            Action::Page(p) => {
                *page = p;
                if *state.get() != AppState::Menu {
                    next.set(AppState::Menu);
                }
            }
            Action::Solo => {
                cfg.coop = false;
                *page = MenuPage::Maps;
            }
            Action::Coop => {
                cfg.coop = true;
                *page = MenuPage::Maps;
            }
            Action::PickMap(id) => {
                if profile.unlocked(map(id).need) {
                    cfg.map = id;
                    *page = MenuPage::Brief(id);
                } else {
                    sfx.play("bad");
                }
            }
            Action::ToggleLive => cfg.live = !cfg.live,
            Action::ToggleCadet => {
                profile.cadet = !profile.cadet;
                crate::save::save(&profile);
            }
            Action::Start | Action::Again => next.set(AppState::Playing),
            Action::Buy(tab, i) => {
                buy(&mut profile, tab, i, &mut sfx);
                crate::save::save(&profile);
            }
            Action::Reflect(q, c) => {
                if q == 0 {
                    refl.helped = Some(c);
                } else {
                    refl.where_ = Some(c);
                }
            }
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- menu screens

#[allow(clippy::too_many_arguments)]
fn build_menu(
    mut commands: Commands,
    roots: Query<Entity, With<ScreenRoot>>,
    page: Res<MenuPage>,
    profile: Res<Profile>,
    lang: Res<Lang>,
    weather: Res<Weather>,
    cfg: Res<GameConfig>,
    ui: Res<UiAssets>,
) {
    let changed = page.is_changed() || profile.is_changed() || lang.is_changed() || weather.is_changed() || cfg.is_changed();
    if !roots.is_empty() && !changed {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let ctx = Ctx { ui: &ui, lang: *lang };
    let wide = matches!(*page, MenuPage::Sources | MenuPage::Badges);
    commands
        .spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Row, ..default() }, ScreenRoot, StateScoped(AppState::Menu)))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(if wide { 100.0 } else { 52.0 }),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(30.0), Val::Px(24.0)),
                    row_gap: Val::Px(11.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.04, 0.10, 0.86)),
            ))
            .with_children(|p| match *page {
                MenuPage::Main => main_page(p, &ctx, &profile),
                MenuPage::Maps => maps_page(p, &ctx, &profile, &cfg),
                MenuPage::Brief(id) => brief_page(p, &ctx, id, &weather, &cfg),
                MenuPage::Shop(tab) => shop_page(p, &ctx, &profile, tab),
                MenuPage::Badges => badges_page(p, &ctx, &profile),
                MenuPage::Sources => sources_page(p, &ctx, &weather, &profile),
            });
            if !wide {
                root.spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexEnd,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|p| {
                    wallet(p, &ctx, &profile);
                    weather_panel(p, &ctx, &weather);
                });
            }
        });
}

fn wallet(p: &mut ChildSpawnerCommands, ctx: &Ctx, profile: &Profile) {
    let ctx = *ctx;
    ctx.panel(p, C_PANEL).with_children(|w| {
        ctx.row(w, 10.0).with_children(|r| {
            ctx.icon(r, "medal", 30.0);
            ctx.head(r, profile.total.to_string(), 24.0, C_TEXT);
            ctx.text(r, ctx.t("очков (открывают базы)", "points (unlock bases)"), 15.0, C_DIM);
        });
        ctx.row(w, 10.0).with_children(|r| {
            ctx.icon(r, "star", 30.0);
            ctx.head(r, profile.stars.to_string(), 24.0, C_GOLD);
            ctx.text(r, ctx.t("звёзд (на скины)", "stars (for skins)"), 15.0, C_DIM);
        });
    });
}

fn weather_panel(p: &mut ChildSpawnerCommands, ctx: &Ctx, w: &Weather) {
    let ctx = *ctx;
    ctx.panel(p, C_PANEL).with_children(|c| {
        ctx.row(c, 10.0).with_children(|r| {
            ctx.icon(r, "sun", 32.0);
            ctx.head(r, ctx.t("Космическая погода · NASA DONKI", "Space weather · NASA DONKI"), 17.0, C_TEXT);
        });
        if w.loading {
            ctx.text(c, ctx.t("Загружаем данные NASA…", "Loading NASA data…"), 15.0, C_DIM);
        } else if w.live {
            let (f, g, s, m) = w.counts;
            ctx.text(c, format!("LIVE · {}", w.range), 15.0, C_FOOD);
            ctx.text(c, format!("{} {f} · {} {g} · {} {s} · {} {m}", ctx.t("вспышки", "flares"), ctx.t("бури", "storms"), ctx.t("частицы", "particles"), ctx.t("выбросы", "CMEs")), 15.0, C_TEXT);
            if let Some(e) = w.events.first() {
                ctx.text(c, format!("{}: {} ({})", ctx.t("Последнее", "Latest"), ctx.t(&e.label_ru, &e.label_en), e.date), 14.0, C_DIM);
            }
        } else {
            ctx.text(c, ctx.t("АРХИВ: нет связи с NASA — используем сохранённые настоящие события", "ARCHIVE: no link to NASA — using saved real events"), 15.0, C_WARN);
        }
    });
}

fn back_btn(p: &mut ChildSpawnerCommands, ctx: &Ctx, to: MenuPage) {
    ctx.btn(p, B::new(Action::Page(to), Some("back"), ctx.t("Назад", "Back")).size(17.0).color(C_LOCK));
}

fn main_page(p: &mut ChildSpawnerCommands, ctx: &Ctx, profile: &Profile) {
    let ctx = *ctx;
    ctx.head(p, ctx.t("ЮНЫЙ АСТРОНАВТ", "JUNIOR ASTRONAUT"), 46.0, C_TEXT);
    ctx.head(p, ctx.t("Аванпост", "Outpost"), 28.0, C_ACCENT);
    ctx.text(p, ctx.t("Управляй базой на Луне и Марсе. Вспышки на Солнце — настоящие, из базы NASA.", "Run a base on the Moon and Mars. The solar flares are real, from NASA's database."), 17.0, C_DIM);
    p.spawn(Node { height: Val::Px(6.0), ..default() });
    let w = Val::Px(460.0);
    ctx.btn(p, B::new(Action::Solo, Some("rocket"), ctx.t("Играть", "Play")).size(26.0).color(C_GO).width(w));
    ctx.btn(p, B::new(Action::Coop, Some("coop"), ctx.t("Вдвоём на одном экране", "Two players, split screen")).width(w));
    ctx.btn(p, B::new(Action::Page(MenuPage::Shop(0)), Some("shop"), ctx.t("Магазин скинов", "Skin shop")).width(w));
    ctx.btn(p, B::new(Action::Page(MenuPage::Badges), Some("medal"), ctx.t("Нашивки", "Patches")).width(w));
    ctx.btn(p, B::new(Action::Page(MenuPage::Sources), Some("satellite"), ctx.t("NASA-данные и источники", "NASA data & sources")).width(w));
    ctx.row(p, 8.0).with_children(|r| {
        ctx.btn(r, B::new(Action::ToggleLang, Some("globe"), if ctx.en() { "English" } else { "Русский" }).size(16.0));
        ctx.btn(r, B::new(Action::ToggleCadet, Some("teacher"), if profile.cadet { ctx.t("Кадет 7–9", "Cadet 7–9") } else { ctx.t("Пилот 10–12", "Pilot 10–12") }).size(16.0));
        ctx.btn(r, B::new(Action::ToggleSound, Some(if profile.sound { "sound" } else { "mute" }), "").size(16.0));
        ctx.btn(r, B::new(Action::Quit, Some("exit"), "").size(16.0).color(C_LOCK));
    });
    let mode = if profile.cadet {
        ctx.t("Кадет: кислород, щит и наука; прогнозы всегда точные.", "Cadet: oxygen, shield and science; forecasts are always right.")
    } else {
        ctx.t("Пилот: ещё и еда; прогноз вспышек может ошибаться — как в жизни.", "Pilot: food too; flare forecasts can be wrong — just like in real life.")
    };
    ctx.text(p, mode, 15.0, C_DIM);
}

fn maps_page(p: &mut ChildSpawnerCommands, ctx: &Ctx, profile: &Profile, cfg: &GameConfig) {
    let ctx = *ctx;
    ctx.head(p, ctx.t("Выбери базу", "Choose a base"), 32.0, C_TEXT);
    ctx.line(p, if cfg.coop { "coop" } else { "astronaut" }, if cfg.coop { ctx.t("Режим: вдвоём", "Mode: two players") } else { ctx.t("Режим: один игрок", "Mode: one player") }, 16.0, C_DIM);
    for m in MAPS {
        let open = profile.unlocked(m.need);
        let best = profile.best.get(&format!("{:?}", m.id)).copied().unwrap_or(0);
        let name = ctx.t(m.name_ru, m.name_en).to_string();
        let info = format!(
            "{} {} · {} {} · {} ×{:.0}",
            m.sols,
            ctx.t("солов", "sols"),
            ctx.t("гравитация", "gravity"),
            if m.gravity <= 0.0 { "0".to_string() } else { format!("{:.2}g", m.gravity) },
            ctx.t("радиация", "radiation"),
            (m.dose / map(MapId::Mars).dose * 10.0).max(0.0).round() / 10.0
        );
        let total = profile.total;
        ctx.btn_custom(p, Action::PickMap(m.id), if open { C_BTN } else { C_LOCK }, Val::Px(560.0), true).with_children(|b| {
            ctx.icon_tint(b, m.icon, 50.0, if open { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.35) });
            b.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, row_gap: Val::Px(3.0), ..default() }).with_children(|c| {
                ctx.head(c, name, 20.0, if open { C_TEXT } else { C_DIM });
                ctx.text(c, info, 14.0, C_DIM);
                if !open {
                    ctx.bar(c, 220.0, 7.0, total as f32 / m.need as f32, C_GOLD);
                }
            });
            if open {
                ctx.row(b, 2.0).with_children(|r| {
                    for i in 0..3 {
                        ctx.icon_tint(r, "star", 24.0, if i < best { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.2) });
                    }
                });
            } else {
                ctx.row(b, 4.0).with_children(|r| {
                    ctx.icon(r, "lock", 24.0);
                    ctx.head(r, format!("{}/{}", total.min(m.need), m.need), 16.0, C_TEXT);
                    ctx.icon(r, "medal", 22.0);
                });
            }
        });
    }
    back_btn(p, &ctx, MenuPage::Main);
}

fn brief_page(p: &mut ChildSpawnerCommands, ctx: &Ctx, id: MapId, weather: &Weather, cfg: &GameConfig) {
    let ctx = *ctx;
    let m = map(id);
    ctx.row(p, 12.0).with_children(|r| {
        ctx.icon(r, m.icon, 54.0);
        ctx.head(r, ctx.t(m.name_ru, m.name_en), 30.0, C_TEXT);
    });
    ctx.panel(p, C_PANEL).with_children(|c| {
        ctx.line(c, "book", ctx.t(m.fact_ru, m.fact_en), 16.0, C_TEXT);
        ctx.line(c, "radiation", format!("{} {}", ctx.t("Радиация:", "Radiation:"), ctx.t(m.dose_real_ru, m.dose_real_en)), 16.0, C_TEXT);
        ctx.line(c, "radio", format!("{} {}", ctx.t("Связь с Землёй:", "Signal to Earth:"), ctx.t(m.delay_real_ru, m.delay_real_en)), 16.0, C_TEXT);
        let g = if m.gravity <= 0.0 {
            ctx.t("Невесомость — на палубе станции ходим в магнитных ботинках", "Weightless — magnetic boots on the station deck").to_string()
        } else {
            format!("{} {:.2} g", ctx.t("Гравитация:", "Gravity:"), m.gravity)
        };
        ctx.line(c, "walk", g, 16.0, C_TEXT);
        ctx.line(c, "hourglass", format!("{} {} {}", ctx.t("Миссия:", "Mission:"), m.sols, ctx.t("солов, каждый сол — 100 секунд", "sols, each sol is 100 seconds")), 16.0, C_TEXT);
    });
    ctx.head(p, ctx.t("План «если — то»", "If–then plan"), 21.0, C_ACCENT);
    ctx.panel(p, C_PANEL).with_children(|c| {
        ctx.line(c, "sun", ctx.t("Если вспышка — то бегу в купол и жду внутри.", "If there is a flare — then I run into the dome and wait."), 16.0, C_TEXT);
        ctx.line(c, "wrench", ctx.t("Если поломка — то подхожу и держу F.", "If something breaks — then I walk up and hold F."), 16.0, C_TEXT);
        ctx.line(c, "oxygen", ctx.t("Если кислород падает — то даю больше энергии на кислород.", "If oxygen drops — then I give more power to oxygen."), 16.0, C_TEXT);
    });
    if id != MapId::Earth {
        if weather.loading {
            ctx.line(p, "hourglass", ctx.t("Загружаем погоду NASA…", "Loading NASA weather…"), 16.0, C_DIM);
        } else if weather.live {
            let on = cfg.live;
            ctx.btn(
                p,
                B::new(Action::ToggleLive, Some("sun"), if on { ctx.t("Настоящая погода NASA: ВКЛ (LIVE)", "Real NASA weather: ON (LIVE)") } else { ctx.t("Настоящая погода NASA: ВЫКЛ", "Real NASA weather: OFF") })
                    .size(17.0)
                    .color(if on { C_GO } else { C_LOCK }),
            );
        } else {
            ctx.line(p, "antenna_bars", ctx.t("Нет связи с NASA — вспышки будут из АРХИВА настоящих событий", "No link to NASA — flares come from the ARCHIVE of real events"), 16.0, C_WARN);
        }
    }
    if cfg.coop {
        ctx.text(p, ctx.t("Игрок 1: WASD, F, пробел · Игрок 2: стрелки, правый Shift, правый Ctrl · или два геймпада", "Player 1: WASD, F, Space · Player 2: arrows, Right Shift, Right Ctrl · or two gamepads"), 15.0, C_DIM);
    }
    ctx.row(p, 12.0).with_children(|r| {
        ctx.btn(r, B::new(Action::Start, Some("rocket"), ctx.t("Старт!", "Launch!")).size(26.0).color(C_GO));
        back_btn(r, &ctx, MenuPage::Maps);
    });
}

fn shop_page(p: &mut ChildSpawnerCommands, ctx: &Ctx, profile: &Profile, tab: u8) {
    let ctx = *ctx;
    ctx.head(p, ctx.t("Магазин скинов", "Skin shop"), 32.0, C_TEXT);
    ctx.text(p, ctx.t("Скины покупаются за звёзды. Базы открываются очками — покупки их не закрывают.", "Skins cost stars. Bases unlock with points — buying never locks them again."), 15.0, C_DIM);
    ctx.row(p, 8.0).with_children(|r| {
        for (i, icon, ru, en) in [(0u8, "shirt", "Скафандр", "Suit"), (1, "goggles", "Визор", "Visor"), (2, "crown", "Шлем", "Helmet")] {
            ctx.btn(r, B::new(Action::Page(MenuPage::Shop(i)), Some(icon), ctx.t(ru, en)).size(17.0).color(if i == tab { C_ACCENT.mix(&C_BTN, 0.5) } else { C_BTN }));
        }
    });
    let (list, cur, owned): (&[Skin], &str, &Vec<String>) = match tab {
        0 => (SUITS, &profile.suit, &profile.owned_suits),
        1 => (VISORS, &profile.visor, &profile.owned_visors),
        _ => (HATS, &profile.hat, &profile.owned_hats),
    };
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|g| {
        for (i, s) in list.iter().enumerate() {
            let on = s.id == cur;
            let have = owned.iter().any(|o| o == s.id);
            let afford = profile.stars >= s.price;
            let name = ctx.t(s.name_ru, s.name_en).to_string();
            ctx.btn_custom(g, Action::Buy(tab, i), if on { C_GO } else if have { C_BTN } else { C_LOCK }, Val::Px(196.0), true).with_children(|b| {
                if tab == 2 {
                    ctx.icon(b, s.icon, 34.0);
                } else {
                    b.spawn((Node { width: Val::Px(30.0), height: Val::Px(30.0), ..default() }, BackgroundColor(s.color), BorderRadius::MAX));
                }
                b.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, ..default() }).with_children(|c| {
                    ctx.text(c, name, 16.0, C_TEXT);
                    if on {
                        ctx.text(c, ctx.t("Надето", "Wearing"), 14.0, C_FOOD);
                    } else if have {
                        ctx.text(c, ctx.t("Есть — надеть", "Owned — wear"), 14.0, C_DIM);
                    } else {
                        ctx.row(c, 4.0).with_children(|r| {
                            ctx.icon(r, "star", 18.0);
                            ctx.text(r, s.price.to_string(), 15.0, if afford { C_GOLD } else { C_DANGER });
                        });
                    }
                });
            });
        }
    });
    back_btn(p, &ctx, MenuPage::Main);
}

fn badges_page(p: &mut ChildSpawnerCommands, ctx: &Ctx, profile: &Profile) {
    let ctx = *ctx;
    let have = BADGES.iter().filter(|b| profile.badges.iter().any(|x| x == b.id)).count();
    ctx.head(p, format!("{} {have}/{}", ctx.t("Нашивки", "Patches"), BADGES.len()), 32.0, C_TEXT);
    ctx.text(p, ctx.t("Нашивки дают за поступки: укрыться от вспышки, поддержать экипаж, попробовать снова.", "Patches are for actions: sheltering from a flare, supporting the crew, trying again."), 16.0, C_DIM);
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(10.0), ..default() }).with_children(|g| {
        for b in BADGES {
            let got = profile.badges.iter().any(|x| x == b.id);
            g.spawn((
                Node { width: Val::Px(176.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(10.0)), row_gap: Val::Px(6.0), ..default() },
                BackgroundColor(if got { Color::srgba(0.30, 0.20, 0.50, 0.9) } else { C_PANEL }),
                BorderRadius::all(Val::Px(16.0)),
            ))
            .with_children(|c| {
                ctx.icon_tint(c, b.icon, 52.0, if got { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.22) });
                ctx.text(c, ctx.t(b.name_ru, b.name_en), 15.0, if got { C_TEXT } else { C_DIM });
            });
        }
    });
    back_btn(p, &ctx, MenuPage::Main);
}

fn sources_page(p: &mut ChildSpawnerCommands, ctx: &Ctx, weather: &Weather, profile: &Profile) {
    let ctx = *ctx;
    ctx.head(p, ctx.t("NASA-данные и источники", "NASA data & sources"), 30.0, C_TEXT);
    ctx.row(p, 14.0).with_children(|r| {
        ctx.panel(r, C_PANEL).with_children(|c| {
            ctx.head(c, ctx.t("Честные правила", "Honesty rules"), 18.0, C_ACCENT);
            ctx.text(c, ctx.t("• Числа упрощены, но соотношения настоящие: на Луне доза выше, чем на Марсе.", "• Numbers are simplified, but the ratios are real: the Moon's dose is higher than Mars'."), 15.0, C_TEXT);
            ctx.text(c, ctx.t("• Архив никогда не выдаётся за живые данные: всегда видно LIVE или АРХИВ.", "• Archived data is never shown as live: you always see LIVE or ARCHIVE."), 15.0, C_TEXT);
            ctx.text(c, ctx.t("• Экипаж не погибает: при неудаче — эвакуация и разбор.", "• The crew never dies: a failure means evacuation and a debrief."), 15.0, C_TEXT);
            ctx.text(c, ctx.t("• Прогресс хранится только на этом компьютере.", "• Progress is stored only on this computer."), 15.0, C_TEXT);
        });
        ctx.panel(r, C_PANEL).with_children(|c| {
            ctx.head(c, "NASA DONKI", 18.0, C_ACCENT);
            let status = if weather.loading {
                ctx.t("загружаем…", "loading…").to_string()
            } else if weather.live {
                let (f, g, s, m) = weather.counts;
                format!("LIVE {} · FLR {f} · GST {g} · SEP {s} · CME {m} · Kp max {:.0}", weather.range, weather.kp_max)
            } else {
                ctx.t("АРХИВ: 7 настоящих событий 2003–2024", "ARCHIVE: 7 real events, 2003–2024").to_string()
            };
            ctx.text(c, status, 15.0, if weather.live { C_FOOD } else { C_TEXT });
            let key = crate::nasa::api_key(profile);
            let key_txt = if key == "DEMO_KEY" {
                ctx.t("Ключ: DEMO_KEY (~30 запросов в час). Свой бесплатный ключ: api.nasa.gov → переменная NASA_API_KEY.", "Key: DEMO_KEY (~30 requests/hour). Free own key: api.nasa.gov → NASA_API_KEY env var.")
            } else {
                ctx.t("Ключ: свой (NASA_API_KEY).", "Key: your own (NASA_API_KEY).")
            };
            ctx.text(c, key_txt, 14.0, C_DIM);
        });
    });
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(6.0), ..default() }).with_children(|g| {
        for s in SOURCES {
            g.spawn((Node { width: Val::Percent(48.0), flex_direction: FlexDirection::Column, padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)), ..default() }, BackgroundColor(C_PANEL), BorderRadius::all(Val::Px(10.0))))
                .with_children(|c| {
                    ctx.text(c, s.title, 15.0, C_TEXT);
                    ctx.text(c, s.url, 13.0, C_ACCENT);
                });
        }
    });
    back_btn(p, &ctx, MenuPage::Main);
}

// ---------------------------------------------------------------- debrief

#[allow(clippy::too_many_arguments)]
fn build_debrief(
    mut commands: Commands,
    roots: Query<Entity, With<ScreenRoot>>,
    res: Option<Res<MissionResult>>,
    refl: Res<Reflection>,
    lang: Res<Lang>,
    ui: Res<UiAssets>,
    profile: Res<Profile>,
) {
    let Some(r) = res else { return };
    if !roots.is_empty() && !(refl.is_changed() || lang.is_changed() || profile.is_changed()) {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let ctx = Ctx { ui: &ui, lang: *lang };
    let col = |w: f32| Node {
        width: Val::Percent(w),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::axes(Val::Px(26.0), Val::Px(22.0)),
        row_gap: Val::Px(10.0),
        ..default()
    };
    commands
        .spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Row, ..default() }, ScreenRoot, StateScoped(AppState::Debrief)))
        .with_children(|root| {
            root.spawn((col(54.0), BackgroundColor(Color::srgba(0.02, 0.04, 0.10, 0.88)))).with_children(|p| results(p, &ctx, &r));
            root.spawn((col(46.0), BackgroundColor(Color::srgba(0.03, 0.06, 0.14, 0.92)))).with_children(|p| reflection(p, &ctx, &refl));
        });
}

fn results(p: &mut ChildSpawnerCommands, ctx: &Ctx, r: &MissionResult) {
    let ctx = *ctx;
    let m = map(r.map);
    ctx.row(p, 12.0).with_children(|row| {
        ctx.icon(row, if r.won { "trophy" } else { "rocket" }, 60.0);
        ctx.head(row, if r.won { ctx.t("Миссия выполнена!", "Mission complete!") } else { ctx.t("Экипаж эвакуирован", "Crew evacuated") }, 34.0, if r.won { C_GOLD } else { C_TEXT });
    });
    if r.won {
        ctx.text(p, format!("{}: {} {}", ctx.t(m.name_ru, m.name_en), r.sols, ctx.t("солов пройдено", "sols completed")), 17.0, C_DIM);
    } else {
        ctx.text(p, format!("{} {} / {}. {}", ctx.t("Сол", "Sol"), r.sol, r.sols, ctx.t("Все целы и уже летят домой. Разберём, что случилось.", "Everyone is safe and flying home. Let's see what happened.")), 17.0, C_DIM);
        let (icon, ru, en) = match r.cause {
            Some(Cause::Oxygen) => ("oxygen", "Кончился кислород. Если — то: давай больше блоков на кислород и сразу чини генератор.", "Oxygen ran out. If–then: give more blocks to oxygen and fix the generator right away."),
            Some(Cause::Food) => ("food", "Кончилась еда. Оранжерея и грузовой корабль помогают продержаться.", "Food ran out. The greenhouse and the cargo ship help you last longer."),
            Some(Cause::Dose) => ("radiation", "Слишком большая доза радиации. Щит и убежище во время вспышки сильно её снижают.", "The radiation dose got too high. The shield and the shelter during a flare cut it a lot."),
            None => ("info", "", ""),
        };
        ctx.line(p, icon, ctx.t(ru, en), 17.0, C_WARN);
    }
    // stars: each one is for an action, not for "being good"
    ctx.row(p, 16.0).with_children(|row| {
        let stars = [
            (r.won, ctx.t("Миссия", "Mission").to_string()),
            (r.won && r.science >= r.goal, format!("{} {:.1}/{:.0}", ctx.t("Наука", "Science"), r.science, r.goal)),
            (r.won && r.dose < 50.0, format!("{} {:.0}% < 50%", ctx.t("Доза", "Dose"), r.dose)),
        ];
        for (got, label) in stars {
            row.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(2.0), ..default() }).with_children(|c| {
                ctx.icon_tint(c, "star", 44.0, if got { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.2) });
                ctx.text(c, label, 14.0, if got { C_TEXT } else { C_DIM });
            });
        }
    });
    ctx.line(p, "medal", format!("+{} {}", r.points, ctx.t("очков и звёзд", "points and stars")), 19.0, C_GOLD);
    if let Some(id) = r.new_map {
        let nm = map(id);
        ctx.panel(p, Color::srgba(0.10, 0.45, 0.25, 0.92)).with_children(|c| {
            ctx.line(c, nm.icon, format!("{} {}!", ctx.t("Открыта новая база:", "New base unlocked:"), ctx.t(nm.name_ru, nm.name_en)), 18.0, C_TEXT);
        });
    }
    if !r.badges.is_empty() {
        ctx.row(p, 10.0).with_children(|row| {
            for id in &r.badges {
                if let Some(b) = BADGES.iter().find(|b| b.id == *id) {
                    ctx.row(row, 4.0).with_children(|x| {
                        ctx.icon(x, b.icon, 30.0);
                        ctx.text(x, ctx.t(b.name_ru, b.name_en), 14.0, C_TEXT);
                    });
                }
            }
        });
    }
    // what went well — describe actions
    if r.sheltered {
        ctx.line(p, "house", ctx.t("Укрытие во время вспышки — сделано.", "Took shelter during a flare — done."), 16.0, C_FOOD);
    }
    if r.helped {
        ctx.line(p, "heart", ctx.t("Поддержка экипажа — сделано.", "Supported the crew — done."), 16.0, C_FOOD);
    }
    if r.won && r.science < r.goal {
        ctx.line(p, "science", ctx.t("Для науки: чаще давай энергию лаборатории и носи туда образцы.", "For science: power the lab more often and carry samples there."), 16.0, C_DIM);
    }
    // per-sol chart: oxygen vs dose
    if !r.history.is_empty() {
        ctx.text(p, ctx.t("Кислород (голубой) и доза (красный) по солам", "Oxygen (blue) and dose (red) per sol"), 15.0, C_DIM);
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexEnd, column_gap: Val::Px(12.0), height: Val::Px(110.0), ..default() }).with_children(|chart| {
            for h in &r.history {
                chart.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0), ..default() }).with_children(|c| {
                    c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexEnd, column_gap: Val::Px(3.0), height: Val::Px(86.0), ..default() }).with_children(|bars| {
                        bars.spawn((Node { width: Val::Px(14.0), height: Val::Px(h.o2.clamp(1.0, 100.0) * 0.86), ..default() }, BackgroundColor(C_O2), BorderRadius::top(Val::Px(4.0))));
                        bars.spawn((Node { width: Val::Px(14.0), height: Val::Px(h.dose.clamp(1.0, 100.0) * 0.86), ..default() }, BackgroundColor(C_DOSE), BorderRadius::top(Val::Px(4.0))));
                    });
                    ctx.text(c, h.sol.to_string(), 13.0, C_DIM);
                });
            }
        });
    }
}

fn reflection(p: &mut ChildSpawnerCommands, ctx: &Ctx, refl: &Reflection) {
    let ctx = *ctx;
    ctx.head(p, ctx.t("Разбор миссии", "Mission debrief"), 28.0, C_TEXT);
    ctx.text(p, ctx.t("Здесь нет правильных ответов и нет очков.", "There are no right answers and no points here."), 15.0, C_DIM);
    let q = |p: &mut ChildSpawnerCommands, qi: u8, title: &str, opts: &[(&'static str, &str, &str)], sel: Option<u8>| {
        ctx.head(p, title.to_string(), 19.0, C_ACCENT);
        p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|g| {
            for (i, (icon, ru, en)) in opts.iter().enumerate() {
                let on = sel == Some(i as u8);
                ctx.btn(g, B::new(Action::Reflect(qi, i as u8), Some(icon), ctx.t(ru, en)).size(16.0).color(if on { C_GO } else { C_BTN }));
            }
        });
    };
    q(
        p,
        0,
        ctx.t("Что помогло больше всего?", "What helped the most?"),
        &[
            ("shield", "Щит", "Shield"),
            ("house", "Убежище", "Shelter"),
            ("clipboard", "План «если — то»", "If–then plan"),
            ("heart", "Команда", "Teamwork"),
            ("beacon", "Пауза", "Pausing"),
            ("power", "Энергия", "Power"),
        ],
        refl.helped,
    );
    q(
        p,
        1,
        ctx.t("Где ещё это пригодится?", "Where else is this useful?"),
        &[("book", "В школе", "At school"), ("home", "Дома", "At home"), ("run", "В спорте", "In sports"), ("coop", "С друзьями", "With friends")],
        refl.where_,
    );
    if refl.helped.is_some() && refl.where_.is_some() {
        ctx.line(p, "sparkles", ctx.t("Отличный разбор! Так делают настоящие экипажи после каждой миссии.", "Great debrief! Real crews do this after every mission."), 17.0, C_FOOD);
    }
    p.spawn(Node { flex_grow: 1.0, ..default() });
    ctx.row(p, 10.0).with_children(|r| {
        ctx.btn(r, B::new(Action::Again, Some("retry"), ctx.t("Ещё раз", "Again")).color(C_GO));
        ctx.btn(r, B::new(Action::Page(MenuPage::Maps), Some("map"), ctx.t("К базам", "Bases")));
        ctx.btn(r, B::new(Action::Page(MenuPage::Main), Some("home"), ctx.t("Меню", "Menu")).color(C_LOCK));
    });
}
