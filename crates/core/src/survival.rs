//! Part B — survival on the Moon (`Mode::Survival`): the day cycle with a forecast, five evening
//! threats plus real NASA flares, heat, air tightness, upgrades with trade-offs, the
//! astronauts' own stats and a report after every threat ("why it went this way").
//!
//! The day: morning — Mission Control's forecast of tonight's threat (after the comms delay,
//! and only if the antenna has power); day — work, coins, building, upgrades; evening
//! (`EVENING`..1 of the sol) — the threat. Then the report, and a new morning.
//!
//! Honesty: there is no wind on the Moon, so the "dust cloud" is a game simplification (dust
//! kicked up by landings and rovers, charged dust at the day–night line). Temperatures are
//! rounded real values: the polar surface swings from about −30 °C in low sunlight to below
//! −170 °C in shadow. Moonquakes are not modeled yet.

use crate::data::{module_def, ModuleKind, SiteKind};
use crate::game::{Game, GameEvent, Mode, Refusal, DOOR};
use serde::{Deserialize, Serialize};

/// The evening starts at this part of the sol.
pub const EVENING: f32 = 0.65;
/// Comfortable temperature inside; heat above it is radiated away.
pub const T_COMFORT: f32 = 21.0;
pub const T_MAX: f32 = 22.0;
/// Heat from the crew and equipment, °C per sol.
pub const HEAT_BASE: f32 = 21.0;
/// One heater power block, °C per sol.
pub const HEAT_PER_BLOCK: f32 = 12.0;
/// Heat loss coefficient without insulation, per sol.
pub const LOSS: f32 = 0.35;
/// Below this the crew must evacuate.
pub const T_EVACUATE: f32 = -20.0;
/// Air pressure in the habitat at 100% tightness, kPa (like at sea level on Earth).
pub const PRESSURE: f32 = 101.3;
/// Suit oxygen use per sol outside (at a calm heartbeat).
pub const SUIT_USE: f32 = 55.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Threat {
    Cold,
    Dust,
    Meteors,
    EnergyCrisis,
    Failure,
    Flare,
}

pub const THREATS: [Threat; 6] = [Threat::Cold, Threat::Dust, Threat::Meteors, Threat::EnergyCrisis, Threat::Failure, Threat::Flare];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upgrade {
    Insulation,
    Armor,
    Heater,
    LifeSupport,
    Service,
    Comfort,
}

pub const UPGRADES: [Upgrade; 6] = [Upgrade::Insulation, Upgrade::Armor, Upgrade::Heater, Upgrade::LifeSupport, Upgrade::Service, Upgrade::Comfort];
pub const MAX_LEVEL: u8 = 3;

pub struct UpgradeDef {
    pub kind: Upgrade,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    /// what it gives
    pub plus_ru: &'static str,
    pub plus_en: &'static str,
    /// what it costs besides coins (the trade-off)
    pub minus_ru: &'static str,
    pub minus_en: &'static str,
    /// coins for levels 1, 2, 3
    pub cost: [u32; 3],
}

pub const UPGRADE_DEFS: [UpgradeDef; 6] = [
    UpgradeDef {
        kind: Upgrade::Insulation, icon: "snow", name_ru: "Теплоизоляция", name_en: "Insulation",
        plus_ru: "База теряет на 22% меньше тепла за уровень", plus_en: "The base loses 22% less heat per level",
        minus_ru: "Дорого: толстые слои надо привезти с Земли", minus_en: "Expensive: thick layers must come from Earth",
        cost: [8, 12, 18],
    },
    UpgradeDef {
        kind: Upgrade::Armor, icon: "shield", name_ru: "Броня модулей", name_en: "Module armor",
        plus_ru: "Метеориты наносят на 20% меньше урона за уровень", plus_en: "Meteorites do 20% less damage per level",
        minus_ru: "Тяжелее: каждый новый модуль стоит на 1 монету дороже за уровень", minus_en: "Heavier: every new module costs 1 more coin per level",
        cost: [8, 12, 18],
    },
    UpgradeDef {
        kind: Upgrade::Heater, icon: "flash", name_ru: "Обогреватель", name_en: "Heater",
        plus_ru: "+1 ячейка обогрева в куполе (+12 °C за сол)", plus_en: "+1 heater slot in the dome (+12 °C per sol)",
        minus_ru: "Каждая включённая ячейка забирает 1 блок энергии", minus_en: "Every switched-on slot takes 1 power block",
        cost: [6, 10, 14],
    },
    UpgradeDef {
        kind: Upgrade::LifeSupport, icon: "oxygen", name_ru: "Система жизнеобеспечения", name_en: "Life support",
        plus_ru: "Генераторы дают на 20% больше кислорода", plus_en: "Generators make 20% more oxygen",
        minus_ru: "Насосы постоянно тратят ½ блока энергии за уровень", minus_en: "Pumps always use ½ power block per level",
        cost: [8, 12, 16],
    },
    UpgradeDef {
        kind: Upgrade::Service, icon: "robot", name_ru: "Сервисный робот", name_en: "Service robot",
        plus_ru: "Ремонт крепче, робот сам чистит пыль и чинит скафандры", plus_en: "Stronger repairs; the robot cleans dust and fixes suits",
        minus_ru: "Дорогой, а пользу видно не сразу", minus_en: "Expensive, and it pays off slowly",
        cost: [10, 14, 20],
    },
    UpgradeDef {
        kind: Upgrade::Comfort, icon: "house", name_ru: "Жилой отсек", name_en: "Living quarters",
        plus_ru: "Настроение падает медленнее, сон восстанавливает быстрее", plus_en: "Morale drops slower, sleep restores faster",
        minus_ru: "Свет и вентиляция: −½ блока энергии за уровень", minus_en: "Lights and air: −½ power block per level",
        cost: [6, 10, 14],
    },
];

