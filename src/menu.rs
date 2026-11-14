//! Menus: main menu with a live 3D preview of the astronaut, map select, briefing,
//! skin shop, patches, NASA data & sources, and the mission debrief.

use crate::data::*;
use crate::game::MissionResult;
use crate::i18n::Lang;
use crate::ui::*;
use crate::world::Spin;
use crate::{AppState, GameConfig};
use bevy::prelude::*;

#[derive(Component)]
pub struct MenuRoot;
#[derive(Component)]
pub struct PreviewScene;
#[derive(Component)]
pub struct PreviewAstro;
#[derive(Component)]
pub struct PreviewCam;

#[derive(Resource, Default)]
pub struct MenuTrack {
    pub state: Option<AppState>,
    pub refresh: u32,
    pub skin: String,
}

pub fn ensure_preview(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, existing: Query<Entity, With<PreviewScene>>, profile: Res<crate::save::Profile>, mut track: ResMut<MenuTrack>) {
    let skin = format!("{}/{}/{}", profile.suit, profile.visor, profile.hat);
    if existing.is_empty() {
        commands.spawn((
            Camera3d::default(),
            Camera { order: 0, clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.07)), ..default() },
            Transform::from_xyz(0.0, 2.0, 5.5).looking_at(Vec3::new(0.0, 1.1, 0.0), Vec3::Y),
            DistanceFog { color: Color::srgb(0.02, 0.02, 0.07), falloff: FogFalloff::Linear { start: 30.0, end: 90.0 }, ..default() },
            PreviewScene,
            PreviewCam,
        ));
        commands.spawn((DirectionalLight { illuminance: 9000.0, shadows_enabled: true, ..default() }, Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y), PreviewScene));
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(60.0, 0.4))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.28, 0.16), perceptual_roughness: 0.95, ..default() })),
            Transform::from_xyz(0.0, -0.2, 0.0),
            PreviewScene,
        ));
        // a few rocks and a planet in the sky
        let rock = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
        let rock_m = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.2, 0.12), ..default() });
        for (x, z, s) in [(-4.0, -3.0, 1.2), (5.0, -5.0, 1.8), (3.5, -1.5, 0.6), (-6.0, 2.0, 0.9)] {
            commands.spawn((Mesh3d(rock.clone()), MeshMaterial3d(rock_m.clone()), Transform::from_xyz(x, s * 0.3, z).with_scale(Vec3::new(s, s * 0.6, s)), PreviewScene));
        }
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(9.0).mesh().uv(32, 18))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.55, 1.0), emissive: LinearRgba::rgb(0.1, 0.25, 0.7), ..default() })),
            Transform::from_xyz(-20.0, 16.0, -45.0),
            PreviewScene,
        ));
        let star_mesh = meshes.add(Sphere::new(0.25).mesh().ico(1).unwrap());
        let star_mat = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::rgb(4.0, 4.0, 4.5), unlit: true, ..default() });
        let mut rng = crate::sim::Rng(0xA5A5_1234);
        for _ in 0..160 {
            let th = rng.range(0.0, std::f32::consts::TAU);
            let ph = rng.range(0.05, 1.4);
            let r = 80.0;
            commands.spawn((Mesh3d(star_mesh.clone()), MeshMaterial3d(star_mat.clone()), Transform::from_translation(Vec3::new(r * ph.sin() * th.cos(), r * ph.cos(), r * ph.sin() * th.sin() - 20.0)), PreviewScene));
        }
        commands.insert_resource(AmbientLight { color: Color::srgb(0.7, 0.75, 1.0), brightness: 250.0, ..default() });
    }
    if track.skin != skin || existing.is_empty() {
        track.skin = skin;
        let suit = SUITS.iter().find(|s| s.id == profile.suit).map(|s| s.color).unwrap_or(Color::WHITE);
        let visor = VISORS.iter().find(|s| s.id == profile.visor).map(|s| s.color).unwrap_or(Color::WHITE);
        let a = crate::world::spawn_astronaut(&mut commands, &mut meshes, &mut materials, suit, visor, &profile.hat);
        commands.entity(a).insert((Transform::from_xyz(1.6, 0.0, 0.0).with_rotation(Quat::from_rotation_y(0.3)), Spin(0.6), PreviewScene, PreviewAstro));
    }
}

