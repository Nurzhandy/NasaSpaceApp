//! Player profile: points unlock maps (never go down), stars buy skins, patches reward actions.
//! Saved locally as JSON (no accounts, no personal data).

use crate::data::*;
use crate::game::MissionResult;
use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Profile {
    pub lang: Lang,
    pub cadet: bool,
    /// lifetime points — unlock maps, never decrease
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
    /// best stars per map, key = `format!("{:?}", MapId)`
    pub best: HashMap<String, u32>,
    pub missions: u32,
    pub last_failed: bool,
    pub sound: bool,
    pub nasa_key: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            lang: Lang::Ru,
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

/// What a finished mission gave the player (for the debrief screen).
#[derive(Clone, Debug, PartialEq)]
pub struct Rewards {
    pub points: u32,
    pub new_map: Option<MapId>,
    pub badges: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopOutcome {
    Equipped,
    Bought,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopError {
    NotEnoughStars,
    UnknownItem,
}

impl Profile {
    pub fn unlocked(&self, map_id: MapId) -> bool {
        self.total >= map(map_id).need
    }
    pub fn has_badge(&self, id: &str) -> bool {
        self.badges.iter().any(|b| b == id)
    }
    /// Award a patch once. Returns true the first time.
    pub fn award(&mut self, id: &str) -> bool {
        if self.has_badge(id) {
            return false;
        }
        self.badges.push(id.to_string());
        true
    }
    pub fn best_stars(&self, map_id: MapId) -> u32 {
        self.best.get(&format!("{map_id:?}")).copied().unwrap_or(0)
    }

    /// Apply a finished mission: points, stars, best result and patches.
    pub fn finish(&mut self, r: &MissionResult) -> Rewards {
        let before: Vec<MapId> = MAPS.iter().filter(|m| self.unlocked(m.id)).map(|m| m.id).collect();
        let mut badges = vec![];
        let mut give = |p: &mut Profile, id: &'static str, cond: bool| {
            if cond && p.award(id) {
                badges.push(id);
            }
        };
        let comeback = self.last_failed;
        give(self, "comeback", comeback);
        give(self, "first", true);
        give(self, "shield", r.flags.sheltered_in_flare);
        give(self, "helper", r.flags.helped);
        give(self, "live", r.live);
        give(self, "collector", r.samples_delivered >= 6);
        give(self, "builder", r.flags.built >= 5);
        if r.won {
            give(self, "scientist", r.science >= r.goal);
            give(self, "moon", r.map == MapId::Moon);
            give(self, "mars", r.map == MapId::Mars);
            give(self, "europa", r.map == MapId::Europa);
            give(self, "storm", r.flags.storm);
            give(self, "night", r.flags.night);
            give(self, "coop", r.coop);
        }
        self.last_failed = !r.won;
        self.missions += 1;
        let key = format!("{:?}", r.map);
        let best = self.best.get(&key).copied().unwrap_or(0);
        self.best.insert(key, best.max(r.stars));
        self.total += r.points;
        self.stars += r.points;
        let new_map = MAPS.iter().filter(|m| self.unlocked(m.id)).map(|m| m.id).find(|id| !before.contains(id));
        Rewards { points: r.points, new_map, badges }
    }

    fn slot(&mut self, slot: SkinSlot) -> (&mut Vec<String>, &mut String) {
        match slot {
            SkinSlot::Suit => (&mut self.owned_suits, &mut self.suit),
            SkinSlot::Visor => (&mut self.owned_visors, &mut self.visor),
            SkinSlot::Hat => (&mut self.owned_hats, &mut self.hat),
        }
    }
    pub fn owns(&self, slot: SkinSlot, id: &str) -> bool {
        let owned = match slot {
            SkinSlot::Suit => &self.owned_suits,
            SkinSlot::Visor => &self.owned_visors,
            SkinSlot::Hat => &self.owned_hats,
        };
        owned.iter().any(|o| o == id)
    }
    /// Wear an owned skin, or buy it with stars. Stars are separate from points,
    /// so buying never relocks a map.
    pub fn buy_or_equip(&mut self, slot: SkinSlot, idx: usize) -> Result<ShopOutcome, ShopError> {
        let item = skins(slot).get(idx).ok_or(ShopError::UnknownItem)?;
        let id = item.id.to_string();
        if self.owns(slot, &id) {
            *self.slot(slot).1 = id;
            return Ok(ShopOutcome::Equipped);
        }
        if self.stars < item.price {
            return Err(ShopError::NotEnoughStars);
        }
        self.stars -= item.price;
        let (owned, cur) = self.slot(slot);
        owned.push(id.clone());
        *cur = id;
        Ok(ShopOutcome::Bought)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("JA_PROFILE") {
        return p.into();
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from).unwrap_or_else(|| ".".into());
    home.join(".junior_astronaut_outpost.json")
}

/// Load the profile (a fresh one if missing or unreadable). Not available on wasm (use your own storage).
#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Profile {
    std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(p: &Profile) {
    if let Ok(s) = serde_json::to_string_pretty(p) {
        let _ = std::fs::write(path(), s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Flags, MissionResult};

    fn result(map_id: MapId, won: bool, points: u32) -> MissionResult {
        MissionResult {
            map: map_id, won, cause: None, stars: if won { 3 } else { 0 }, points, sol: 4, sols: 4, science: 9.0, goal: 5.0, dose: 10.0,
            history: vec![], cadet: false, coop: false, live: false, flags: Flags::default(), samples_delivered: 0,
        }
    }

    #[test]
    fn buying_a_skin_never_relocks_a_map() {
        let mut p = Profile::default();
        let r = p.finish(&result(MapId::Earth, true, 70));
        assert_eq!(r.new_map, Some(MapId::Moon));
        assert!(r.badges.contains(&"first"));
        assert_eq!(p.buy_or_equip(SkinSlot::Suit, 6), Err(ShopError::NotEnoughStars));
        assert_eq!(p.buy_or_equip(SkinSlot::Suit, 1), Ok(ShopOutcome::Bought));
        assert_eq!(p.stars, 30);
        assert!(p.unlocked(MapId::Moon), "spending stars keeps the Moon open");
        assert_eq!(p.buy_or_equip(SkinSlot::Suit, 0), Ok(ShopOutcome::Equipped));
        assert_eq!(p.suit, "white");
    }

    #[test]
    fn trying_again_after_a_failure_earns_a_patch() {
        let mut p = Profile::default();
        p.finish(&result(MapId::Earth, false, 10));
        let r = p.finish(&result(MapId::Earth, true, 60));
        assert!(r.badges.contains(&"comeback"));
        assert_eq!(p.best_stars(MapId::Earth), 3);
    }

    #[test]
    fn profile_round_trips_and_tolerates_old_files() {
        let p = Profile { total: 123, ..Default::default() };
        let back: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back, p);
        let old: Profile = serde_json::from_str(r#"{"total": 5}"#).unwrap();
        assert_eq!(old.total, 5);
        assert_eq!(old.suit, "white");
    }
}
