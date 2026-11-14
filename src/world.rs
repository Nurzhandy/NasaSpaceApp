//! 3D world: terrain, sky, lighting, the base and its modules, props, the astronaut model.

use crate::data::*;
use crate::sim::Sim;
use crate::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::f32::consts::{PI, TAU};

pub const WORLD_R: f32 = 58.0;
pub const BASE_R: f32 = 14.0;

/// Interaction points in the world.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum StationKind {
    Board,
    Airlock,
    Generator,
    Panels,
    Greenhouse,
    Lab,
    Crewmate,
    Cargo,
    Leak,
    Sample,
}

#[derive(Component)]
pub struct Station {
    pub kind: StationKind,
    pub radius: f32,
}

#[derive(Component)]
pub struct Beam(pub StationKind);

#[derive(Component)]
pub struct Obstacle {
    pub radius: f32,
}

#[derive(Component)]
pub struct SunLight;
#[derive(Component)]
pub struct SunDisc;
#[derive(Component)]
pub struct SkyDome;
#[derive(Component)]
pub struct PanelGlass;
#[derive(Component)]
pub struct GreenGlass;
#[derive(Component)]
pub struct ShieldRing;
#[derive(Component)]
pub struct BatteryBar(pub u32);
#[derive(Component)]
pub struct WarnLamp(pub StationKind);
#[derive(Component)]
pub struct Spin(pub f32);
#[derive(Component)]
pub struct Bob {
    pub base: f32,
    pub amp: f32,
    pub speed: f32,
}
#[derive(Component)]
pub struct Crewmate {
    pub t: f32,
    pub from: Vec3,
    pub to: Vec3,
    pub wait: f32,
}
#[derive(Component)]
pub struct Exclaim;
#[derive(Component)]
pub struct CargoCapsule {
    pub landing: f32,
}
#[derive(Component)]
pub struct DustCloud;
#[derive(Component)]
pub struct FlareGlow;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Limb {
    LeftLeg,
    RightLeg,
    LeftArm,
    RightArm,
}

#[derive(Resource)]
pub struct WorldAssets {
    pub gem_mesh: Handle<Mesh>,
    pub gem_mat: Handle<StandardMaterial>,
    pub beam_mesh: Handle<Mesh>,
    pub beam_mat: Handle<StandardMaterial>,
    pub shield_mat: Handle<StandardMaterial>,
    pub bubble_mat: Handle<StandardMaterial>,
}

// ---------------------------------------------------------------- terrain

fn hash2(x: f32, z: f32) -> f32 {
    let v = (x * 127.1 + z * 311.7).sin() * 43758.547;
    v - v.floor()
}
fn noise(x: f32, z: f32) -> f32 {
    let (ix, iz) = (x.floor(), z.floor());
    let (fx, fz) = (x - ix, z - iz);
    let (ux, uz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let a = hash2(ix, iz);
    let b = hash2(ix + 1.0, iz);
    let c = hash2(ix, iz + 1.0);
    let d = hash2(ix + 1.0, iz + 1.0);
    a + (b - a) * ux + (c - a) * uz + (a - b - c + d) * ux * uz
}

pub fn height(map: MapId, x: f32, z: f32) -> f32 {
    let r = (x * x + z * z).sqrt();
    let rough = match map {
        MapId::Earth => noise(x * 0.06, z * 0.06) * 2.2 + noise(x * 0.25, z * 0.25) * 0.4,
        MapId::Moon => {
            let mut h = noise(x * 0.05, z * 0.05) * 2.6 + noise(x * 0.2, z * 0.2) * 0.5;
            // craters
            for (cx, cz, cr) in [(28.0, -18.0, 9.0), (-30.0, 22.0, 7.0), (18.0, 34.0, 6.0), (-24.0, -30.0, 10.0)] {
                let d = ((x - cx) * (x - cx) + (z - cz) * (z - cz)).sqrt() / cr;
                if d < 1.3 {
                    let rim = (1.0 - (d - 1.0).abs() * 3.0).max(0.0) * 1.2;
                    let bowl = if d < 1.0 { -(1.0 - d * d) * 2.2 } else { 0.0 };
                    h += rim + bowl;
                }
            }
            h
        }
        MapId::Mars => noise(x * 0.045, z * 0.045) * 3.4 + noise(x * 0.18, z * 0.18) * 0.6 + (x * 0.15).sin() * 0.3,
        MapId::Orbit => 0.0,
        MapId::Europa => {
            let ridge = ((x * 0.35 + z * 0.2).sin() * 4.0).abs().min(1.0) * 0.5;
            noise(x * 0.07, z * 0.07) * 1.4 + ridge
        }
    };
    // flat pad around the base, blended
    let k = ((r - BASE_R) / 10.0).clamp(0.0, 1.0);
    let k = k * k * (3.0 - 2.0 * k);
    rough * k
}

fn terrain_mesh(map: MapId) -> Mesh {
    let n = 96usize;
    let size = WORLD_R * 2.4;
    let m = map_def(map);
    let (g1, g2) = (m.ground.to_linear(), m.ground2.to_linear());
    let mut pos = Vec::with_capacity((n + 1) * (n + 1));
    let mut uv = Vec::with_capacity((n + 1) * (n + 1));
    let mut col = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let x = (i as f32 / n as f32 - 0.5) * size;
            let z = (j as f32 / n as f32 - 0.5) * size;
            let y = height(map, x, z);
            pos.push([x, y, z]);
            uv.push([i as f32 / n as f32 * 8.0, j as f32 / n as f32 * 8.0]);
            let t = noise(x * 0.3 + 7.0, z * 0.3) * 0.7 + (y * 0.15).clamp(0.0, 0.3);
            let c = g1.mix(&g2, t.clamp(0.0, 1.0));
            col.push([c.red, c.green, c.blue, 1.0]);
        }
    }
    let mut idx = Vec::with_capacity(n * n * 6);
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let c = a + (n + 1) as u32;
            let d = c + 1;
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx))
        .with_computed_smooth_normals()
}