pub fn upgrade_def(u: Upgrade) -> &'static UpgradeDef {
    &UPGRADE_DEFS[u as usize]
}

/// Tonight's threat (strength 1..=3), with a second one on hard late days.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tonight {
    pub threat: Threat,
    pub strength: u8,
    pub second: Option<Threat>,
}

/// One line of the report after a threat. `better` is what would have happened with one more
/// level of the matching upgrade — the cause-and-effect the child should see.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReportLine {
    TempDrop { from: f32, min: f32, insulation: u8, heater_on: u8, better: f32 },
    Dusty { modules: u32, auto_cleaned: u32 },
    Hits { hits: u8, damage: f32, armor: u8, seal_lost: f32, better: f32 },
    Power { battery_ran_out: bool, battery_cap: f32 },
    Broken { count: u8, repaired: u8 },
    Dose { added: f32, sheltered: bool, shield: f32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventReport {
    pub sol: u32,
    pub tonight: Tonight,
    /// the crew handled it the right way
    pub countered: bool,
    pub coins: u32,
    pub lines: Vec<ReportLine>,
}

/// The evening threat in progress (internal bookkeeping for the report).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveThreat {
    pub tonight: Tonight,
    pub temp_from: f32,
    pub temp_min: f32,
    /// the same evening with one more insulation level
    pub shadow_temp: f32,
    pub shadow_min: f32,
    pub hits: u8,
    pub damage: f32,
    pub seal_lost: f32,
    pub battery_ran_out: bool,
    pub broken: u8,
    pub dusty: u32,
    pub dose_from: f32,
    pub sheltered: bool,
}

/// Survival state shown on the HUD. Present in every mode, simulated only in `Mode::Survival`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Survival {
    /// landing site 0..=2 (`landing::SITES`)
    pub site: u8,
    /// °C inside the habitat / outside right now
    pub temp: f32,
    pub outside: f32,
    /// air tightness of the habitat, %
    pub seal: f32,
    /// air pressure, kPa (follows the tightness)
    pub pressure: f32,
    /// upgrade levels, indexed by `Upgrade as usize`
    pub levels: [u8; 6],
    /// what's coming tonight (known once the forecast arrives)
    pub tonight: Option<Tonight>,
    pub active: Option<ActiveThreat>,
    /// last report, until the next evening
    pub report: Option<EventReport>,
    /// energy crisis: share of solar power left
    pub crisis: f32,
    pub threats: u32,
    pub countered: u32,
}

impl Default for Survival {
    fn default() -> Self {
        Self { site: 1, temp: T_COMFORT, outside: -40.0, seal: 100.0, pressure: PRESSURE, levels: [0; 6], tonight: None, active: None, report: None, crisis: 1.0, threats: 0, countered: 0 }
    }
}

impl Survival {
    pub fn level(&self, u: Upgrade) -> u8 {
        self.levels[u as usize]
    }
    /// Coins for the next level, `None` at the top.
    pub fn next_cost(&self, u: Upgrade) -> Option<u32> {
        upgrade_def(u).cost.get(self.level(u) as usize).copied()
    }
    pub fn loss(&self, junior: bool) -> f32 {
        Self::loss_at(self.level(Upgrade::Insulation), junior)
    }
    pub fn loss_at(insulation: u8, junior: bool) -> f32 {
        LOSS * (1.0 - 0.22 * insulation as f32) * if junior { 0.7 } else { 1.0 }
    }
    /// share of meteorite damage that gets through the armor
    pub fn armor_factor(&self) -> f32 {
        1.0 - 0.2 * self.level(Upgrade::Armor) as f32
    }
    /// extra coins per new module (heavier armored modules)
    pub fn build_extra(&self) -> u32 {
        self.level(Upgrade::Armor) as u32
    }
    /// power blocks per sol always used by upgrades (pumps, lights)
    pub fn upkeep(&self) -> f32 {
        0.5 * (self.level(Upgrade::LifeSupport) + self.level(Upgrade::Comfort)) as f32
    }
    pub fn is_evening(t: f32) -> bool {
        t >= EVENING
    }
}

