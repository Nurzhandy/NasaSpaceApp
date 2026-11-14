//! The look of the ship "Aurora": a bright, detailed interior (each room has its own floor,
//! furniture, lights), thick walls with door frames, the hull with a nose, solar wings and
//! engine flames, and space flying past in layers. Pure decoration: the rules don't see any of it.
//! Original art made from simple shapes for this project.

use super::{to_px, SC};
use crate::art::Art;
use crate::AppState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use ja_core::content::{Room, ShipLayout};
use ja_core::Lang;

/// Moves down the screen and wraps around: the ship flies "up" through space.
#[derive(Component)]
pub struct Drift {
    pub speed: f32,
    pub span: f32,
}
/// Glows on and off (lights, screens, LEDs).
#[derive(Component)]
pub struct Blink {
    pub speed: f32,
    pub phase: f32,
    pub base: f32,
}
/// Engine flame flicker.
#[derive(Component)]
pub struct Flame(pub f32);

type Ms<'a> = (&'a mut Assets<Mesh>, &'a mut Assets<ColorMaterial>);

fn hex(c: u32) -> Color {
    Color::srgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
fn hexa(c: u32, a: f32) -> Color {
    hex(c).with_alpha(a)
}

struct Painter<'w, 's, 'a> {
    c: &'a mut Commands<'w, 's>,
    sc: StateScoped<AppState>,
}

impl Painter<'_, '_, '_> {
    /// A rectangle in ship cells (x, y = bottom-left).
    fn cells(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color, z: f32) -> Entity {
        let p = to_px(x, y);
        self.c.spawn((Sprite { color, custom_size: Some(Vec2::new(w * SC, h * SC)), anchor: Anchor::BottomLeft, ..default() }, Transform::from_xyz(p.x, p.y, z), self.sc.clone())).id()
    }
    /// A rectangle in pixels (centre).
    fn px(&mut self, at: Vec2, size: Vec2, color: Color, z: f32) -> Entity {
        self.c.spawn((Sprite { color, custom_size: Some(size), ..default() }, Transform::from_xyz(at.x, at.y, z), self.sc.clone())).id()
    }
    fn circle(&mut self, m: &mut Ms, at: Vec2, r: f32, color: Color, z: f32) -> Entity {
        let mesh = m.0.add(Circle::new(r));
        let mat = m.1.add(ColorMaterial::from(color));
        self.c.spawn((Mesh2d(mesh), MeshMaterial2d(mat), Transform::from_xyz(at.x, at.y, z), self.sc.clone())).id()
    }
    fn blink(&mut self, e: Entity, speed: f32, phase: f32, base: f32) {
        self.c.entity(e).insert(Blink { speed, phase, base });
    }
}

/// Room colours: floor, floor detail, accent (lights, trims).
fn palette(id: &str) -> (u32, u32, u32) {
    match id {
        "cockpit" => (0x2b4c8c, 0x3a64b0, 0x6fd3ff),
        "observatory" => (0x4a2f7a, 0x5f3f96, 0xd99bff),
        "lab" => (0xd9e6ee, 0xbcd0dc, 0x3ec7c0),
        "habitat" => (0xb8643a, 0xcf7a4a, 0xffd36b),
        "gym" => (0x2f7d4f, 0x3b9a62, 0xb6ff7a),
        "life_support" => (0x1f6f78, 0x2a8a94, 0x7dfcff),
        "airlock" => (0x5a6070, 0x6b7385, 0xffcc33),
        "toilet" => (0x8fc6e8, 0xa9d8f2, 0xffffff),
        "engine" => (0x7a3328, 0x96402f, 0xff8a3d),
        _ => (0x3a4060, 0x4a5070, 0xffffff),
    }
}