pub fn clear_old_astro(mut commands: Commands, q: Query<Entity, With<PreviewAstro>>) {
    // keep only the newest preview astronaut
    let all: Vec<Entity> = q.iter().collect();
    if all.len() > 1 {
        for e in &all[..all.len() - 1] {
            commands.entity(*e).despawn();
        }
    }
}

pub fn despawn_preview(mut commands: Commands, q: Query<Entity, With<PreviewScene>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

pub fn menu_render(
    mut commands: Commands,
    state: Res<State<AppState>>,
    cfg: Res<GameConfig>,
    mut track: ResMut<MenuTrack>,
    old: Query<Entity, With<MenuRoot>>,
    icons: Res<Icons>,
    fonts: Res<Fonts>,
    profile: Res<crate::save::Profile>,
    weather: Res<crate::nasa::Weather>,
    result: Option<Res<MissionResult>>,
    mut focus: ResMut<Focus>,
) {
    let st = *state.get();
    if st == AppState::Playing {
        return;
    }
    let changed = track.state != Some(st) || track.refresh != cfg.refresh || weather.is_changed();
    if !changed {
        return;
    }
    track.state = Some(st);
    track.refresh = cfg.refresh;
    for e in &old {
        commands.entity(e).despawn();
    }
    focus.idx = 0;
    focus.dirty = true;
    let l = lang(&profile);
    let root = commands
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, ..default() }, MenuRoot, GlobalZIndex(10)))
        .id();
    commands.entity(root).with_children(|p| {
        topbar(p, &icons, &fonts, &profile, l, st);
        match st {
            AppState::Menu => main_menu(p, &icons, &fonts, &profile, l, &weather),
            AppState::Maps => maps(p, &icons, &fonts, &profile, l, &cfg),
            AppState::Briefing => briefing(p, &icons, &fonts, &profile, l, &cfg, &weather),
            AppState::Shop => shop(p, &icons, &fonts, &profile, l, &cfg),
            AppState::Badges => badges(p, &icons, &fonts, &profile, l),
            AppState::Info => info(p, &icons, &fonts, l, &weather),
            AppState::Debrief => {
                if let Some(r) = result.as_deref() {
                    debrief(p, &icons, &fonts, l, r, &cfg);
                }
            }
            AppState::Playing => {}
        }
    });
}

fn topbar(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang, st: AppState) {
    p.spawn((Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)), ..default() }, BackgroundColor(Color::srgba(0.03, 0.04, 0.12, 0.8))))
        .with_children(|r| {
            r.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|b| {
                if st != AppState::Menu && st != AppState::Debrief {
                    let back = match st { AppState::Briefing => Action::GoMaps(false), _ => Action::GoMenu };
                    let back = if st == AppState::Briefing { Action::GoMaps(false) } else { back };
                    b.spawn((Button, UiAction(back), FocusOrder(800), Node { width: Val::Px(48.0), height: Val::Px(48.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(C_PANEL2), BorderColor(C_LINE), BorderRadius::all(Val::Px(14.0))))
                        .with_children(|x| {
                            icon(x, icons, "back", 28.0);
                        });
                }
                icon(b, icons, "rocket", 34.0);
                text(b, l.t("ЮНЫЙ АСТРОНАВТ: АВАНПОСТ", "JUNIOR ASTRONAUT: OUTPOST"), &fonts.head, 24.0, C_TEXT);
            });
            r.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|b| {
                pill(b, icons, fonts, "medal", &format!("{}", profile.total), C_TEXT);
                pill(b, icons, fonts, "star", &format!("{}", profile.stars), Color::srgb(1.0, 0.88, 0.5));
                if !profile.name_placeholder() {
                    pill(b, icons, fonts, if profile.cadet { "seedling" } else { "rocket" }, if profile.cadet { l.t("Кадет 7–9", "Cadet 7–9") } else { l.t("Пилот 10–12", "Pilot 10–12") }, C_MUTED);
                }
            });
        });
}

fn pill(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, ic: &str, label: &str, color: Color) {
    p.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)), border: UiRect::all(Val::Px(1.0)), ..default() }, BackgroundColor(C_PANEL2), BorderColor(C_LINE), BorderRadius::all(Val::Px(999.0))))
        .with_children(|r| {
            icon(r, icons, ic, 24.0);
            text(r, label, &fonts.head, 20.0, color);
        });
}