fn map_def(id: MapId) -> &'static MapDef {
    crate::data::map(id)
}

// ---------------------------------------------------------------- spawning

pub fn spawn_world(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    map: MapId,
    rng: &mut crate::sim::Rng,
    sample_count: u32,
) {
    let m = map_def(map);
    let scoped = StateScoped(AppState::Playing);

    // terrain
    commands.spawn((
        Mesh3d(meshes.add(terrain_mesh(map))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.95, ..default() })),
        Transform::IDENTITY,
        scoped.clone(),
        Name::new("terrain"),
    ));
    if map == MapId::Orbit {
        // station deck: a big ring plate with a rail
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(WORLD_R + 6.0, 0.6))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.65, 0.68, 0.76), metallic: 0.6, perceptual_roughness: 0.4, ..default() })),
            Transform::from_xyz(0.0, -0.3, 0.0),
            scoped.clone(),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Torus::new(WORLD_R + 4.5, WORLD_R + 5.5))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.5, 0.2), ..default() })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            scoped.clone(),
        ));
    }

    // sky dome + stars + sun disc
    let dark = m.sky.to_linear().luminance() < 0.1;
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(420.0).mesh().uv(24, 12))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: m.sky, unlit: true, cull_mode: None, ..default() })),
        Transform::IDENTITY,
        SkyDome,
        scoped.clone(),
    ));
    if dark {
        let star_mesh = meshes.add(Sphere::new(0.9).mesh().ico(1).unwrap());
        let star_mat = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::rgb(4.0, 4.0, 4.5), unlit: true, ..default() });
        for _ in 0..320 {
            let th = rng.range(0.0, TAU);
            let ph = rng.range(0.02, PI * 0.55);
            let r = 380.0;
            let p = Vec3::new(r * ph.sin() * th.cos(), r * ph.cos(), r * ph.sin() * th.sin());
            let s = rng.range(0.5, 1.6);
            commands.spawn((Mesh3d(star_mesh.clone()), MeshMaterial3d(star_mat.clone()), Transform::from_translation(p).with_scale(Vec3::splat(s)), scoped.clone()));
        }
        // Earth in the lunar sky / Jupiter over Europa / Moon under the station
        match map {
            MapId::Moon => {
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(14.0).mesh().uv(32, 18))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.55, 1.0), emissive: LinearRgba::rgb(0.15, 0.3, 0.8), perceptual_roughness: 0.6, ..default() })),
                    Transform::from_xyz(-120.0, 150.0, -300.0),
                    scoped.clone(),
                ));
            }
            MapId::Europa => {
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(70.0).mesh().uv(32, 18))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.7, 0.55), emissive: LinearRgba::rgb(0.3, 0.22, 0.15), perceptual_roughness: 0.9, ..default() })),
                    Transform::from_xyz(160.0, 120.0, -300.0),
                    scoped.clone(),
                ));
            }
            MapId::Orbit => {
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(160.0).mesh().uv(32, 18))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.62, 0.68), perceptual_roughness: 0.95, ..default() })),
                    Transform::from_xyz(60.0, -230.0, -120.0),
                    scoped.clone(),
                ));
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(20.0).mesh().uv(32, 18))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.55, 1.0), emissive: LinearRgba::rgb(0.15, 0.3, 0.8), ..default() })),
                    Transform::from_xyz(-200.0, 90.0, -320.0),
                    scoped.clone(),
                ));
            }
            _ => {}
        }
    }
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(12.0).mesh().uv(16, 10))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.97, 0.85), emissive: LinearRgba::rgb(30.0, 28.0, 20.0), unlit: true, ..default() })),
        Transform::from_xyz(0.0, 200.0, -300.0),
        SunDisc,
        scoped.clone(),
    ));

    // lights
    commands.spawn((
        DirectionalLight { illuminance: 12000.0, shadows_enabled: true, color: Color::srgb(1.0, 0.97, 0.9), ..default() },
        CascadeShadowConfigBuilder { num_cascades: 2, maximum_distance: 70.0, first_cascade_far_bound: 20.0, ..default() }.build(),
        Transform::from_xyz(40.0, 60.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        SunLight,
        scoped.clone(),
    ));
    commands.spawn((
        PointLight { intensity: 400_000.0, range: 26.0, color: Color::srgb(1.0, 0.9, 0.75), shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 6.5, 4.0),
        scoped.clone(),
    ));

    // shared materials
    let white = materials.add(StandardMaterial { base_color: Color::srgb(0.93, 0.94, 0.97), perceptual_roughness: 0.45, ..default() });
    let dark = materials.add(StandardMaterial { base_color: Color::srgb(0.16, 0.18, 0.24), metallic: 0.5, perceptual_roughness: 0.5, ..default() });
    let orange = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.55, 0.15), ..default() });
    let metal = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.73, 0.8), metallic: 0.8, perceptual_roughness: 0.35, ..default() });
    let glass_blue = materials.add(StandardMaterial { base_color: Color::srgba(0.5, 0.85, 1.0, 0.55), emissive: LinearRgba::rgb(0.1, 0.3, 0.5), alpha_mode: AlphaMode::Blend, ..default() });

    // ---- habitat dome
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(4.2).mesh().uv(32, 18))),
        MeshMaterial3d(white.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(1.0, 0.8, 1.0)),
        Obstacle { radius: 4.6 },
        scoped.clone(),
        Name::new("habitat"),
    ));
    commands.spawn((Mesh3d(meshes.add(Torus::new(4.1, 4.6))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, 0.25, 0.0), scoped.clone()));
    // windows
    for a in [0.6f32, 2.4, 3.9, 5.5] {
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.55))),
            MeshMaterial3d(glass_blue.clone()),
            Transform::from_xyz(a.cos() * 3.7, 1.7, a.sin() * 3.7).with_scale(Vec3::new(1.0, 0.8, 1.0)),
            scoped.clone(),
        ));
    }
    // door (airlock) at +Z
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 2.3, 0.6))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, 1.15, 4.0), scoped.clone()));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.2, 0.25, 2.2))), MeshMaterial3d(metal.clone()), Transform::from_xyz(0.0, 0.1, 5.0), scoped.clone()));
    commands.spawn((Transform::from_xyz(0.0, 0.0, 5.6), Station { kind: StationKind::Airlock, radius: 1.8 }, scoped.clone()));
    // antenna dish
    commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.08, 2.2))), MeshMaterial3d(metal.clone()), Transform::from_xyz(0.0, 4.2, 0.0), scoped.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(16, 8))),
        MeshMaterial3d(white.clone()),
        Transform::from_xyz(0.0, 5.3, 0.0).with_scale(Vec3::new(1.0, 0.25, 1.0)).with_rotation(Quat::from_rotation_x(-0.5)),
        scoped.clone(),
    ));
    // shield ring (visual feedback for shielding allocation)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(4.7).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgba(0.4, 1.0, 0.9, 0.0), emissive: LinearRgba::rgb(0.0, 0.0, 0.0), alpha_mode: AlphaMode::Blend, unlit: true, cull_mode: None, ..default() })),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(1.0, 0.82, 1.0)),
        ShieldRing,
        scoped.clone(),
    ));
    // leak patch (hidden until a leak)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.55, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.2, 0.2), emissive: LinearRgba::rgb(6.0, 0.4, 0.4), ..default() })),
        Transform::from_xyz(3.9, 1.6, 1.6).with_rotation(Quat::from_rotation_z(-1.2) * Quat::from_rotation_x(0.4)),
        WarnLamp(StationKind::Leak),
        Visibility::Hidden,
        scoped.clone(),
    ));
    commands.spawn((Transform::from_xyz(4.6, 0.0, 2.4), Station { kind: StationKind::Leak, radius: 1.8 }, scoped.clone()));

    // ---- power board
    commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.08, 1.4))), MeshMaterial3d(metal.clone()), Transform::from_xyz(-3.4, 0.7, 4.4), scoped.clone()));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.5, 1.1, 0.15))), MeshMaterial3d(dark.clone()), Transform::from_xyz(-3.4, 1.7, 4.4).with_rotation(Quat::from_rotation_x(-0.25)), scoped.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.25, 0.85, 0.05))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.8, 1.0), emissive: LinearRgba::rgb(0.6, 1.6, 2.4), ..default() })),
        Transform::from_xyz(-3.4, 1.72, 4.5).with_rotation(Quat::from_rotation_x(-0.25)),
        scoped.clone(),
    ));
    commands.spawn((Transform::from_xyz(-3.4, 0.0, 5.4), Station { kind: StationKind::Board, radius: 1.7 }, scoped.clone()));

    // ---- solar panels
    let panel_mesh = meshes.add(Cuboid::new(2.4, 0.08, 1.6));
    let panel_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.15, 0.35, 0.95), emissive: LinearRgba::rgb(0.1, 0.3, 0.9), metallic: 0.7, perceptual_roughness: 0.25, ..default() });
    for z in [-3.2f32, 0.0, 3.2] {
        commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.1, 1.2))), MeshMaterial3d(metal.clone()), Transform::from_xyz(-10.5, 0.6, z), scoped.clone()));
        commands.spawn((
            Mesh3d(panel_mesh.clone()),
            MeshMaterial3d(panel_mat.clone()),
            Transform::from_xyz(-10.5, 1.3, z).with_rotation(Quat::from_rotation_z(-0.6)),
            PanelGlass,
            Obstacle { radius: 1.1 },
            scoped.clone(),
        ));
    }
    commands.spawn((Transform::from_xyz(-8.6, 0.0, 0.0), Station { kind: StationKind::Panels, radius: 2.4 }, scoped.clone()));

    // ---- greenhouse
    let green_mat = materials.add(StandardMaterial { base_color: Color::srgba(0.6, 1.0, 0.75, 0.45), emissive: LinearRgba::rgb(0.05, 0.3, 0.1), alpha_mode: AlphaMode::Blend, ..default() });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.7, 6.0))),
        MeshMaterial3d(green_mat.clone()),
        Transform::from_xyz(7.8, 0.6, 1.5).with_rotation(Quat::from_rotation_z(PI / 2.0)),
        GreenGlass,
        Obstacle { radius: 2.0 },
        scoped.clone(),
    ));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(6.2, 0.5, 3.0))), MeshMaterial3d(white.clone()), Transform::from_xyz(7.8, 0.25, 1.5), scoped.clone()));
    for dx in [-2.0f32, 0.0, 2.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.4, 0.5, 1.4))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.25, 0.75, 0.3), ..default() })),
            Transform::from_xyz(7.8 + dx, 0.75, 1.5),
            scoped.clone(),
        ));
    }
    commands.spawn((Transform::from_xyz(7.8, 0.0, 4.4), Station { kind: StationKind::Greenhouse, radius: 2.0 }, scoped.clone()));

    // ---- oxygen generator
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.1, 2.8))),
        MeshMaterial3d(metal.clone()),
        Transform::from_xyz(-6.8, 1.4, 6.2),
        Obstacle { radius: 1.4 },
        scoped.clone(),
    ));
    commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.18, 5.0))), MeshMaterial3d(metal.clone()), Transform::from_xyz(-4.0, 0.7, 4.3).with_rotation(Quat::from_rotation_z(PI / 2.0) * Quat::from_rotation_x(0.6)), scoped.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.32))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.25, 0.2), emissive: LinearRgba::rgb(8.0, 0.6, 0.4), ..default() })),
        Transform::from_xyz(-6.8, 3.1, 6.2),
        WarnLamp(StationKind::Generator),
        Visibility::Hidden,
        scoped.clone(),
    ));
    commands.spawn((Transform::from_xyz(-6.8, 0.0, 8.2), Station { kind: StationKind::Generator, radius: 1.9 }, scoped.clone()));

    // ---- lab
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.8, 2.7, 3.2))),
        MeshMaterial3d(white.clone()),
        Transform::from_xyz(7.0, 1.35, -6.8),
        Obstacle { radius: 2.6 },
        scoped.clone(),
    ));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.4, 1.0, 0.1))), MeshMaterial3d(glass_blue.clone()), Transform::from_xyz(7.0, 1.6, -5.17), scoped.clone()));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 1.2, 0.2))), MeshMaterial3d(orange.clone()), Transform::from_xyz(7.0, 0.6, -5.15), scoped.clone()));
    commands.spawn((Transform::from_xyz(7.0, 0.0, -4.2), Station { kind: StationKind::Lab, radius: 2.0 }, scoped.clone()));

    // ---- battery
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.2, 1.5, 1.3))), MeshMaterial3d(dark.clone()), Transform::from_xyz(-4.5, 0.75, -6.8), Obstacle { radius: 1.5 }, scoped.clone()));
    for i in 0..5u32 {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.9, 0.08))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.4, 1.0, 0.5), emissive: LinearRgba::rgb(0.5, 4.0, 1.0), ..default() })),
            Transform::from_xyz(-5.3 + i as f32 * 0.4, 0.8, -6.1),
            BatteryBar(i),
            scoped.clone(),
        ));
    }

    // ---- rover (decor)
    let rv = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.35, 0.2), ..default() });
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.6, 0.9, 1.6))), MeshMaterial3d(rv.clone()), Transform::from_xyz(12.0, 0.95, 7.5), Obstacle { radius: 1.8 }, scoped.clone()));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.2, 0.6, 1.2))), MeshMaterial3d(glass_blue.clone()), Transform::from_xyz(12.6, 1.6, 7.5), scoped.clone()));
    for (dx, dz) in [(-0.9, -0.9), (0.9, -0.9), (-0.9, 0.9), (0.9, 0.9)] {
        commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.45, 0.4))), MeshMaterial3d(dark.clone()), Transform::from_xyz(12.0 + dx, 0.45, 7.5 + dz).with_rotation(Quat::from_rotation_x(PI / 2.0)), scoped.clone()));
    }
    // flag
    commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.05, 3.0))), MeshMaterial3d(metal.clone()), Transform::from_xyz(4.5, 1.5, 7.5), scoped.clone()));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.2, 0.8, 0.04))), MeshMaterial3d(orange.clone()), Transform::from_xyz(5.1, 2.6, 7.5), scoped.clone()));

    // ---- rocks
    let rock_mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let rock_mat = materials.add(StandardMaterial { base_color: m.ground2.mix(&Color::BLACK, 0.25), perceptual_roughness: 1.0, ..default() });
    if map != MapId::Orbit {
        for _ in 0..70 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(BASE_R + 2.0, WORLD_R - 3.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            let s = rng.range(0.4, 2.2);
            commands.spawn((
                Mesh3d(rock_mesh.clone()),
                MeshMaterial3d(rock_mat.clone()),
                Transform::from_xyz(x, height(map, x, z) + s * 0.25, z).with_scale(Vec3::new(s, s * rng.range(0.5, 0.9), s * rng.range(0.7, 1.3))).with_rotation(Quat::from_rotation_y(rng.range(0.0, TAU))),
                Obstacle { radius: if s > 1.2 { s * 0.9 } else { 0.0 } },
                scoped.clone(),
            ));
        }
    }

    // ---- shared assets for runtime spawns
    let gem_mesh = meshes.add(Mesh::from(Cone { radius: 0.35, height: 0.6 }));
    let gem_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.95, 1.0), emissive: LinearRgba::rgb(0.6, 2.5, 3.0), ..default() });
    let beam_mesh = meshes.add(Cylinder::new(0.45, 16.0));
    let beam_mat = materials.add(StandardMaterial { base_color: Color::srgba(1.0, 0.85, 0.3, 0.35), emissive: LinearRgba::rgb(2.0, 1.5, 0.4), alpha_mode: AlphaMode::Blend, unlit: true, cull_mode: None, ..default() });
    let shield_mat = materials.add(StandardMaterial { base_color: Color::srgba(0.4, 1.0, 0.9, 0.2), alpha_mode: AlphaMode::Blend, unlit: true, ..default() });
    let bubble_mat = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.2), emissive: LinearRgba::rgb(4.0, 3.0, 0.5), unlit: true, ..default() });
    // beams over every station (hidden by default)
    for (kind, p) in [
        (StationKind::Generator, Vec3::new(-6.8, 0.0, 6.2)),
        (StationKind::Panels, Vec3::new(-10.5, 0.0, 0.0)),
        (StationKind::Greenhouse, Vec3::new(7.8, 0.0, 1.5)),
        (StationKind::Lab, Vec3::new(7.0, 0.0, -6.8)),
        (StationKind::Airlock, Vec3::new(0.0, 0.0, 5.6)),
        (StationKind::Leak, Vec3::new(4.6, 0.0, 2.4)),
        (StationKind::Cargo, Vec3::new(0.0, 0.0, 13.0)),
        (StationKind::Board, Vec3::new(-3.4, 0.0, 5.4)),
    ] {
        commands.spawn((Mesh3d(beam_mesh.clone()), MeshMaterial3d(beam_mat.clone()), Transform::from_translation(p + Vec3::Y * 8.0), Beam(kind), Visibility::Hidden, scoped.clone()));
    }
    // samples
    for _ in 0..sample_count {
        spawn_sample(commands, &gem_mesh, &gem_mat, map, rng);
    }
    commands.insert_resource(WorldAssets { gem_mesh, gem_mat, beam_mesh, beam_mat, shield_mat, bubble_mat });

    // ---- crewmate NPC
    let npc = spawn_astronaut(commands, meshes, materials, Color::srgb(1.0, 0.55, 0.2), Color::srgb(1.0, 0.8, 0.3), "none");
    commands.entity(npc).insert((
        Transform::from_xyz(4.0, 0.0, 4.0),
        Crewmate { t: 1.0, from: Vec3::new(4.0, 0.0, 4.0), to: Vec3::new(4.0, 0.0, 4.0), wait: 1.0 },
        WalkAnim::default(),
        Station { kind: StationKind::Crewmate, radius: 2.0 },
        scoped.clone(),
    ));
    commands.entity(npc).with_children(|p| {
        p.spawn((
            Mesh3d(meshes.add(Sphere::new(0.28))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.2), emissive: LinearRgba::rgb(4.0, 3.0, 0.5), unlit: true, ..default() })),
            Transform::from_xyz(0.0, 2.6, 0.0),
            Exclaim,
            Visibility::Hidden,
        ));
    });

    // dust cloud (storm) and flare glow, hidden
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(40.0).mesh().uv(16, 8))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgba(0.85, 0.55, 0.3, 0.45), alpha_mode: AlphaMode::Blend, unlit: true, cull_mode: None, double_sided: true, ..default() })),
        Transform::IDENTITY,
        DustCloud,
        Visibility::Hidden,
        scoped.clone(),
    ));
}

