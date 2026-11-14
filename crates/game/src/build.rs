//! B4 — build mode: pick a module in the bottom bar, a ghost snaps to the grid (green = OK,
//! red = refused with the reason from `astro_core`), click / F to build, right-click / X to remove.
//! Works with mouse, keyboard and gamepad.

use crate::art::{self, Art};
use crate::audio::Play;
use crate::kit::{self, Act, Action, Inputs, C_BAD, C_GOOD, C_LINE, C_MUTED, C_TEXT};
use crate::player::Player;
use crate::windows::Modal;
use crate::world::{depth, run, to_world, Happened, Refused};
use crate::{AppState, MainCam, Prof, Set, Sim, CELL};
use astro_core::catalog::module_spec;
use astro_core::{i18n, Command, GameApi, ModuleKind};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

#[derive(Resource, Default)]
pub struct BuildMode {
    pub active: bool,
    pub player: u8,
    pub sel: usize,
    /// last time the mouse moved (mouse aims the ghost; otherwise it sits in front of the player)
    mouse_aim: bool,
    cell: IVec2,
}

#[derive(Component)]
struct Ghost;
#[derive(Component)]
struct GhostText;
#[derive(Component)]
struct BaseRing;
#[derive(Component)]
struct BuildBar;
#[derive(Component)]
struct BuildButton(usize);
#[derive(Component)]
struct MaterialsText;

pub struct BuildPlugin;
impl Plugin for BuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildMode>()
            .add_systems(Update, spawn_ghost.run_if(resource_added::<Sim>))
            .add_systems(Update, (toggle, aim, place).chain().in_set(Set::Logic).run_if(in_state(AppState::Playing)))
            .add_systems(Update, (ghost_view, bar_view).in_set(Set::View).run_if(in_state(AppState::Playing)))
            .add_systems(OnExit(AppState::Playing), |mut b: ResMut<BuildMode>| *b = BuildMode::default());
    }
}

fn spawn_ghost(mut commands: Commands, art: Res<Art>) {
    commands.spawn((Sprite { image: art.sprite("solar"), anchor: Anchor::BottomLeft, ..default() }, Transform::from_xyz(0.0, 0.0, 50.0), Visibility::Hidden, Ghost, StateScoped(AppState::Playing)));
    commands
        .spawn((Text2d::new(""), TextFont { font: art.body.clone(), font_size: 20.0, ..default() }, TextColor(Color::WHITE), Transform::from_xyz(0.0, 0.0, 61.0), Visibility::Hidden, GhostText, StateScoped(AppState::Playing)))
        .with_children(|c| {
            c.spawn((Sprite { color: Color::srgba(0.04, 0.05, 0.15, 0.85), custom_size: Some(Vec2::new(380.0, 30.0)), ..default() }, Transform::from_xyz(0.0, 0.0, -0.1)));
        });
    commands.spawn((Sprite { image: art.sprite("bubble"), color: Color::srgba(1.0, 1.0, 1.0, 0.35), ..default() }, Transform::from_xyz(0.0, 0.0, 0.8), Visibility::Hidden, BaseRing, StateScoped(AppState::Playing)));
}