fn main_menu(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang, weather: &crate::nasa::Weather) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(28.0)), column_gap: Val::Px(24.0), ..default() }).with_children(|row| {
        row.spawn(Node { width: Val::Px(560.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(16.0), ..default() }).with_children(|col| {
            col.spawn((Text::new(l.t("Управляй базой на Луне и Марсе", "Run a base on the Moon and Mars")), TextFont { font: fonts.head.clone(), font_size: 40.0, ..default() }, TextColor(C_TEXT)));
            col.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), align_items: AlignItems::Center, ..default() }).with_children(|r| {
                for ic in ["oxygen", "food", "shield", "science"] {
                    icon(r, icons, ic, 36.0);
                }
                text(r, l.t("— энергии на всё не хватит", "— there isn’t power for everything"), &fonts.body, 20.0, C_MUTED);
            });
            col.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
                tile_button(r, icons, fonts, Action::GoMaps(false), 0, "astronaut", l.t("Играть", "Play"), 180.0, Color::srgba(0.95, 0.45, 0.2, 0.95));
                tile_button(r, icons, fonts, Action::GoMaps(true), 1, "coop", l.t("Вдвоём", "2 players"), 180.0, Color::srgba(0.25, 0.5, 0.95, 0.95));
                tile_button(r, icons, fonts, Action::GoShop(0), 2, "shop", l.t("Магазин", "Shop"), 160.0, C_PANEL2);
            });
            col.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
                tile_button(r, icons, fonts, Action::GoBadges, 3, "medal", l.t("Нашивки", "Patches"), 160.0, C_PANEL2);
                tile_button(r, icons, fonts, Action::GoInfo, 4, "book", l.t("Данные NASA", "NASA data"), 180.0, C_PANEL2);
                tile_button(r, icons, fonts, Action::ToggleAge, 5, if profile.cadet { "seedling" } else { "rocket" }, if profile.cadet { l.t("Кадет 7–9", "Cadet 7–9") } else { l.t("Пилот 10–12", "Pilot 10–12") }, 180.0, C_PANEL2);
            });
            col.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                icon_button(r, icons, fonts, Action::ToggleLang, 6, "globe", if profile.lang_en { "EN" } else { "RU" }, false);
                icon_button(r, icons, fonts, Action::ToggleSound, 7, if profile.sound { "sound" } else { "mute" }, "", false);
                icon_button(r, icons, fonts, Action::Quit, 8, "exit", "", false);
            });
            weather_card(col, icons, fonts, l, weather);
        });
    });
}

fn weather_card(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, l: Lang, w: &crate::nasa::Weather) {
    p.spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(2.0)), max_width: Val::Px(560.0), ..default() }, BackgroundColor(C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(18.0))))
        .with_children(|c| {
            c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                icon(r, icons, "sun", 30.0);
                text(r, l.t("Космическая погода", "Space weather"), &fonts.head, 20.0, C_TEXT);
                let (tag, col) = if w.loading { (l.t("загрузка…", "loading…"), C_MUTED) } else if w.live { ("LIVE · NASA DONKI", C_GOOD) } else { (l.t("АРХИВ · NASA DONKI", "ARCHIVE · NASA DONKI"), C_WARN) };
                r.spawn((Node { padding: UiRect::axes(Val::Px(10.0), Val::Px(3.0)), ..default() }, BackgroundColor(col.with_alpha(0.2)), BorderRadius::all(Val::Px(999.0)))).with_children(|t| {
                    text(t, tag, &fonts.head, 14.0, col);
                });
            });
            if w.live {
                c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(14.0), align_items: AlignItems::Center, ..default() }).with_children(|r| {
                    for (ic, n) in [("sun", w.counts.0), ("flash", w.counts.3), ("planet", w.counts.1)] {
                        icon(r, icons, ic, 22.0);
                        text(r, format!("{n}"), &fonts.head, 20.0, C_TEXT);
                    }
                    text(r, l.t("за 30 дней", "last 30 days"), &fonts.body, 16.0, C_MUTED);
                    let risk = w.risk();
                    let (rt, rc) = match risk { 3 => (l.t("радиация: высокая", "radiation: high"), C_BAD), 2 => (l.t("радиация: средняя", "radiation: medium"), C_WARN), _ => (l.t("радиация: спокойно", "radiation: calm"), C_GOOD) };
                    text(r, rt, &fonts.body, 16.0, rc);
                });
            }
            for e in w.events.iter().take(3) {
                c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() }).with_children(|r| {
                    let col = match e.kind.as_str() { "FLR" => C_ACCENT, "GST" => Color::srgb(0.65, 0.5, 1.0), "SEP" => C_BAD, _ => C_BLUE };
                    r.spawn((Node { padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)), ..default() }, BackgroundColor(col), BorderRadius::all(Val::Px(6.0)))).with_children(|t| {
                        text(t, &e.kind, &fonts.head, 12.0, Color::BLACK);
                    });
                    r.spawn((Text::new(format!("{} · {}", l.t(&e.label_ru, &e.label_en), e.date)), TextFont { font: fonts.body.clone(), font_size: 15.0, ..default() }, TextColor(C_MUTED), Node { max_width: Val::Px(440.0), ..default() }));
                });
            }
            text(c, l.t("Вспышка в миссии — это настоящая вспышка на Солнце.", "A flare in your mission is a real flare from the Sun."), &fonts.body, 14.0, C_MUTED);
        });
}

