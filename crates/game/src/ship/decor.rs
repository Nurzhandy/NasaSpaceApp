//! The look of the ship "Aurora": a bright, detailed interior (each room has its own floor,
//! furniture, machines, lights and handrails), thick walls, sliding metal doors, the hull with a
//! nose, solar wings and engine flames, and space flying past in layers. Pure decoration: the
//! rules don't see any of it. Original art made from simple shapes for this project.
//!
//! Rooms are 10×7 cells. Furniture keeps to the corners: the cross from door to door and the
//! floor in front of every console stay free to walk.

use super::{to_px, SC, SHIP_H, SHIP_W};
use crate::art::Art;
use crate::AppState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use ja_core::content::{Room, ShipLayout};
use ja_core::ship::DOOR_HALF;
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
/// One half of a sliding metal door: slides into the wall when someone comes close.
#[derive(Component)]
pub struct DoorLeaf {
    /// door centre in cells
    pub at: Vec2,
    pub vertical: bool,
    pub side: f32,
    pub open: f32,
}
/// The little light on a door frame: red closed, green open.
#[derive(Component)]
pub struct DoorLamp(pub Vec2);

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
    /// A circle at a cell position (centre, radius in cells).
    fn disc(&mut self, m: &mut Ms, x: f32, y: f32, r: f32, c: u32, z: f32) -> Entity {
        self.circle(m, to_px(x, y), r * SC, hex(c), z)
    }
    fn icon(&mut self, art: &Art, n: &str, x: f32, y: f32, size: f32, z: f32) {
        let p = to_px(x, y);
        self.c.spawn((Sprite { image: art.icon(n), custom_size: Some(Vec2::splat(size * SC)), ..default() }, Transform::from_xyz(p.x, p.y, z), self.sc.clone()));
    }
    fn blink(&mut self, e: Entity, speed: f32, phase: f32, base: f32) {
        self.c.entity(e).insert(Blink { speed, phase, base });
    }
    /// A box with a lighter top edge and a shadow: furniture looks solid.
    fn block(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, z: f32) {
        self.cells(x + 0.08, y - 0.1, w, h, hexa(0x000000, 0.3), z - 0.02);
        self.cells(x, y, w, h, hex(c), z);
        self.cells(x, y + h - 0.12, w, 0.12, hexa(0xffffff, 0.25), z + 0.01);
    }
    /// A small screen that glows.
    fn screen(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, z: f32, phase: f32) {
        self.cells(x - 0.06, y - 0.06, w + 0.12, h + 0.12, hex(0x1c2030), z);
        let e = self.cells(x, y, w, h, hex(c), z + 0.01);
        self.blink(e, 0.7, phase, 0.95);
        for i in 0..3 {
            self.cells(x + 0.1, y + h - 0.2 - i as f32 * 0.18, w * (0.4 + 0.15 * i as f32), 0.06, hexa(0xffffff, 0.55), z + 0.02);
        }
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

/// Everything around and inside the ship except the consoles, the crew and the pet.
pub fn spawn(commands: &mut Commands, layout: &ShipLayout, meshes: &mut Assets<Mesh>, mats: &mut Assets<ColorMaterial>, art: &Art, l: Lang) {
    let mut m: Ms = (meshes, mats);
    let mut p = Painter { c: commands, sc: StateScoped(AppState::Ship) };
    space(&mut p, &mut m);
    hull(&mut p, &mut m, art);
    for r in &layout.rooms {
        floor(&mut p, r);
        props(&mut p, &mut m, art, r);
    }
    for r in &layout.rooms {
        walls(&mut p, layout, r);
        label(&mut p, art, r, l);
    }
    doors(&mut p, layout);
}

// ---------------------------------------------------------------- space

fn space(p: &mut Painter, m: &mut Ms) {
    let span = 2600.0;
    p.px(Vec2::ZERO, Vec2::new(5000.0, 3600.0), hex(0x05061a), -20.0);
    // nebulas: big soft clouds drifting very slowly
    for (x, y, r, c, a) in [(-900.0, 400.0, 340.0, 0x6a2fa0, 0.18), (850.0, -300.0, 380.0, 0x1f5fa8, 0.16), (-700.0, -700.0, 260.0, 0xa02f6a, 0.12), (900.0, 700.0, 240.0, 0x2fa08a, 0.10)] {
        for k in 0..3 {
            let e = p.circle(m, Vec2::new(x + k as f32 * 50.0, y - k as f32 * 40.0), r * (1.0 - k as f32 * 0.25), hexa(c, a), -19.0 + k as f32 * 0.01);
            p.c.entity(e).insert(Drift { speed: 6.0, span });
        }
    }
    // a far planet with a ring passing by
    let pl = Vec2::new(950.0, 500.0);
    for (r, c, z) in [(120.0, 0xd9773f, -18.0), (112.0, 0xe8935a, -17.99), (52.0, 0xf2b07a, -17.98)] {
        let e = p.circle(m, pl + Vec2::new(-(120.0 - r) * 0.3, (120.0 - r) * 0.4), r, hex(c), z);
        p.c.entity(e).insert(Drift { speed: 10.0, span });
    }
    let ring = p.px(pl, Vec2::new(290.0, 10.0), hexa(0xf6dcae, 0.8), -17.9);
    p.c.entity(ring).insert((Drift { speed: 10.0, span }, Transform::from_xyz(pl.x, pl.y, -17.9).with_rotation(Quat::from_rotation_z(-0.3))));
    // three layers of stars: far = slow and dim, near = fast streaks
    let mut seed = 11u64;
    let mut rnd = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as f32 / (1u64 << 31) as f32
    };
    for (n, speed, size, alpha, z) in [(320, 18.0, Vec2::splat(2.0), 0.55, -16.0), (150, 45.0, Vec2::splat(3.0), 0.8, -15.0), (45, 170.0, Vec2::new(2.0, 20.0), 0.7, -14.0)] {
        for _ in 0..n {
            let at = Vec2::new(rnd() * 3200.0 - 1600.0, rnd() * span - span / 2.0);
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
    let (w, h) = (SHIP_W, SHIP_H);
    let o = 0.6; // hull thickness in cells
    // solar wings (left and right), two panels each
    for side in [-1.0f32, 1.0] {
        let x0 = if side < 0.0 { -o - 6.2 } else { w + o + 0.6 };
        p.cells(if side < 0.0 { -o - 0.6 } else { w + o }, 9.6, 0.6, 1.8, hex(0x8c93a8), -8.0);
        for k in 0..2 {
            let y0 = 3.0 + k as f32 * 8.0;
            p.cells(x0, y0, 5.6, 7.0, hex(0x9aa3b8), -8.0);
            p.cells(x0 + 0.15, y0 + 0.15, 5.3, 6.7, hex(0x1d3f8f), -7.9);
            for gx in 0..5 {
                for gy in 0..6 {
                    let e = p.cells(x0 + 0.25 + gx as f32 * 1.03, y0 + 0.25 + gy as f32 * 1.08, 0.92, 0.95, hex(0x2f6fd8), -7.8);
                    if (gx + gy) % 3 == 0 {
                        p.blink(e, 0.6, (gx * 3 + gy) as f32, 1.0);
                    }
                }
            }
        }
    }
    // a rounded nose over the cockpit row, with a red band and a window
    let top = to_px(w / 2.0, h + o - 0.3);
    let half = w * SC / 2.0 + o * SC;
    for (rx, ry, c, z) in [(half, 4.0 * SC, 0xb8c0d4, -9.0), (half - 30.0, 3.45 * SC, 0x8a93ab, -8.95), (half * 0.62, 2.9 * SC, 0xe04a4a, -8.9), (half * 0.42, 2.1 * SC, 0x6fd3ff, -8.85)] {
        let mesh = m.0.add(Ellipse::new(rx, ry));
        let mat = m.1.add(ColorMaterial::from(hex(c)));
        p.c.spawn((Mesh2d(mesh), MeshMaterial2d(mat), Transform::from_xyz(top.x, top.y, z), p.sc.clone()));
    }
    let tip = p.circle(m, top + Vec2::new(0.0, 4.0 * SC + 4.0), 9.0, hex(0xff5a5a), -8.8);
    p.blink(tip, 2.5, 0.0, 1.0);
    // the hull plate
    p.cells(-o, -o, w + 2.0 * o, h + 2.0 * o, hex(0xb8c0d4), -7.0);
    p.cells(-o + 0.12, -o + 0.12, w + 2.0 * o - 0.24, h + 2.0 * o - 0.24, hex(0x8a93ab), -6.9);
    for i in 0..31 {
        let x = -o + 0.3 + i as f32 * ((w + 2.0 * o - 0.6) / 30.0);
        p.cells(x, -o + 0.2, 0.12, 0.12, hex(0xdfe5f2), -6.8);
        p.cells(x, h + o - 0.32, 0.12, 0.12, hex(0xdfe5f2), -6.8);
    }
    p.cells(-o, h + o - 0.05, w + 2.0 * o, 0.14, hex(0xe04a4a), -6.8);
    // engines under the bottom row
    for (cx, nw) in [(5.0f32, 2.4f32), (15.0, 3.4), (25.0, 3.4)] {
        p.cells(cx - nw / 2.0, -o - 1.3, nw, 1.3, hex(0x5a6178), -7.5);
        p.cells(cx - nw / 2.0 + 0.2, -o - 1.3, nw - 0.4, 0.3, hex(0x2b2f3e), -7.4);
        let f = to_px(cx, -o - 1.3);
        for (k, col) in [(0, Color::srgba(1.0, 0.55, 0.15, 0.85)), (1, Color::srgba(1.0, 0.9, 0.45, 0.95))] {
            let size = Vec2::new((nw - 0.6) * SC * (1.0 - k as f32 * 0.45), 3.0 * SC * (1.0 - k as f32 * 0.4));
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
    let (base, detail, accent) = palette(&r.id);
    let (x, y, w, h) = (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
    p.cells(x, y, w, h, hex(base), -3.0);
    match r.id.as_str() {
        // carpet with a border and a pattern
        "habitat" => {
            p.cells(x + 0.6, y + 0.6, w - 1.2, h - 1.2, hex(detail), -2.9);
            p.cells(x + 0.8, y + 0.8, w - 1.6, h - 1.6, hex(0xb8643a), -2.85);
            for i in 0..9 {
                p.cells(x + 1.0 + i as f32 * 0.95, y + 1.0, 0.12, h - 2.0, hexa(0xffd36b, 0.18), -2.8);
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
            p.cells(x + w / 2.0 - 1.0, y + h / 2.0 - 1.0, 2.0, 2.0, hexa(0xffffff, 0.12), -2.84);
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
        // big plates with seams and bolts
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
    // the walkway between the doors (a slightly lighter path) shows where to go
    p.cells(x + w / 2.0 - 1.0, y, 2.0, h, hexa(0xffffff, 0.04), -2.7);
    p.cells(x, y + h / 2.0 - 1.0, w, 2.0, hexa(0xffffff, 0.04), -2.7);
    // light pool, shadows along the walls (depth)
    p.cells(x + 1.5, y + 1.5, w - 3.0, h - 3.0, hexa(accent, 0.05), -2.7);
    p.cells(x, y + h - 0.5, w, 0.5, hexa(0x000000, 0.28), -2.65);
    p.cells(x, y, 0.35, h, hexa(0x000000, 0.18), -2.65);
    p.cells(x + w - 0.35, y, 0.35, h, hexa(0x000000, 0.12), -2.65);
    // floor vent and ISS-style handrails (things to hold on to in weightlessness)
    p.cells(x + w - 2.0, y + 0.25, 1.0, 0.32, hex(0x2a2f45), -2.6);
    for i in 0..5 {
        p.cells(x + w - 1.95 + i as f32 * 0.19, y + 0.29, 0.1, 0.24, hex(0x6a7390), -2.55);
    }
    for hx in [1.6, w - 3.2] {
        p.cells(x + hx, y + h - 0.68, 1.6, 0.14, hex(0xff9a3d), -2.5);
        p.cells(x + hx, y + h - 0.72, 0.12, 0.22, hex(0xc06a20), -2.49);
        p.cells(x + hx + 1.48, y + h - 0.72, 0.12, 0.22, hex(0xc06a20), -2.49);
    }
    // ceiling lamps (top wall) with a light cone
    for lx in [2.2, w - 2.8] {
        let e = p.cells(x + lx, y + h - 0.3, 0.8, 0.16, hex(accent), -2.4);
        p.blink(e, 0.8, lx, 1.0);
        p.cells(x + lx - 0.4, y + h - 1.4, 1.6, 1.1, hexa(accent, 0.07), -2.45);
    }
    // cable trays along the bottom wall
    p.cells(x + 0.5, y + 0.06, w - 1.0, 0.1, hex(0x3a4058), -2.6);
    p.cells(x + 0.5, y + 0.1, w - 1.0, 0.04, hex(0xffcc33), -2.59);
}

/// Furniture, machines and lights, room by room (cells relative to the room's corner).
fn props(p: &mut Painter, m: &mut Ms, art: &Art, r: &Room) {
    let (x, y) = (r.x as f32, r.y as f32);
    let z = -2.0;
    match r.id.as_str() {
        "cockpit" => {
            // the big front windows with stars, under the nose
            p.cells(x + 0.3, y + 6.45, 9.4, 0.5, hex(0x0b0f2e), z);
            for i in 0..22 {
                p.cells(x + 0.5 + i as f32 * 0.42, y + 6.55 + (i % 3) as f32 * 0.12, 0.07, 0.07, hex(0xffffff), z + 0.1);
            }
            // the long control desk with blinking buttons
            p.block(x + 0.4, y + 5.4, 3.4, 0.5, 0x1c2a4f, z);
            p.block(x + 5.4, y + 5.4, 3.2, 0.5, 0x1c2a4f, z);
            for i in 0..14 {
                let c = [0xff5a5a, 0x5aff8a, 0xffd35a, 0x5ac8ff][i % 4];
                let bx = if i < 7 { x + 0.6 + i as f32 * 0.45 } else { x + 5.6 + (i - 7) as f32 * 0.42 };
                let e = p.cells(bx, y + 5.58, 0.2, 0.16, hex(c), z + 0.2);
                p.blink(e, 1.0 + (i % 3) as f32, i as f32, 1.0);
            }
            // two pilot seats with straps
            for sx in [2.4, 6.6] {
                p.block(x + sx, y + 4.2, 1.0, 0.85, 0x8a2f2f, z);
                p.cells(x + sx + 0.1, y + 4.85, 0.8, 0.28, hex(0xb84444), z + 0.1);
                p.cells(x + sx + 0.45, y + 4.25, 0.1, 0.75, hex(0xffcc33), z + 0.11);
            }
            // a navigation screen on the hull wall and a map table
            p.screen(x + 0.15, y + 1.0, 0.3, 2.0, 0x6fd3ff, z, 1.0);
            p.block(x + 7.0, y + 0.5, 2.4, 1.2, 0x2a3a6a, z);
            p.cells(x + 7.15, y + 0.65, 2.1, 0.9, hex(0x3a8bff), z + 0.05);
            p.disc(m, x + 7.8, y + 1.1, 0.25, 0x5aff8a, z + 0.1);
            p.icon(art, "earth", x + 8.6, y + 1.1, 0.5, z + 0.1);
        }
        "observatory" => {
            // a big round window full of stars, Earth at its edge
            let c = to_px(x + 2.3, y + 1.9);
            p.circle(m, c, 1.55 * SC, hex(0xc7cde0), z);
            p.circle(m, c, 1.38 * SC, hex(0x070a24), z + 0.05);
            p.circle(m, c + Vec2::new(26.0, -22.0), 0.7 * SC, hex(0x2f7fd8), z + 0.1);
            p.circle(m, c + Vec2::new(32.0, -14.0), 0.36 * SC, hex(0x4fbf6a), z + 0.12);
            for (dx, dy) in [(-30.0, 28.0), (-12.0, 40.0), (8.0, 20.0), (-34.0, -6.0), (14.0, 44.0), (30.0, 30.0)] {
                let e = p.px(c + Vec2::new(dx, dy), Vec2::splat(3.0), hex(0xffffff), z + 0.15);
                p.blink(e, 2.0, dx, 1.0);
            }
            // the telescope on a tripod
            p.cells(x + 6.6, y + 1.5, 1.8, 0.4, hex(0xe0e4f0), z);
            p.cells(x + 8.3, y + 1.4, 0.4, 0.6, hex(0xb0b8c8), z);
            p.cells(x + 7.3, y + 0.6, 0.12, 0.9, hex(0x9aa0b8), z);
            p.cells(x + 6.9, y + 0.5, 0.9, 0.1, hex(0x9aa0b8), z);
            // a star chart table and two bean-bag seats
            p.block(x + 6.6, y + 4.9, 2.2, 0.9, 0x2a1f4a, z);
            for (dx, dy) in [(6.9, 5.2), (7.4, 5.5), (7.9, 5.25), (8.4, 5.6)] {
                p.cells(x + dx, y + dy, 0.12, 0.12, hexa(0xd99bff, 0.9), z + 0.1);
            }
            p.disc(m, x + 1.4, y + 5.6, 0.45, 0xd99bff, z);
            p.disc(m, x + 2.6, y + 5.7, 0.45, 0x9b6bff, z);
        }
        "lab" => {
            // white benches with coloured flasks
            p.block(x + 0.4, y + 0.3, 3.4, 0.8, 0xf6fafc, z);
            p.block(x + 6.2, y + 0.3, 3.4, 0.8, 0xf6fafc, z);
            for (i, c) in [0x3ec7c0, 0xff6fb5, 0xffd35a, 0x7a8cff, 0x5aff8a, 0xff8a3d, 0x3ec7c0].iter().enumerate() {
                let bx = if i < 4 { x + 0.6 + i as f32 * 0.8 } else { x + 6.5 + (i - 4) as f32 * 0.9 };
                p.cells(bx, y + 0.55, 0.3, 0.45, hex(*c), z + 0.1);
                p.cells(bx + 0.08, y + 0.95, 0.14, 0.12, hex(0xdfe5f2), z + 0.1);
            }
            // the glove box on the hull wall
            p.block(x + 8.3, y + 4.6, 1.4, 1.6, 0xb6c6d2, z);
            p.cells(x + 8.4, y + 4.75, 1.2, 1.2, hexa(0x7dfcff, 0.5), z + 0.05);
            p.disc(m, x + 8.7, y + 4.9, 0.17, 0x2a3442, z + 0.1);
            p.disc(m, x + 9.3, y + 4.9, 0.17, 0x2a3442, z + 0.1);
            // a plant growth chamber with pink LED light (like the ISS "Veggie")
            p.block(x + 0.3, y + 4.6, 1.6, 1.6, 0xdfe5f2, z);
            let e = p.cells(x + 0.4, y + 4.7, 1.4, 1.4, hexa(0xff6fd8, 0.55), z + 0.05);
            p.blink(e, 0.5, 0.0, 0.7);
            for i in 0..3 {
                p.disc(m, x + 0.7 + i as f32 * 0.4, y + 5.0, 0.18, 0x4fbf6a, z + 0.1);
            }
            // a microscope
            p.block(x + 6.6, y + 5.4, 0.8, 0.7, 0x5a6178, z);
            p.disc(m, x + 7.0, y + 6.0, 0.15, 0x3ec7c0, z + 0.1);
        }
        "habitat" => {
            // two sleeping bags strapped to the hull wall
            for (i, c) in [0x6f8cff, 0xff8a6f].iter().enumerate() {
                let py = y + 2.4 + i as f32 * 1.1;
                p.block(x + 0.25, py, 0.9, 0.95, 0x3a3f5a, z);
                p.cells(x + 0.32, py + 0.08, 0.76, 0.6, hex(*c), z + 0.1);
                p.cells(x + 0.4, py + 0.66, 0.6, 0.22, hex(0xffffff), z + 0.1);
            }
            // a round table with two seats near the galley
            p.disc(m, x + 2.6, y + 5.2, 0.3, 0xffd36b, z);
            p.disc(m, x + 4.0, y + 5.2, 0.3, 0xffd36b, z);
            p.disc(m, x + 3.3, y + 5.2, 0.55, 0xf0e6d2, z + 0.1);
            p.disc(m, x + 3.4, y + 5.3, 0.12, 0xff6f5a, z + 0.2);
            // the pet corner: a soft rug, a bowl and a cushion
            p.cells(x + 5.7, y + 0.2, 4.1, 2.7, hex(0xe8a0c8), z - 0.05);
            p.cells(x + 5.85, y + 0.35, 3.8, 2.4, hex(0xf2b8d8), z - 0.04);
            p.disc(m, x + 9.3, y + 2.3, 0.3, 0xdfe5f2, z);
            p.disc(m, x + 6.1, y + 0.7, 0.35, 0xffd36b, z);
            // a plant, family photos and a guitar on the wall
            p.block(x + 6.6, y + 5.4, 0.6, 0.6, 0x8a5a3a, z);
            p.disc(m, x + 6.9, y + 6.2, 0.38, 0x4fbf6a, z + 0.1);
            for i in 0..3 {
                p.cells(x + 4.9 + i as f32 * 0.45, y + 6.35, 0.32, 0.3, hex([0xffd36b, 0x6fd3ff, 0xff8a8a][i]), z);
            }
            p.icon(art, "s_music", x + 9.4, y + 4.4, 0.7, z);
        }
        "gym" => {
            // a bike in the corner
            p.disc(m, x + 7.6, y + 5.6, 0.4, 0x22252f, z);
            p.disc(m, x + 8.8, y + 5.6, 0.4, 0x22252f, z);
            p.cells(x + 7.6, y + 5.55, 1.2, 0.14, hex(0xff8a3d), z + 0.1);
            p.cells(x + 8.1, y + 5.6, 0.12, 0.6, hex(0xff8a3d), z + 0.1);
            // a weights rack and the resistance machine (like ISS "ARED")
            p.block(x + 0.4, y + 0.4, 2.4, 0.7, 0x22252f, z);
            for i in 0..5 {
                p.disc(m, x + 0.7 + i as f32 * 0.45, y + 0.75, 0.16, [0xff5a5a, 0xffd35a, 0x5ac8ff, 0x5aff8a, 0xff8a3d][i], z + 0.1);
            }
            p.block(x + 7.0, y + 0.4, 2.6, 1.4, 0x4a4f63, z);
            p.cells(x + 7.2, y + 0.6, 2.2, 0.15, hex(0xb6ff7a), z + 0.1);
            p.cells(x + 7.2, y + 1.3, 2.2, 0.15, hex(0xb6ff7a), z + 0.1);
            // exercise bands and a heart-rate screen
            for i in 0..3 {
                p.cells(x + 2.6 + i as f32 * 0.3, y + 5.0, 0.1, 1.5, hex([0xff5a5a, 0xffd35a, 0x5ac8ff][i]), z);
            }
            p.screen(x + 9.5, y + 1.0, 0.3, 1.2, 0xff6a8a, z, 2.0);
        }
        "life_support" => {
            // tanks: oxygen (green), nitrogen (blue), water (white)
            for (i, c) in [0x4fbf6a, 0x3a7bd5, 0xe8f2ff].iter().enumerate() {
                let (tx, ty) = (x + 1.0 + i as f32 * 1.25, y + 0.9);
                p.disc(m, tx, ty, 0.55, 0x2a3442, z);
                p.disc(m, tx, ty, 0.43, *c, z + 0.1);
                p.disc(m, tx - 0.15, ty + 0.15, 0.1, 0xffffff, z + 0.2);
            }
            // pipes along the top wall, colour-coded
            for (i, c) in [0x7dfcff, 0xffcc33, 0x4fbf6a].iter().enumerate() {
                p.cells(x + 2.4, y + 6.25 - i as f32 * 0.2, 5.8, 0.12, hex(*c), z);
            }
            // the electrolysis unit (water → oxygen) and a plant rack that helps the air
            p.block(x + 2.5, y + 4.8, 1.4, 1.0, 0xb6c6d2, z);
            let e = p.disc(m, x + 3.2, y + 5.3, 0.22, 0x7dfcff, z + 0.1);
            p.blink(e, 1.2, 0.0, 1.0);
            p.block(x + 7.0, y + 0.4, 2.6, 1.8, 0x2a3442, z);
            for i in 0..5 {
                p.disc(m, x + 7.3 + i as f32 * 0.5, y + 1.2, 0.22, 0x4fbf6a, z + 0.1);
            }
        }
        "airlock" => {
            // the outer hatch on the hull wall with hazard stripes and a red lamp
            p.cells(x + 0.05, y + 2.4, 0.5, 2.2, hex(0x9aa3b8), z);
            for i in 0..6 {
                p.cells(x + 0.55, y + 2.4 + i as f32 * 0.37, 0.25, 0.37, hex(if i % 2 == 0 { 0xffcc33 } else { 0x22252f }), z);
            }
            let e = p.disc(m, x + 0.3, y + 4.85, 0.12, 0xff5a5a, z + 0.1);
            p.blink(e, 2.0, 0.0, 1.0);
            // cargo crates and water bags by the store
            for (cx, cy, c) in [(5.9, 0.3, 0xd9883a), (6.8, 0.3, 0xc0703a), (6.3, 1.1, 0xe8a04a)] {
                p.block(x + cx, y + cy, 0.8, 0.75, c, z);
                p.cells(x + cx + 0.08, y + cy + 0.32, 0.64, 0.08, hexa(0x000000, 0.3), z + 0.1);
            }
            for i in 0..3 {
                p.disc(m, x + 9.3, y + 1.6 + i as f32 * 0.6, 0.25, 0x3a8bff, z);
            }
            // spacesuits on their racks
            for i in 0..2 {
                p.block(x + 2.4 + i as f32 * 1.1, y + 5.4, 0.9, 1.3, 0xdfe5f2, z);
                p.icon(art, "astronaut", x + 2.85 + i as f32 * 1.1, y + 6.05, 0.9, z + 0.1);
            }
        }
        "toilet" => {
            // the hygiene corner: sink, mirror, towels; a trash compactor
            p.block(x + 0.4, y + 5.6, 1.8, 0.7, 0xffffff, z);
            p.cells(x + 0.7, y + 5.75, 1.2, 0.35, hex(0x8fc6e8), z + 0.1);
            p.cells(x + 0.5, y + 6.45, 1.6, 0.35, hex(0xdfe5f2), z);
            for i in 0..3 {
                p.cells(x + 7.6 + i as f32 * 0.5, y + 5.6, 0.35, 0.9, hex([0xff8a8a, 0x6fd3ff, 0xffd36b][i]), z);
            }
            p.block(x + 8.3, y + 0.4, 1.3, 1.3, 0x5a6070, z);
            p.cells(x + 8.45, y + 1.2, 1.0, 0.12, hex(0xffcc33), z + 0.1);
            let e = p.disc(m, x + 2.0, y + 1.0, 0.45, 0x5a6070, z);
            p.blink(e, 3.0, 0.0, 1.0);
        }
        "engine" => {
            // the glowing reactor core
            let c = (x + 2.2, y + 5.2);
            p.disc(m, c.0, c.1, 1.2, 0x3a2420, z);
            let e = p.disc(m, c.0, c.1, 0.9, 0xff8a3d, z + 0.1);
            p.blink(e, 1.6, 0.0, 1.0);
            let e = p.disc(m, c.0, c.1, 0.48, 0xffe08a, z + 0.2);
            p.blink(e, 2.4, 1.0, 1.0);
            // fuel tanks and pipes
            for i in 0..2 {
                p.disc(m, x + 7.2 + i as f32 * 1.3, y + 1.1, 0.6, 0x5a6178, z);
                p.disc(m, x + 7.2 + i as f32 * 1.3, y + 1.1, 0.45, 0xb0b8c8, z + 0.1);
            }
            for (i, c) in [0xff8a3d, 0xffcc33, 0xb0b8c8].iter().enumerate() {
                p.cells(x + 3.6, y + 5.6 + i as f32 * 0.3, 3.2, 0.15, hex(*c), z);
            }
            // warning stripes around the core
            for i in 0..7 {
                p.cells(x + 0.4 + i as f32 * 0.55, y + 3.6, 0.28, 0.2, hex(0xffcc33), z);
            }
            p.screen(x + 0.15, y + 0.8, 0.3, 1.6, 0xff8a3d, z, 0.5);
        }
        _ => {}
    }
}

fn walls(p: &mut Painter, layout: &ShipLayout, r: &Room) {
    let t = 0.32;
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
            .map(|d| if vertical { (d.1 - DOOR_HALF, d.1 + DOOR_HALF) } else { (d.0 - DOOR_HALF, d.0 + DOOR_HALF) })
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
            let hl = if vertical { Vec2::new(size.x * 0.4, size.y - 4.0) } else { Vec2::new(size.x - 4.0, size.y * 0.4) };
            p.px(at + if vertical { Vec2::new(-size.x * 0.2, 0.0) } else { Vec2::new(0.0, size.y * 0.2) }, hl, hex(0x9aa3c4), -0.55);
        }
        for (c0, c1) in cuts {
            for edge in [c0, c1] {
                let at = if vertical { to_px(a.0, edge) } else { to_px(edge, a.1) };
                p.px(at, Vec2::splat(t * SC + 6.0), hex(0xffcc33), -0.5);
            }
            let mid = (c0 + c1) / 2.0;
            let (at, size) = if vertical { (to_px(a.0, mid), Vec2::new(t * SC, (c1 - c0) * SC)) } else { (to_px(mid, a.1), Vec2::new((c1 - c0) * SC, t * SC)) };
            p.px(at, size, hexa(0x6fd3ff, 0.35), -0.7);
        }
    }
}

/// Sliding metal doors: two leaves per doorway, a yellow safety edge, a lamp on the frame.
fn doors(p: &mut Painter, layout: &ShipLayout) {
    for &(dx, dy) in &layout.doors {
        let vertical = dx.fract() == 0.0;
        let at = Vec2::new(dx, dy);
        for side in [-1.0f32, 1.0] {
            let size = if vertical { Vec2::new(0.42 * SC, DOOR_HALF * SC) } else { Vec2::new(DOOR_HALF * SC, 0.42 * SC) };
            let c = to_px(dx, dy);
            p.c.spawn((Sprite { color: hex(0xc3cbdc), custom_size: Some(size), ..default() }, Transform::from_xyz(c.x, c.y, -0.65), DoorLeaf { at, vertical, side, open: 0.0 }, p.sc.clone()))
                .with_children(|l| {
                    let (g, e, eo) = if vertical {
                        (Vec2::new(size.x * 0.25, size.y - 6.0), Vec2::new(size.x, 5.0), Vec3::new(0.0, -side * (size.y / 2.0 - 2.5), 0.02))
                    } else {
                        (Vec2::new(size.x - 6.0, size.y * 0.25), Vec2::new(5.0, size.y), Vec3::new(-side * (size.x / 2.0 - 2.5), 0.0, 0.02))
                    };
                    l.spawn((Sprite { color: hex(0x1c2030), custom_size: Some(size + Vec2::splat(4.0)), ..default() }, Transform::from_xyz(0.0, 0.0, -0.01)));
                    l.spawn((Sprite { color: hex(0x6a7390), custom_size: Some(g), ..default() }, Transform::from_xyz(0.0, 0.0, 0.01)));
                    for k in [-1.0f32, 1.0] {
                        let rv = if vertical { Vec3::new(0.0, k * size.y * 0.3, 0.02) } else { Vec3::new(k * size.x * 0.3, 0.0, 0.02) };
                        l.spawn((Sprite { color: hex(0x4a5068), custom_size: Some(Vec2::splat(4.0)), ..default() }, Transform::from_translation(rv)));
                    }
                    l.spawn((Sprite { color: hex(0xffcc33), custom_size: Some(e), ..default() }, Transform::from_translation(eo)));
                });
        }
        let lamp = to_px(dx, dy) + if vertical { Vec2::new(0.0, (DOOR_HALF + 0.2) * SC) } else { Vec2::new((DOOR_HALF + 0.2) * SC, 0.0) };
        p.c.spawn((Sprite { color: hex(0xff4a4a), custom_size: Some(Vec2::splat(8.0)), ..default() }, Transform::from_xyz(lamp.x, lamp.y, -0.4), DoorLamp(at), p.sc.clone()));
    }
}

/// Doors slide open when a crew member is near and close behind them.
pub fn animate_doors(time: Res<Time>, crew: Query<&super::Crew>, mut leaves: Query<(&mut Transform, &mut DoorLeaf)>, mut lamps: Query<(&mut Sprite, &DoorLamp)>) {
    let dt = time.delta_secs();
    let near = |at: Vec2| crew.iter().any(|c| c.pos.distance(at) < 2.2);
    for (mut tr, mut d) in &mut leaves {
        let want = if near(d.at) { 1.0 } else { 0.0 };
        d.open += (want - d.open).clamp(-dt * 3.0, dt * 4.0);
        let off = d.side * (DOOR_HALF / 2.0 + d.open * DOOR_HALF * 0.95) * SC;
        let c = to_px(d.at.x, d.at.y);
        tr.translation.x = c.x + if d.vertical { 0.0 } else { off };
        tr.translation.y = c.y + if d.vertical { off } else { 0.0 };
    }
    for (mut s, l) in &mut lamps {
        s.color = if near(l.0) { hex(0x5aff8a) } else { hex(0xff4a4a) };
    }
}

fn label(p: &mut Painter, art: &Art, r: &Room, l: Lang) {
    // a sign on the top wall, so it never covers a console
    let at = to_px(r.x as f32 + 0.45, (r.y + r.h) as f32);
    let name = r.name.get(l);
    let w = name.chars().count() as f32 * 8.4 + 14.0;
    p.c.spawn((Sprite { color: hexa(0x0b0e22, 0.72), custom_size: Some(Vec2::new(w, 22.0)), anchor: Anchor::CenterLeft, ..default() }, Transform::from_xyz(at.x - 4.0, at.y, -0.3), p.sc.clone()));
    p.c.spawn((Text2d::new(name), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(hex(0xffffff)), Anchor::CenterLeft, Transform::from_xyz(at.x + 2.0, at.y, -0.2), p.sc.clone()));
}

/// Space flies past; lights glow; flames flicker.
#[allow(clippy::type_complexity)]
pub fn animate(time: Res<Time>, mut drift: Query<(&mut Transform, &Drift)>, mut blink: Query<(&mut Sprite, &Blink), Without<Drift>>, mut blink_star: Query<(&mut Sprite, &Blink), With<Drift>>, mut mats: ResMut<Assets<ColorMaterial>>, blink_mesh: Query<(&MeshMaterial2d<ColorMaterial>, &Blink)>, mut flames: Query<(&mut Transform, &Flame), Without<Drift>>) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    for (mut tr, d) in &mut drift {
        tr.translation.y -= d.speed * dt;
        if tr.translation.y < -d.span / 2.0 - 300.0 {
            tr.translation.y += d.span + 600.0;
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
