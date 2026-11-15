//! Player profiles: points unlock maps (never go down), stars buy skins, patches reward actions,
//! lifetime counters grow the six achievement directions, and an expedition history.
//! Several profiles live in one local JSON file (no accounts, no personal data: the name is a
//! space callsign picked from a list, never typed in).

use crate::achieve::{self, Track, TIER_STARS};
use crate::data::*;
use crate::game::{Cause, MissionResult, Mode};
use crate::i18n::Lang;
use crate::role::Role;
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
    /// callsign from `CALLSIGNS` and its number: «Вега-7»
    pub callsign: u8,
    pub tag: u8,
    pub role: Role,
    pub stats: Stats,
    /// newest first, at most `HISTORY_LEN`
    pub history: Vec<Expedition>,
    /// settings: music, fullscreen, touch controls (0 auto, 1 always, 2 never)
    pub music: bool,
    pub fullscreen: bool,
    pub touch: u8,
}

/// Lifetime counters (they only grow).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Stats {
    pub collected: u32,
    pub regions: u32,
    pub repairs: u32,
    pub built: u32,
    pub upgrades: u32,
    pub landings: u32,
    pub landing_stars: u32,
    /// sols lived through in survival
    pub sols: u32,
    pub threats: u32,
    pub countered: u32,
    pub science: u32,
    pub samples: u32,
    pub coop: u32,
    pub helped: u32,
}

/// One line of the expedition history.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Expedition {
    pub map: MapId,
    pub mode: Mode,
    #[serde(default)]
    pub site: Option<u8>,
    pub won: bool,
    pub stars: u32,
    pub sol: u32,
    pub sols: u32,
    #[serde(default)]
    pub cause: Option<Cause>,
}

pub const HISTORY_LEN: usize = 12;
pub const XP_PER_LEVEL: u32 = 100;

/// Callsigns instead of typed names (ru, en): a child never types personal data.
pub const CALLSIGNS: [(&str, &str); 20] = [
    ("Вега", "Vega"), ("Орион", "Orion"), ("Сириус", "Sirius"), ("Альтаир", "Altair"), ("Комета", "Comet"),
    ("Квазар", "Quasar"), ("Пульсар", "Pulsar"), ("Нова", "Nova"), ("Полярис", "Polaris"), ("Антарес", "Antares"),
    ("Ригель", "Rigel"), ("Денеб", "Deneb"), ("Лира", "Lyra"), ("Феникс", "Phoenix"), ("Пегас", "Pegasus"),
    ("Андромеда", "Andromeda"), ("Кассиопея", "Cassiopeia"), ("Сатурн", "Saturn"), ("Титан", "Titan"), ("Европа", "Europa"),
];

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
            callsign: 0,
            tag: 1,
            role: Role::None,
            stats: Stats::default(),
            history: vec![],
            music: true,
            fullscreen: false,
            touch: 0,
        }
    }
}

/// What a finished mission gave the player (for the debrief screen).
#[derive(Clone, Debug, PartialEq)]
pub struct Rewards {
    pub points: u32,
    pub new_map: Option<MapId>,
    pub badges: Vec<&'static str>,
    /// achievement tiers reached by this mission, and the stars they gave
    pub tiers: Vec<(Track, u8)>,
    pub tier_stars: u32,
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
    /// «Вега-7» / «Vega-7» (Kazakh uses the Russian spelling of the star names).
    pub fn name(&self, lang: Lang) -> String {
        let (ru, en) = CALLSIGNS[self.callsign as usize % CALLSIGNS.len()];
        format!("{}-{}", lang.pick(ru, en), self.tag)
    }
    pub fn next_callsign(&mut self, step: i32) {
        let n = CALLSIGNS.len() as i32;
        self.callsign = ((self.callsign as i32 + step).rem_euclid(n)) as u8;
    }
    pub fn next_tag(&mut self, step: i32) {
        self.tag = ((self.tag as i32 - 1 + step).rem_euclid(99) + 1) as u8;
    }
    /// Level from lifetime points: every `XP_PER_LEVEL` points is a level.
    pub fn level(&self) -> u32 {
        1 + self.total / XP_PER_LEVEL
    }
    pub fn xp_in_level(&self) -> u32 {
        self.total % XP_PER_LEVEL
    }
    pub fn tier(&self, t: Track) -> u8 {
        t.tier(&self.stats)
    }