/// The astronaut's own stats (each player). Simulated only in `Mode::Survival`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Body {
    /// suit oxygen tank, %
    pub suit_o2: f32,
    pub fatigue: f32,
    /// suit condition, %
    pub suit: f32,
    /// heartbeat, beats per minute (a game value, not a real sensor)
    pub heart: f32,
    pub low_warned: bool,
    pub tired_warned: bool,
}

impl Default for Body {
    fn default() -> Self {
        Self { suit_o2: 100.0, fatigue: 10.0, suit: 100.0, heart: 72.0, low_warned: false, tired_warned: false }
    }
}

/// Outside temperature at the base before any threat: less sun — colder.
pub fn base_outside(site_sun: u8) -> f32 {
    match site_sun {
        3 => -20.0,
        2 => -40.0,
        _ => -60.0,
    }
}

pub fn cold_outside(strength: u8) -> f32 {
    -(90.0 + 30.0 * strength as f32)
}

/// Night sol: no sunlight at all.
pub const NIGHT_OUTSIDE: f32 = -170.0;

impl Game {
    /// The survival mission after landing: Moon, chosen site, coins from the landing.
    pub fn survival(cfg: crate::game::MissionConfig, site: u8, landing_coins: u32, weather: &crate::weather::Weather) -> Game {
        let mut g = Game::with_mode(cfg, Mode::Survival, weather);
        g.apply_site(site, landing_coins);
        g
    }

    pub(crate) fn apply_site(&mut self, site: u8, landing_coins: u32) {
        let site = site.min(2);
        let def = crate::landing::SITES[site as usize];
        self.s.surv.site = site;
        self.s.surv.outside = base_outside(def.sun);
        self.s.coins += landing_coins;
        // ice next to the base: water → oxygen
        for _ in 1..def.ice {
            self.spawn_site_pub(SiteKind::Ice);
        }
        self.refresh_power_pub();
    }

    /// Plan the threats of every evening (called once at the start).
    pub(crate) fn plan_threats(&mut self) {
        let sols = self.s.sols;
        let night = self.s.def().night_sols;
        let advanced = !self.s.cadet;
        let mut plan = vec![None; sols as usize + 1];
        let mut last: Option<Threat> = None;
        let mut used = [0u8; 6];
        for sol in 1..sols {
            let strength = (1 + (sol - 1) / 2).min(3) as u8;
            let threat = if sol == 1 {
                Threat::Cold
            } else if night.contains(&sol) {
                Threat::EnergyCrisis
            } else {
                let mut t = THREATS[self.rng.below(THREATS.len())];
                for _ in 0..30 {
                    if Some(t) != last && used[t as usize] < 2 {
                        break;
                    }
                    t = THREATS[self.rng.below(THREATS.len())];
                }
                t
            };
            used[threat as usize] += 1;
            let second = if advanced && sol >= 4 {
                let mut t = THREATS[self.rng.below(THREATS.len())];
                for _ in 0..30 {
                    if t != threat && t != Threat::Flare {
                        break;
                    }
                    t = THREATS[self.rng.below(THREATS.len())];
                }
                Some(t)
            } else {
                None
            };
            plan[sol as usize] = Some(Tonight { threat, strength, second });
            last = Some(threat);
        }
        self.threat_plan = plan;
    }

    pub(crate) fn tonight_plan(&self) -> Option<Tonight> {
        self.threat_plan.get(self.s.sol as usize).copied().flatten()
    }

    fn start_threat(&mut self, t: Threat, k: u8, ev: &mut Vec<GameEvent>) {
        match t {
            Threat::Cold => {}
            Threat::Dust => {
                let mut n = 0;
                for m in self.s.modules.iter_mut().filter(|m| matches!(m.kind, ModuleKind::Solar | ModuleKind::OxygenGen)) {
                    m.dusty = true;
                    n += 1;
                }
                self.s.flags.storm = true;
                if let Some(a) = &mut self.s.surv.active {
                    a.dusty += n;
                }
                let _ = k;
            }
            Threat::Meteors => {}
            Threat::EnergyCrisis => {
                self.s.surv.crisis = 1.0 - 0.2 * k as f32;
                self.s.battery = (self.s.battery - k as f32).max(0.0);
            }
            Threat::Failure => {
                let mut ids: Vec<u32> = self.s.modules.iter().filter(|m| !m.broken && matches!(m.kind, ModuleKind::OxygenGen | ModuleKind::Greenhouse | ModuleKind::Lab | ModuleKind::Antenna | ModuleKind::Solar | ModuleKind::Shield)).map(|m| m.id).collect();
                for _ in 0..k.min(ids.len() as u8) {
                    let i = self.rng.below(ids.len());
                    let id = ids.remove(i);
                    if let Some(m) = self.s.modules.iter_mut().find(|m| m.id == id) {
                        m.broken = true;
                        m.hp = 30.0;
                    }
                    if let Some(a) = &mut self.s.surv.active {
                        a.broken += 1;
                    }
                    ev.push(GameEvent::ModuleBroken(id));
                }
            }
            Threat::Flare => {
                let (event, live) = self.weather.pick_flare(&mut self.rng);
                let live = live && self.s.live;
                ev.push(GameEvent::Flare { label_ru: event.label_ru.clone(), label_en: event.label_en.clone(), date: event.date.clone(), live });
                let boost = if self.s.cadet { 0.5 + 0.15 * k as f32 } else { 0.8 + 0.2 * k as f32 };
                self.start_flare_pub(event, live, 1.0 - EVENING, boost);
            }
        }
    }

