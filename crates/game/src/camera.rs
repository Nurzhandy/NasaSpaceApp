//! B2 — top-down camera: smooth follow; in co-op it frames both astronauts (shared camera).

use crate::player::Player;
use crate::world::to_world;
use crate::{AppState, MainCam, Set, CELL};
use ja_core::game::WORLD_R;
use bevy::prelude::*;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, follow.in_set(Set::View).run_if(in_state(AppState::Playing)))
            .add_systems(OnExit(AppState::Playing), reset)
            .add_systems(OnEnter(AppState::Playing), snap);
    }
}

fn ortho(p: &mut Projection) -> Option<&mut OrthographicProjection> {
    match p {
        Projection::Orthographic(o) => Some(o),
        _ => None,
    }
}

fn snap(mut cam: Query<&mut Transform, With<MainCam>>) {
    if let Ok(mut t) = cam.single_mut() {
        let c = to_world(0.5, 1.5);
        t.translation = Vec3::new(c.x, c.y, t.translation.z);
    }
}

fn follow(time: Res<Time>, players: Query<&Player>, mut cam: Query<(&mut Transform, &mut Projection), With<MainCam>>, windows: Query<&Window>, net: Res<crate::net::Net>) {
    let Ok((mut t, mut proj)) = cam.single_mut() else { return };
    let remote = net.remote_idx();
    let pts: Vec<Vec2> = players.iter().filter(|p| Some(p.idx) != remote).map(|p| to_world(p.pos.x, p.pos.y + 0.8)).collect();
    if pts.is_empty() {
        return;
    }
    let center = pts.iter().copied().sum::<Vec2>() / pts.len() as f32;
    let (min, max) = pts.iter().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(a, b), p| (a.min(*p), b.max(*p)));
    let span = max - min;
    let (ww, wh) = windows.single().map(|w| (w.width(), w.height())).unwrap_or((1280.0, 760.0));
    let want_scale = ((span.x + 700.0) / ww).max((span.y + 450.0) / wh).clamp(1.0, 1.9);
    let k = (6.0 * time.delta_secs()).min(1.0);
    if let Some(o) = ortho(&mut proj) {
        o.scale += (want_scale - o.scale) * k;
        // keep the view inside the map (with a little margin)
        let half = Vec2::new(ww, wh) * o.scale / 2.0;
        let edge = (WORLD_R + 4.0) * CELL;
        let lo = Vec2::splat(-edge) + half;
        let hi = Vec2::splat(edge) - half;
        let target = Vec2::new(if lo.x < hi.x { center.x.clamp(lo.x, hi.x) } else { (lo.x + hi.x) / 2.0 }, if lo.y < hi.y { center.y.clamp(lo.y, hi.y) } else { (lo.y + hi.y) / 2.0 });
        let cur = t.translation.truncate();
        let n = cur + (target - cur) * k;
        t.translation = Vec3::new(n.x, n.y, t.translation.z);
    }
}

fn reset(mut cam: Query<(&mut Transform, &mut Projection), With<MainCam>>) {
    if let Ok((mut t, mut p)) = cam.single_mut() {
        t.translation = Vec3::new(0.0, 0.0, t.translation.z);
        if let Some(o) = ortho(&mut p) {
            o.scale = 1.0;
        }
    }
}