    /// A finished landing: pilot achievements. Returns the tiers reached (stars already added).
    pub fn record_landing(&mut self, stars: u32) -> Vec<(Track, u8)> {
        let before = self.stats.clone();
        self.stats.landings += 1;
        self.stats.landing_stars += stars;
        let tiers = achieve::new_tiers(&before, &self.stats);
        self.stars += tiers.iter().map(|(_, k)| TIER_STARS[*k as usize - 1]).sum::<u32>();
        tiers
    }

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
        give(self, "explorer", r.flags.regions_visited.iter().all(|v| *v));
        give(self, "story", r.quests_done as usize >= STORY.len());
        give(self, "boss", r.boss_done);
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
        // lifetime counters → achievement tiers
        let was = self.stats.clone();
        let st = &mut self.stats;
        let f = &r.flags;
        st.collected += f.collected;
        st.regions += f.regions_visited.iter().filter(|v| **v).count() as u32;
        st.repairs += f.repairs;
        st.built += f.built;
        st.upgrades += f.upgrades;
        st.threats += f.threats;
        st.countered += f.countered;
        st.science += r.science.max(0.0) as u32;
        st.samples += r.samples_delivered;
        st.coop += r.coop as u32;
        st.helped += f.helped as u32;
        if r.mode == Mode::Survival {
            st.sols += if r.won { r.sols } else { r.sol.saturating_sub(1) };
        }
        let tiers = achieve::new_tiers(&was, &self.stats);
        let tier_stars = tiers.iter().map(|(_, k)| TIER_STARS[*k as usize - 1]).sum::<u32>();
        self.stars += tier_stars;
        self.history.insert(0, Expedition { map: r.map, mode: r.mode, site: r.site, won: r.won, stars: r.stars, sol: r.sol, sols: r.sols, cause: r.cause });
        self.history.truncate(HISTORY_LEN);
        Rewards { points: r.points, new_map, badges, tiers, tier_stars }
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

/// All profiles on this computer and which one is playing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Book {
    pub active: usize,
    pub profiles: Vec<Profile>,
}

pub const MAX_PROFILES: usize = 6;

impl Default for Book {
    fn default() -> Self {
        Self { active: 0, profiles: vec![Profile::default()] }
    }
}

impl Book {
    /// A book, or an old single-profile file (it becomes the first profile).
    pub fn from_json(s: &str) -> Option<Book> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        if v.get("profiles").is_some() {
            let mut b: Book = serde_json::from_value(v).ok()?;
            if b.profiles.is_empty() {
                return None;
            }
            b.active = b.active.min(b.profiles.len() - 1);
            return Some(b);
        }
        serde_json::from_value::<Profile>(v).ok().map(|p| Book { active: 0, profiles: vec![p] })
    }
    pub fn current(&self) -> &Profile {
        &self.profiles[self.active.min(self.profiles.len() - 1)]
    }
    /// New profile with a callsign nobody here uses yet; it becomes the active one.
    pub fn add(&mut self) -> Option<usize> {
        if self.profiles.len() >= MAX_PROFILES {
            return None;
        }
        let lang = self.current().lang;
        let free = (0..CALLSIGNS.len() as u8).find(|c| !self.profiles.iter().any(|p| p.callsign == *c)).unwrap_or(0);
        self.profiles.push(Profile { lang, callsign: free, tag: 1 + (self.profiles.len() as u8 * 3) % 9, ..Default::default() });
        self.active = self.profiles.len() - 1;
        Some(self.active)
    }
    /// Delete a profile. The last one is never removed — it starts over instead.
    pub fn remove(&mut self, i: usize) {
        if i >= self.profiles.len() {
            return;
        }
        if self.profiles.len() == 1 {
            let lang = self.profiles[0].lang;
            self.profiles[0] = Profile { lang, ..Default::default() };
            self.active = 0;
            return;
        }
        self.profiles.remove(i);
        if self.active > i || self.active >= self.profiles.len() {
            self.active = self.active.saturating_sub(1);
        }
    }
    pub fn select(&mut self, i: usize) {
        if i < self.profiles.len() {
            self.active = i;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn home() -> std::path::PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from).unwrap_or_else(|| ".".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("JA_PROFILE") {
        return p.into();
    }
    home().join(".space_explorers.json")
}

/// All profiles (the old single-profile file is picked up once). Not available on wasm.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_book() -> Book {
    let read = |p: std::path::PathBuf| std::fs::read_to_string(p).ok().and_then(|s| Book::from_json(&s));
    read(path()).or_else(|| if std::env::var_os("JA_PROFILE").is_none() { read(home().join(".junior_astronaut_outpost.json")) } else { None }).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_book(b: &Book) {
    if let Ok(s) = serde_json::to_string_pretty(b) {
        let _ = std::fs::write(path(), s);
    }
}

/// The active profile (a fresh one if missing or unreadable).
#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Profile {
    load_book().current().clone()
}