/// Everything around and inside the ship except the consoles and the crew.
pub fn spawn(commands: &mut Commands, layout: &ShipLayout, meshes: &mut Assets<Mesh>, mats: &mut Assets<ColorMaterial>, art: &Art, l: Lang) {
    let mut m: Ms = (meshes, mats);
    let mut p = Painter { c: commands, sc: StateScoped(AppState::Ship) };
    space(&mut p, &mut m);
    hull(&mut p, &mut m, art);
    for r in &layout.rooms {
        floor(&mut p, r);
        props(&mut p, &mut m, r);
    }
    for r in &layout.rooms {
        walls(&mut p, layout, r);
        label(&mut p, art, r, l);
    }
}

// ---------------------------------------------------------------- space

fn space(p: &mut Painter, m: &mut Ms) {
    let span = 1400.0;
    p.px(Vec2::ZERO, Vec2::new(3200.0, 2200.0), hex(0x05061a), -20.0);
    // nebulas: big soft clouds drifting very slowly
    for (x, y, r, c, a) in [(-560.0, 260.0, 260.0, 0x6a2fa0, 0.18), (520.0, -180.0, 300.0, 0x1f5fa8, 0.16), (-420.0, -420.0, 200.0, 0xa02f6a, 0.12), (620.0, 420.0, 180.0, 0x2fa08a, 0.10)] {
        for k in 0..3 {
            let e = p.circle(m, Vec2::new(x + k as f32 * 40.0, y - k as f32 * 30.0), r * (1.0 - k as f32 * 0.25), hexa(c, a), -19.0 + k as f32 * 0.01);
            p.c.entity(e).insert(Drift { speed: 6.0, span });
        }
    }
    // a far planet with a ring passing by
    let pl = Vec2::new(600.0, 330.0);
    for (r, c, z) in [(92.0, 0xd9773f, -18.0), (86.0, 0xe8935a, -17.99), (40.0, 0xf2b07a, -17.98)] {
        let e = p.circle(m, pl + Vec2::new(-(92.0 - r) * 0.3, (92.0 - r) * 0.4), r, hex(c), z);
        p.c.entity(e).insert(Drift { speed: 10.0, span });
    }
    let ring = p.px(pl, Vec2::new(220.0, 8.0), hexa(0xf6dcae, 0.8), -17.9);
    p.c.entity(ring).insert((Drift { speed: 10.0, span }, Transform::from_xyz(pl.x, pl.y, -17.9).with_rotation(Quat::from_rotation_z(-0.3))));
    // three layers of stars: far = slow and dim, near = fast streaks
    let mut seed = 11u64;
    let mut rnd = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as f32 / (1u64 << 31) as f32
    };
    for (n, speed, size, alpha, z) in [(170, 18.0, Vec2::splat(2.0), 0.55, -16.0), (80, 45.0, Vec2::splat(3.0), 0.8, -15.0), (26, 160.0, Vec2::new(2.0, 18.0), 0.7, -14.0)] {
        for _ in 0..n {
            let at = Vec2::new(rnd() * 1800.0 - 900.0, rnd() * span - span / 2.0);
            let tint = [0xffffff, 0xcfe3ff, 0xfff1c9, 0xffd0e8][(rnd() * 4.0) as usize % 4];
            let e = p.px(at, size, hexa(tint, alpha), z);
            p.c.entity(e).insert(Drift { speed: speed * (0.8 + rnd() * 0.4), span });
            if rnd() < 0.25 {
                p.blink(e, 1.5 + rnd() * 2.0, rnd() * 6.0, alpha);
            }
        }
    }
}

// ---------------------------------------------------------------- hull

