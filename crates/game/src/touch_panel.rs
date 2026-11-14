//! A4 — the Touch Panel: a space map in the cockpit. Touch or click where the ship should go
//! (arrows / WASD / stick also steer); rocks fly in waves. A hit damages the hull and may punch a
//! hole in a random room, which becomes a repair task. In co-op the ship flies to the average touch.

use crate::art::Art;
use crate::kit::{self, Inputs, C_MUTED};
use crate::ship::{Camp, ShipUi};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use ja_core::ship::{SHIP_R, WAVE_SECONDS};
use ja_core::{CampaignCommand, Lang};

const W: f32 = 700.0;
const H: f32 = 380.0;

#[derive(Component)]
pub struct Panel;
#[derive(Component)]
pub struct RockDot;
#[derive(Component)]
pub struct ShipDot;
#[derive(Component)]
pub struct WaveBar;

pub fn build(p: &mut ChildSpawnerCommands, art: &Art, l: Lang) {
    kit::text(p, l.pick("Касайся карты (или стрелки / WASD), чтобы увести корабль от камней", "Touch the map (or arrows / WASD) to steer away from the rocks"), &art.head, 17.0, kit::C_WARN);
    p.spawn((
        Button,
        Panel,
        RelativeCursorPosition::default(),
        Node { width: Val::Px(W), height: Val::Px(H), overflow: Overflow::clip(), ..default() },
        BackgroundColor(Color::srgb(0.01, 0.02, 0.07)),
        BorderRadius::all(Val::Px(8.0)),
    ))
    .with_children(|m| {
        // a few fixed stars
        for i in 0..40u32 {
            let h = i.wrapping_mul(2_654_435_761);
            let x = ((h >> 8) % 700) as f32;
            let y = ((h >> 20) % 380) as f32;
            m.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(x), top: Val::Px(y), width: Val::Px(2.0), height: Val::Px(2.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.5))));
        }
        m.spawn((ImageNode::new(art.icon("rocket")), Node { position_type: PositionType::Absolute, width: Val::Px(40.0), height: Val::Px(40.0), ..default() }, ShipDot));
    });
    p.spawn((Node { width: Val::Px(W), height: Val::Px(8.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)))).with_children(|b| {
        b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(kit::C_ACCENT), WaveBar));
    });
    kit::text(p, l.pick("Игровая условность: между Землёй и Луной почти нет астероидов.", "Game convention: there are almost no asteroids between Earth and the Moon."), &art.body, 13.0, C_MUTED);
}

/// Steering from touch / mouse / keys, and moving the rocks on the map every frame.
#[allow(clippy::too_many_arguments)]
pub fn update_rocks(
    mut commands: Commands,
    mut camp: ResMut<Camp>,
    ui: Res<ShipUi>,
    inputs: Res<Inputs>,
    art: Res<Art>,
    panel: Query<(Entity, &Interaction, &RelativeCursorPosition), With<Panel>>,
    old: Query<Entity, With<RockDot>>,
    mut ship: Query<&mut Node, (With<ShipDot>, Without<WaveBar>)>,
    mut wave: Query<&mut Node, (With<WaveBar>, Without<ShipDot>)>,
) {
    let Some((player, _)) = ui.open else { return };
    let Ok((panel, inter, rel)) = panel.single() else { return };
    // touch / click: fly there
    if *inter == Interaction::Pressed {
        if let Some(n) = rel.normalized {
            let _ = camp.0.apply(player, CampaignCommand::Steer { x: n.x, y: 1.0 - n.y });
        }
    }
    // keys / stick: nudge the target
    if let Some(pad) = inputs.0.get(player as usize) {
        if pad.mv.length() > 0.1 {
            if let Some(f) = &camp.0.state().field {
                let (x, y) = (f.ship.0 + pad.mv.x * 0.2, f.ship.1 + pad.mv.y * 0.2);
                let _ = camp.0.apply(player, CampaignCommand::Steer { x, y });
            }
        }
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(f) = &camp.0.state().field else { return };
    for mut n in &mut ship {
        n.left = Val::Px(f.ship.0 * W - 20.0);
        n.top = Val::Px((1.0 - f.ship.1) * H - 20.0);
    }
    for mut n in &mut wave {
        n.width = Val::Percent((f.t_left / WAVE_SECONDS).clamp(0.0, 1.0) * 100.0);
    }
    let _ = SHIP_R;
    commands.entity(panel).with_children(|m| {
        for r in &f.rocks {
            let s = r.r * 2.0 * H;
            m.spawn((ImageNode::new(art.icon("rock")), Node { position_type: PositionType::Absolute, left: Val::Px(r.x * W - s / 2.0), top: Val::Px((1.0 - r.y) * H - s / 2.0), width: Val::Px(s), height: Val::Px(s), ..default() }, RockDot));
        }
    });
}