fn maps(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang, cfg: &GameConfig) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(28.0)), row_gap: Val::Px(16.0), ..default() }).with_children(|col| {
        col.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
            icon(r, icons, if cfg.coop { "coop" } else { "astronaut" }, 40.0);
            text(r, if cfg.coop { l.t("Вдвоём: выбери базу", "2 players: choose a base") } else { l.t("Выбери базу", "Choose a base") }, &fonts.head, 32.0, C_TEXT);
            if cfg.coop {
                text(r, l.t("один экран, игрок 2: стрелки + правый Shift", "one screen, player 2: arrows + right Shift"), &fonts.body, 18.0, C_MUTED);
            }
        });
        col.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(16.0), row_gap: Val::Px(16.0), ..default() }).with_children(|grid| {
            for (i, m) in MAPS.iter().enumerate() {
                let open = profile.unlocked(m.need);
                let best = profile.best.get(&format!("{:?}", m.id)).copied().unwrap_or(0);
                let mut e = grid.spawn((
                    Node { width: Val::Px(228.0), height: Val::Px(230.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() },
                    BackgroundColor(if open { C_PANEL2 } else { Color::srgba(0.1, 0.1, 0.16, 0.9) }),
                    BorderColor(C_LINE),
                    BorderRadius::all(Val::Px(24.0)),
                ));
                if open {
                    e.insert((Button, UiAction(Action::PickMap(m.id)), FocusOrder(i as u16)));
                }
                e.with_children(|c| {
                    icon(c, icons, if open { m.icon } else { "lock" }, 72.0);
                    c.spawn((Text::new(l.t(m.name_ru, m.name_en)), TextFont { font: fonts.body.clone(), font_size: 20.0, ..default() }, TextColor(if open { C_TEXT } else { C_MUTED }), TextLayout::new_with_justify(JustifyText::Center)));
                    if open {
                        c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), ..default() }).with_children(|s| {
                            for k in 0..3 {
                                s.spawn((ImageNode { image: icons.get("star"), color: if k < best { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.2) }, ..default() }, Node { width: Val::Px(28.0), height: Val::Px(28.0), ..default() }));
                            }
                        });
                    } else {
                        c.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), width: Val::Percent(100.0), ..default() }).with_children(|s| {
                            s.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                                icon(r, icons, "medal", 22.0);
                                text(r, format!("{} / {}", profile.total, m.need), &fonts.head, 18.0, C_MUTED);
                            });
                            s.spawn((Node { width: Val::Percent(100.0), height: Val::Px(8.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1)), BorderRadius::all(Val::Px(4.0)))).with_children(|b| {
                                b.spawn((Node { width: Val::Percent((profile.total as f32 / m.need as f32 * 100.0).min(100.0)), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_BLUE), BorderRadius::all(Val::Px(4.0))));
                            });
                        });
                    }
                });
            }
        });
        text(col, l.t("Очки 🏅 за миссии открывают новые базы. Звёзды ⭐ тратятся в магазине — базы не закроются.", "Points 🏅 from missions unlock bases. Stars ⭐ are spent in the shop — bases never relock."), &fonts.body, 18.0, C_MUTED);
    });
}

