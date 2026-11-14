//! B2 — the world: tile map, decor, modules, sample sites, crewmate, cargo pod, shield bubble,
//! day/night, dust storm and flare effects. Also ticks the rules and routes their events.

use crate::art::{self, Art};
use crate::{AppState, LastResult, Prof, Set, Sim, Speed, CELL, PX};
use astro_core::catalog::{module_spec, MAPS};
use astro_core::{Command, GameApi, GameEvent, ModuleId, ModuleKind, Refusal, SiteId, SiteKind};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::collections::HashMap;

/// Something the rules reported (alerts, sounds, windows react to it).
#[derive(Event, Clone, Debug)]
pub struct Happened(pub GameEvent);
/// An action the rules refused (shown as a friendly toast).
#[derive(Event, Clone, Copy, Debug)]
pub struct Refused(pub Refusal);

/// Pauses the rules (solo play while a window is open).
#[derive(Resource, Default)]
pub struct Paused(pub bool);

/// Static round obstacles (big rocks) in cell units.
#[derive(Resource, Default)]
pub struct Colliders(pub Vec<(Vec2, f32)>);

#[derive(Resource, Default)]
struct Views {
    modules: HashMap<ModuleId, Entity>,
    sites: HashMap<SiteId, Entity>,
}

#[derive(Component)]
pub struct ModuleView(#[allow(dead_code)] pub ModuleId);
#[derive(Component)]
struct SiteView(SiteKind);
#[derive(Component)]
struct Spark;
#[derive(Component)]
struct LeakFx;
#[derive(Component)]
struct Pip(u8);
#[derive(Component)]
struct Crewmate {
    last: Vec2,
    t: f32,
}
#[derive(Component)]
struct CrewAlert;
#[derive(Component)]
struct CargoView {
    y: f32,
}
#[derive(Component)]
struct Flame;
#[derive(Component)]
struct ShieldBubble;
#[derive(Component)]
struct DustMote(Vec2);
#[derive(Component)]
struct NightTint;
#[derive(Component)]
struct DustTint;
#[derive(Component)]
struct FlareFrame;

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Happened>()
            .add_event::<Refused>()
            .init_resource::<Paused>()
            .init_resource::<Colliders>()
            .init_resource::<Views>()
            .add_systems(Update, spawn_world.run_if(resource_added::<Sim>))
            .add_systems(Update, tick_rules.in_set(Set::Logic).run_if(in_state(AppState::Playing)))
            .add_systems(Update, (sync_modules, sync_sites, crewmate, cargo, shield_bubble, sky_fx, mission_end).in_set(Set::View).run_if(in_state(AppState::Playing)))
            .add_systems(OnExit(AppState::Playing), |mut v: ResMut<Views>| *v = Views::default());
    }
}

/// Converts cell coordinates to world position.
pub fn to_world(x: f32, y: f32) -> Vec2 {
    Vec2::new(x * CELL, y * CELL)
}
/// Depth so lower things are drawn in front.
pub fn depth(y_cells: f32) -> f32 {
    10.0 - y_cells * 0.01
}

/// Runs a command, emitting events or a refusal. Returns true on success.
pub fn run(sim: &mut Sim, player: u8, cmd: Command, ok: &mut EventWriter<Happened>, no: &mut EventWriter<Refused>) -> bool {
    match sim.0.apply(player, cmd) {
        Ok(evs) => {
            for e in evs {
                ok.write(Happened(e));
            }
            true
        }
        Err(r) => {
            no.write(Refused(r));
            false
        }
    }
}

