//! B8 — sound effects for game events and UI, plus a looping chiptune soundtrack.

use crate::kit::Act;
use crate::world::{Happened, Refused};
use crate::{Music, Prof};
use ja_core::GameEvent;
use bevy::audio::Volume;
use bevy::prelude::*;
use std::collections::HashMap;

/// Ask for a one-shot sound by name (file in assets/sounds).
#[derive(Event, Clone, Copy)]
pub struct Play(pub &'static str);

#[derive(Resource)]
struct Lib(HashMap<&'static str, Handle<AudioSource>>);

#[derive(Component)]
struct MusicTrack;

const NAMES: &[&str] = &["click", "coin", "good", "bad", "alarm", "radio", "badge", "repair", "pickup", "step", "launch", "wind", "music"];

pub struct AudioFx;
impl Plugin for AudioFx {
    fn build(&self, app: &mut App) {
        app.add_event::<Play>().add_systems(Startup, load).add_systems(Update, (from_events, play, music).chain().after(crate::Set::View));
    }
}

fn load(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Lib(NAMES.iter().map(|n| (*n, server.load(format!("sounds/{n}.wav")))).collect()));
}

fn from_events(mut ev: EventReader<Happened>, mut refused: EventReader<Refused>, mut acts: EventReader<Act>, mut out: EventWriter<Play>) {
    for Happened(e) in ev.read() {
        let s = match e {
            GameEvent::Flare { .. } | GameEvent::DustStorm => "alarm",
            GameEvent::ModuleBroken(_) | GameEvent::Leak(_) | GameEvent::LowOxygen | GameEvent::BatteryEmpty => "bad",
            GameEvent::HarvestReady | GameEvent::Repaired(_) | GameEvent::Cleaned(_) | GameEvent::Supported(_) | GameEvent::FlareEnded | GameEvent::FeelingRight(_) | GameEvent::Harvested { .. } | GameEvent::CargoUnloaded { .. } => "good",
            GameEvent::CargoLanding { .. } => "launch",
            GameEvent::CrewNeedsSupport(_) | GameEvent::ForecastArrived(_) | GameEvent::NewSol(_) | GameEvent::SignalLost | GameEvent::Night | GameEvent::Discovery(_) => "radio",
            GameEvent::Collected { .. } => "pickup",
            GameEvent::SamplesDelivered { .. } => "coin",
            GameEvent::Built { .. } | GameEvent::Demolished { .. } => "repair",
            GameEvent::Sheltered { .. } | GameEvent::LeftShelter { .. } | GameEvent::FeelingTryAgain(_) | GameEvent::DustGone | GameEvent::ForecastSent { .. } => "click",
            GameEvent::MissionEnded(r) => {
                if r.won { "badge" } else { "launch" }
            }
        };
        out.write(Play(s));
    }
    for _ in refused.read() {
        out.write(Play("bad"));
    }
    for _ in acts.read() {
        out.write(Play("click"));
    }
}

fn play(mut commands: Commands, mut ev: EventReader<Play>, lib: Res<Lib>, prof: Res<Prof>) {
    let mut seen: Vec<&str> = vec![];
    for Play(n) in ev.read() {
        if !prof.0.sound || seen.contains(n) {
            continue;
        }
        seen.push(n);
        if let Some(h) = lib.0.get(n) {
            let vol = if *n == "step" { 0.25 } else { 0.7 };
            commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol))));
        }
    }
}

fn music(mut commands: Commands, on: Res<Music>, lib: Res<Lib>, q: Query<Entity, With<MusicTrack>>) {
    let playing = !q.is_empty();
    if on.0 && !playing {
        if let Some(h) = lib.0.get("music") {
            commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.35)), MusicTrack));
        }
    } else if !on.0 && playing {
        for e in &q {
            commands.entity(e).despawn();
        }
    }
}