    fn meteor_strike(&mut self, k: u8, ev: &mut Vec<GameEvent>) {
        let ids: Vec<u32> = self.s.modules.iter().map(|m| m.id).collect();
        let hab = self.s.habitat().id;
        let id = if self.rng.chance(0.35) { hab } else { ids[self.rng.below(ids.len())] };
        let damage = 40.0 * (0.75 + 0.25 * k as f32) * self.s.surv.armor_factor();
        let mut seal_lost = 0.0;
        if id == hab {
            seal_lost = (damage * 0.5).min(self.s.surv.seal);
            self.s.surv.seal -= seal_lost;
            self.s.surv.pressure = PRESSURE * self.s.surv.seal / 100.0;
            self.s.leak = true;
            ev.push(GameEvent::Leak(hab));
        } else if let Some(m) = self.s.modules.iter_mut().find(|m| m.id == id) {
            m.hp = (m.hp - damage).max(0.0);
            if m.hp <= 0.0 && !m.broken {
                m.broken = true;
                if let Some(a) = &mut self.s.surv.active {
                    a.broken += 1;
                }
                ev.push(GameEvent::ModuleBroken(id));
            }
        }
        // astronauts outside get scratched suits
        for b in self.s.players.iter_mut().filter(|p| !p.inside) {
            b.body.suit = (b.body.suit - 8.0).max(0.0);
        }
        if let Some(a) = &mut self.s.surv.active {
            a.hits += 1;
            a.damage += damage;
            a.seal_lost += seal_lost;
        }
        ev.push(GameEvent::MeteorHit { module: id, damage });
    }

