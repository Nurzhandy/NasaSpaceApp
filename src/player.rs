//! Third-person astronaut controller, camera, split-screen support.

use crate::sim::Sim;
use crate::world::{height, Obstacle, WalkAnim, BASE_R, WORLD_R};
use crate::{AppState, GameConfig};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::PrimaryWindow;

#[derive(Component)]
pub struct Player {
    pub idx: u8,
    pub vel_y: f32,
    pub on_ground: bool,
    pub yaw: f32,
    pub in_shelter: bool,
    pub carried: u32,
    pub step_timer: f32,
    pub frozen: bool,
}

#[derive(Component)]
pub struct PlayerCam {
    pub idx: u8,
    pub yaw: f32,
    pub manual: f32,
}

/// Per-player input snapshot (keyboard set or gamepad), built each frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub mv: Vec2,
    pub look: f32,
    pub jump: bool,
    pub act: bool,
    pub act_held: bool,
    pub pause: bool,
    pub back: bool,
    pub nav: Vec2,
    pub nav_pressed: bool,
}

#[derive(Resource, Default)]
pub struct Inputs(pub [Input; 2]);

pub fn read_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut inputs: ResMut<Inputs>,
    cfg: Res<GameConfig>,
) {
    let mut pads: Vec<(Entity, &Gamepad)> = gamepads.iter().collect();
    pads.sort_by_key(|(e, _)| *e);
    let axis = |neg: bool, pos: bool| (pos as i32 - neg as i32) as f32;

    // Player 1: WASD + E/Space, mouse drag to look; gamepad 1 as well
    let mut p1 = Input {
        mv: Vec2::new(axis(keys.pressed(KeyCode::KeyA), keys.pressed(KeyCode::KeyD)), axis(keys.pressed(KeyCode::KeyS), keys.pressed(KeyCode::KeyW))),
        look: if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) { -motion.delta.x * 0.005 } else { 0.0 } + axis(keys.pressed(KeyCode::KeyQ), keys.pressed(KeyCode::KeyE)) * -0.03,
        jump: keys.just_pressed(KeyCode::Space),
        act: keys.just_pressed(KeyCode::KeyF) || keys.just_pressed(KeyCode::Enter),
        act_held: keys.pressed(KeyCode::KeyF) || keys.pressed(KeyCode::Enter),
        pause: keys.just_pressed(KeyCode::Escape),
        back: keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace),
        nav: Vec2::new(axis(keys.just_pressed(KeyCode::KeyA), keys.just_pressed(KeyCode::KeyD)), axis(keys.just_pressed(KeyCode::KeyS), keys.just_pressed(KeyCode::KeyW))),
        nav_pressed: false,
    };
    // In solo mode the arrow keys also drive player 1
    if !cfg.coop {
        p1.mv += Vec2::new(axis(keys.pressed(KeyCode::ArrowLeft), keys.pressed(KeyCode::ArrowRight)), axis(keys.pressed(KeyCode::ArrowDown), keys.pressed(KeyCode::ArrowUp)));
        p1.nav += Vec2::new(axis(keys.just_pressed(KeyCode::ArrowLeft), keys.just_pressed(KeyCode::ArrowRight)), axis(keys.just_pressed(KeyCode::ArrowDown), keys.just_pressed(KeyCode::ArrowUp)));
    }
    let mut p2 = Input {
        mv: Vec2::new(axis(keys.pressed(KeyCode::ArrowLeft), keys.pressed(KeyCode::ArrowRight)), axis(keys.pressed(KeyCode::ArrowDown), keys.pressed(KeyCode::ArrowUp))),
        look: axis(keys.pressed(KeyCode::Comma), keys.pressed(KeyCode::Period)) * -0.03,
        jump: keys.just_pressed(KeyCode::ControlRight) || keys.just_pressed(KeyCode::Numpad0),
        act: keys.just_pressed(KeyCode::ShiftRight) || keys.just_pressed(KeyCode::NumpadEnter),
        act_held: keys.pressed(KeyCode::ShiftRight) || keys.pressed(KeyCode::NumpadEnter),
        pause: false,
        back: false,
        nav: Vec2::new(axis(keys.just_pressed(KeyCode::ArrowLeft), keys.just_pressed(KeyCode::ArrowRight)), axis(keys.just_pressed(KeyCode::ArrowDown), keys.just_pressed(KeyCode::ArrowUp))),
        nav_pressed: false,
    };
    for (i, (_, pad)) in pads.iter().enumerate().take(2) {
        let target = if i == 0 { &mut p1 } else { &mut p2 };
        let ls = pad.left_stick();
        let rs = pad.right_stick();
        if ls.length() > 0.2 {
            target.mv += Vec2::new(ls.x, ls.y);
        }
        if rs.x.abs() > 0.2 {
            target.look += -rs.x * 0.04;
        }
        target.jump |= pad.just_pressed(GamepadButton::West) || pad.just_pressed(GamepadButton::North);
        target.act |= pad.just_pressed(GamepadButton::South);
        target.act_held |= pad.pressed(GamepadButton::South);
        target.pause |= pad.just_pressed(GamepadButton::Start);
        target.back |= pad.just_pressed(GamepadButton::East);
        target.nav += Vec2::new(
            axis(pad.just_pressed(GamepadButton::DPadLeft), pad.just_pressed(GamepadButton::DPadRight)),
            axis(pad.just_pressed(GamepadButton::DPadDown), pad.just_pressed(GamepadButton::DPadUp)),
        );
    }
    p1.mv = p1.mv.clamp_length_max(1.0);
    p2.mv = p2.mv.clamp_length_max(1.0);
    inputs.0 = [p1, p2];
}

