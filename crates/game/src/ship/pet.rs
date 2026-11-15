//! The ship's pet, drawn with a real body: a cat (body, head, ears, four legs, a wagging tail), a
//! robot (box body, screen face, antenna, tracks) or a little alien (a blob with a big eye and
//! waving tentacles). It walks after its owner with a walking animation, sits when told, and asks
//! for food when hungry. Before anyone adopts it, all three wait in the living quarters.

use super::{to_px, Camp, Crew, SC};
use crate::kit::UiBusy;
use crate::{AppState, MainCam};
use bevy::prelude::*;
use ja_core::CampaignCommand;

#[derive(Component)]
pub struct PetBody {
    pub kind: u8,
    pub pos: Vec2,
    vel: Vec2,
    face: f32,
}

#[derive(Component, Clone, Copy)]
pub enum Part {
    Leg(f32),
    Tail,
    Tentacle(f32),
    Blink,
    Body,
    Track,
}

#[derive(Component)]
pub struct PetBubble(u8);

fn hex(c: u32) -> Color {
    Color::srgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

type Ms<'a> = (&'a mut Assets<Mesh>, &'a mut Assets<ColorMaterial>);

fn disc(c: &mut ChildSpawnerCommands, m: &mut Ms, at: Vec3, rx: f32, ry: f32, color: u32) -> Entity {
    let mesh = m.0.add(Ellipse::new(rx, ry));
    let mat = m.1.add(ColorMaterial::from(hex(color)));
    c.spawn((Mesh2d(mesh), MeshMaterial2d(mat), Transform::from_translation(at))).id()
}
fn tri(c: &mut ChildSpawnerCommands, m: &mut Ms, at: Vec3, w: f32, h: f32, color: u32) {
    let mesh = m.0.add(Triangle2d::new(Vec2::new(-w / 2.0, 0.0), Vec2::new(w / 2.0, 0.0), Vec2::new(0.0, h)));
    let mat = m.1.add(ColorMaterial::from(hex(color)));
    c.spawn((Mesh2d(mesh), MeshMaterial2d(mat), Transform::from_translation(at)));
}
fn rect(c: &mut ChildSpawnerCommands, at: Vec3, w: f32, h: f32, color: u32) -> Entity {
    c.spawn((Sprite { color: hex(color), custom_size: Some(Vec2::new(w, h)), ..default() }, Transform::from_translation(at))).id()
}

/// Build one pet (scale 1 = about one ship cell long).
fn build(commands: &mut Commands, m: &mut Ms, kind: u8, at: Vec2) -> Entity {
    let p = to_px(at.x, at.y);
    let s = SC / 44.0;
    commands
        .spawn((Transform::from_xyz(p.x, p.y, 2.4).with_scale(Vec3::splat(s)), Visibility::default(), PetBody { kind, pos: at, vel: Vec2::ZERO, face: 1.0 }, StateScoped(AppState::Ship)))
        .with_children(|c| {
            // a soft shadow under every pet
            disc(c, m, Vec3::new(0.0, 0.0, -0.05), 20.0, 5.0, 0x000000);
            match kind {
                // the cat: orange with stripes
                1 => {
                    let tail = rect(c, Vec3::new(-20.0, 16.0, -0.02), 6.0, 26.0, 0xe08a2a);
                    c.commands().entity(tail).insert(Part::Tail);
                    for (i, x) in [-13.0f32, -6.0, 7.0, 14.0].into_iter().enumerate() {
                        let e = rect(c, Vec3::new(x, 6.0, -0.01), 5.0, 12.0, if i % 2 == 0 { 0xd9822a } else { 0xe8963a });
                        c.commands().entity(e).insert(Part::Leg(i as f32 * 1.6));
                    }
                    let b = disc(c, m, Vec3::new(0.0, 15.0, 0.0), 20.0, 11.0, 0xf2a03d);
                    c.commands().entity(b).insert(Part::Body);
                    for x in [-8.0, 0.0, 8.0] {
                        rect(c, Vec3::new(x, 20.0, 0.01), 3.0, 8.0, 0xc8701e);
                    }
                    disc(c, m, Vec3::new(21.0, 25.0, 0.02), 11.0, 10.0, 0xf2a03d);
                    tri(c, m, Vec3::new(15.0, 31.0, 0.01), 7.0, 9.0, 0xf2a03d);
                    tri(c, m, Vec3::new(27.0, 31.0, 0.01), 7.0, 9.0, 0xf2a03d);
                    tri(c, m, Vec3::new(15.0, 32.0, 0.015), 4.0, 5.0, 0xffb8c8);
                    tri(c, m, Vec3::new(27.0, 32.0, 0.015), 4.0, 5.0, 0xffb8c8);
                    for x in [17.0, 25.0] {
                        let e = disc(c, m, Vec3::new(x, 27.0, 0.03), 2.0, 2.4, 0x1a1a1a);
                        c.commands().entity(e).insert(Part::Blink);
                    }
                    disc(c, m, Vec3::new(21.0, 22.5, 0.03), 1.6, 1.2, 0xff8aa8);
                }
                // the robot: grey box, screen face, antenna, tracks
                0 => {
                    let tr = rect(c, Vec3::new(0.0, 4.0, -0.01), 34.0, 8.0, 0x2a2f45);
                    c.commands().entity(tr).insert(Part::Track);
                    for x in [-12.0, 0.0, 12.0] {
                        disc(c, m, Vec3::new(x, 4.0, 0.0), 3.5, 3.5, 0x6a7390);
                    }
                    let b = rect(c, Vec3::new(0.0, 19.0, 0.0), 30.0, 22.0, 0xb8c0d4);
                    c.commands().entity(b).insert(Part::Body);
                    rect(c, Vec3::new(0.0, 20.0, 0.01), 22.0, 14.0, 0x0b1a2a);
                    for x in [-5.0, 5.0] {
                        let e = disc(c, m, Vec3::new(x, 21.0, 0.02), 2.6, 3.2, 0x5ae8ff);
                        c.commands().entity(e).insert(Part::Blink);
                    }
                    rect(c, Vec3::new(0.0, 16.0, 0.02), 8.0, 1.6, 0x5ae8ff);
                    rect(c, Vec3::new(0.0, 34.0, 0.0), 2.0, 9.0, 0x6a7390);
                    disc(c, m, Vec3::new(0.0, 39.0, 0.01), 3.0, 3.0, 0xff4a4a);
                }
                // the alien: a green blob, one big eye, waving tentacles
                _ => {
                    for (i, x) in [-9.0f32, 0.0, 9.0].into_iter().enumerate() {
                        let e = rect(c, Vec3::new(x, 6.0, -0.01), 5.0, 14.0, 0x3ab85a);
                        c.commands().entity(e).insert(Part::Tentacle(i as f32 * 2.0));
                    }
                    let b = disc(c, m, Vec3::new(0.0, 18.0, 0.0), 16.0, 15.0, 0x5ad87a);
                    c.commands().entity(b).insert(Part::Body);
                    disc(c, m, Vec3::new(0.0, 21.0, 0.01), 8.0, 8.0, 0xffffff);
                    let e = disc(c, m, Vec3::new(2.0, 21.0, 0.02), 4.0, 4.5, 0x1a1a2a);
                    c.commands().entity(e).insert(Part::Blink);
                    for x in [-7.0, 7.0] {
                        rect(c, Vec3::new(x, 35.0, 0.0), 2.0, 8.0, 0x3ab85a);
                        disc(c, m, Vec3::new(x, 39.0, 0.01), 2.8, 2.8, 0xffd35a);
                    }
                }
            }
        })
        .id()
}

/// Keep the right pets in the ship: three to choose from, then only the adopted one.
pub fn sync(mut commands: Commands, camp: Res<Camp>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<ColorMaterial>>, q: Query<(Entity, &PetBody)>) {
    let st = camp.0.state();
    let want: Vec<(u8, Vec2)> = match &st.pet {
        None => camp.0.pet_spots().iter().enumerate().map(|(k, s)| (k as u8, Vec2::new(s.0, s.1))).collect(),
        Some(p) => vec![(p.kind, Vec2::new(p.at.0, p.at.1))],
    };
    let mut have = vec![];
    for (e, b) in &q {
        if want.iter().any(|w| w.0 == b.kind) {
            have.push(b.kind);
        } else {
            commands.entity(e).despawn();
        }
    }
    let mut m: Ms = (&mut meshes, &mut mats);
    for (k, at) in want {
        if !have.contains(&k) {
            build(&mut commands, &mut m, k, at);
        }
    }
}

/// Walk after the owner (or sit), with legs, tail and tentacles animated.
#[allow(clippy::type_complexity)]
pub fn animate(time: Res<Time>, camp: Res<Camp>, crew: Query<&Crew>, mut pets: Query<(&mut PetBody, &mut Transform, &Children), Without<Part>>, mut parts: Query<(&Part, &mut Transform), Without<PetBody>>) {
    let dt = time.delta_secs().min(0.05);
    let t = time.elapsed_secs();
    let st = camp.0.state();
    for (mut b, mut tr, children) in &mut pets {
        let goal = match &st.pet {
            Some(p) if p.kind == b.kind && p.follow => {
                let owner = crew.iter().find(|c| c.idx == p.owner).map(|c| c.pos).unwrap_or(b.pos);
                let off = Vec2::new(-0.9 * b.face.signum(), 0.25);
                // keep a little distance; don't walk into the owner
                if owner.distance(b.pos) > 1.1 {
                    owner + off
                } else {
                    b.pos
                }
            }
            Some(p) if p.kind == b.kind => Vec2::new(p.at.0, p.at.1),
            _ => camp.0.pet_spots().get(b.kind as usize).map(|s| Vec2::new(s.0, s.1)).unwrap_or(b.pos),
        };
        let d = goal - b.pos;
        let want_v = if d.length() > 0.05 { d.normalize() * (d.length() * 3.0).min(3.6) } else { Vec2::ZERO };
        b.vel = b.vel.lerp(want_v, (8.0 * dt).min(1.0));
        let v = b.vel;
        b.pos += v * dt;
        if v.x.abs() > 0.15 {
            b.face = v.x.signum();
        }
        let moving = v.length() > 0.25;
        let sitting = matches!(&st.pet, Some(p) if p.kind == b.kind && !p.follow) && !moving;
        let p = to_px(b.pos.x, b.pos.y - 0.3);
        let bob = if moving { (t * 14.0).sin().abs() * 2.0 } else { 0.0 };
        tr.translation = Vec3::new(p.x, p.y + bob, 2.4 - b.pos.y * 0.001);
        let s = tr.scale.y.abs();
        tr.scale = Vec3::new(s * b.face, s, 1.0);
        for c in children.iter() {
            let Ok((part, mut pt)) = parts.get_mut(c) else { continue };
            match *part {
                Part::Leg(ph) => {
                    let swing = if moving { (t * 14.0 + ph).sin() * 3.5 } else { 0.0 };
                    pt.translation.y = if sitting { 9.0 } else { 6.0 + swing.max(0.0) };
                    pt.translation.x += 0.0;
                    pt.scale.y = if sitting { 0.4 } else { 1.0 };
                }
                Part::Tail => {
                    let wag = if moving { (t * 10.0).sin() * 0.5 } else { (t * 2.5).sin() * 0.35 };
                    pt.rotation = Quat::from_rotation_z(0.5 + wag);
                }
                Part::Tentacle(ph) => {
                    pt.rotation = Quat::from_rotation_z((t * (if moving { 9.0 } else { 3.0 }) + ph).sin() * 0.45);
                }
                Part::Blink => {
                    // blink every few seconds
                    pt.scale.y = if (t * 0.7 + b.kind as f32).fract() < 0.05 { 0.15 } else { 1.0 };
                }
                Part::Body => {
                    pt.scale.y = if sitting { 0.92 } else { 1.0 + (t * 2.0).sin() * 0.03 };
                }
                Part::Track => {
                    pt.translation.y = 4.0 + if moving { (t * 20.0).sin() * 0.6 } else { 0.0 };
                }
            }
        }
    }
}

/// "I'm hungry!" over a hungry pet.
pub fn hunger(mut commands: Commands, camp: Res<Camp>, art: Res<crate::art::Art>, prof: Res<crate::Prof>, pets: Query<(&PetBody, &Transform)>, mut bubbles: Query<(Entity, &PetBubble, &mut Transform), Without<PetBody>>, time: Res<Time>) {
    let st = camp.0.state();
    let hungry = st.pet.as_ref().filter(|p| p.mood < 50.0);
    let l = prof.lang();
    match hungry.and_then(|p| pets.iter().find(|(b, _)| b.kind == p.kind)) {
        Some((b, tr)) => {
            let at = tr.translation + Vec3::new(0.0, 62.0 + (time.elapsed_secs() * 3.0).sin() * 3.0, 3.0);
            if let Some((_, _, mut bt)) = bubbles.iter_mut().find(|x| x.1 .0 == b.kind) {
                bt.translation = at;
            } else {
                for (e, ..) in &bubbles {
                    commands.entity(e).despawn();
                }
                let line = match b.kind {
                    1 => l.pick("Мяу! Хочу есть!", "Meow! I'm hungry!"),
                    0 => l.pick("Бип! Нужна подзарядка!", "Beep! Need a recharge!"),
                    _ => l.pick("Зу-зу! Покорми меня!", "Zoo-zoo! Feed me!"),
                };
                commands
                    .spawn((Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.95), custom_size: Some(Vec2::new(line.chars().count() as f32 * 7.4 + 44.0, 30.0)), ..default() }, Transform::from_translation(at), PetBubble(b.kind), StateScoped(AppState::Ship)))
                    .with_children(|c| {
                        c.spawn((Sprite { image: art.icon("food"), custom_size: Some(Vec2::splat(20.0)), ..default() }, Transform::from_xyz(-(line.chars().count() as f32 * 3.7) - 6.0, 0.0, 0.1)));
                        c.spawn((Text2d::new(line), TextFont { font: art.body.clone(), font_size: 14.0, ..default() }, TextColor(Color::srgb(0.6, 0.15, 0.1)), Transform::from_xyz(10.0, 0.0, 0.1)));
                    });
            }
        }
        None => {
            for (e, ..) in &bubbles {
                commands.entity(e).despawn();
            }
        }
    }
}