fn hull(p: &mut Painter, m: &mut Ms, art: &Art) {
    let (w, h) = (21.0, 15.0);
    let o = 0.55; // hull thickness in cells
    // solar wings (left and right)
    for side in [-1.0f32, 1.0] {
        let x0 = if side < 0.0 { -o - 4.6 } else { w + o + 0.4 };
        p.cells(if side < 0.0 { -o - 0.4 } else { w + o }, 4.4, 0.4, 1.4, hex(0x8c93a8), -8.0);
        for k in 0..2 {
            let y0 = 1.4 + k as f32 * 3.8;
            p.cells(x0, y0, 4.2, 3.4, hex(0x9aa3b8), -8.0);
            p.cells(x0 + 0.15, y0 + 0.15, 3.9, 3.1, hex(0x1d3f8f), -7.9);
            for gx in 0..4 {
                for gy in 0..3 {
                    let e = p.cells(x0 + 0.25 + gx as f32 * 0.95, y0 + 0.25 + gy as f32 * 1.0, 0.82, 0.85, hex(0x2f6fd8), -7.8);
                    if (gx + gy) % 3 == 0 {
                        p.blink(e, 0.6, (gx * 3 + gy) as f32, 1.0);
                    }
                }
            }
        }
    }
    // a rounded nose over the cockpit row, with a red band and a beacon
    let top = to_px(w / 2.0, h + o - 0.3);
    let half = w * SC / 2.0 + o * SC;
    for (rx, ry, c, z) in [(half, 3.0 * SC, 0xb8c0d4, -9.0), (half - 26.0, 2.55 * SC, 0x8a93ab, -8.95), (half * 0.62, 2.15 * SC, 0xe04a4a, -8.9), (half * 0.42, 1.55 * SC, 0x6fd3ff, -8.85)] {
        let mesh = m.0.add(Ellipse::new(rx, ry));
        let mat = m.1.add(ColorMaterial::from(hex(c)));
        p.c.spawn((Mesh2d(mesh), MeshMaterial2d(mat), Transform::from_xyz(top.x, top.y, z), p.sc.clone()));
    }
    let tip = p.circle(m, top + Vec2::new(0.0, 3.0 * SC + 4.0), 7.0, hex(0xff5a5a), -8.8);
    p.blink(tip, 2.5, 0.0, 1.0);
    // the hull plate
    p.cells(-o, -o, w + 2.0 * o, h + 2.0 * o, hex(0xb8c0d4), -7.0);
    p.cells(-o + 0.12, -o + 0.12, w + 2.0 * o - 0.24, h + 2.0 * o - 0.24, hex(0x8a93ab), -6.9);
    // rivets and a stripe
    for i in 0..22 {
        let x = -o + 0.3 + i as f32 * ((w + 2.0 * o - 0.6) / 21.0);
        p.cells(x, -o + 0.18, 0.12, 0.12, hex(0xdfe5f2), -6.8);
        p.cells(x, h + o - 0.3, 0.12, 0.12, hex(0xdfe5f2), -6.8);
    }
    p.cells(-o, h + o - 0.05, w + 2.0 * o, 0.12, hex(0xe04a4a), -6.8);
    // engines under the engine room and the airlock
    for (cx, big) in [(3.5f32, false), (10.5, true), (17.5, true)] {
        let nw = if big { 2.6 } else { 1.8 };
        p.cells(cx - nw / 2.0, -o - 1.0, nw, 1.0, hex(0x5a6178), -7.5);
        p.cells(cx - nw / 2.0 + 0.2, -o - 1.0, nw - 0.4, 0.25, hex(0x2b2f3e), -7.4);
        let f = to_px(cx, -o - 1.0);
        for (k, col) in [(0, Color::srgba(1.0, 0.55, 0.15, 0.85)), (1, Color::srgba(1.0, 0.9, 0.45, 0.95))] {
            let size = Vec2::new((nw - 0.6) * SC * (1.0 - k as f32 * 0.45), 2.4 * SC * (1.0 - k as f32 * 0.4));
            p.c.spawn((
                Sprite { image: art.sprite("flame"), color: col, custom_size: Some(size), anchor: Anchor::TopCenter, ..default() },
                Transform::from_xyz(f.x, f.y, -7.6 + k as f32 * 0.01),
                Flame(cx + k as f32),
                p.sc.clone(),
            ));
        }
    }
}

// ---------------------------------------------------------------- rooms