pub fn spawn_players(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    profile: &crate::save::Profile,
    coop: bool,
    map: crate::data::MapId,
) {
    let suit = crate::data::SUITS.iter().find(|s| s.id == profile.suit).map(|s| s.color).unwrap_or(Color::WHITE);
    let visor = crate::data::VISORS.iter().find(|s| s.id == profile.visor).map(|s| s.color).unwrap_or(Color::srgb(0.5, 0.8, 1.0));
    let n = if coop { 2 } else { 1 };
    for i in 0..n {
        let (s, v, h) = if i == 0 { (suit, visor, profile.hat.as_str()) } else { (Color::srgb(0.3, 0.75, 1.0), Color::srgb(1.0, 0.8, 0.3), "antenna") };
        let x = -1.5 + i as f32 * 3.0;
        let z = 9.0;
        let root = crate::world::spawn_astronaut(commands, meshes, materials, s, v, h);
        commands.entity(root).insert((
            Transform::from_xyz(x, height(map, x, z), z),
            Player { idx: i as u8, vel_y: 0.0, on_ground: true, yaw: std::f32::consts::PI, in_shelter: false, carried: 0, step_timer: 0.0, frozen: false },
            WalkAnim::default(),
            StateScoped(AppState::Playing),
        ));
        let m = crate::data::map(map);
        commands.spawn((
            Camera3d::default(),
            Camera { order: i as isize, clear_color: ClearColorConfig::Custom(m.sky), ..default() },
            Projection::from(PerspectiveProjection { fov: 60f32.to_radians(), near: 0.1, far: 900.0, ..default() }),
            DistanceFog { color: m.sky, falloff: FogFalloff::Linear { start: m.fog * 0.45, end: m.fog }, ..default() },
            Transform::from_xyz(x, 4.0, z + 7.0).looking_at(Vec3::new(x, 1.0, z), Vec3::Y),
            PlayerCam { idx: i as u8, yaw: 0.0, manual: 0.0 },
            StateScoped(AppState::Playing),
        ));
    }
}

/// Keeps camera viewports split when two players are present.
pub fn split_viewports(window: Query<&Window, With<PrimaryWindow>>, mut cams: Query<(&PlayerCam, &mut Camera)>, cfg: Res<GameConfig>) {
    let Ok(win) = window.get_single() else { return };
    let w = win.physical_width();
    let h = win.physical_height();
    if w == 0 || h == 0 {
        return;
    }
    for (pc, mut cam) in &mut cams {
        if cfg.coop {
            let half = w / 2;
            cam.viewport = Some(Viewport { physical_position: UVec2::new(pc.idx as u32 * half, 0), physical_size: UVec2::new(half, h), ..default() });
        } else {
            cam.viewport = None;
        }
    }
}