    /// One frame of survival rules (only in `Mode::Survival`). `mult` < 1 means a blackout.
    pub(crate) fn survival_tick(&mut self, ds: f32, mult: f32, ev: &mut Vec<GameEvent>) {
        let junior = self.s.cadet;
        let night = self.s.is_night();
        let evening = Survival::is_evening(self.s.t);
        // ---- the evening threat
        if evening && self.s.surv.active.is_none() && self.s.surv.report.as_ref().map_or(true, |r| r.sol != self.s.sol) {
            if let Some(tonight) = self.tonight_plan() {
                let s = &mut self.s;
                s.surv.threats += 1;
                s.surv.active = Some(ActiveThreat { tonight, temp_from: s.surv.temp, temp_min: s.surv.temp, shadow_temp: s.surv.temp, shadow_min: s.surv.temp, hits: 0, damage: 0.0, seal_lost: 0.0, battery_ran_out: false, broken: 0, dusty: 0, dose_from: s.dose, sheltered: false });
                ev.push(GameEvent::Threat(tonight));
                self.start_threat(tonight.threat, tonight.strength, ev);
                if let Some(t2) = tonight.second {
                    self.start_threat(t2, tonight.strength.saturating_sub(1).max(1), ev);
                }
            }
        }
        // meteorites fall during the evening at random moments
        if let Some(a) = self.s.surv.active {
            let k = a.tonight.strength;
            let meteors = a.tonight.threat == Threat::Meteors || a.tonight.second == Some(Threat::Meteors);
            let total = k + 1 + (!junior && k == 3) as u8;
            if meteors && a.hits < total {
                let span = (1.0 - EVENING) * 0.85;
                let due = EVENING + span * (a.hits as f32 + 0.5) / total as f32;
                if self.s.t >= due {
                    self.meteor_strike(k, ev);
                }
            }
        }
        // ---- heat
        let cold = self.s.surv.active.and_then(|a| {
            let k = a.tonight.strength;
            if a.tonight.threat == Threat::Cold {
                Some(k)
            } else if a.tonight.second == Some(Threat::Cold) {
                Some(k.saturating_sub(1).max(1))
            } else {
                None
            }
        });
        let site_sun = crate::landing::SITES[self.s.surv.site as usize].sun;
        let mut outside = base_outside(site_sun);
        if night {
            outside = NIGHT_OUTSIDE;
        }
        if let Some(k) = cold {
            outside = outside.min(cold_outside(k));
        }
        let heater_on = self.s.habitat().power as f32;
        let heat_in = (HEAT_BASE + heater_on * HEAT_PER_BLOCK) * mult;
        let s = &mut self.s;
        s.surv.outside = outside;
        let loss = s.surv.loss(junior);
        s.surv.temp = (s.surv.temp + (heat_in - loss * (s.surv.temp - outside)) * ds).min(T_MAX);
        if let Some(a) = &mut s.surv.active {
            let better_loss = Survival::loss_at((s.surv.levels[Upgrade::Insulation as usize] + 1).min(MAX_LEVEL + 1), junior);
            a.shadow_temp = (a.shadow_temp + (heat_in - better_loss * (a.shadow_temp - outside)) * ds).min(T_MAX);
            a.temp_min = a.temp_min.min(s.surv.temp);
            a.shadow_min = a.shadow_min.min(a.shadow_temp);
            if s.battery <= 0.0 && s.demand as f32 > s.supply {
                a.battery_ran_out = true;
            }
            if s.players.iter().any(|p| p.inside) {
                a.sheltered = true;
            }
        }
        if s.surv.temp < 10.0 {
            s.morale -= 10.0 * ds;
        }
        // ---- air tightness and pressure
        s.surv.pressure = PRESSURE * s.surv.seal / 100.0;
        if s.surv.seal < 100.0 {
            s.o2 -= (100.0 - s.surv.seal) * 0.12 * ds;
        }
        // ---- the service robot slowly repairs suits; comfort keeps spirits up
        let service = s.surv.levels[Upgrade::Service as usize] as f32;
        let comfort = s.surv.levels[Upgrade::Comfort as usize] as f32;
        s.morale += 4.0 * 0.25 * comfort * ds;
        if service >= 3.0 {
            for m in s.modules.iter_mut().filter(|m| !m.broken) {
                m.hp = (m.hp + 10.0 * ds).min(100.0);
            }
        }
        // ---- the astronauts
        let threat_on = s.surv.active.is_some();
        let sleep_time = evening || night;
        let mut body_ev = vec![];
        for (i, p) in s.players.iter_mut().enumerate() {
            let b = &mut p.body;
            let at_airlock = (p.pos.0 - DOOR.0).hypot(p.pos.1 - DOOR.1) < 1.6;
            if p.inside || at_airlock {
                b.suit_o2 = (b.suit_o2 + 1500.0 * ds).min(100.0);
                b.suit = (b.suit + (25.0 + 15.0 * service) * ds * p.inside as u8 as f32).min(100.0);
            } else {
                let leaky = if b.suit < 40.0 { 1.4 } else { 1.0 };
                let use_rate = SUIT_USE * (b.heart / 80.0) * leaky * if junior { 0.65 } else { 1.0 };
                b.suit_o2 = (b.suit_o2 - use_rate * ds).max(0.0);
                let dusty = threat_on && s.surv.active.map_or(false, |a| a.tonight.threat == Threat::Dust || a.tonight.second == Some(Threat::Dust));
                b.suit = (b.suit - (3.0 + if dusty { 20.0 } else { 0.0 }) * ds).max(0.0);
            }
            if p.inside && sleep_time {
                b.fatigue = (b.fatigue - (80.0 + 20.0 * comfort) * ds).max(0.0);
            } else {
                let rate = if p.inside { 10.0 } else { 25.0 };
                b.fatigue = (b.fatigue + rate * ds).min(100.0);
            }
            let target = 72.0 + if p.inside { 0.0 } else { 18.0 } + if threat_on { 20.0 } else { 0.0 } + if b.suit_o2 < 25.0 && !p.inside { 25.0 } else { 0.0 } + if b.fatigue > 70.0 { 10.0 } else { 0.0 };
            b.heart += (target - b.heart) * (3.0 * ds).min(1.0);
            // warnings and the safe return
            if b.suit_o2 < 25.0 && !b.low_warned && !p.inside {
                b.low_warned = true;
                body_ev.push(GameEvent::SuitLow { player: i as u8 });
            } else if b.suit_o2 > 50.0 {
                b.low_warned = false;
            }
            if b.suit_o2 <= 0.0 && !p.inside {
                // the suit's emergency system brings the astronaut back to the airlock
                p.pos = DOOR;
                b.suit_o2 = 30.0;
                b.suit = (b.suit - 10.0).max(0.0);
                body_ev.push(GameEvent::SuitEmpty { player: i as u8 });
            }
            if b.fatigue >= 75.0 && !b.tired_warned {
                b.tired_warned = true;
                body_ev.push(GameEvent::Tired { player: i as u8 });
            } else if b.fatigue < 50.0 {
                b.tired_warned = false;
            }
        }
        if body_ev.iter().any(|e| matches!(e, GameEvent::SuitEmpty { .. })) {
            s.morale -= 5.0;
        }
        ev.extend(body_ev);
        // ---- cold consequences
        if s.surv.temp <= T_EVACUATE {
            s.status = crate::game::Status::Lost(crate::game::Cause::Cold);
        }
    }