pub fn spawn_sample(commands: &mut Commands, mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, map: MapId, rng: &mut crate::sim::Rng) {
    let a = rng.range(0.0, TAU);
    let r = rng.range(BASE_R + 4.0, WORLD_R - 8.0);
    let (x, z) = (a.cos() * r, a.sin() * r);
    let y = height(map, x, z);
    commands
        .spawn((
            Transform::from_xyz(x, y + 0.8, z),
            Visibility::default(),
            Station { kind: StationKind::Sample, radius: 1.6 },
            Spin(1.6),
            Bob { base: y + 0.8, amp: 0.2, speed: 2.0 },
            StateScoped(AppState::Playing),
        ))
        .with_children(|p| {
            p.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.0, 0.3, 0.0)));
            p.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.0, -0.3, 0.0).with_rotation(Quat::from_rotation_x(PI))));
        });
}

/// Builds a low-poly astronaut. The model faces -Z (Bevy forward). Returns the root entity
/// (no Transform set — the caller adds it).
pub fn spawn_astronaut(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    suit: Color,
    visor: Color,
    hat: &str,
) -> Entity {
    let suit_m = materials.add(StandardMaterial { base_color: suit, perceptual_roughness: 0.6, ..default() });
    let white = materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.96, 1.0), perceptual_roughness: 0.4, ..default() });
    let visor_m = materials.add(StandardMaterial { base_color: visor, metallic: 0.9, perceptual_roughness: 0.15, ..default() });
    let dark = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.22, 0.3), ..default() });
    let accent = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.2), emissive: LinearRgba::rgb(1.5, 1.1, 0.2), ..default() });
    let body = meshes.add(Capsule3d::new(0.32, 0.55));
    let limb = meshes.add(Capsule3d::new(0.11, 0.45));
    let helmet = meshes.add(Sphere::new(0.38).mesh().uv(24, 12));

    let root = commands.spawn((Transform::IDENTITY, Visibility::default(), Name::new("astronaut"))).id();
    commands.entity(root).with_children(|p| {
        p.spawn((Mesh3d(body.clone()), MeshMaterial3d(suit_m.clone()), Transform::from_xyz(0.0, 1.0, 0.0)));
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 0.6, 0.28))), MeshMaterial3d(white.clone()), Transform::from_xyz(0.0, 1.1, 0.34)));
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.36, 0.22, 0.12))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, 1.05, -0.3)));
        p.spawn((Mesh3d(helmet), MeshMaterial3d(white.clone()), Transform::from_xyz(0.0, 1.72, 0.0)));
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.3).mesh().uv(20, 10))), MeshMaterial3d(visor_m), Transform::from_xyz(0.0, 1.74, -0.16).with_scale(Vec3::new(1.0, 0.75, 0.6))));
        // limbs (pivot at the top so they swing)
        for (kind, x, y) in [(Limb::LeftArm, -0.42, 1.25), (Limb::RightArm, 0.42, 1.25), (Limb::LeftLeg, -0.17, 0.62), (Limb::RightLeg, 0.17, 0.62)] {
            p.spawn((Transform::from_xyz(x, y, 0.0), Visibility::default(), kind)).with_children(|l| {
                l.spawn((Mesh3d(limb.clone()), MeshMaterial3d(suit_m.clone()), Transform::from_xyz(0.0, -0.3, 0.0)));
                let is_leg = matches!(kind, Limb::LeftLeg | Limb::RightLeg);
                l.spawn((Mesh3d(meshes.add(Sphere::new(if is_leg { 0.15 } else { 0.12 }))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, -0.58, if is_leg { -0.04 } else { 0.0 })));
            });
        }
        match hat {
            "antenna" => {
                p.spawn((Mesh3d(meshes.add(Cylinder::new(0.03, 0.5))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.22, 2.25, 0.0)));
                p.spawn((Mesh3d(meshes.add(Sphere::new(0.07))), MeshMaterial3d(accent.clone()), Transform::from_xyz(0.22, 2.52, 0.0)));
            }
            "star" => {
                p.spawn((Mesh3d(meshes.add(Mesh::from(Cone { radius: 0.16, height: 0.28 }))), MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 2.45, 0.0), Spin(3.0)));
                p.spawn((Mesh3d(meshes.add(Mesh::from(Cone { radius: 0.16, height: 0.28 }))), MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 2.17, 0.0).with_rotation(Quat::from_rotation_x(PI))));
            }
            "cat" => {
                let ear = meshes.add(Mesh::from(Cone { radius: 0.13, height: 0.3 }));
                let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.6, 0.7), ..default() });
                p.spawn((Mesh3d(ear.clone()), MeshMaterial3d(pink.clone()), Transform::from_xyz(-0.22, 2.12, 0.0).with_rotation(Quat::from_rotation_z(0.3))));
                p.spawn((Mesh3d(ear), MeshMaterial3d(pink), Transform::from_xyz(0.22, 2.12, 0.0).with_rotation(Quat::from_rotation_z(-0.3))));
            }
            "crown" => {
                p.spawn((Mesh3d(meshes.add(Torus::new(0.22, 0.3))), MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 2.06, 0.0)));
                for i in 0..5 {
                    let a = i as f32 / 5.0 * TAU;
                    p.spawn((Mesh3d(meshes.add(Mesh::from(Cone { radius: 0.06, height: 0.18 }))), MeshMaterial3d(accent.clone()), Transform::from_xyz(a.cos() * 0.26, 2.16, a.sin() * 0.26)));
                }
            }
            "alien" => {
                let green = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 1.0, 0.4), ..default() });
                p.spawn((Mesh3d(meshes.add(Sphere::new(0.17))), MeshMaterial3d(green), Transform::from_xyz(-0.5, 1.62, 0.0), Bob { base: 1.62, amp: 0.06, speed: 3.0 }));
                p.spawn((Mesh3d(meshes.add(Sphere::new(0.05))), MeshMaterial3d(dark.clone()), Transform::from_xyz(-0.56, 1.66, -0.13)));
                p.spawn((Mesh3d(meshes.add(Sphere::new(0.05))), MeshMaterial3d(dark.clone()), Transform::from_xyz(-0.44, 1.66, -0.13)));
            }
            _ => {}
        }
    });
    root
}