fn briefing(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang, cfg: &GameConfig, weather: &crate::nasa::Weather) {
    let m = map(cfg.map);
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(28.0)), column_gap: Val::Px(24.0), ..default() }).with_children(|row| {
        row.spawn((Node { width: Val::Px(620.0), ..panel_node() }, BackgroundColor(C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(26.0)))).with_children(|c| {
            c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
                icon(r, icons, m.icon, 64.0);
                text(r, l.t(m.name_ru, m.name_en), &fonts.head, 30.0, C_TEXT);
            });
            // facts as icon rows
            for (ic, label, value) in [
                ("radiation", l.t("Радиация", "Radiation"), l.t(m.dose_real_ru, m.dose_real_en).to_string()),
                ("radio", l.t("Задержка связи", "Comms delay"), l.t(m.delay_real_ru, m.delay_real_en).to_string()),
                ("run", l.t("Гравитация", "Gravity"), format!("{:.0}%", m.gravity * 100.0)),
                ("hourglass", l.t("Солов", "Sols"), format!("{}", m.sols)),
            ] {
                c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                    icon(r, icons, ic, 30.0);
                    text(r, label, &fonts.body, 20.0, C_MUTED);
                    text(r, value, &fonts.body, 20.0, C_TEXT);
                });
            }
            // goal: 3 stars
            c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), padding: UiRect::all(Val::Px(10.0)), ..default() }).with_children(|r| {
                for (ic, ru, en) in [("check", "выжить", "survive"), ("science", "наука", "science"), ("radiation", "доза < 50%", "dose < 50%")] {
                    r.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|s| {
                        icon(s, icons, "star", 26.0);
                        icon(s, icons, ic, 22.0);
                        text(s, l.t(ru, en), &fonts.body, 16.0, C_MUTED);
                    });
                }
            });
            c.spawn((Text::new(l.t(m.fact_ru, m.fact_en)), TextFont { font: fonts.body.clone(), font_size: 18.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(560.0), ..default() }));
            // if-then plan
            text(c, l.t("Мой план «если — то»:", "My if–then plan:"), &fonts.body, 18.0, C_MUTED);
            c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|r| {
                for (i, (ic, ru, en)) in [("hint", "Если трудно — паузу и подсказку", "If it’s hard — pause and hint"), ("beacon", "Если волнуюсь — маяк", "If I’m nervous — the beacon"), ("wrench", "Если сломалось — найти причину", "If it breaks — find the cause")].iter().enumerate() {
                    let on = cfg.plan == i as u8;
                    let e = icon_button(r, icons, fonts, Action::SetPlan(i as u8), i as u16, ic, l.t(ru, en), on);
                    let _ = e;
                }
            });
            if m.id != MapId::Earth {
                let live = weather.live && cfg.live;
                c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                    icon_button(r, icons, fonts, Action::ToggleLive, 3, if live { "check" } else { "cross" }, l.t("Реальная космическая погода (NASA DONKI)", "Real space weather (NASA DONKI)"), false);
                    text(r, if weather.live { "LIVE" } else { l.t("АРХИВ", "ARCHIVE") }, &fonts.head, 16.0, if weather.live { C_GOOD } else { C_WARN });
                });
            }
            c.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::FlexEnd, ..default() }).with_children(|r| {
                icon_button(r, icons, fonts, Action::Launch, 4, "rocket", l.t("Старт миссии", "Launch mission"), true);
            });
        });
        let _ = profile;
    });
}