    /// End of the evening: write the report, pay coins for doing it right.
    pub(crate) fn finish_threat(&mut self, ev: &mut Vec<GameEvent>) {
        let Some(a) = self.s.surv.active.take() else { return };
        let mut kinds = vec![a.tonight.threat];
        kinds.extend(a.tonight.second);
        let s = &self.s;
        let mut lines = vec![];
        let mut ok = true;
        for t in kinds {
            match t {
                Threat::Cold => {
                    lines.push(ReportLine::TempDrop { from: a.temp_from, min: a.temp_min, insulation: s.surv.level(Upgrade::Insulation), heater_on: s.habitat().power, better: a.shadow_min });
                    ok &= a.temp_min >= 10.0;
                }
                Threat::Dust => {
                    let left = s.modules.iter().filter(|m| m.dusty).count() as u32;
                    let auto = if s.surv.level(Upgrade::Service) >= 2 { left.min(2 + s.surv.level(Upgrade::Service) as u32) } else { 0 };
                    lines.push(ReportLine::Dusty { modules: left, auto_cleaned: auto });
                    ok &= left.saturating_sub(auto) <= 1;
                }
                Threat::Meteors => {
                    let af = s.surv.armor_factor();
                    let better = if af > 0.21 { a.damage * (af - 0.2) / af } else { a.damage };
                    lines.push(ReportLine::Hits { hits: a.hits, damage: a.damage, armor: s.surv.level(Upgrade::Armor), seal_lost: a.seal_lost, better });
                    ok &= s.surv.seal >= 70.0 && !s.modules.iter().any(|m| m.broken);
                }
                Threat::EnergyCrisis => {
                    lines.push(ReportLine::Power { battery_ran_out: a.battery_ran_out, battery_cap: s.battery_cap });
                    ok &= !a.battery_ran_out;
                }
                Threat::Failure => {
                    let still = s.modules.iter().filter(|m| m.broken).count() as u8;
                    lines.push(ReportLine::Broken { count: a.broken, repaired: a.broken.saturating_sub(still) });
                    ok &= still == 0;
                }
                Threat::Flare => {
                    let added = s.dose - a.dose_from;
                    lines.push(ReportLine::Dose { added, sheltered: a.sheltered, shield: s.shield_red() });
                    ok &= a.sheltered && added < 10.0;
                }
            }
        }
        let k = a.tonight.strength as u32;
        let coins = if ok { 4 + 2 * k } else { 1 };
        // the service robot cleans some dust overnight
        if self.s.surv.level(Upgrade::Service) >= 2 {
            let n = 2 + self.s.surv.level(Upgrade::Service) as usize;
            for m in self.s.modules.iter_mut().filter(|m| m.dusty).take(n) {
                m.dusty = false;
            }
        }
        let s = &mut self.s;
        s.surv.crisis = 1.0;
        s.coins += coins;
        s.surv.countered += ok as u32;
        let report = EventReport { sol: s.sol, tonight: a.tonight, countered: ok, coins, lines };
        s.surv.report = Some(report.clone());
        ev.push(GameEvent::ThreatOver(report));
    }

    pub(crate) fn apply_upgrade(&mut self, u: Upgrade) -> Result<Vec<GameEvent>, Refusal> {
        if !matches!(self.s.mode, Mode::Survival | Mode::Sandbox) {
            return Err(Refusal::NotAvailableHere);
        }
        let cost = self.s.surv.next_cost(u).ok_or(Refusal::MaxLevel)?;
        if self.s.coins < cost {
            return Err(Refusal::NotEnoughCoins);
        }
        self.s.coins -= cost;
        self.s.surv.levels[u as usize] += 1;
        Ok(vec![GameEvent::Upgraded { kind: u, level: self.s.surv.levels[u as usize] }])
    }

    /// Max heater blocks in the habitat (0 without the heater upgrade).
    pub(crate) fn heater_slots(&self) -> u8 {
        self.s.surv.level(Upgrade::Heater)
    }

    /// Repair outcome for survival: any damaged module or the habitat's tightness.
    pub(crate) fn survival_repair(&mut self, id: u32) -> Result<(), Refusal> {
        let service = self.s.surv.level(Upgrade::Service);
        let hab = self.s.habitat().id;
        if id == hab {
            if self.s.surv.seal >= 100.0 && !self.s.leak {
                return Err(Refusal::NothingToRepair);
            }
            self.s.surv.seal = (self.s.surv.seal + 50.0 + 20.0 * service as f32).min(100.0);
            self.s.surv.pressure = PRESSURE * self.s.surv.seal / 100.0;
            if self.s.surv.seal >= 100.0 || service >= 1 {
                self.s.leak = false;
            }
            self.s.battery = (self.s.battery - 1.0).max(0.0);
            return Ok(());
        }
        let m = self.s.modules.iter_mut().find(|m| m.id == id).ok_or(Refusal::UnknownId)?;
        if !m.broken && m.hp >= 100.0 {
            return Err(Refusal::NothingToRepair);
        }
        m.broken = false;
        m.hp = (m.hp + 60.0 + 20.0 * service as f32).min(100.0).max(50.0);
        self.s.battery = (self.s.battery - 1.0).max(0.0);
        Ok(())
    }
}