/// Environment that reacts to the simulation: sun, sky, panels, glass, shield, battery, beams, lamps.
pub fn environment_system(
    sim: Option<Res<Sim>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), (With<SunLight>, Without<SunDisc>)>,
    mut disc: Query<&mut Transform, (With<SunDisc>, Without<SunLight>)>,
    mut ambient: ResMut<AmbientLight>,
    panels: Query<&MeshMaterial3d<StandardMaterial>, With<PanelGlass>>,
    green: Query<&MeshMaterial3d<StandardMaterial>, With<GreenGlass>>,
    shield: Query<&MeshMaterial3d<StandardMaterial>, With<ShieldRing>>,
    mut bars: Query<(&BatteryBar, &mut Visibility), (Without<Beam>, Without<WarnLamp>, Without<Exclaim>, Without<DustCloud>)>,
    mut beams: Query<(&Beam, &mut Visibility), (Without<BatteryBar>, Without<WarnLamp>, Without<Exclaim>, Without<DustCloud>)>,
    mut lamps: Query<(&WarnLamp, &mut Visibility), (Without<BatteryBar>, Without<Beam>, Without<Exclaim>, Without<DustCloud>)>,
    mut excl: Query<&mut Visibility, (With<Exclaim>, Without<BatteryBar>, Without<Beam>, Without<WarnLamp>, Without<DustCloud>)>,
    mut dust: Query<(&mut Visibility, &mut Transform), (With<DustCloud>, Without<BatteryBar>, Without<Beam>, Without<WarnLamp>, Without<Exclaim>, Without<SunLight>, Without<SunDisc>)>,
    players: Query<&crate::player::Player>,
) {
    let Some(sim) = sim else { return };
    let day = sim.daylight();
    let m = sim.def();
    let t = time.elapsed_secs();
    // sun position: rises at t=0, sets at t=1
    let ang = sim.t * PI;
    let dir = Vec3::new(ang.cos() * 0.8, ang.sin().max(0.02), -0.5).normalize();
    for (mut l, mut tr) in &mut sun {
        l.illuminance = 1500.0 + 10500.0 * day;
        if sim.flare.is_some() {
            l.color = Color::srgb(1.0, 0.85, 0.7);
        } else {
            l.color = Color::srgb(1.0, 0.97, 0.9);
        }
        *tr = Transform::from_translation(dir * 80.0).looking_at(Vec3::ZERO, Vec3::Y);
    }
    for mut tr in &mut disc {
        tr.translation = dir * 330.0;
        let s = if sim.flare.is_some() { 1.6 + (t * 6.0).sin() * 0.2 } else { 1.0 };
        tr.scale = Vec3::splat(s);
    }
    ambient.brightness = 120.0 + 260.0 * day + if m.sky.to_linear().luminance() < 0.1 { 0.0 } else { 120.0 };

    let any_power = sim.gen_now > 0.5;
    for h in &panels {
        if let Some(mat) = materials.get_mut(&h.0) {
            let e = if any_power && !sim.is_night() { 0.3 * day + 0.1 } else { 0.02 };
            let dusty = sim.dust_left > 0 && !sim.panels_cleaned;
            mat.emissive = if dusty { LinearRgba::rgb(0.05, 0.04, 0.02) } else { LinearRgba::rgb(0.1 * e * 3.0, 0.3 * e * 3.0, 0.9 * e * 3.0) };
            mat.base_color = if dusty { Color::srgb(0.5, 0.4, 0.3) } else { Color::srgb(0.15, 0.35, 0.95) };
        }
    }
    for h in &green {
        if let Some(mat) = materials.get_mut(&h.0) {
            let e = sim.alloc.green as f32 * 0.25;
            mat.emissive = LinearRgba::rgb(0.05 + e * 0.3, 0.3 + e * 1.5, 0.1 + e * 0.4);
        }
    }
    for h in &shield {
        if let Some(mat) = materials.get_mut(&h.0) {
            let s = sim.shield_red() / 0.9;
            let pulse = 0.5 + 0.5 * (t * 2.0).sin();
            mat.base_color = Color::srgba(0.4, 1.0, 0.9, 0.08 * s + 0.06 * s * pulse);
            mat.emissive = LinearRgba::rgb(0.2 * s, 0.8 * s, 0.7 * s);
        }
    }
    let charge = (sim.battery / crate::sim::BATTERY_MAX * 5.0).ceil() as u32;
    for (bar, mut v) in &mut bars {
        *v = if bar.0 < charge { Visibility::Visible } else { Visibility::Hidden };
    }
    let carrying: u32 = players.iter().map(|p| p.carried).sum();
    for (beam, mut v) in &mut beams {
        let on = match beam.0 {
            StationKind::Generator => sim.gen_broken,
            StationKind::Panels => sim.dust_left > 0 && !sim.panels_cleaned,
            StationKind::Greenhouse => sim.harvest_ready,
            StationKind::Lab => carrying > 0 && sim.alloc.sci > 0,
            StationKind::Airlock => sim.flare.is_some(),
            StationKind::Leak => sim.leak,
            StationKind::Cargo => sim.cargo,
            StationKind::Board => sim.sol == 1 && sim.t < 0.5 && sim.alloc.used() <= 4,
            _ => false,
        };
        *v = if on { Visibility::Visible } else { Visibility::Hidden };
    }
    for (lamp, mut v) in &mut lamps {
        let on = match lamp.0 {
            StationKind::Generator => sim.gen_broken && (t * 6.0).sin() > 0.0,
            StationKind::Leak => sim.leak,
            _ => false,
        };
        *v = if on { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut v in &mut excl {
        *v = if sim.emotion.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    for (mut v, mut tr) in &mut dust {
        *v = if sim.dust_left > 0 { Visibility::Visible } else { Visibility::Hidden };
        tr.rotation = Quat::from_rotation_y(t * 0.15);
    }
}

pub fn animate_props(time: Res<Time>, mut spins: Query<(&Spin, &mut Transform), Without<Bob>>, mut bobs: Query<(&Bob, Option<&Spin>, &mut Transform)>) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    for (s, mut tr) in &mut spins {
        tr.rotate_y(s.0 * dt);
    }
    for (b, s, mut tr) in &mut bobs {
        tr.translation.y = b.base + (t * b.speed).sin() * b.amp;
        if let Some(s) = s {
            tr.rotate_y(s.0 * dt);
        }
    }
}

#[derive(Component, Default)]
pub struct WalkAnim {
    pub moving: bool,
    pub phase: f32,
}

pub fn crewmate_walk(time: Res<Time>, sim: Option<Res<Sim>>, mut q: Query<(&mut Crewmate, &mut Transform, &mut WalkAnim)>) {
    let Some(sim) = sim else { return };
    let dt = time.delta_secs();
    let points = [Vec3::new(4.0, 0.0, 4.5), Vec3::new(9.5, 0.0, 5.5), Vec3::new(3.0, 0.0, -3.5), Vec3::new(-5.5, 0.0, -3.0), Vec3::new(-7.5, 0.0, 9.0)];
    for (mut c, mut tr, mut anim) in &mut q {
        if sim.emotion.is_some() || sim.paused {
            anim.moving = false;
            continue;
        }
        if c.wait > 0.0 {
            c.wait -= dt;
            anim.moving = false;
            continue;
        }
        c.t += dt / (c.from.distance(c.to).max(1.0) / 1.6);
        if c.t >= 1.0 {
            c.from = c.to;
            let idx = ((time.elapsed_secs() * 7.3) as usize + 1) % points.len();
            c.to = points[idx];
            c.t = 0.0;
            c.wait = 2.5;
        }
        let p = c.from.lerp(c.to, c.t.min(1.0));
        let d = c.to - c.from;
        if d.length() > 0.01 {
            tr.rotation = Quat::from_rotation_y((-d.x).atan2(-d.z));
        }
        tr.translation = Vec3::new(p.x, height(sim.map, p.x, p.z), p.z);
        anim.moving = c.t < 1.0;
        anim.phase += dt * 7.0;
    }
}

/// Swings arms and legs of any astronaut whose root has a `WalkAnim`.
pub fn animate_limbs(mut limbs: Query<(&Limb, &mut Transform, &ChildOf)>, roots: Query<&WalkAnim>) {
    for (limb, mut tr, parent) in &mut limbs {
        let Ok(anim) = roots.get(parent.parent()) else { continue };
        let sgn = match limb { Limb::LeftLeg | Limb::RightArm => 1.0, _ => -1.0 };
        let target = if anim.moving { anim.phase.sin() * 0.65 * sgn } else { 0.0 };
        tr.rotation = tr.rotation.slerp(Quat::from_rotation_x(target), 0.35);
    }
}