fn toggle(mut commands: Commands, inputs: Res<Inputs>, modal: Res<Modal>, mut bm: ResMut<BuildMode>, art: Res<Art>, prof: Res<Prof>, sim: Res<Sim>, bars: Query<Entity, With<BuildBar>>, mut acts: EventReader<Act>, mut play: EventWriter<Play>) {
    let mut changed = false;
    for (i, pad) in inputs.0.iter().enumerate() {
        if modal.blocks(i as u8) {
            continue;
        }
        if pad.build {
            if bm.active && bm.player == i as u8 {
                bm.active = false;
            } else {
                bm.active = true;
                bm.player = i as u8;
            }
            changed = true;
        }
        if bm.active && bm.player == i as u8 {
            if pad.back {
                bm.active = false;
                changed = true;
            }
            let n = ModuleKind::BUILDABLE.len();
            if pad.next {
                bm.sel = (bm.sel + 1) % n;
                changed = true;
            }
            if pad.prev {
                bm.sel = (bm.sel + n - 1) % n;
                changed = true;
            }
        }
    }
    for Act(a) in acts.read() {
        if let Action::BuildPick(i) = a {
            bm.sel = *i;
            bm.active = true;
            changed = true;
        }
    }
    let has_bar = !bars.is_empty();
    if !changed && bm.active == has_bar {
        return;
    }
    if changed {
        play.write(Play("click"));
    }
    for e in &bars {
        commands.entity(e).despawn();
    }
    if !bm.active {
        return;
    }
    let l = prof.lang();
    let mats = sim.0.state().materials;
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, bottom: Val::Px(12.0), left: Val::Percent(50.0), margin: UiRect::left(Val::Px(-470.0)), width: Val::Px(940.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(10.0)), border: UiRect::all(Val::Px(3.0)), ..default() },
            BackgroundColor(kit::C_BG),
            BorderColor(C_LINE),
            BorderRadius::all(Val::Px(8.0)),
            GlobalZIndex(20),
            BuildBar,
            StateScoped(AppState::Playing),
        ))
        .with_children(|p| {
            p.spawn(kit::row(10.0)).with_children(|r| {
                kit::icon(r, &art, "hammer", 30.0);
                kit::text(r, l.t("Стройка", "Build"), &art.head, 22.0, C_TEXT);
                kit::icon(r, &art, "wrench", 24.0);
                r.spawn((Text::new(format!("{mats}")), TextFont { font: art.head.clone(), font_size: 22.0, ..default() }, TextColor(kit::C_ACCENT), MaterialsText));
                kit::text(r, l.t("·  клик/F — строить · ПКМ/X — разобрать · Q/R — выбор · B — выход", "·  click/F build · RMB/X remove · Q/R choose · B exit"), &art.body, 17.0, C_MUTED);
            });
            p.spawn(kit::row(8.0)).with_children(|r| {
                for (i, k) in ModuleKind::BUILDABLE.iter().enumerate() {
                    let s = module_spec(*k);
                    let on = i == bm.sel;
                    let bg = if on { Color::srgb(0.32, 0.4, 0.85) } else { kit::C_PANEL };
                    r.spawn((
                        Button,
                        kit::UiAction(Action::BuildPick(i)),
                        BuildButton(i),
                        Node { width: Val::Px(120.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(6.0)), border: UiRect::all(Val::Px(3.0)), ..default() },
                        BackgroundColor(bg),
                        kit::Base(bg),
                        BorderColor(if on { kit::C_FOCUS } else { C_LINE }),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((ImageNode::new(art.sprite(art::module_sprite(*k))), Node { width: Val::Px(48.0), height: Val::Px(48.0 * img_aspect(*k)), ..default() }));
                        b.spawn((Text::new(l.t(s.name_ru, s.name_en)), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center)));
                        b.spawn(kit::row(4.0)).with_children(|c| {
                            kit::icon(c, &art, "wrench", 16.0);
                            kit::text(c, format!("{}", s.materials), &art.head, 16.0, if mats >= s.materials { kit::C_ACCENT } else { C_BAD });
                            if s.max_power > 0 {
                                kit::icon(c, &art, "power", 16.0);
                                kit::text(c, format!("≤{}", s.max_power), &art.body, 14.0, C_MUTED);
                            }
                        });
                    });
                }
            });
        });
}

fn img_aspect(k: ModuleKind) -> f32 {
    match k {
        ModuleKind::Greenhouse => 0.67,
        ModuleKind::Antenna => 2.0,
        _ => 1.0,
    }
}

fn aim(mut bm: ResMut<BuildMode>, mut moved: EventReader<CursorMoved>, window: Query<&Window, With<PrimaryWindow>>, cam: Query<(&Camera, &GlobalTransform), With<MainCam>>, players: Query<&Player>, inputs: Res<Inputs>) {
    if !bm.active {
        moved.clear();
        return;
    }
    if moved.read().count() > 0 && bm.player == 0 {
        bm.mouse_aim = true;
    }
    if inputs.0[bm.player as usize].mv.length() > 0.1 {
        bm.mouse_aim = false;
    }
    let k = ModuleKind::BUILDABLE[bm.sel];
    let s = module_spec(k);
    if bm.mouse_aim {
        let (Ok(w), Ok((c, gt))) = (window.single(), cam.single()) else { return };
        if let Some(cur) = w.cursor_position() {
            if let Ok(p) = c.viewport_to_world_2d(gt, cur) {
                bm.cell = IVec2::new((p.x / CELL - s.w as f32 / 2.0).round() as i32, (p.y / CELL - s.h as f32 / 2.0).round() as i32);
            }
        }
    } else if let Some(p) = players.iter().find(|p| p.idx == bm.player) {
        let f = p.facing;
        let ahead = p.pos + f * (1.2 + s.w.max(s.h) as f32 * 0.6);
        bm.cell = IVec2::new((ahead.x - s.w as f32 / 2.0).round() as i32, (ahead.y - s.h as f32 / 2.0).round() as i32);
    }
}

