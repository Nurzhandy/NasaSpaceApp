//! The crew on board (Aya, Timur, BEEP the robot): they walk from room to room through the doors,
//! stop at consoles, say things in passing, chat when they meet — and talk to you: walk up and
//! press E (the conversation itself is a dialog in `assets/content/dialogs/talk.json`).
//! Their walking is decoration on each computer; what they say in a conversation is in the rules.

use super::{to_px, Camp, Crew, SC};
use crate::art::{self, Art};
use crate::AppState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use ja_core::content::ShipLayout;
use ja_core::Lang;

#[derive(Component)]
pub struct Npc {
    /// index into `Content::npcs`
    pub idx: u8,
    pub pos: Vec2,
    path: Vec<Vec2>,
    wait: f32,
    pub dir: usize,
    anim: f32,
    seed: u64,
    robot: bool,
}

/// A speech bubble over an NPC.
#[derive(Component)]
pub struct Bubble {
    owner: Entity,
    life: f32,
}

#[derive(Resource, Default)]
pub struct Chatter {
    t: f32,
    /// someone answers this NPC soon: (who answers, after seconds, line)
    reply: Option<(Entity, f32, String)>,
}

fn rnd(s: &mut u64) -> f32 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*s >> 40) as f32 / (1u64 << 24) as f32
}

/// The room a point is in.
fn room_of(l: &ShipLayout, p: Vec2) -> Option<usize> {
    l.rooms.iter().position(|r| p.x >= r.x as f32 && p.x <= (r.x + r.w) as f32 && p.y >= r.y as f32 && p.y <= (r.y + r.h) as f32)
}

/// Rooms joined by a door, and the door between them.
fn neighbours(l: &ShipLayout, a: usize) -> Vec<(usize, Vec2)> {
    let ra = &l.rooms[a];
    let mut out = vec![];
    for &(dx, dy) in &l.doors {
        let d = Vec2::new(dx, dy);
        let touches = |r: &ja_core::content::Room| d.x >= r.x as f32 - 0.01 && d.x <= (r.x + r.w) as f32 + 0.01 && d.y >= r.y as f32 - 0.01 && d.y <= (r.y + r.h) as f32 + 0.01;
        if !touches(ra) {
            continue;
        }
        for (b, rb) in l.rooms.iter().enumerate() {
            if b != a && touches(rb) {
                out.push((b, d));
            }
        }
    }
    out
}

/// Waypoints from `from` to `to`: through the doors, room by room (breadth-first).
fn route(l: &ShipLayout, from: Vec2, to: Vec2) -> Vec<Vec2> {
    let (Some(a), Some(b)) = (room_of(l, from), room_of(l, to)) else { return vec![to] };
    let mut prev: Vec<Option<(usize, Vec2)>> = vec![None; l.rooms.len()];
    let mut seen = vec![false; l.rooms.len()];
    let mut q = std::collections::VecDeque::from([a]);
    seen[a] = true;
    while let Some(r) = q.pop_front() {
        if r == b {
            break;
        }
        for (n, d) in neighbours(l, r) {
            if !seen[n] {
                seen[n] = true;
                prev[n] = Some((r, d));
                q.push_back(n);
            }
        }
    }
    let mut doors = vec![];
    let mut cur = b;
    while cur != a {
        let Some((p, d)) = prev[cur] else { return vec![to] };
        doors.push((p, d, cur));
        cur = p;
    }
    doors.reverse();
    let mut pts = vec![];
    for (from_room, d, to_room) in doors {
        // step up to the door, through it, and a little into the next room
        let c_from = room_centre(l, from_room);
        let c_to = room_centre(l, to_room);
        pts.push(d + (c_from - d).normalize_or_zero() * 1.0);
        pts.push(d);
        pts.push(d + (c_to - d).normalize_or_zero() * 1.0);
    }
    pts.push(to);
    pts
}

fn room_centre(l: &ShipLayout, i: usize) -> Vec2 {
    let r = &l.rooms[i];
    Vec2::new(r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0)
}

/// A random spot to walk to (on the floor, not on a console).
fn random_spot(l: &ShipLayout, seed: &mut u64) -> Vec2 {
    for _ in 0..40 {
        let r = &l.rooms[(rnd(seed) * l.rooms.len() as f32) as usize % l.rooms.len()];
        let p = Vec2::new(r.x as f32 + 1.5 + rnd(seed) * (r.w as f32 - 3.0), r.y as f32 + 1.5 + rnd(seed) * (r.h as f32 - 3.0));
        if ja_core::ship::walkable(l, p.x, p.y) {
            return p;
        }
    }
    Vec2::new(5.0, 10.5)
}