fn shop(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang, cfg: &GameConfig) {
    let tab = cfg.shop_tab;
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(28.0)), row_gap: Val::Px(14.0), width: Val::Px(760.0), ..default() }).with_children(|col| {
        col.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
            for (i, (ic, ru, en)) in [("shirt", "Скафандр", "Suit"), ("goggles", "Визор", "Visor"), ("crown", "Украшение", "Accessory")].iter().enumerate() {
                icon_button(r, icons, fonts, Action::GoShop(i as u8), 100 + i as u16, ic, l.t(ru, en), tab == i as u8);
            }
        });
        let (list, owned, wearing): (&[Skin], &Vec<String>, &String) = match tab { 0 => (SUITS, &profile.owned_suits, &profile.suit), 1 => (VISORS, &profile.owned_visors, &profile.visor), _ => (HATS, &profile.owned_hats, &profile.hat) };
        col.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(12.0), row_gap: Val::Px(12.0), ..default() }).with_children(|grid| {
            for (i, s) in list.iter().enumerate() {
                let own = owned.iter().any(|o| o == s.id);
                let wear = wearing == s.id;
                let action = if own { Action::Wear(tab, i) } else { Action::Buy(tab, i) };
                grid.spawn((Button, UiAction(action), FocusOrder(i as u16), Node { width: Val::Px(170.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(C_PANEL2), BorderColor(if wear { C_GOOD } else { C_LINE }), BorderRadius::all(Val::Px(20.0))))
                    .with_children(|c| {
                        if tab == 2 {
                            icon(c, icons, s.icon, 56.0);
                        } else {
                            c.spawn((Node { width: Val::Px(56.0), height: Val::Px(56.0), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(s.color), BorderColor(Color::WHITE), BorderRadius::MAX));
                        }
                        c.spawn((Text::new(l.t(s.name_ru, s.name_en)), TextFont { font: fonts.body.clone(), font_size: 17.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
                        c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|r| {
                            if wear {
                                icon(r, icons, "check", 22.0);
                                text(r, l.t("надето", "wearing"), &fonts.body, 16.0, C_GOOD);
                            } else if own {
                                text(r, l.t("надеть", "wear"), &fonts.body, 16.0, C_TEXT);
                            } else {
                                icon(r, icons, "star", 22.0);
                                text(r, format!("{}", s.price), &fonts.head, 18.0, if profile.stars >= s.price { C_ACCENT } else { C_MUTED });
                            }
                        });
                    });
            }
        });
        text(col, l.t("Покупки не закрывают базы: базы открываются по сумме всех заработанных очков 🏅.", "Buying never relocks bases: bases unlock by total points ever earned 🏅."), &fonts.body, 16.0, C_MUTED);
    });
}

fn badges(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, profile: &crate::save::Profile, l: Lang) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(28.0)), row_gap: Val::Px(14.0), ..default() }).with_children(|col| {
        text(col, format!("{} {}/{}", l.t("Нашивки миссий", "Mission patches"), profile.badges.len(), BADGES.len()), &fonts.head, 30.0, C_TEXT);
        col.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(12.0), row_gap: Val::Px(12.0), ..default() }).with_children(|grid| {
            for b in BADGES {
                let has = profile.badges.iter().any(|x| x == b.id);
                grid.spawn((Node { width: Val::Px(170.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(if has { Color::srgba(0.35, 0.3, 0.1, 0.9) } else { C_PANEL }), BorderColor(if has { C_ACCENT } else { C_LINE }), BorderRadius::all(Val::Px(20.0))))
                    .with_children(|c| {
                        c.spawn((ImageNode { image: icons.get(b.icon), color: if has { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.25) }, ..default() }, Node { width: Val::Px(56.0), height: Val::Px(56.0), ..default() }));
                        c.spawn((Text::new(l.t(b.name_ru, b.name_en)), TextFont { font: fonts.body.clone(), font_size: 16.0, ..default() }, TextColor(if has { C_TEXT } else { C_MUTED }), TextLayout::new_with_justify(JustifyText::Center)));
                    });
            }
        });
    });
}

