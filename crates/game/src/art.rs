//! Loads sprites, atlases, icons and fonts; holds the presentation-only colors
//! (skins, map palettes) because `astro_core` stores no colors.

use ja_core::data::{skins, SkinSlot};
use ja_core::{MapId, ModuleKind, SiteKind};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct Art {
    pub sprites: HashMap<&'static str, Handle<Image>>,
    pub icons: HashMap<&'static str, Handle<Image>>,
    pub astro_layout: Handle<TextureAtlasLayout>,
    pub tile_layout: Handle<TextureAtlasLayout>,
    pub hat_layout: Handle<TextureAtlasLayout>,
    pub head: Handle<Font>,
    pub body: Handle<Font>,
}

impl Art {
    pub fn sprite(&self, n: &str) -> Handle<Image> {
        self.sprites.get(n).cloned().unwrap_or_default()
    }
    pub fn icon(&self, n: &str) -> Handle<Image> {
        self.icons.get(n).or_else(|| self.icons.get("warning")).cloned().unwrap_or_default()
    }
}

const SPRITES: &[&str] = &[
    "antenna", "arrow", "astronaut", "astronaut_visor", "battery", "bubble", "cargo", "crater", "dot", "flame", "ghost", "greenhouse", "greenhouse_ripe", "habitat", "hats", "ice", "lab", "leak", "oxygen",
    "rock_big", "rock_small", "rover", "sample_0", "sample_1", "shield", "solar", "solar_dusty", "spark", "tiles_earth", "tiles_europa", "tiles_mars", "tiles_moon", "tiles_orbit", "wreck", "app_icon", "rtg", "ore", "lander",
];

pub const ICONS: &[&str] = &[
    "alien", "antenna_bars", "astronaut", "back", "battery", "beacon", "book", "box", "bread", "broom", "cat", "check", "clipboard", "coin", "comet", "coop", "cross", "crown", "desert", "droplet", "earth", "exit",
    "f_angry", "f_happy", "f_lonely", "f_proud", "f_sad", "f_tired", "f_worried", "flag", "flash", "food", "frown", "gamepad", "gear", "gem", "globe", "goggles", "hammer", "heart", "hint", "home",
    "hourglass", "house", "ice", "info", "key", "lab", "lock", "map", "mars", "medal", "moon", "mute", "neutral", "night", "ok_hand", "oxygen", "paint", "pause", "planet", "plant", "plate", "play",
    "power", "radiation", "radio", "retry", "robot", "rock", "rocket", "rover", "run", "s_call", "s_hug", "s_music", "s_photo", "s_talk", "sad_no", "satellite", "science", "seedling", "shield",
    "shirt", "shop", "smile", "snow", "sos", "sound", "sparkles", "speech", "star", "stopwatch", "strong", "sun", "teacher", "test", "thumbs", "tornado", "trophy", "walk", "warning", "wind", "wrench",
];

pub fn load(mut commands: Commands, server: Res<AssetServer>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let sprites = SPRITES.iter().map(|n| (*n, server.load(format!("sprites/{n}.png")))).collect();
    // icons are smooth pictures drawn bigger than shown: linear filtering keeps them clear when scaled
    let icons = ICONS.iter().map(|n| (*n, server.load_with_settings(format!("icons/{n}.png"), |s: &mut bevy::image::ImageLoaderSettings| s.sampler = bevy::image::ImageSampler::linear()))).collect();
    commands.insert_resource(Art {
        sprites,
        icons,
        astro_layout: layouts.add(TextureAtlasLayout::from_grid(UVec2::new(16, 24), 4, 4, None, None)),
        tile_layout: layouts.add(TextureAtlasLayout::from_grid(UVec2::new(16, 16), 4, 1, None, None)),
        hat_layout: layouts.add(TextureAtlasLayout::from_grid(UVec2::new(16, 16), 5, 1, None, None)),
        head: server.load("fonts/RussoOne-Regular.ttf"),
        body: server.load("fonts/PTSans-Bold.ttf"),
    });
}

pub fn module_sprite(kind: ModuleKind) -> &'static str {
    match kind {
        ModuleKind::Habitat => "habitat",
        ModuleKind::Solar => "solar",
        ModuleKind::Battery => "battery",
        ModuleKind::Rtg => "rtg",
        ModuleKind::OxygenGen => "oxygen",
        ModuleKind::Greenhouse => "greenhouse",
        ModuleKind::Shield => "shield",
        ModuleKind::Lab => "lab",
        ModuleKind::Antenna => "antenna",
        ModuleKind::Garage => "rover",
    }
}

/// Native pixel size of a module sprite.
pub fn module_px(kind: ModuleKind) -> Vec2 {
    match kind {
        ModuleKind::Habitat => Vec2::new(48.0, 48.0),
        ModuleKind::Greenhouse => Vec2::new(48.0, 32.0),
        ModuleKind::Battery => Vec2::new(16.0, 16.0),
        ModuleKind::Antenna => Vec2::new(16.0, 32.0),
        ModuleKind::Garage => Vec2::new(32.0, 24.0),
        _ => Vec2::new(32.0, 32.0),
    }
}

/// On-screen size of a module: as wide as its footprint, height keeps the art's aspect ratio
/// (tall things like the antenna stick up above their cell).
pub fn module_size(kind: ModuleKind, w: i32) -> Vec2 {
    let px = module_px(kind);
    let width = w as f32 * crate::CELL;
    Vec2::new(width, width * px.y / px.x)
}

pub fn site_sprite(kind: SiteKind) -> &'static str {
    match kind {
        SiteKind::Sample => "sample_0",
        SiteKind::Ore => "ore",
        SiteKind::Wreck => "wreck",
        SiteKind::Ice => "ice",
    }
}

pub fn tiles_for(map: MapId) -> &'static str {
    match map {
        MapId::Earth => "tiles_earth",
        MapId::Moon => "tiles_moon",
        MapId::Mars => "tiles_mars",
        MapId::Orbit => "tiles_orbit",
        MapId::Europa => "tiles_europa",
    }
}

/// Tint for grayscale rocks/craters and the night overlay color per map.
pub fn rock_tint(map: MapId) -> Color {
    match map {
        MapId::Earth => Color::srgb(0.85, 0.66, 0.45),
        MapId::Moon => Color::srgb(0.75, 0.76, 0.82),
        MapId::Mars => Color::srgb(0.85, 0.45, 0.30),
        MapId::Orbit => Color::srgb(0.6, 0.65, 0.8),
        MapId::Europa => Color::srgb(0.75, 0.9, 1.0),
    }
}

fn skin_color(slot: SkinSlot, id: &str) -> Color {
    skins(slot).iter().find(|s| s.id == id).map(|s| Color::srgb(s.color[0], s.color[1], s.color[2])).unwrap_or(Color::WHITE)
}
pub fn suit_color(id: &str) -> Color {
    skin_color(SkinSlot::Suit, id)
}
pub fn visor_color(id: &str) -> Color {
    skin_color(SkinSlot::Visor, id)
}

/// Index in hats.png, or None.
pub fn hat_index(id: &str) -> Option<usize> {
    match id {
        "antenna" => Some(0),
        "star" => Some(1),
        "cat" => Some(2),
        "crown" => Some(3),
        "alien" => Some(4),
        _ => None,
    }
}