pub fn spawn(mut commands: Commands, camp: Res<Camp>, art: Res<Art>) {
    let c = camp.0.content();
    let Some(l) = &c.ship else { return };
    let mut seed = 77u64;
    for (i, n) in c.npcs.iter().enumerate().filter(|(_, n)| n.on_board) {
        let robot = n.id == "beep";
        let pos = random_spot(l, &mut seed);
        let p = to_px(pos.x, pos.y);
        let mut e = commands.spawn((
            Transform::from_xyz(p.x, p.y, 1.0),
            Visibility::default(),
            Npc { idx: i as u8, pos, path: vec![], wait: 1.0 + i as f32, dir: 0, anim: 0.0, seed: seed ^ (i as u64 * 977), robot },
            StateScoped(AppState::Ship),
        ));
        if robot {
            e.with_children(|c| {
                c.spawn((Sprite { image: art.icon("robot"), custom_size: Some(Vec2::splat(SC * 1.0)), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(0.0, 8.0, 0.0)));
                // a little jet underneath: BEEP hovers
                c.spawn((Sprite { color: Color::srgba(0.4, 0.8, 1.0, 0.7), custom_size: Some(Vec2::new(14.0, 8.0)), ..default() }, Transform::from_xyz(0.0, 2.0, -0.01)));
            });
        } else {
            let (suit, visor) = if n.id == "aya" { (Color::srgb(0.95, 0.55, 0.85), Color::srgb(0.6, 0.95, 1.0)) } else { (Color::srgb(0.55, 0.95, 0.6), Color::srgb(1.0, 0.85, 0.4)) };
            let size = Vec2::new(16.0, 24.0) * 2.6;
            e.insert(Sprite { image: art.sprite("astronaut"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: suit, custom_size: Some(size), anchor: Anchor::BottomCenter, ..default() });
            e.with_children(|c| {
                c.spawn((Sprite { image: art.sprite("astronaut_visor"), texture_atlas: Some(TextureAtlas { layout: art.astro_layout.clone(), index: 0 }), color: visor, custom_size: Some(size), anchor: Anchor::BottomCenter, ..default() }, Transform::from_xyz(0.0, 0.0, 0.01)));
            });
        }
        // the name over the head
        let name = n.name.get(Lang::Ru).to_string();
        let id = e.id();
        commands.entity(id).with_children(|c| {
            c.spawn((Text2d::new(name), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)), Transform::from_xyz(0.0, 76.0, 0.1)));
        });
        let _ = art::suit_color;
    }
}

/// Walk, pause, walk on. Stop while talking to someone.
pub fn walk(time: Res<Time>, camp: Res<Camp>, mut q: Query<(&mut Npc, &mut Transform, Option<&mut Sprite>)>, crew: Query<&Crew>) {
    let dt = time.delta_secs().min(0.05);
    let st = camp.0.state();
    let Some(l) = camp.0.content().ship.clone() else { return };
    let talking = st.dialog.as_ref().and_then(|d| camp.0.content().dialog(&d.id)).and_then(|d| d.nodes.iter().find(|n| n.id == st.dialog.as_ref().map(|x| x.node.clone()).unwrap_or_default())).map(|n| n.npc.clone());
    let t = time.elapsed_secs();
    for (mut n, mut tr, sprite) in &mut q {
        let my_id = camp.0.content().npcs.get(n.idx as usize).map(|x| x.id.clone()).unwrap_or_default();
        let frozen = camp.0.frozen() || talking.as_deref() == Some(my_id.as_str());
        // turn to face a crew member standing close by
        let close = crew.iter().map(|c| c.pos).find(|p| p.distance(n.pos) < 1.8);
        if frozen || close.is_some() {
            n.anim = 0.0;
            if let Some(p) = close {
                let d = p - n.pos;
                n.dir = if d.x.abs() > d.y.abs() { if d.x < 0.0 { 1 } else { 2 } } else if d.y > 0.0 { 3 } else { 0 };
            }
        } else if n.wait > 0.0 {
            n.wait -= dt;
            n.anim = 0.0;
            if n.wait <= 0.0 && n.path.is_empty() {
                let mut seed = n.seed;
                let goal = random_spot(&l, &mut seed);
                n.seed = seed;
                n.path = route(&l, n.pos, goal);
            }
        } else if let Some(&next) = n.path.first() {
            let d = next - n.pos;
            let speed = if n.robot { 2.6 } else { 2.2 };
            if d.length() < 0.15 {
                n.path.remove(0);
                if n.path.is_empty() {
                    let mut seed = n.seed;
                    n.wait = 2.0 + rnd(&mut seed) * 5.0;
                    n.seed = seed;
                }
            } else {
                let step = d.normalize() * speed * dt;
                let to = n.pos + if step.length() > d.length() { d } else { step };
                let moved = ja_core::ship::step(&l, (n.pos.x, n.pos.y), (to.x, to.y));
                let moved = Vec2::new(moved.0, moved.1);
                if moved.distance(n.pos) < 0.001 {
                    // stuck on something: rethink in a moment
                    n.path.clear();
                    n.wait = 0.5;
                }
                n.pos = moved;
                n.dir = if d.x.abs() > d.y.abs() { if d.x < 0.0 { 1 } else { 2 } } else if d.y > 0.0 { 3 } else { 0 };
                n.anim += dt * 9.0;
            }
        } else {
            n.wait = 1.0;
        }
        let p = to_px(n.pos.x, n.pos.y - 0.4);
        let hover = if n.robot { (t * 3.0 + n.idx as f32).sin() * 4.0 } else { 0.0 };
        tr.translation = Vec3::new(p.x, p.y + hover, 1.0 - n.pos.y * 0.001);
        if let Some(mut s) = sprite {
            let frame = if n.anim > 0.0 { (n.anim as usize) % 4 } else { 0 };
            if let Some(a) = s.texture_atlas.as_mut() {
                a.index = n.dir * 4 + frame;
            }
        }
    }
}