fn floor(p: &mut Painter, r: &Room) {
    let (base, detail, _) = palette(&r.id);
    let (x, y, w, h) = (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
    p.cells(x, y, w, h, hex(base), -3.0);
    match r.id.as_str() {
        // carpet with a border
        "habitat" => {
            p.cells(x + 0.5, y + 0.5, w - 1.0, h - 1.0, hex(detail), -2.9);
            p.cells(x + 0.7, y + 0.7, w - 1.4, h - 1.4, hex(0xb8643a), -2.85);
            for i in 0..6 {
                p.cells(x + 0.9 + i as f32 * 0.9, y + 0.9, 0.12, h - 1.8, hexa(0xffd36b, 0.18), -2.8);
            }
        }
        // small tiles
        "lab" | "toilet" => {
            for gx in 0..(w as i32 * 2) {
                for gy in 0..(h as i32 * 2) {
                    if (gx + gy) % 2 == 0 {
                        p.cells(x + gx as f32 * 0.5, y + gy as f32 * 0.5, 0.5, 0.5, hex(detail), -2.9);
                    }
                }
            }
        }
        // rubber mat with court lines
        "gym" => {
            p.cells(x + 0.6, y + 0.6, w - 1.2, h - 1.2, hex(detail), -2.9);
            p.cells(x + w / 2.0 - 0.05, y + 0.6, 0.1, h - 1.2, hexa(0xffffff, 0.5), -2.85);
            p.cells(x + 0.6, y + h / 2.0 - 0.05, w - 1.2, 0.1, hexa(0xffffff, 0.5), -2.85);
        }
        // metal grating
        "life_support" | "engine" => {
            for gx in 0..(w as i32 * 3) {
                p.cells(x + gx as f32 / 3.0 + 0.05, y, 0.08, h, hexa(0x000000, 0.22), -2.9);
            }
            for gy in 0..(h as i32) {
                p.cells(x, y + gy as f32 + 0.5, w, 0.06, hexa(0xffffff, 0.12), -2.88);
            }
        }
        // big plates with seams
        _ => {
            for gx in 0..(w as i32) {
                for gy in 0..(h as i32) {
                    let c = if (gx + gy) % 2 == 0 { hex(detail) } else { hex(base) };
                    p.cells(x + gx as f32 + 0.04, y + gy as f32 + 0.04, 0.92, 0.92, c, -2.9);
                    p.cells(x + gx as f32 + 0.12, y + gy as f32 + 0.78, 0.1, 0.1, hexa(0xffffff, 0.25), -2.85);
                }
            }
        }
    }
    // soft light pool in the middle of the room
    let (_, _, accent) = palette(&r.id);
    p.cells(x + 1.0, y + 1.0, w - 2.0, h - 2.0, hexa(accent, 0.05), -2.7);
}

/// Furniture, machines and lights. Placed around the consoles and the doorways.
fn props(p: &mut Painter, m: &mut Ms, r: &Room) {
    let (x, y) = (r.x as f32, r.y as f32);
    let (_, _, accent) = palette(&r.id);
    let z = -2.0;
    // wall lamps (top wall)
    for lx in [1.2, 5.8] {
        let e = p.cells(x + lx, y + 4.68, 0.6, 0.14, hex(accent), z + 0.3);
        p.blink(e, 0.8, lx, 1.0);
        p.cells(x + lx - 0.3, y + 4.0, 1.2, 0.68, hexa(accent, 0.08), z);
    }
    match r.id.as_str() {
        "cockpit" => {
            // front windows with stars
            p.cells(x + 0.4, y + 4.4, 6.2, 0.55, hex(0x0b0f2e), z);
            for i in 0..12 {
                p.cells(x + 0.6 + i as f32 * 0.5, y + 4.55 + (i % 3) as f32 * 0.12, 0.07, 0.07, hex(0xffffff), z + 0.1);
            }
            // control desk with buttons
            p.cells(x + 0.5, y + 3.75, 6.0, 0.55, hex(0x1c2a4f), z + 0.1);
            for i in 0..10 {
                let c = [0xff5a5a, 0x5aff8a, 0xffd35a, 0x5ac8ff][i % 4];
                let e = p.cells(x + 0.7 + i as f32 * 0.58, y + 3.95, 0.2, 0.16, hex(c), z + 0.2);
                p.blink(e, 1.0 + (i % 3) as f32, i as f32, 1.0);
            }
            // two pilot seats
            for sx in [1.3, 3.7] {
                p.cells(x + sx, y + 2.2, 0.9, 0.8, hex(0x8a2f2f), z);
                p.cells(x + sx + 0.1, y + 2.9, 0.7, 0.25, hex(0xb84444), z + 0.1);
            }
            // big screen on the left wall
            let e = p.cells(x + 0.12, y + 0.7, 0.25, 1.6, hex(0x6fd3ff), z + 0.1);
            p.blink(e, 0.7, 1.0, 0.9);
        }
        "observatory" => {
            // a round window full of stars and a blue planet edge
            let c = to_px(x + 1.7, y + 2.2);
            p.circle(m, c, 1.25 * SC, hex(0xc7cde0), z);
            p.circle(m, c, 1.1 * SC, hex(0x070a24), z + 0.05);
            p.circle(m, c + Vec2::new(18.0, -16.0), 0.55 * SC, hex(0x2f7fd8), z + 0.1);
            p.circle(m, c + Vec2::new(22.0, -10.0), 0.3 * SC, hex(0x4fbf6a), z + 0.12);
            for (dx, dy) in [(-20.0, 18.0), (-8.0, 26.0), (6.0, 14.0), (-24.0, -4.0), (10.0, 28.0)] {
                let e = p.px(c + Vec2::new(dx, dy), Vec2::splat(2.5), hex(0xffffff), z + 0.15);
                p.blink(e, 2.0, dx, 1.0);
            }
            // telescope on a tripod
            p.cells(x + 5.6, y + 3.6, 1.2, 0.3, hex(0xe0e4f0), z);
            p.cells(x + 5.9, y + 3.2, 0.12, 0.45, hex(0x9aa0b8), z);
            // star map on the floor
            for (dx, dy) in [(3.0, 0.6), (3.6, 1.0), (4.2, 0.7), (4.6, 1.2)] {
                p.cells(x + dx, y + dy, 0.15, 0.15, hexa(0xd99bff, 0.8), z);
            }
        }
        "lab" => {
            // white benches with coloured flasks
            p.cells(x + 0.3, y + 0.3, 2.6, 0.7, hex(0xf6fafc), z);
            p.cells(x + 4.2, y + 0.3, 2.5, 0.7, hex(0xf6fafc), z);
            for (i, c) in [0x3ec7c0, 0xff6fb5, 0xffd35a, 0x7a8cff, 0x5aff8a].iter().enumerate() {
                let bx = if i < 3 { x + 0.5 + i as f32 * 0.8 } else { x + 4.5 + (i - 3) as f32 * 0.9 };
                p.cells(bx, y + 0.5, 0.3, 0.4, hex(*c), z + 0.1);
            }
            // a microscope and a glowing sample fridge
            p.cells(x + 5.9, y + 3.0, 0.8, 1.6, hex(0xdce6ee), z);
            let e = p.cells(x + 6.0, y + 3.1, 0.6, 1.4, hexa(0x7dfcff, 0.6), z + 0.1);
            p.blink(e, 0.9, 0.5, 0.6);
            p.cells(x + 0.4, y + 3.2, 0.9, 0.9, hex(0xb6c6d2), z);
            let e = p.circle(m, to_px(x + 0.85, y + 3.65), 7.0, hex(0x3ec7c0), z + 0.1);
            p.blink(e, 1.4, 0.0, 1.0);
        }
        "habitat" => {
            // two sleeping pods
            for py in [0.3, 1.3] {
                p.cells(x + 4.6, y + py, 2.1, 0.8, hex(0x3a3f5a), z);
                p.cells(x + 4.7, y + py + 0.1, 1.5, 0.6, hex(0x6f8cff), z + 0.1);
                p.cells(x + 6.2, y + py + 0.1, 0.4, 0.6, hex(0xffffff), z + 0.1);
            }
            // round table with chairs
            let c = to_px(x + 5.3, y + 3.5);
            for (dx, dy) in [(-0.85, 0.0), (0.85, 0.0), (0.0, -0.85)] {
                p.circle(m, c + Vec2::new(dx * SC, dy * SC), 0.28 * SC, hex(0xffd36b), z);
            }
            p.circle(m, c, 0.6 * SC, hex(0xf0e6d2), z + 0.1);
            p.circle(m, c + Vec2::new(6.0, 4.0), 0.12 * SC, hex(0xff6f5a), z + 0.2);
            // plant
            p.cells(x + 0.3, y + 0.3, 0.6, 0.6, hex(0x8a5a3a), z);
            p.circle(m, to_px(x + 0.6, y + 1.0), 0.4 * SC, hex(0x4fbf6a), z + 0.1);
        }
        "gym" => {
            // treadmill belt and weights
            p.cells(x + 0.3, y + 0.4, 1.8, 0.5, hex(0x22252f), z);
            for i in 0..4 {
                p.cells(x + 0.4 + i as f32 * 0.45, y + 0.25, 0.16, 0.8, hex(0xb6ff7a), z + 0.1);
            }
            p.cells(x + 5.0, y + 0.4, 1.7, 0.6, hex(0x22252f), z);
            p.cells(x + 5.2, y + 0.5, 1.3, 0.4, hex(0x4a4f63), z + 0.1);
            // bike
            p.circle(m, to_px(x + 5.6, y + 3.6), 0.35 * SC, hex(0x22252f), z);
            p.circle(m, to_px(x + 6.4, y + 3.6), 0.35 * SC, hex(0x22252f), z);
            p.cells(x + 5.6, y + 3.55, 0.8, 0.12, hex(0xff8a3d), z + 0.1);
            // exercise bands on the wall
            for i in 0..3 {
                p.cells(x + 0.25 + i as f32 * 0.3, y + 3.0, 0.1, 1.3, hex([0xff5a5a, 0xffd35a, 0x5ac8ff][i]), z);
            }
        }
        "life_support" => {
            // tanks: oxygen (green), nitrogen (blue), water (white)
            for (i, c) in [0x4fbf6a, 0x3a7bd5, 0xe8f2ff].iter().enumerate() {
                let at = to_px(x + 0.7, y + 0.8 + i as f32 * 1.15);
                p.circle(m, at, 0.48 * SC, hex(0x2a3442), z);
                p.circle(m, at, 0.38 * SC, hex(*c), z + 0.1);
            }
            // pipes along the top
            for (i, c) in [0x7dfcff, 0xffcc33].iter().enumerate() {
                p.cells(x + 1.4, y + 4.3 - i as f32 * 0.2, 5.4, 0.12, hex(*c), z);
            }
            // green plant rack (helps the air)
            p.cells(x + 5.9, y + 2.2, 0.9, 1.6, hex(0x2a3442), z);
            for i in 0..3 {
                p.circle(m, to_px(x + 6.35, y + 2.5 + i as f32 * 0.5), 0.22 * SC, hex(0x4fbf6a), z + 0.1);
            }
        }
        "airlock" => {
            // the hatch on the outer wall with hazard stripes
            for i in 0..6 {
                p.cells(x + 0.05, y + 0.9 + i as f32 * 0.4, 0.35, 0.4, hex(if i % 2 == 0 { 0xffcc33 } else { 0x22252f }), z);
            }
            let e = p.cells(x + 0.12, y + 3.4, 0.2, 0.3, hex(0xff5a5a), z + 0.1);
            p.blink(e, 2.0, 0.0, 1.0);
            // cargo crates
            for (cx, cy, c) in [(3.2, 0.3, 0xd9883a), (4.0, 0.3, 0xc0703a), (3.6, 1.05, 0xe8a04a)] {
                p.cells(x + cx, y + cy, 0.7, 0.7, hex(c), z);
                p.cells(x + cx + 0.08, y + cy + 0.3, 0.54, 0.08, hexa(0x000000, 0.3), z + 0.1);
            }
            // suit lockers
            for i in 0..2 {
                p.cells(x + 3.4 + i as f32 * 0.8, y + 3.7, 0.7, 1.0, hex(0xdfe5f2), z);
                p.cells(x + 3.55 + i as f32 * 0.8, y + 3.85, 0.4, 0.5, hex(0xff8a3d), z + 0.1);
            }
        }
        "toilet" => {
            // sink and mirror, a vent fan
            p.cells(x + 0.4, y + 3.9, 1.4, 0.55, hex(0xffffff), z);
            p.cells(x + 0.7, y + 4.0, 0.8, 0.3, hex(0x8fc6e8), z + 0.1);
            p.cells(x + 5.2, y + 0.4, 1.4, 1.2, hex(0xe8f2ff), z);
            let e = p.circle(m, to_px(x + 5.9, y + 1.0), 0.45 * SC, hex(0x5a6070), z + 0.1);
            p.blink(e, 3.0, 0.0, 1.0);
            p.cells(x + 0.4, y + 0.4, 0.5, 1.4, hex(0x3ec7c0), z);
        }
        "engine" => {
            // the glowing reactor core
            let c = to_px(x + 1.5, y + 2.5);
            p.circle(m, c, 1.0 * SC, hex(0x3a2420), z);
            let e = p.circle(m, c, 0.75 * SC, hex(0xff8a3d), z + 0.1);
            p.blink(e, 1.6, 0.0, 1.0);
            let e = p.circle(m, c, 0.4 * SC, hex(0xffe08a), z + 0.2);
            p.blink(e, 2.4, 1.0, 1.0);
            // pipes
            for (i, c) in [0xff8a3d, 0xffcc33, 0xb0b8c8].iter().enumerate() {
                p.cells(x + 2.6, y + 1.9 + i as f32 * 0.35, 2.0, 0.15, hex(*c), z);
            }
            // warning stripes near the core
            for i in 0..5 {
                p.cells(x + 0.3 + i as f32 * 0.5, y + 0.25, 0.25, 0.2, hex(0xffcc33), z);
            }
        }
        _ => {}
    }
}

fn walls(p: &mut Painter, layout: &ShipLayout, r: &Room) {
    let t = 0.3;
    let edges = [
        ((r.x as f32, r.y as f32), (r.x as f32, (r.y + r.h) as f32)),
        (((r.x + r.w) as f32, r.y as f32), ((r.x + r.w) as f32, (r.y + r.h) as f32)),
        ((r.x as f32, r.y as f32), ((r.x + r.w) as f32, r.y as f32)),
        ((r.x as f32, (r.y + r.h) as f32), ((r.x + r.w) as f32, (r.y + r.h) as f32)),
    ];
    for (a, b) in edges {
        let vertical = a.0 == b.0;
        let (lo, hi) = if vertical { (a.1, b.1) } else { (a.0, b.0) };
        let mut cuts: Vec<(f32, f32)> = layout
            .doors
            .iter()
            .filter(|d| if vertical { (d.0 - a.0).abs() < 0.01 && d.1 > lo && d.1 < hi } else { (d.1 - a.1).abs() < 0.01 && d.0 > lo && d.0 < hi })
            .map(|d| if vertical { (d.1 - 0.8, d.1 + 0.8) } else { (d.0 - 0.8, d.0 + 0.8) })
            .collect();
        cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut s = lo;
        let mut pieces = vec![];
        for (c0, c1) in &cuts {
            pieces.push((s, *c0));
            s = *c1;
        }
        pieces.push((s, hi));
        for (p0, p1) in pieces {
            if p1 - p0 < 0.05 {
                continue;
            }
            let (at, size) = if vertical { (to_px(a.0, (p0 + p1) / 2.0), Vec2::new(t * SC, (p1 - p0) * SC + t * SC)) } else { (to_px((p0 + p1) / 2.0, a.1), Vec2::new((p1 - p0) * SC + t * SC, t * SC)) };
            p.px(at, size, hex(0x2a2f45), -0.6);
            // light top edge: walls look raised
            let hl = if vertical { Vec2::new(size.x * 0.4, size.y - 4.0) } else { Vec2::new(size.x - 4.0, size.y * 0.4) };
            p.px(at + if vertical { Vec2::new(-size.x * 0.2, 0.0) } else { Vec2::new(0.0, size.y * 0.2) }, hl, hex(0x9aa3c4), -0.55);
        }
        // door frames: yellow posts and a floor plate
        for (c0, c1) in cuts {
            for edge in [c0, c1] {
                let at = if vertical { to_px(a.0, edge) } else { to_px(edge, a.1) };
                p.px(at, Vec2::splat(t * SC + 4.0), hex(0xffcc33), -0.5);
            }
            let mid = (c0 + c1) / 2.0;
            let (at, size) = if vertical { (to_px(a.0, mid), Vec2::new(t * SC, (c1 - c0) * SC)) } else { (to_px(mid, a.1), Vec2::new((c1 - c0) * SC, t * SC)) };
            p.px(at, size, hexa(0x6fd3ff, 0.35), -0.58);
        }
    }
}

fn label(p: &mut Painter, art: &Art, r: &Room, l: Lang) {
    let at = to_px(r.x as f32 + 0.25, (r.y + r.h) as f32 - 0.42);
    let name = r.name.get(l);
    let w = name.chars().count() as f32 * 8.6 + 14.0;
    p.c.spawn((Sprite { color: hexa(0x0b0e22, 0.72), custom_size: Some(Vec2::new(w, 20.0)), anchor: Anchor::CenterLeft, ..default() }, Transform::from_xyz(at.x - 4.0, at.y, -0.3), p.sc.clone()));
    p.c.spawn((Text2d::new(name), TextFont { font: art.body.clone(), font_size: 15.0, ..default() }, TextColor(hex(0xffffff)), Anchor::CenterLeft, Transform::from_xyz(at.x + 2.0, at.y, -0.2), p.sc.clone()));
}

/// Space flies past; lights glow; flames flicker.
pub fn animate(time: Res<Time>, mut drift: Query<(&mut Transform, &Drift)>, mut blink: Query<(&mut Sprite, &Blink), Without<Drift>>, mut blink_star: Query<(&mut Sprite, &Blink), With<Drift>>, mut mats: ResMut<Assets<ColorMaterial>>, blink_mesh: Query<(&MeshMaterial2d<ColorMaterial>, &Blink)>, mut flames: Query<(&mut Transform, &Flame), Without<Drift>>) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    for (mut tr, d) in &mut drift {
        tr.translation.y -= d.speed * dt;
        if tr.translation.y < -d.span / 2.0 - 200.0 {
            tr.translation.y += d.span + 400.0;
        }
    }
    let glow = |b: &Blink| b.base * (0.55 + 0.45 * (t * b.speed + b.phase).sin().abs());
    for (mut s, b) in blink.iter_mut().chain(blink_star.iter_mut()) {
        let a = glow(b);
        s.color.set_alpha(a);
    }
    for (h, b) in &blink_mesh {
        if let Some(m) = mats.get_mut(&h.0) {
            m.color.set_alpha(glow(b));
        }
    }
    for (mut tr, f) in &mut flames {
        let k = 1.0 + 0.18 * (t * 18.0 + f.0 * 3.0).sin() + 0.08 * (t * 31.0 + f.0).sin();
        tr.scale = Vec3::new(1.0 + 0.05 * (t * 23.0 + f.0).sin(), k, 1.0);
    }
}
