//! Player profile: points, stars, skins, patches. Saved locally as JSON (no accounts).
//! PART A (A6) owns the progression rules; this is a working first version.

use crate::catalog::{BADGES, HATS, MAPS, SUITS, VISORS};
use crate::{MapId, MissionResult, Refusal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Profile {
    pub lang_en: bool,
    pub cadet: bool,
    /// lifetime points — unlock maps, never decrease
    pub total: u32,
    /// spendable balance — buys skins
    pub stars: u32,
    pub suit: String,
    pub visor: String,
    pub hat: String,
    pub owned: Vec<String>,
    pub badges: Vec<String>,
    pub best: HashMap<String, u32>,
    pub missions: u32,
    pub last_failed: bool,
    pub sound: bool,
    pub music: bool,
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
            owned: vec!["suit:white".into(), "visor:blue".into(), "hat:none".into()],
            badges: vec![],
            best: HashMap::new(),
            missions: 0,
            last_failed: false,
            sound: true,
            music: true,
        }
    }
}

/// Skin categories in the shop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkinCat {
    Suit,
    Visor,
    Hat,
}
impl SkinCat {
    pub fn list(self) -> &'static [crate::catalog::Skin] {
        match self {
            SkinCat::Suit => SUITS,
            SkinCat::Visor => VISORS,
            SkinCat::Hat => HATS,
        }
    }
    fn key(self, id: &str) -> String {
        let p = match self {
            SkinCat::Suit => "suit",
            SkinCat::Visor => "visor",
            SkinCat::Hat => "hat",
        };
        format!("{p}:{id}")
    }
}

impl Profile {
    pub fn map_key(m: MapId) -> String {
        format!("{m:?}")
    }
    pub fn unlocked(&self, m: MapId) -> bool {
        MAPS.iter().find(|d| d.id == m).map(|d| self.total >= d.need).unwrap_or(false)
    }
    pub fn owns(&self, cat: SkinCat, id: &str) -> bool {
        self.owned.iter().any(|o| *o == cat.key(id))
    }
    pub fn wearing(&self, cat: SkinCat) -> &str {
        match cat {
            SkinCat::Suit => &self.suit,
            SkinCat::Visor => &self.visor,
            SkinCat::Hat => &self.hat,
        }
    }
    /// Buy (if needed) and wear. Buying never changes `total`, so maps never relock.
    pub fn buy_or_wear(&mut self, cat: SkinCat, id: &str) -> Result<bool, Refusal> {
        let Some(item) = cat.list().iter().find(|s| s.id == id) else { return Err(Refusal::NothingToDo) };
        let mut bought = false;
        if !self.owns(cat, id) {
            if self.stars < item.price {
                return Err(Refusal::NotEnoughMaterials);
            }
            self.stars -= item.price;
            self.owned.push(cat.key(id));
            bought = true;
        }
        match cat {
            SkinCat::Suit => self.suit = id.into(),
            SkinCat::Visor => self.visor = id.into(),
            SkinCat::Hat => self.hat = id.into(),
        }
        Ok(bought)
    }
    pub fn add_points(&mut self, n: u32) {
        self.total += n;
        self.stars += n;
    }
    /// Apply a finished mission. Returns ids of patches that are new for this profile.
    pub fn record(&mut self, r: &MissionResult) -> Vec<&'static str> {
        self.add_points(r.points);
        self.missions += 1;
        let key = Self::map_key(r.map);
        let best = self.best.get(&key).copied().unwrap_or(0);
        self.best.insert(key, best.max(r.stars));
        let mut new = vec![];
        let mut ids: Vec<&'static str> = r.badges.clone();
        if self.last_failed {
            ids.push("comeback");
        }
        self.last_failed = !r.won;
        for id in ids {
            if BADGES.iter().any(|b| b.id == id) && !self.badges.iter().any(|b| b == id) {
                self.badges.push(id.into());
                new.push(id);
            }
        }
        new
    }
    pub fn award(&mut self, id: &'static str) -> bool {
        if self.badges.iter().any(|b| b == id) {
            return false;
        }
        self.badges.push(id.into());
        true
    }

    fn path() -> std::path::PathBuf {
        if let Some(p) = std::env::var_os("JA2D_PROFILE") {
            return p.into();
        }
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from).unwrap_or_else(|| ".".into());
        home.join(".junior_astronaut_2d.json")
    }
    pub fn load() -> Profile {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), s);
        }
    }
}