fn spawn_world(mut commands: Commands, sim: Res<Sim>, art: Res<Art>, mut colliders: ResMut<Colliders>) {
    let st = sim.0.state();
    let scoped = StateScoped(AppState::Playing);
    let tiles = art.sprite(art::tiles_for(st.map));
    let mut rng = Lcg(st.map as u64 * 7919 + 13);
    let margin = 12;
    for y in -margin..st.world_h + margin {
        for x in -margin..st.world_w + margin {
            let inside = x >= 0 && y >= 0 && x < st.world_w && y < st.world_h;
            let v = (rng.next() % 10) as usize;
            let idx = if v < 6 { 0 } else { v - 5 }.min(3);
            let p = to_world(x as f32, y as f32);
            commands.spawn((
                Sprite {
                    image: tiles.clone(),
                    texture_atlas: Some(TextureAtlas { layout: art.tile_layout.clone(), index: idx }),
                    custom_size: Some(Vec2::splat(CELL)),
                    color: if inside { Color::WHITE } else { Color::srgb(0.62, 0.62, 0.7) },
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                Transform::from_xyz(p.x, p.y, 0.0),
                scoped.clone(),
            ));
        }
    }
    // decor outside the base zone
    colliders.0.clear();
    let tint = art::rock_tint(st.map);
    let (bx, by, br) = (st.base_x as f32, st.base_y as f32, st.base_r as f32);
    if st.map != astro_core::MapId::Orbit {
        for i in 0..70 {
            let x = rng.f(1.0, st.world_w as f32 - 1.0);
            let y = rng.f(1.0, st.world_h as f32 - 1.0);
            if ((x - bx).powi(2) + (y - by).powi(2)).sqrt() < br + 2.0 {
                continue;
            }
            let (name, size, solid) = match i % 7 {
                0 => ("crater", 32.0, false),
                1 | 2 => ("rock_big", 32.0, true),
                _ => ("rock_small", 16.0, false),
            };
            let p = to_world(x, y);
            commands.spawn((
                Sprite { image: art.sprite(name), color: tint, custom_size: Some(Vec2::splat(size * PX)), anchor: Anchor::BottomCenter, ..default() },
                Transform::from_xyz(p.x, p.y, if solid { depth(y) } else { 0.5 }),
                scoped.clone(),
            ));
            if solid {
                colliders.0.push((Vec2::new(x, y + 0.5), 0.8));
            }
        }
    }
    // decorative rover parked near the base
    let rp = to_world(bx + 5.5, by - 5.0);
    commands.spawn((Sprite { image: art.sprite("rover"), custom_size: Some(Vec2::new(32.0, 24.0) * PX), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(rp.x, rp.y, depth(by - 5.0)), scoped.clone()));
    colliders.0.push((Vec2::new(bx + 5.5, by - 4.6), 0.9));

    // crewmate
    let (cx, cy) = st.crew_pos;
    let cp = to_world(cx, cy);
    commands
        .spawn((
            Sprite { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: Color::srgb(1.0, 0.6, 0.3), custom_size: Some(Vec2::new(16.0, 24.0) * PX), anchor: Anchor::BottomCenter, ..default() },
            Transform::from_xyz(cp.x, cp.y, depth(cy)),
            Crewmate { last: Vec2::new(cx, cy), t: 0.0 },
            scoped.clone(),
        ))
        .with_children(|p| {
            p.spawn((Sprite { image: art.sprite("astronaut_visor"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: Color::srgb(1.0, 0.8, 0.3), custom_size: Some(Vec2::new(16.0, 24.0) * PX), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(0.0, 0.0, 0.01)));
            p.spawn((Sprite { image: art.icon("speech"), custom_size: Some(Vec2::splat(40.0)), ..default() }, Transform::from_xyz(0.0, 92.0, 0.5), Visibility::Hidden, CrewAlert));
        });
    // shield bubble
    commands.spawn((Sprite { image: art.sprite("bubble"), custom_size: Some(Vec2::splat(10.0)), ..default() }, Transform::from_xyz(0.0, 0.0, 30.0), Visibility::Hidden, ShieldBubble, scoped.clone()));
    // dust motes (moved around the camera)
    for i in 0..90 {
        let off = Vec2::new(rng.f(-900.0, 900.0), rng.f(-520.0, 520.0));
        commands.spawn((Sprite { image: art.sprite("dot"), color: Color::srgba(0.95, 0.7, 0.45, 0.7), custom_size: Some(Vec2::splat(6.0 + (i % 3) as f32 * 3.0)), ..default() }, Transform::from_xyz(off.x, off.y, 40.0), Visibility::Hidden, DustMote(off), scoped.clone()));
    }
    // screen tints (UI layer, below the HUD)
    commands.spawn((crate::kit::overlay(false, 1), NightTint, scoped.clone()));
    commands.spawn((crate::kit::overlay(false, 2), DustTint, scoped.clone()));
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), border: UiRect::all(Val::Px(16.0)), ..default() },
        BorderColor(Color::NONE),
        GlobalZIndex(3),
        FlareFrame,
        scoped,
    ));
}