fn info(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, l: Lang, w: &crate::nasa::Weather) {
    p.spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(24.0)), margin: UiRect::all(Val::Px(24.0)), max_width: Val::Px(900.0), overflow: Overflow::clip_y(), max_height: Val::Px(640.0), ..default() }, BackgroundColor(C_PANEL), BorderRadius::all(Val::Px(24.0))))
        .with_children(|c| {
            c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
                icon(r, icons, "book", 40.0);
                text(r, l.t("Для взрослых, учителей и судей", "For adults, teachers & judges"), &fonts.head, 26.0, C_TEXT);
            });
            for (ic, ru, en) in [
                ("power", "Каждый сол энергия делится между кислородом, едой, щитом и наукой. Бесплатных решений нет.", "Every sol, power is split between oxygen, food, shielding and science. No decision is free."),
                ("radiation", "Дозы на картах пропорциональны реальным измерениям: Луна 1,369 мЗв/сут (Chang’e-4), Марс 0,64 (Curiosity RAD), глубокий космос 1,84. Игровой масштаб ×7,3.", "Map doses are proportional to real measurements: Moon 1.369 mSv/day (Chang’e-4), Mars 0.64 (Curiosity RAD), deep space 1.84. Game scale ×7.3."),
                ("sun", "Вспышки — реальные события NASA DONKI. Если живые данные недоступны, игра честно показывает «АРХИВ».", "Flares are real NASA DONKI events. When live data is unavailable the game honestly shows “ARCHIVE”."),
                ("radio", "Сообщения ЦУП приходят с задержкой: Луна 1,3 с (реально), Марс 4–24 мин (в игре ускорено и подписано).", "Mission Control messages arrive late: Moon 1.3 s (real), Mars 4–24 min (sped up and labeled in game)."),
                ("heart", "Экипаж не погибает: при провале он эвакуируется, а игра объясняет причину. Подсказки бесплатны, маяк-дыхание добровольный, рейтинга нет.", "No crew deaths: on failure the crew evacuates and the game explains why. Hints are free, the breathing beacon is voluntary, there is no ranking."),
                ("key", "Ключ NASA API: запустите игру с переменной NASA_API_KEY=… (бесплатно на api.nasa.gov). DEMO_KEY даёт ~30 запросов/час.", "NASA API key: run the game with NASA_API_KEY=… (free at api.nasa.gov). DEMO_KEY allows ~30 requests/hour."),
            ] {
                c.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                    icon(r, icons, ic, 28.0);
                    r.spawn((Text::new(l.t(ru, en)), TextFont { font: fonts.body.clone(), font_size: 17.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(800.0), ..default() }));
                });
            }
            let status = if w.loading { l.t("DONKI: загрузка…", "DONKI: loading…").to_string() } else if w.live { format!("DONKI LIVE · {} · {} {}", w.range, w.events.len(), l.t("событий", "events")) } else { l.t("DONKI: архив (нет сети или лимит ключа)", "DONKI: archive (offline or key limit)").to_string() };
            text(c, status, &fonts.head, 16.0, if w.live { C_GOOD } else { C_WARN });
            text(c, l.t("Источники:", "Sources:"), &fonts.head, 18.0, C_TEXT);
            for s in SOURCES {
                c.spawn((Text::new(format!("• {} — {}", s.title, s.url)), TextFont { font: fonts.body.clone(), font_size: 13.0, ..default() }, TextColor(C_MUTED), Node { max_width: Val::Px(820.0), ..default() }));
            }
        });
}