pub fn move_players(
    time: Res<Time>,
    inputs: Res<Inputs>,
    sim: Option<Res<Sim>>,
    modal: Res<crate::ui::Modal>,
    cams: Query<&PlayerCam>,
    obstacles: Query<(&Transform, &Obstacle), Without<Player>>,
    mut players: Query<(&mut Player, &mut Transform, &mut WalkAnim, &mut Visibility)>,
    mut sfx: crate::audio::Sfx,
) {
    let Some(sim) = sim else { return };
    if sim.paused && !sim.coop {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    let g = 9.81 * sim.def().gravity.max(0.16);
    let map = sim.map;
    for (mut p, mut tr, mut anim, mut vis) in &mut players {
        let inp = inputs.0[p.idx as usize];
        let blocked = p.frozen || p.in_shelter || modal.blocks(p.idx);
        *vis = if p.in_shelter { Visibility::Hidden } else { Visibility::Visible };
        let cam_yaw = cams.iter().find(|c| c.idx == p.idx).map(|c| c.yaw).unwrap_or(0.0);
        let mut mv = Vec3::ZERO;
        if !blocked && inp.mv.length() > 0.05 {
            let fwd = Quat::from_rotation_y(cam_yaw) * Vec3::NEG_Z;
            let right = Quat::from_rotation_y(cam_yaw) * Vec3::X;
            mv = (fwd * inp.mv.y + right * inp.mv.x).normalize_or_zero() * inp.mv.length();
        }
        let speed = 5.2;
        let mut pos = tr.translation + mv * speed * dt;
        // obstacles (circles) + world edge
        for (ot, ob) in &obstacles {
            if ob.radius <= 0.0 {
                continue;
            }
            let d = Vec2::new(pos.x - ot.translation.x, pos.z - ot.translation.z);
            let min = ob.radius + 0.45;
            let len = d.length();
            if len < min && len > 0.001 {
                let push = d / len * (min - len);
                pos.x += push.x;
                pos.z += push.y;
            }
        }
        let r = (pos.x * pos.x + pos.z * pos.z).sqrt();
        if r > WORLD_R {
            pos.x *= WORLD_R / r;
            pos.z *= WORLD_R / r;
        }
        // vertical
        let ground = height(map, pos.x, pos.z);
        if !blocked && inp.jump && p.on_ground {
            p.vel_y = (2.0 * g * 1.1).sqrt();
            p.on_ground = false;
            sfx.play("step");
        }
        p.vel_y -= g * dt;
        pos.y += p.vel_y * dt;
        if pos.y <= ground {
            pos.y = ground;
            p.vel_y = 0.0;
            p.on_ground = true;
        }
        tr.translation = pos;
        // facing
        if mv.length() > 0.05 {
            let target = (-mv.x).atan2(-mv.z);
            let mut d = target - p.yaw;
            while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
            while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
            p.yaw += d * (10.0 * dt).min(1.0);
        }
        tr.rotation = Quat::from_rotation_y(p.yaw);
        anim.moving = mv.length() > 0.05 && p.on_ground;
        if anim.moving {
            anim.phase += dt * 9.0 * mv.length();
            p.step_timer -= dt;
            if p.step_timer <= 0.0 {
                p.step_timer = 0.38;
                sfx.play_vol("step", 0.35);
            }
        }
        let _ = BASE_R;
    }
}

pub fn follow_camera(time: Res<Time>, inputs: Res<Inputs>, players: Query<(&Player, &Transform), Without<PlayerCam>>, mut cams: Query<(&mut PlayerCam, &mut Transform)>) {
    let dt = time.delta_secs().min(0.05);
    for (mut cam, mut ct) in &mut cams {
        let Some((p, pt)) = players.iter().find(|(p, _)| p.idx == cam.idx) else { continue };
        let inp = inputs.0[cam.idx as usize];
        if inp.look.abs() > 0.0001 {
            cam.yaw += inp.look;
            cam.manual = 2.0;
        } else {
            cam.manual = (cam.manual - dt).max(0.0);
        }
        // auto-follow behind the player when they walk
        if cam.manual <= 0.0 && inp.mv.length() > 0.2 {
            let mut d = p.yaw - cam.yaw;
            while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
            while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
            cam.yaw += d * (1.5 * dt).min(1.0);
        }
        let anchor = if p.in_shelter { Vec3::new(0.0, 1.5, 3.0) } else { pt.translation };
        let offset = Quat::from_rotation_y(cam.yaw) * Vec3::new(0.0, 3.0, 6.8);
        let mut goal = anchor + offset;
        let gh = height(p_map(), goal.x, goal.z) + 1.2;
        if goal.y < gh {
            goal.y = gh;
        }
        ct.translation = ct.translation.lerp(goal, (8.0 * dt).min(1.0));
        let look = anchor + Vec3::new(0.0, 1.4, 0.0);
        let target_rot = Transform::from_translation(ct.translation).looking_at(look, Vec3::Y).rotation;
        ct.rotation = ct.rotation.slerp(target_rot, (10.0 * dt).min(1.0));
    }
}

// Current map for camera ground clamping (set when a game starts).
static CURRENT_MAP: std::sync::Mutex<crate::data::MapId> = std::sync::Mutex::new(crate::data::MapId::Earth);
pub fn set_map(m: crate::data::MapId) {
    if let Ok(mut g) = CURRENT_MAP.lock() {
        *g = m;
    }
}
fn p_map() -> crate::data::MapId {
    CURRENT_MAP.lock().map(|g| *g).unwrap_or(crate::data::MapId::Earth)
}