/// The visor layer follows the body's animation frame.
pub fn visor(npcs: Query<(&Sprite, &Children), With<Npc>>, mut layers: Query<&mut Sprite, Without<Npc>>) {
    for (s, ch) in &npcs {
        let Some(i) = s.texture_atlas.as_ref().map(|a| a.index) else { continue };
        for c in ch.iter() {
            if let Ok(mut l) = layers.get_mut(c) {
                if let Some(a) = l.texture_atlas.as_mut() {
                    a.index = i;
                }
            }
        }
    }
}

/// Lines in passing, and little chats when two crew members meet.
#[allow(clippy::too_many_arguments)]
pub fn chatter(mut commands: Commands, time: Res<Time>, camp: Res<Camp>, art: Res<Art>, prof: Res<crate::Prof>, mut ch: ResMut<Chatter>, npcs: Query<(Entity, &Npc)>, mut bubbles: Query<(Entity, &mut Bubble)>) {
    let dt = time.delta_secs();
    for (e, mut b) in &mut bubbles {
        b.life -= dt;
        if b.life <= 0.0 || npcs.get(b.owner).is_err() {
            commands.entity(e).despawn();
        }
    }
    if camp.0.frozen() {
        return;
    }
    let l = prof.lang();
    let c = camp.0.content();
    let say = |commands: &mut Commands, who: Entity, line: String| {
        commands.entity(who).with_children(|p| {
            let w = (line.chars().count() as f32 * 7.6 + 24.0).min(320.0);
            p.spawn((Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.95), custom_size: Some(Vec2::new(w, 30.0)), ..default() }, Transform::from_xyz(0.0, 104.0, 3.0), Bubble { owner: who, life: 3.5 }))
                .with_children(|b| {
                    b.spawn((Text2d::new(line), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(Color::srgb(0.1, 0.1, 0.2)), Transform::from_xyz(0.0, 0.0, 0.1)));
                    b.spawn((Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.95), custom_size: Some(Vec2::new(10.0, 10.0)), ..default() }, Transform::from_xyz(-10.0, -18.0, -0.01).with_rotation(Quat::from_rotation_z(0.785))));
                });
        });
    };
    if let Some((who, t, line)) = ch.reply.take() {
        let t = t - dt;
        if t <= 0.0 {
            say(&mut commands, who, line);
        } else {
            ch.reply = Some((who, t, line));
        }
    }
    ch.t -= dt;
    if ch.t > 0.0 || !bubbles.is_empty() {
        return;
    }
    let list: Vec<(Entity, &Npc)> = npcs.iter().collect();
    if list.is_empty() {
        return;
    }
    let k = (time.elapsed_secs() * 7.3) as usize % list.len();
    let (e, n) = list[k];
    let Some(def) = c.npcs.get(n.idx as usize) else { return };
    if def.chatter.is_empty() {
        return;
    }
    let line = def.chatter[(time.elapsed_secs() * 3.1) as usize % def.chatter.len()].get(l).to_string();
    say(&mut commands, e, line);
    // someone close by answers
    if let Some((e2, n2)) = list.iter().find(|(e2, n2)| *e2 != e && n2.pos.distance(n.pos) < 4.0) {
        if let Some(d2) = c.npcs.get(n2.idx as usize) {
            if !d2.chatter.is_empty() {
                let reply = d2.chatter[(time.elapsed_secs() * 5.7) as usize % d2.chatter.len()].get(l).to_string();
                ch.reply = Some((*e2, 2.0, reply));
            }
        }
    }
    ch.t = 7.0 + (time.elapsed_secs() * 13.0) % 6.0;
}

/// The crew member next to this point (for E and the prompt).
pub fn near(npcs: &Query<&Npc>, pos: Vec2) -> Option<u8> {
    npcs.iter().filter(|n| n.pos.distance(pos) < 1.7).min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos))).map(|n| n.idx)
}
