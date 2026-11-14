//! B3 — the astronaut(s): walking with collisions, 4-direction animation, skins,
//! "walk up and press F / hold F" interactions with a prompt above the head.

use crate::art::{self, Art};
use crate::audio::Play;
use crate::kit::Inputs;
use crate::windows::{Modal, Win};
use crate::world::{depth, run, to_world, Colliders, Happened, Refused};
use crate::{AppState, Prof, Set, Sim, PX};
use astro_core::catalog::module_spec;
use astro_core::{Command, GameApi, ModuleId, ModuleKind, SiteId, SiteKind, State};
use bevy::prelude::*;
use bevy::sprite::Anchor;

pub const SPEED: f32 = 4.4; // cells per second
pub const HOLD: f32 = 2.5; // seconds to repair
const R: f32 = 0.32; // body radius in cells

#[derive(Component)]
pub struct Player {
    pub idx: u8,
    pub pos: Vec2,
    pub dir: usize,
    pub facing: Vec2,
    anim: f32,
    hold: f32,
    hold_on: Option<ModuleId>,
}
#[derive(Component)]
struct Layer;
#[derive(Component)]
struct PromptView(u8);
#[derive(Component)]
struct PromptIcon;
#[derive(Component)]
struct PromptText;
#[derive(Component)]
struct PromptBar;
#[derive(Component)]
struct PromptBg;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Exit,
    Site(SiteId, SiteKind),
    Repair(ModuleId),
    Harvest,
    Deliver,
    Cargo,
    Crew,
    Shelter,
    Power(ModuleId),
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_players.run_if(resource_added::<Sim>))
            .add_systems(Update, (walk, interact).chain().in_set(Set::Logic).run_if(in_state(AppState::Playing)))
            .add_systems(Update, (animate, prompts).in_set(Set::View).run_if(in_state(AppState::Playing)));
    }
}