fn place(bm: Res<BuildMode>, inputs: Res<Inputs>, mouse: Res<ButtonInput<MouseButton>>, ui_hover: Query<&Interaction>, mut sim: ResMut<Sim>, mut ok: EventWriter<Happened>, mut no: EventWriter<Refused>, mut play: EventWriter<Play>) {
    if !bm.active {
        return;
    }
    let over_ui = ui_hover.iter().any(|i| *i != Interaction::None);
    let pad = inputs.0[bm.player as usize];
    let build = pad.act || (bm.player == 0 && bm.mouse_aim && mouse.just_pressed(MouseButton::Left) && !over_ui);
    let remove = pad.demolish || (bm.player == 0 && mouse.just_pressed(MouseButton::Right) && !over_ui);
    let k = ModuleKind::BUILDABLE[bm.sel];
    if build {
        if run(&mut sim, bm.player, Command::Build { kind: k, x: bm.cell.x, y: bm.cell.y }, &mut ok, &mut no) {
            play.write(Play("good"));
        } else {
            play.write(Play("bad"));
        }
    } else if remove {
        let s = module_spec(k);
        let c = bm.cell + IVec2::new(s.w / 2, s.h / 2);
        run(&mut sim, bm.player, Command::Demolish { x: c.x, y: c.y }, &mut ok, &mut no);
    }
}

fn ghost_view(bm: Res<BuildMode>, sim: Res<Sim>, art: Res<Art>, prof: Res<Prof>, time: Res<Time>, mut ghost: Query<(&mut Sprite, &mut Transform, &mut Visibility), (With<Ghost>, Without<BaseRing>, Without<GhostText>)>, mut txt: Query<(&mut Text2d, &mut TextColor, &mut Transform, &mut Visibility), (With<GhostText>, Without<Ghost>, Without<BaseRing>)>, mut ring: Query<(&mut Sprite, &mut Transform, &mut Visibility), (With<BaseRing>, Without<Ghost>, Without<GhostText>)>) {
    let (Ok((mut gs, mut gt, mut gv)), Ok((mut tx, mut tc, mut tt, mut tv)), Ok((mut rs, mut rt, mut rv))) = (ghost.single_mut(), txt.single_mut(), ring.single_mut()) else { return };
    if !bm.active {
        *gv = Visibility::Hidden;
        *tv = Visibility::Hidden;
        *rv = Visibility::Hidden;
        return;
    }
    let st = sim.0.state();
    let k = ModuleKind::BUILDABLE[bm.sel];
    let s = module_spec(k);
    let check = sim.0.check(bm.player, &Command::Build { kind: k, x: bm.cell.x, y: bm.cell.y });
    let pulse = 0.55 + 0.2 * (time.elapsed_secs() * 5.0).sin();
    gs.image = art.sprite(art::module_sprite(k));
    gs.custom_size = Some(Vec2::new(s.w as f32, s.h as f32 * if k == ModuleKind::Antenna { 1.0 } else { 1.0 }) * CELL);
    gs.color = match check {
        Ok(()) => Color::srgba(0.5, 1.0, 0.6, pulse),
        Err(_) => Color::srgba(1.0, 0.4, 0.4, pulse),
    };
    let p = to_world(bm.cell.x as f32, bm.cell.y as f32);
    gt.translation = Vec3::new(p.x, p.y, depth(bm.cell.y as f32) + 0.5);
    *gv = Visibility::Visible;
    let l = prof.lang();
    tx.0 = match &check {
        Ok(()) => format!("{} · {} {}", l.t(s.name_ru, s.name_en), s.materials, l.t("материалов", "materials")),
        Err(r) => i18n::refusal(r, l),
    };
    tc.0 = if check.is_ok() { C_GOOD } else { Color::srgb(1.0, 0.7, 0.7) };
    tt.translation = Vec3::new(p.x + s.w as f32 * CELL / 2.0, p.y - 18.0, 61.0);
    *tv = Visibility::Visible;
    let c = to_world(st.base_x as f32, st.base_y as f32);
    rt.translation = Vec3::new(c.x, c.y, 0.8);
    rs.custom_size = Some(Vec2::splat(st.base_r as f32 * 2.0 * CELL));
    *rv = Visibility::Visible;
}

fn bar_view(bm: Res<BuildMode>, sim: Res<Sim>, mut q: Query<(&BuildButton, &mut BorderColor, &mut BackgroundColor, &mut kit::Base)>, mut mats: Query<&mut Text, With<MaterialsText>>) {
    if !bm.active {
        return;
    }
    for (b, mut bc, mut bg, mut base) in &mut q {
        let on = b.0 == bm.sel;
        bc.0 = if on { kit::C_FOCUS } else { C_LINE };
        let want = if on { Color::srgb(0.32, 0.4, 0.85) } else { kit::C_PANEL };
        if base.0 != want {
            base.0 = want;
            bg.0 = want;
        }
    }
    for mut t in &mut mats {
        t.0 = format!("{}", sim.0.state().materials);
    }
}