fn debrief(p: &mut ChildSpawnerCommands, icons: &Icons, fonts: &Fonts, l: Lang, r: &MissionResult, cfg: &GameConfig) {
    let m = map(r.map);
    p.spawn(Node { flex_grow: 1.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|c| {
        c.spawn((Node { width: Val::Px(720.0), align_items: AlignItems::Center, ..panel_node() }, BackgroundColor(C_PANEL), BorderColor(if r.won { C_GOOD } else { C_WARN }), BorderRadius::all(Val::Px(28.0)))).with_children(|d| {
            icon(d, icons, if r.won { "trophy" } else { "sos" }, 96.0);
            text(d, if r.won { l.t("Миссия выполнена!", "Mission complete!") } else { l.t("Экипаж безопасно вернулся домой", "The crew returned home safely") }, &fonts.head, 30.0, C_TEXT);
            d.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), ..default() }).with_children(|s| {
                for k in 0..3 {
                    s.spawn((ImageNode { image: icons.get("star"), color: if k < r.stars { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.2) }, ..default() }, Node { width: Val::Px(56.0), height: Val::Px(56.0), ..default() }));
                }
            });
            d.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(22.0), align_items: AlignItems::Center, ..default() }).with_children(|s| {
                for (ic, v) in [(m.icon, l.t(m.name_ru, m.name_en).to_string()), ("hourglass", format!("{}/{}", r.sol, r.sols)), ("science", format!("{:.0}/{}", r.science, r.goal)), ("radiation", format!("{:.0}%", r.dose))] {
                    s.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|x| {
                        icon(x, icons, ic, 28.0);
                        text(x, v, &fonts.body, 20.0, C_TEXT);
                    });
                }
            });
            d.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() }).with_children(|s| {
                text(s, format!("+{}", r.points), &fonts.head, 40.0, C_ACCENT);
                icon(s, icons, "star", 36.0);
                icon(s, icons, "medal", 36.0);
            });
            if let Some(nm) = r.new_map {
                let nmap = map(nm);
                d.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() }, BackgroundColor(C_GOOD.with_alpha(0.2)), BorderRadius::all(Val::Px(14.0)))).with_children(|s| {
                    icon(s, icons, "key", 28.0);
                    icon(s, icons, nmap.icon, 28.0);
                    text(s, format!("{}: {}", l.t("Открыта новая база", "New base unlocked"), l.t(nmap.name_ru, nmap.name_en)), &fonts.body, 20.0, C_TEXT);
                });
            }
            for b in &r.badges {
                if let Some(bd) = BADGES.iter().find(|x| x.id == *b) {
                    d.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() }).with_children(|s| {
                        icon(s, icons, bd.icon, 28.0);
                        text(s, format!("{}: {}", l.t("Нашивка", "Patch"), l.t(bd.name_ru, bd.name_en)), &fonts.body, 18.0, C_ACCENT);
                    });
                }
            }
            if let Some(cause) = r.cause {
                let (ic, ru, en) = match cause {
                    crate::sim::Cause::Oxygen => ("oxygen", "Кислород закончился. Экипаж тратит 12% за сол, а 1 блок энергии даёт только 6% — нужно хотя бы 2 блока.", "Oxygen ran out. The crew uses 12% per sol and 1 power block makes only 6% — you need at least 2 blocks."),
                    crate::sim::Cause::Food => ("food", "Еда закончилась. Оранжерее нужно около 2 блоков, чтобы кормить экипаж.", "Food ran out. The greenhouse needs about 2 blocks to feed the crew."),
                    crate::sim::Cause::Dose => ("radiation", "Доза дошла до предела. Щит снимает 30% за блок, а во время вспышки спасает убежище. У NASA тоже есть предел: 600 мЗв за карьеру.", "The dose hit the limit. The shield cuts 30% per block, and the shelter saves you during a flare. NASA has a limit too: 600 mSv per career."),
                };
                d.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(12.0)), ..default() }, BackgroundColor(C_PANEL2), BorderRadius::all(Val::Px(16.0)))).with_children(|s| {
                    icon(s, icons, ic, 32.0);
                    s.spawn((Text::new(l.t(ru, en)), TextFont { font: fonts.body.clone(), font_size: 18.0, ..default() }, TextColor(C_TEXT), Node { max_width: Val::Px(580.0), ..default() }));
                });
                d.spawn((Text::new(l.t("Ошибка — это данные. Теперь ты знаешь, что изменить.", "A mistake is data. Now you know what to change.")), TextFont { font: fonts.body.clone(), font_size: 18.0, ..default() }, TextColor(C_GOOD)));
            }
            // reflection: what helped? (+5 once)
            text(d, l.t("Что тебе помогло?", "What helped you?"), &fonts.body, 18.0, C_MUTED);
            d.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), justify_content: JustifyContent::Center, ..default() }).with_children(|s| {
                for (i, (ic, ru, en)) in [("radio", "Прогноз ЦУП", "Forecast"), ("battery", "Запас в батарее", "Battery reserve"), ("house", "Убежище", "Shelter"), ("beacon", "Маяк", "Beacon"), ("clipboard", "План", "Plan"), ("coop", "Напарник", "Partner")].iter().enumerate() {
                    icon_button(s, icons, fonts, Action::Helped(i as u8), 10 + i as u16, ic, l.t(ru, en), cfg.reflected);
                }
            });
            d.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), ..default() }).with_children(|s| {
                icon_button(s, icons, fonts, Action::PlayAgain, 0, "retry", l.t("Ещё раз", "Again"), true);
                icon_button(s, icons, fonts, Action::GoMaps(r.coop), 1, "map", l.t("К базам", "Bases"), false);
                icon_button(s, icons, fonts, Action::GoMenu, 2, "home", l.t("Меню", "Menu"), false);
            });
        });
    });
}