/// Coins for each new module, with the armor trade-off.
pub fn build_cost(kind: ModuleKind, surv: &Survival) -> u32 {
    module_def(kind).cost + surv.build_extra()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Command, Forecast, MissionConfig, Status};
    use crate::weather::Weather;

    fn moon(cadet: bool, seed: u64) -> Game {
        Game::survival(MissionConfig { map: crate::MapId::Moon, cadet, coop: false, live: false, seed }, 1, 0, &Weather::archive())
    }

    /// keep the astronaut at the airlock so the suit never runs dry
    fn run_until(g: &mut Game, mut stop: impl FnMut(&Game, &[GameEvent]) -> bool) -> Vec<GameEvent> {
        let mut all = vec![];
        for _ in 0..20_000 {
            g.set_player_pos(0, DOOR.0, DOOR.1);
            let ev = g.tick(0.25);
            let done = stop(g, &ev);
            all.extend(ev);
            if done || g.state().status != Status::Playing {
                break;
            }
        }
        all
    }

    #[test]
    fn every_evening_but_the_last_has_a_threat_and_the_first_is_a_mild_cold() {
        let g = moon(false, 1);
        let sols = g.state().sols as usize;
        assert_eq!(g.threat_plan.len(), sols + 1);
        assert!(g.threat_plan[1..sols].iter().all(|t| t.is_some()));
        assert!(g.threat_plan[sols].is_none(), "the last sol belongs to the finale");
        assert_eq!(g.threat_plan[1].unwrap().threat, Threat::Cold);
        assert_eq!(g.threat_plan[1].unwrap().strength, 1);
        // juniors never get two threats at once
        assert!(moon(true, 1).threat_plan.iter().flatten().all(|t| t.second.is_none()));
    }

    #[test]
    fn the_morning_forecast_tells_about_tonight_then_the_threat_comes_and_a_report_follows() {
        let mut g = moon(false, 2);
        let ev = run_until(&mut g, |_, ev| ev.iter().any(|e| matches!(e, GameEvent::ThreatOver(_))));
        let fc = ev.iter().position(|e| matches!(e, GameEvent::ForecastArrived(Forecast::Tonight(_)))).expect("forecast");
        let th = ev.iter().position(|e| matches!(e, GameEvent::Threat(_))).expect("threat");
        let rep = ev.iter().position(|e| matches!(e, GameEvent::ThreatOver(_))).expect("report");
        assert!(fc < th && th < rep);
        let GameEvent::ThreatOver(r) = &ev[rep] else { unreachable!() };
        assert_eq!(r.tonight.threat, Threat::Cold);
        assert!(matches!(r.lines[0], ReportLine::TempDrop { .. }));
        assert!(r.coins >= 1);
    }

    #[test]
    fn no_antenna_power_means_no_forecast() {
        let mut g = moon(false, 3);
        let ant = g.state().modules.iter().find(|m| m.kind == ModuleKind::Antenna).unwrap().id;
        g.apply(0, Command::SetPower { module: ant, blocks: 0 }).unwrap();
        let ev = run_until(&mut g, |g, ev| g.state().sol >= 2 && ev.iter().any(|e| matches!(e, GameEvent::ForecastArrived(_))));
        assert!(ev.iter().any(|e| matches!(e, GameEvent::ForecastArrived(Forecast::NoAntenna))));
    }

    #[test]
    fn insulation_and_the_heater_keep_the_dome_warm_in_a_cold_snap() {
        let cold_min = |insulate: bool| {
            let mut g = moon(false, 4);
            g.s.surv.temp = T_COMFORT;
            if insulate {
                g.s.coins = 100;
                g.apply(0, Command::Upgrade(Upgrade::Insulation)).unwrap();
                g.apply(0, Command::Upgrade(Upgrade::Heater)).unwrap();
                let hab = g.state().habitat().id;
                g.s.battery = g.s.battery_cap;
                g.apply(0, Command::SetPower { module: hab, blocks: 1 }).unwrap();
            }
            let ev = run_until(&mut g, |_, ev| ev.iter().any(|e| matches!(e, GameEvent::ThreatOver(_))));
            let r = ev.iter().find_map(|e| if let GameEvent::ThreatOver(r) = e { Some(r.clone()) } else { None }).unwrap();
            let ReportLine::TempDrop { min, better, .. } = r.lines[0] else { panic!() };
            assert!(better >= min, "one more insulation level is never worse");
            min
        };
        let (bare, ready) = (cold_min(false), cold_min(true));
        assert!(ready > bare + 3.0, "prepared {ready:.1} vs bare {bare:.1}");
    }

    #[test]
    fn upgrades_cost_coins_have_a_top_level_and_armor_makes_modules_dearer() {
        let mut g = moon(false, 5);
        g.s.coins = 0;
        assert_eq!(g.apply(0, Command::Upgrade(Upgrade::Armor)), Err(Refusal::NotEnoughCoins));
        g.s.coins = 1000;
        let base = build_cost(ModuleKind::Lab, &g.state().surv);
        for lvl in 1..=MAX_LEVEL {
            let ev = g.apply(0, Command::Upgrade(Upgrade::Armor)).unwrap();
            assert_eq!(ev, vec![GameEvent::Upgraded { kind: Upgrade::Armor, level: lvl }]);
        }
        assert_eq!(g.apply(0, Command::Upgrade(Upgrade::Armor)), Err(Refusal::MaxLevel));
        assert_eq!(build_cost(ModuleKind::Lab, &g.state().surv), base + 3);
        assert!((g.state().surv.armor_factor() - 0.4).abs() < 1e-4);
    }

    #[test]
    fn the_heater_needs_the_upgrade_and_takes_power() {
        let mut g = moon(false, 6);
        let hab = g.state().habitat().id;
        assert_eq!(g.apply(0, Command::SetPower { module: hab, blocks: 1 }), Err(Refusal::NoPowerSlot));
        g.s.coins = 100;
        g.apply(0, Command::Upgrade(Upgrade::Heater)).unwrap();
        g.s.battery = g.s.battery_cap;
        let before = g.state().demand;
        g.apply(0, Command::SetPower { module: hab, blocks: 1 }).unwrap();
        assert_eq!(g.state().demand, before + 1);
    }

    #[test]
    fn meteorites_break_things_and_repairs_bring_tightness_back_with_coins() {
        let mut g = moon(false, 7);
        let hab = g.state().habitat().id;
        let mut ev = vec![];
        // hit the habitat directly
        g.s.surv.active = Some(ActiveThreat { tonight: Tonight { threat: Threat::Meteors, strength: 2, second: None }, temp_from: 21.0, temp_min: 21.0, shadow_temp: 21.0, shadow_min: 21.0, hits: 0, damage: 0.0, seal_lost: 0.0, battery_ran_out: false, broken: 0, dusty: 0, dose_from: 0.0, sheltered: false });
        while g.state().surv.seal >= 100.0 {
            g.meteor_strike(2, &mut ev);
        }
        assert!(g.state().leak && g.state().surv.pressure < PRESSURE);
        let coins = g.state().coins;
        let c = g.state().habitat().center();
        g.set_player_pos(0, c.0, c.1 + 2.0);
        g.s.surv.seal = 60.0;
        g.apply(0, Command::Repair { module: hab }).unwrap();
        assert!(g.state().surv.seal > 60.0);
        assert_eq!(g.state().coins, coins + 2);
    }

    #[test]
    fn suits_run_out_outside_and_refill_at_the_airlock() {
        let mut g = moon(false, 8);
        g.set_player_pos(0, 20.0, 20.0);
        let mut empty = false;
        for _ in 0..2_000 {
            for e in g.tick(0.25) {
                if matches!(e, GameEvent::SuitEmpty { player: 0 }) {
                    empty = true;
                }
            }
            if empty {
                break;
            }
            g.set_player_pos(0, 20.0, 20.0);
        }
        assert!(empty, "the tank runs dry far from the base");
        assert_eq!(g.state().players[0].pos, DOOR, "brought back to the airlock");
        for _ in 0..40 {
            g.set_player_pos(0, DOOR.0, DOOR.1);
            g.tick(0.25);
        }
        assert!(g.state().players[0].body.suit_o2 > 90.0);
    }

    #[test]
    fn calm_breathing_lowers_the_heartbeat() {
        let mut g = moon(false, 9);
        g.s.players[0].body.heart = 120.0;
        g.apply(0, Command::Calm).unwrap();
        assert_eq!(g.state().players[0].body.heart, 95.0);
    }

    #[test]
    fn creative_mode_never_drains_the_astronaut() {
        let mut g = Game::with_mode(MissionConfig { map: crate::MapId::Moon, cadet: false, coop: false, live: false, seed: 1 }, Mode::Sandbox, &Weather::archive());
        g.set_player_pos(0, 20.0, 20.0);
        for _ in 0..400 {
            g.tick(0.25);
        }
        assert_eq!(g.state().players[0].body.suit_o2, 100.0);
        // upgrades still work for building freely
        g.s.coins = 100;
        assert!(g.apply(0, Command::Upgrade(Upgrade::Insulation)).is_ok());
    }

    #[test]
    fn the_landing_site_shapes_the_base() {
        let w = Weather::archive();
        let cfg = MissionConfig { map: crate::MapId::Moon, cadet: false, coop: false, live: false, seed: 11 };
        let sunny = Game::survival(cfg, 0, 9, &w);
        let icy = Game::survival(cfg, 2, 0, &w);
        assert!(sunny.state().supply > icy.state().supply, "more sun on Malapert");
        let ice = |g: &Game| g.state().sites.iter().filter(|s| s.kind == SiteKind::Ice && s.region.is_none()).count();
        assert!(ice(&icy) > ice(&sunny), "more ice near Nobile");
        assert!(icy.state().surv.outside < sunny.state().surv.outside);
        assert_eq!(sunny.state().coins, Game::survival(cfg, 0, 0, &w).state().coins + 9);
    }

    #[test]
    fn same_seed_same_survival_mission() {
        let mut a = moon(false, 12);
        let mut b = moon(false, 12);
        run_until(&mut a, |g, _| g.state().sol >= 3);
        run_until(&mut b, |g, _| g.state().sol >= 3);
        assert_eq!(a.state(), b.state());
    }
}
