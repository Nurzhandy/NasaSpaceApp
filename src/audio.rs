//! Sound effects (generated WAVs in assets/sounds) and ambient loop.

use bevy::audio::Volume;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::HashMap;

pub const SOUNDS: &[&str] = &["click", "coin", "good", "bad", "alarm", "radio", "badge", "repair", "pickup", "step", "launch", "wind"];

#[derive(Resource, Default)]
pub struct SfxLib(pub HashMap<&'static str, Handle<AudioSource>>);

#[derive(Component)]
pub struct Ambient;

pub fn load_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut lib = SfxLib::default();
    for s in SOUNDS {
        lib.0.insert(s, asset_server.load(format!("sounds/{s}.wav")));
    }
    commands.insert_resource(lib);
}

#[derive(SystemParam)]
pub struct Sfx<'w, 's> {
    commands: Commands<'w, 's>,
    lib: Res<'w, SfxLib>,
    profile: Res<'w, crate::save::Profile>,
}

impl Sfx<'_, '_> {
    pub fn play(&mut self, name: &str) {
        self.play_vol(name, 0.8);
    }
    pub fn play_vol(&mut self, name: &str, vol: f32) {
        if !self.profile.sound {
            return;
        }
        if let Some(h) = self.lib.0.get(name) {
            self.commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol))));
        }
    }
}

pub fn start_ambient(mut commands: Commands, lib: Res<SfxLib>, profile: Res<crate::save::Profile>) {
    if !profile.sound {
        return;
    }
    if let Some(h) = lib.0.get("wind") {
        commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.25)), Ambient, StateScoped(crate::AppState::Playing)));
    }
}