fn spawn_players(mut commands: Commands, sim: Res<Sim>, art: Res<Art>, prof: Res<Prof>) {
    for (i, p) in sim.0.state().players.iter().enumerate() {
        let (suit, visor, hat) = if i == 0 {
            (art::suit_color(&prof.0.suit), art::visor_color(&prof.0.visor), art::hat_index(&prof.0.hat))
        } else {
            (Color::srgb(0.5, 0.75, 1.0), Color::srgb(1.0, 0.8, 0.3), Some(0))
        };
        let w = to_world(p.x, p.y);
        let size = Vec2::new(16.0, 24.0) * PX;
        commands
            .spawn((
                Sprite { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: suit, custom_size: Some(size), anchor: Anchor::BottomCenter, ..default() },
                Transform::from_xyz(w.x, w.y, depth(p.y)),
                Player { idx: i as u8, pos: Vec2::new(p.x, p.y), dir: 0, facing: Vec2::NEG_Y, anim: 0.0, hold: 0.0, hold_on: None },
                StateScoped(AppState::Playing),
            ))
            .with_children(|c| {
                c.spawn((Sprite { image: art.sprite("astronaut_visor"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: visor, custom_size: Some(size), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(0.0, 0.0, 0.01), Layer));
                if let Some(h) = hat {
                    c.spawn((Sprite { image: art.sprite("hats"), texture_atlas: Some(TextureAtlas { layout: art.hat_layout.clone(), index: h }), custom_size: Some(Vec2::splat(16.0 * PX)), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(0.0, 52.0, 0.02)));
                }
                // player number marker in co-op
                if sim.0.state().coop {
                    c.spawn((Sprite { image: art.sprite("arrow"), color: if i == 0 { Color::srgb(1.0, 0.75, 0.3) } else { Color::srgb(0.5, 0.8, 1.0) }, custom_size: Some(Vec2::splat(24.0)), flip_y: true, ..default() }, Transform::from_xyz(0.0, 100.0, 0.03)));
                }
            });
        // prompt above the head
        commands
            .spawn((Transform::from_xyz(w.x, w.y + 110.0, 60.0), Visibility::Hidden, PromptView(i as u8), StateScoped(AppState::Playing)))
            .with_children(|c| {
                c.spawn((Sprite { color: Color::srgba(0.06, 0.07, 0.2, 0.92), custom_size: Some(Vec2::new(280.0, 50.0)), ..default() }, Transform::from_xyz(0.0, 0.0, 0.0), PromptBg));
                c.spawn((Sprite { image: art.icon("check"), custom_size: Some(Vec2::splat(36.0)), ..default() }, Transform::from_xyz(-112.0, 0.0, 0.1), PromptIcon));
                c.spawn((Text2d::new(""), TextFont { font: art.body.clone(), font_size: 21.0, ..default() }, TextColor(Color::WHITE), Anchor::CenterLeft, Transform::from_xyz(-86.0, 2.0, 0.1), PromptText));
                c.spawn((Sprite { color: Color::srgb(0.3, 0.9, 0.6), custom_size: Some(Vec2::new(1.0, 6.0)), anchor: Anchor::CenterLeft, ..default() }, Transform::from_xyz(-136.0, -21.0, 0.2), PromptBar));
            });
    }
}

/// Pushes a point out of a rectangle (cells).
fn push_rect(p: &mut Vec2, x: f32, y: f32, w: f32, h: f32) {
    let (x0, x1, y0, y1) = (x - R, x + w + R, y - R, y + h * 0.75 + R);
    if p.x > x0 && p.x < x1 && p.y > y0 && p.y < y1 {
        let dl = p.x - x0;
        let dr = x1 - p.x;
        let db = p.y - y0;
        let dt = y1 - p.y;
        let m = dl.min(dr).min(db).min(dt);
        if m == dl { p.x = x0 } else if m == dr { p.x = x1 } else if m == db { p.y = y0 } else { p.y = y1 }
    }
}

fn walk(time: Res<Time>, inputs: Res<Inputs>, modal: Res<Modal>, colliders: Res<Colliders>, mut sim: ResMut<Sim>, mut q: Query<&mut Player>, mut play: EventWriter<Play>, mut step_t: Local<f32>) {
    let dt = time.delta_secs().min(0.05);
    let (w, h, mods) = {
        let st = sim.0.state();
        (st.world_w as f32, st.world_h as f32, st.modules.iter().map(|m| { let s = module_spec(m.kind); (m.x as f32, m.y as f32, s.w as f32, s.h as f32) }).collect::<Vec<_>>())
    };
    let mut moved_any = false;
    for mut p in &mut q {
        let pad = inputs.0[p.idx as usize];
        let in_shelter = sim.0.state().players.get(p.idx as usize).map(|s| s.in_shelter).unwrap_or(false);
        if modal.blocks(p.idx) || in_shelter {
            p.anim = 0.0;
            continue;
        }
        let mv = pad.mv;
        if mv.length() > 0.1 {
            moved_any = true;
            p.facing = mv.normalize();
            p.dir = if mv.x.abs() > mv.y.abs() { if mv.x < 0.0 { 1 } else { 2 } } else if mv.y > 0.0 { 3 } else { 0 };
            p.anim += dt * 9.0 * mv.length();
            let mut np = p.pos + mv * SPEED * dt;
            for &(x, y, mw, mh) in &mods {
                push_rect(&mut np, x, y, mw, mh);
            }
            for &(c, r) in &colliders.0 {
                let d = np - c;
                let min = r + R;
                if d.length() < min && d.length() > 0.0001 {
                    np = c + d.normalize() * min;
                }
            }
            np.x = np.x.clamp(0.6, w - 0.6);
            np.y = np.y.clamp(0.4, h - 1.2);
            p.pos = np;
        } else {
            p.anim = 0.0;
        }
        let pos = p.pos;
        sim.0.set_player(p.idx, pos.x, pos.y);
    }
    if moved_any {
        *step_t -= dt;
        if *step_t <= 0.0 {
            *step_t = 0.36;
            play.write(Play("step"));
        }
    }
}

/// What can this player do right now, and how close is it?
pub fn find_target(st: &State, idx: u8, pos: Vec2) -> Option<Target> {
    let ps = st.players.get(idx as usize)?;
    if ps.in_shelter {
        return Some(Target::Exit);
    }
    let dist_rect = |x: f32, y: f32, w: f32, h: f32| {
        let dx = (x - pos.x).max(pos.x - (x + w)).max(0.0);
        let dy = (y - pos.y).max(pos.y - (y + h)).max(0.0);
        (dx * dx + dy * dy).sqrt()
    };
    if let Some(s) = st.sites.iter().filter(|s| (Vec2::new(s.x, s.y) - pos).length() < 1.4).min_by(|a, b| (Vec2::new(a.x, a.y) - pos).length().total_cmp(&(Vec2::new(b.x, b.y) - pos).length())) {
        return Some(Target::Site(s.id, s.kind));
    }
    let near: Vec<_> = st
        .modules
        .iter()
        .map(|m| {
            let s = module_spec(m.kind);
            (m, dist_rect(m.x as f32, m.y as f32, s.w as f32, s.h as f32))
        })
        .filter(|(_, d)| *d < 1.1)
        .collect();
    if let Some((m, _)) = near.iter().find(|(m, _)| m.broken || m.leaking || m.dusty) {
        return Some(Target::Repair(m.id));
    }
    if st.harvest_ready && near.iter().any(|(m, _)| m.kind == ModuleKind::Greenhouse) {
        return Some(Target::Harvest);
    }
    if ps.carried > 0 && near.iter().any(|(m, _)| m.kind == ModuleKind::Lab) {
        return Some(Target::Deliver);
    }
    if let Some((cx, cy)) = st.cargo {
        if (Vec2::new(cx, cy) - pos).length() < 1.8 {
            return Some(Target::Cargo);
        }
    }
    if st.crew_mood.is_some() && (Vec2::new(st.crew_pos.0, st.crew_pos.1) - pos).length() < 1.6 {
        return Some(Target::Crew);
    }
    if near.iter().any(|(m, _)| m.kind == ModuleKind::Habitat) {
        return Some(Target::Shelter);
    }
    near.iter().filter(|(m, _)| module_spec(m.kind).max_power > 0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(m, _)| Target::Power(m.id))
}

fn interact(
    time: Res<Time>,
    inputs: Res<Inputs>,
    mut modal: ResMut<Modal>,
    build: Res<crate::build::BuildMode>,
    mut sim: ResMut<Sim>,
    mut q: Query<&mut Player>,
    mut ok: EventWriter<Happened>,
    mut no: EventWriter<Refused>,
    mut play: EventWriter<Play>,
) {
    let dt = time.delta_secs();
    for mut p in &mut q {
        let pad = inputs.0[p.idx as usize];
        if modal.blocks(p.idx) || (build.active && build.player == p.idx) {
            p.hold = 0.0;
            continue;
        }
        let target = find_target(sim.0.state(), p.idx, p.pos);
        match target {
            Some(Target::Repair(id)) => {
                if pad.act_held {
                    if p.hold_on != Some(id) {
                        p.hold_on = Some(id);
                        p.hold = 0.0;
                    }
                    let before = p.hold;
                    p.hold += dt;
                    if (p.hold * 3.0) as i32 != (before * 3.0) as i32 {
                        play.write(Play("repair"));
                    }
                    if p.hold >= HOLD {
                        p.hold = 0.0;
                        p.hold_on = None;
                        run(&mut sim, p.idx, Command::Repair { module: id }, &mut ok, &mut no);
                    }
                } else {
                    p.hold = 0.0;
                }
            }
            Some(t) if pad.act => {
                p.hold = 0.0;
                match t {
                    Target::Exit => {
                        run(&mut sim, p.idx, Command::ExitShelter, &mut ok, &mut no);
                    }
                    Target::Shelter => {
                        if run(&mut sim, p.idx, Command::EnterShelter, &mut ok, &mut no) {
                            play.write(Play("click"));
                        }
                    }
                    Target::Site(id, _) => {
                        run(&mut sim, p.idx, Command::PickSample { site: id }, &mut ok, &mut no);
                    }
                    Target::Deliver => {
                        run(&mut sim, p.idx, Command::DeliverSamples, &mut ok, &mut no);
                    }
                    Target::Harvest => modal.open(Win::Harvest, p.idx),
                    Target::Cargo => modal.open(Win::Cargo, p.idx),
                    Target::Crew => modal.open(Win::Crew, p.idx),
                    Target::Power(id) => {
                        // F cycles power: +1 block, wrapping back to 0 at the maximum
                        let st = sim.0.state();
                        if let Some(m) = st.modules.iter().find(|m| m.id == id) {
                            let max = module_spec(m.kind).max_power;
                            let next = if m.power >= max { 0 } else { m.power + 1 };
                            if run(&mut sim, p.idx, Command::SetPower { module: id, blocks: next }, &mut ok, &mut no) {
                                play.write(Play("click"));
                            }
                        }
                    }
                    Target::Repair(_) => {}
                }
            }
            _ => p.hold = 0.0,
        }
    }
}

fn animate(sim: Res<Sim>, mut q: Query<(&Player, &mut Transform, &mut Sprite, &mut Visibility, &Children)>, mut layers: Query<&mut Sprite, (With<Layer>, Without<Player>)>) {
    let st = sim.0.state();
    for (p, mut tr, mut sprite, mut vis, children) in &mut q {
        let in_shelter = st.players.get(p.idx as usize).map(|s| s.in_shelter).unwrap_or(false);
        *vis = if in_shelter { Visibility::Hidden } else { Visibility::Visible };
        let frame = if p.anim > 0.0 { (p.anim as usize) % 4 } else { 0 };
        let idx = p.dir * 4 + frame;
        if let Some(a) = sprite.texture_atlas.as_mut() {
            a.index = idx;
        }
        for c in children.iter() {
            if let Ok(mut l) = layers.get_mut(c) {
                if let Some(a) = l.texture_atlas.as_mut() {
                    a.index = idx;
                }
            }
        }
        let w = to_world(p.pos.x, p.pos.y);
        tr.translation = Vec3::new(w.x, w.y, depth(p.pos.y) + 0.005);
    }
}

fn prompts(
    sim: Res<Sim>,
    art: Res<Art>,
    prof: Res<Prof>,
    modal: Res<Modal>,
    build: Res<crate::build::BuildMode>,
    players: Query<&Player>,
    mut views: Query<(&PromptView, &mut Transform, &mut Visibility, &Children)>,
    mut icons: Query<&mut Sprite, (With<PromptIcon>, Without<PromptBar>, Without<PromptBg>)>,
    mut texts: Query<&mut Text2d, With<PromptText>>,
    mut bars: Query<&mut Sprite, (With<PromptBar>, Without<PromptIcon>, Without<PromptBg>)>,
    mut bgs: Query<&mut Sprite, (With<PromptBg>, Without<PromptIcon>, Without<PromptBar>)>,
) {
    let st = sim.0.state();
    let l = prof.lang();
    for (v, mut tr, mut vis, children) in &mut views {
        let Some(p) = players.iter().find(|p| p.idx == v.0) else { continue };
        let target = if modal.blocks(p.idx) || (build.active && build.player == p.idx) { None } else { find_target(st, p.idx, p.pos) };
        let Some(t) = target else {
            *vis = Visibility::Hidden;
            continue;
        };
        let key = if st.coop && p.idx == 1 { "Shift" } else { "F" };
        let (ic, label, warn): (&str, String, bool) = match t {
            Target::Exit => ("exit", l.t("Выйти наружу", "Go outside").into(), false),
            Target::Shelter => ("house", l.t("В убежище", "Shelter").into(), st.flare.is_some()),
            Target::Site(_, SiteKind::Sample) => ("gem", l.t("Взять образец", "Pick sample").into(), false),
            Target::Site(_, SiteKind::Ice) => ("ice", l.t("Добыть лёд", "Mine ice").into(), false),
            Target::Site(_, SiteKind::Wreck) => ("wrench", l.t("Взять материалы", "Take materials").into(), false),
            Target::Repair(id) => {
                let m = st.modules.iter().find(|m| m.id == id);
                let what = match m {
                    Some(m) if m.dusty => l.t("Чистить (держи)", "Clean (hold)"),
                    Some(m) if m.leaking => l.t("Заделать (держи)", "Patch (hold)"),
                    _ => l.t("Чинить (держи)", "Repair (hold)"),
                };
                ("wrench", what.into(), true)
            }
            Target::Harvest => ("plant", l.t("Урожай", "Harvest").into(), false),
            Target::Deliver => ("science", l.t("Сдать образцы", "Deliver samples").into(), false),
            Target::Cargo => ("box", l.t("Разгрузить", "Unload").into(), false),
            Target::Crew => ("speech", l.t("Поговорить", "Talk").into(), false),
            Target::Power(id) => {
                let m = st.modules.iter().find(|m| m.id == id).unwrap();
                let s = module_spec(m.kind);
                ("power", format!("{} {}/{}", l.t(s.name_ru, s.name_en), m.power, s.max_power), false)
            }
        };
        *vis = Visibility::Visible;
        let w = to_world(p.pos.x, p.pos.y);
        tr.translation = Vec3::new(w.x, w.y + if st.players[p.idx as usize].in_shelter { 40.0 } else { 120.0 }, 60.0);
        for c in children.iter() {
            if let Ok(mut s) = icons.get_mut(c) {
                s.image = art.icon(ic);
            }
            if let Ok(mut tx) = texts.get_mut(c) {
                tx.0 = format!("[{key}] {label}");
            }
            if let Ok(mut s) = bars.get_mut(c) {
                let prog = if matches!(t, Target::Repair(_)) && p.hold_on.is_some() { (p.hold / HOLD).clamp(0.0, 1.0) } else { 0.0 };
                s.custom_size = Some(Vec2::new(272.0 * prog, 6.0));
            }
            if let Ok(mut s) = bgs.get_mut(c) {
                s.color = if warn { Color::srgba(0.45, 0.08, 0.12, 0.94) } else { Color::srgba(0.06, 0.07, 0.2, 0.92) };
            }
        }
    }
}
