//! Player profile: points, stars, skins, badges. Saved locally only (no accounts).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub lang_en: bool,
    pub cadet: bool,
    /// lifetime points — unlock maps, never decreases
    pub total: u32,
    /// spendable balance — buys skins
    pub stars: u32,
    pub suit: String,
    pub visor: String,
    pub hat: String,
    pub owned_suits: Vec<String>,
    pub owned_visors: Vec<String>,
    pub owned_hats: Vec<String>,
    pub badges: Vec<String>,
    pub best: HashMap<String, u32>,
    pub missions: u32,
    pub last_failed: bool,
    pub sound: bool,
    pub nasa_key: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            lang_en: false,
            cadet: false,
            total: 0,
            stars: 0,
            suit: "white".into(),
            visor: "blue".into(),
            hat: "none".into(),
            owned_suits: vec!["white".into()],
            owned_visors: vec!["blue".into()],
            owned_hats: vec!["none".into()],
            badges: vec![],
            best: HashMap::new(),
            missions: 0,
            last_failed: false,
            sound: true,
            nasa_key: String::new(),
        }
    }
}

impl Profile {
    pub fn add_points(&mut self, n: u32) {
        self.total += n;
        self.stars += n;
    }
    pub fn award(&mut self, id: &str) -> bool {
        if self.badges.iter().any(|b| b == id) {
            return false;
        }
        self.badges.push(id.to_string());
        true
    }
    pub fn unlocked(&self, need: u32) -> bool {
        self.total >= need
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> std::path::PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".junior_astronaut_profile.json")
}

pub fn load() -> Profile {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(s) = std::fs::read_to_string(path()) {
            if let Ok(p) = serde_json::from_str::<Profile>(&s) {
                return p;
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(st) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            if let Ok(Some(s)) = st.get_item("ja-profile") {
                if let Ok(p) = serde_json::from_str::<Profile>(&s) {
                    return p;
                }
            }
        }
    }
    Profile::default()
}

pub fn save(p: &Profile) {
    let Ok(s) = serde_json::to_string(p) else { return };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::write(path(), s);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(st) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = st.set_item("ja-profile", &s);
        }
    }
}