fn tick_rules(time: Res<Time>, speed: Res<Speed>, paused: Res<Paused>, mut sim: ResMut<Sim>, mut out: EventWriter<Happened>) {
    if paused.0 {
        return;
    }
    let dt = time.delta_secs().min(0.1) * speed.0;
    // big steps are split so the rules stay stable when fast-forwarding
    let steps = (dt / 0.1).ceil().max(1.0) as u32;
    for _ in 0..steps {
        for e in sim.0.tick(dt / steps as f32) {
            out.write(Happened(e));
        }
    }
}

fn sync_modules(mut commands: Commands, sim: Res<Sim>, art: Res<Art>, time: Res<Time>, mut views: ResMut<Views>, mut q: Query<(&mut Sprite, &Children), With<ModuleView>>, mut sparks: Query<&mut Visibility, (With<Spark>, Without<LeakFx>, Without<Pip>)>, mut leaks: Query<(&mut Visibility, &mut Transform), (With<LeakFx>, Without<Spark>, Without<Pip>)>, mut pips: Query<(&Pip, &mut Sprite), Without<ModuleView>>) {
    let st = sim.0.state();
    let t = time.elapsed_secs();
    // spawn new
    for m in &st.modules {
        if views.modules.contains_key(&m.id) {
            continue;
        }
        let s = module_spec(m.kind);
        let size = Vec2::new(s.w as f32, s.h as f32) * CELL;
        let img_size = match m.kind {
            ModuleKind::Habitat => Vec2::new(48.0, 48.0),
            ModuleKind::Greenhouse => Vec2::new(48.0, 32.0),
            ModuleKind::Battery => Vec2::new(16.0, 16.0),
            ModuleKind::Antenna => Vec2::new(16.0, 32.0),
            _ => Vec2::new(32.0, 32.0),
        } * PX;
        let p = to_world(m.x as f32, m.y as f32);
        let e = commands
            .spawn((
                Sprite { image: art.sprite(art::module_sprite(m.kind)), custom_size: Some(img_size), anchor: Anchor::BottomLeft, ..default() },
                Transform::from_xyz(p.x, p.y, depth(m.y as f32)),
                ModuleView(m.id),
                StateScoped(AppState::Playing),
            ))
            .with_children(|c| {
                c.spawn((Sprite { image: art.sprite("spark"), custom_size: Some(Vec2::splat(42.0)), ..default() }, Transform::from_xyz(size.x - 10.0, size.y - 6.0, 0.2), Visibility::Hidden, Spark));
                c.spawn((Sprite { image: art.sprite("leak"), custom_size: Some(Vec2::splat(54.0)), ..default() }, Transform::from_xyz(size.x * 0.8, size.y * 0.55, 0.2), Visibility::Hidden, LeakFx));
                for i in 0..s.max_power {
                    let x = size.x / 2.0 + (i as f32 - (s.max_power as f32 - 1.0) / 2.0) * 16.0;
                    c.spawn((Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.2), custom_size: Some(Vec2::new(11.0, 15.0)), ..default() }, Transform::from_xyz(x, -12.0, 0.3), Pip(i)));
                }
            })
            .id();
        views.modules.insert(m.id, e);
    }
    // despawn removed
    let ids: Vec<ModuleId> = st.modules.iter().map(|m| m.id).collect();
    views.modules.retain(|id, e| {
        let keep = ids.contains(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    // update
    for m in &st.modules {
        let Some(&e) = views.modules.get(&m.id) else { continue };
        let Ok((mut sprite, children)) = q.get_mut(e) else { continue };
        let name = match m.kind {
            ModuleKind::Solar if m.dusty => "solar_dusty",
            ModuleKind::Greenhouse if m.ripe => "greenhouse_ripe",
            k => art::module_sprite(k),
        };
        let img = art.sprite(name);
        if sprite.image != img {
            sprite.image = img;
        }
        let consumer = module_spec(m.kind).max_power > 0;
        sprite.color = if m.broken { Color::srgb(0.7, 0.55, 0.55) } else if consumer && m.power == 0 { Color::srgb(0.6, 0.6, 0.68) } else { Color::WHITE };
        for c in children.iter() {
            if let Ok(mut v) = sparks.get_mut(c) {
                *v = if m.broken && (t * 6.0).sin() > -0.3 { Visibility::Visible } else { Visibility::Hidden };
            }
            if let Ok((mut v, mut tr)) = leaks.get_mut(c) {
                *v = if m.leaking { Visibility::Visible } else { Visibility::Hidden };
                tr.scale = Vec3::splat(0.8 + 0.3 * (t * 4.0).sin().abs());
            }
            if let Ok((pip, mut s)) = pips.get_mut(c) {
                s.color = if pip.0 < m.power { Color::srgb(1.0, 0.85, 0.25) } else { Color::srgba(1.0, 1.0, 1.0, 0.22) };
            }
        }
    }
}

fn sync_sites(mut commands: Commands, sim: Res<Sim>, art: Res<Art>, time: Res<Time>, mut views: ResMut<Views>, mut q: Query<(&SiteView, &mut Sprite, &mut Transform)>) {
    let st = sim.0.state();
    for s in &st.sites {
        if views.sites.contains_key(&s.id) {
            continue;
        }
        let name = match s.kind {
            SiteKind::Sample => "sample_0",
            SiteKind::Ice => "ice",
            SiteKind::Wreck => "wreck",
        };
        let p = to_world(s.x, s.y);
        let e = commands
            .spawn((Sprite { image: art.sprite(name), custom_size: Some(Vec2::splat(16.0 * PX)), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(p.x, p.y, depth(s.y)), SiteView(s.kind), StateScoped(AppState::Playing)))
            .id();
        views.sites.insert(s.id, e);
    }
    let ids: Vec<SiteId> = st.sites.iter().map(|s| s.id).collect();
    views.sites.retain(|id, e| {
        let keep = ids.contains(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    let t = time.elapsed_secs();
    for (v, mut s, mut tr) in &mut q {
        if v.0 == SiteKind::Sample {
            let img = art.sprite(if (t * 3.0) as i32 % 2 == 0 { "sample_0" } else { "sample_1" });
            if s.image != img {
                s.image = img;
            }
            tr.scale = Vec3::splat(1.0 + 0.06 * (t * 4.0).sin());
        }
    }
}

fn crewmate(sim: Res<Sim>, time: Res<Time>, mut q: Query<(&mut Crewmate, &mut Transform, &mut Sprite, &Children)>, mut visors: Query<&mut Sprite, Without<Crewmate>>, mut alerts: Query<(&mut Visibility, &mut Transform), (With<CrewAlert>, Without<Crewmate>)>) {
    let st = sim.0.state();
    let pos = Vec2::new(st.crew_pos.0, st.crew_pos.1);
    for (mut c, mut tr, mut sprite, children) in &mut q {
        let d = pos - c.last;
        let moving = d.length() > 0.001;
        if moving {
            c.t += time.delta_secs();
        }
        let dir = if d.x.abs() > d.y.abs() { if d.x < 0.0 { 1 } else { 2 } } else if d.y > 0.0 { 3 } else { 0 };
        let frame = if moving { ((c.t * 8.0) as usize) % 4 } else { 0 };
        let idx = if moving { dir * 4 + frame } else { 0 };
        c.last = pos;
        let p = to_world(pos.x, pos.y);
        tr.translation = Vec3::new(p.x, p.y, depth(pos.y));
        if let Some(a) = sprite.texture_atlas.as_mut() {
            a.index = idx;
        }
        for ch in children.iter() {
            if let Ok(mut v) = visors.get_mut(ch) {
                if let Some(a) = v.texture_atlas.as_mut() {
                    a.index = idx;
                }
            }
            if let Ok((mut vis, mut at)) = alerts.get_mut(ch) {
                *vis = if st.crew_mood.is_some() { Visibility::Visible } else { Visibility::Hidden };
                at.translation.y = 92.0 + (time.elapsed_secs() * 4.0).sin() * 5.0;
            }
        }
    }
}

fn cargo(mut commands: Commands, sim: Res<Sim>, art: Res<Art>, time: Res<Time>, mut q: Query<(Entity, &mut CargoView, &mut Transform, &Children)>, mut flames: Query<&mut Visibility, With<Flame>>) {
    let st = sim.0.state();
    match (st.cargo, q.iter_mut().next()) {
        (Some((x, y)), None) => {
            let p = to_world(x, y);
            commands
                .spawn((Sprite { image: art.sprite("cargo"), custom_size: Some(Vec2::new(32.0, 40.0) * PX), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(p.x, p.y + 900.0, depth(y)), CargoView { y: p.y }, StateScoped(AppState::Playing)))
                .with_children(|c| {
                    c.spawn((Sprite { image: art.sprite("flame"), custom_size: Some(Vec2::splat(48.0)), anchor: Anchor::TopCenter, ..default() }, Transform::from_xyz(0.0, 4.0, -0.1), Flame));
                });
        }
        (Some(_), Some((_, cv, mut tr, children))) => {
            if tr.translation.y > cv.y {
                tr.translation.y = (tr.translation.y - time.delta_secs() * 420.0).max(cv.y);
            } else {
                for c in children.iter() {
                    if let Ok(mut v) = flames.get_mut(c) {
                        *v = Visibility::Hidden;
                    }
                }
            }
        }
        (None, Some((e, ..))) => {
            commands.entity(e).despawn();
        }
        (None, None) => {}
    }
}

fn shield_bubble(sim: Res<Sim>, time: Res<Time>, mut q: Query<(&mut Visibility, &mut Transform, &mut Sprite), With<ShieldBubble>>) {
    let st = sim.0.state();
    let Ok((mut v, mut tr, mut s)) = q.single_mut() else { return };
    let shield = st.modules.iter().find(|m| m.kind == ModuleKind::Shield && m.power > 0 && !m.broken);
    match (shield, st.shield_radius > 0.0) {
        (Some(m), true) => {
            let sp = module_spec(m.kind);
            let c = to_world(m.x as f32 + sp.w as f32 / 2.0, m.y as f32 + sp.h as f32 / 2.0);
            *v = Visibility::Visible;
            tr.translation = Vec3::new(c.x, c.y, 30.0);
            s.custom_size = Some(Vec2::splat(st.shield_radius * 2.0 * CELL));
            let pulse = 0.75 + 0.25 * (time.elapsed_secs() * 2.0).sin();
            s.color = Color::srgba(1.0, 1.0, 1.0, if st.flare.is_some() { 1.0 } else { pulse * 0.8 });
        }
        _ => *v = Visibility::Hidden,
    }
}

fn sky_fx(
    sim: Res<Sim>,
    time: Res<Time>,
    cam: Query<&Transform, (With<crate::MainCam>, Without<DustMote>)>,
    mut night: Query<&mut BackgroundColor, (With<NightTint>, Without<DustTint>)>,
    mut dust: Query<&mut BackgroundColor, (With<DustTint>, Without<NightTint>)>,
    mut frame: Query<&mut BorderColor, With<FlareFrame>>,
    mut motes: Query<(&DustMote, &mut Transform, &mut Visibility)>,
) {
    let st = sim.0.state();
    let t = time.elapsed_secs();
    if let Ok(mut b) = night.single_mut() {
        let dark = (1.0 - st.daylight).clamp(0.0, 1.0) * if st.night { 0.62 } else { 0.42 };
        b.0 = Color::srgba(0.02, 0.03, 0.16, dark);
    }
    if let Ok(mut b) = dust.single_mut() {
        b.0 = if st.dust_storm { Color::srgba(0.75, 0.42, 0.2, 0.28) } else { Color::NONE };
    }
    if let Ok(mut b) = frame.single_mut() {
        b.0 = if st.flare.is_some() { Color::srgba(1.0, 0.45, 0.2, 0.35 + 0.35 * (t * 5.0).sin().abs()) } else { Color::NONE };
    }
    let c = cam.single().map(|t| t.translation.truncate()).unwrap_or_default();
    for (m, mut tr, mut v) in &mut motes {
        *v = if st.dust_storm { Visibility::Visible } else { Visibility::Hidden };
        if st.dust_storm {
            let x = (m.0.x + t * 260.0).rem_euclid(1800.0) - 900.0;
            let y = m.0.y + (t * 2.0 + m.0.x).sin() * 20.0;
            tr.translation = Vec3::new(c.x + x, c.y + y, 40.0);
        }
    }
}

fn mission_end(mut ev: EventReader<Happened>, mut prof: ResMut<Prof>, mut next: ResMut<NextState<AppState>>, mut commands: Commands) {
    for Happened(e) in ev.read() {
        if let GameEvent::MissionEnded(r) = e {
            let before: Vec<_> = MAPS.iter().filter(|m| prof.0.unlocked(m.id)).map(|m| m.id).collect();
            let new_badges = prof.0.record(r);
            prof.0.save();
            let new_map = MAPS.iter().map(|m| m.id).find(|id| prof.0.unlocked(*id) && !before.contains(id));
            commands.insert_resource(LastResult { result: r.clone(), new_badges, new_map, reflected: false });
            next.set(AppState::Debrief);
        }
    }
}

/// Tiny deterministic RNG for decor placement.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn f(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * (self.next() % 10000) as f32 / 10000.0
    }
}