/// Save the active profile into its slot of the book.
#[cfg(not(target_arch = "wasm32"))]
pub fn save(p: &Profile) {
    let mut b = load_book();
    let i = b.active.min(b.profiles.len() - 1);
    b.profiles[i] = p.clone();
    save_book(&b);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Flags, MissionResult, Mode};

    fn result(map_id: MapId, won: bool, points: u32) -> MissionResult {
        MissionResult {
            map: map_id, mode: Mode::Story, quests_done: 0, boss_done: won, boss_countered: 0, won, cause: None, stars: if won { 3 } else { 0 }, points, sol: 4, sols: 4, science: 9.0, goal: 5.0, dose: 10.0,
            history: vec![], cadet: false, coop: false, live: false, flags: Flags::default(), samples_delivered: 0, site: None,
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

    #[test]
    fn old_single_profile_file_becomes_a_book() {
        let b = Book::from_json(r#"{"total": 40, "stars": 7}"#).unwrap();
        assert_eq!(b.profiles.len(), 1);
        assert_eq!(b.current().total, 40);
        let back = Book::from_json(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn profiles_can_be_added_switched_and_deleted() {
        let mut b = Book::default();
        assert_eq!(b.add(), Some(1));
        assert_ne!(b.profiles[0].callsign, b.profiles[1].callsign, "a new callsign");
        assert_eq!(b.active, 1);
        b.select(0);
        b.remove(1);
        assert_eq!((b.profiles.len(), b.active), (1, 0));
        b.profiles[0].total = 500;
        b.remove(0);
        assert_eq!(b.profiles.len(), 1, "the last profile starts over instead of vanishing");
        assert_eq!(b.current().total, 0);
        for _ in 0..10 {
            b.add();
        }
        assert_eq!(b.profiles.len(), MAX_PROFILES);
    }

    #[test]
    fn missions_grow_achievements_and_history() {
        let mut p = Profile::default();
        let mut r = result(MapId::Moon, true, 50);
        r.mode = Mode::Survival;
        r.site = Some(2);
        r.flags.repairs = 6;
        r.flags.built = 4;
        r.flags.countered = 3;
        let stars0 = p.stars;
        let rw = p.finish(&r);
        assert!(rw.tiers.contains(&(Track::Engineer, 1)), "10 repairs+builds = bronze engineer");
        assert!(rw.tiers.contains(&(Track::Survivor, 1)));
        assert_eq!(p.stars, stars0 + 50 + rw.tier_stars);
        assert_eq!(p.history[0].site, Some(2));
        assert_eq!(p.level(), 1);
        let t = p.record_landing(3);
        assert_eq!(t, vec![(Track::Pilot, 1)]);
        assert!(p.record_landing(2).is_empty());
        p.name(Lang::En);
        p.next_tag(-1);
        assert_eq!(p.tag, 99);
    }
}