/// A mouse click on your pet: sit / follow (like pressing E next to it).
#[allow(clippy::too_many_arguments)]
pub fn click(mouse: Res<ButtonInput<MouseButton>>, windows: Query<&Window>, cam: Query<(&Camera, &GlobalTransform), With<MainCam>>, pets: Query<&Transform, With<PetBody>>, mut camp: ResMut<Camp>, ui: Res<super::ShipUi>, busy: Res<UiBusy>, net: Res<crate::net::Net>, mut play: EventWriter<crate::audio::Play>) {
    if !mouse.just_pressed(MouseButton::Left) || ui.open.is_some() || busy.0 || camp.0.state().pet.is_none() {
        return;
    }
    let (Ok(w), Ok((c, gt))) = (windows.single(), cam.single()) else { return };
    let Some(world) = w.cursor_position().and_then(|p| c.viewport_to_world_2d(gt, p).ok()) else { return };
    if pets.iter().any(|t| (t.translation.truncate() + Vec2::new(0.0, 18.0)).distance(world) < SC * 0.7) {
        let me = net.local_idx().unwrap_or(0);
        if camp.0.apply(me, CampaignCommand::PetToggle).is_ok() {
            play.write(crate::audio::Play("click"));
        }
    }
}

/// Where the pet is now (for "E next to the pet").
pub fn pet_pos(pets: &Query<&PetBody>, kind: u8) -> Option<Vec2> {
    pets.iter().find(|b| b.kind == kind).map(|b| b.pos)
}
