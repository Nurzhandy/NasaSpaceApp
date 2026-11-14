//! Real-time outpost simulation. Pure data; no rendering. Shared by solo and co-op.
//!
//! Rates are "per sol"; a sol lasts `sol_len` real seconds. Every choice has a cost.

use crate::data::*;
use crate::nasa::{Weather, WeatherEvent};
use bevy::prelude::*;
use std::collections::HashMap;

pub const O2_USE: f32 = 12.0;
pub const O2_PER: f32 = 6.0;
pub const FOOD_USE: f32 = 10.0;
pub const FOOD_PER: f32 = 5.0;
pub const SHIELD_PER: f32 = 0.3;
pub const SHIELD_MAX: u32 = 3;
pub const BATTERY_MAX: f32 = 10.0;
pub const SOL_LEN: f32 = 100.0;

/// Tiny deterministic xorshift RNG (no extra crates, works on wasm).
pub struct Rng(pub u64);
impl Rng {
    pub fn seeded() -> Self {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x9E37);
        Rng(t | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Alloc {
    pub ls: u32,
    pub green: u32,
    pub shield: u32,
    pub sci: u32,
}
impl Alloc {
    pub fn used(&self) -> u32 {
        self.ls + self.green + self.shield + self.sci
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Playing,
    Won,
    Lost(Cause),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    Oxygen,
    Food,
    Dose,
}

#[derive(Clone, Debug)]
pub struct Flare {
    pub event: WeatherEvent,
    pub live: bool,
    /// fraction of sol remaining
    pub t_left: f32,
    /// total dose % if fully exposed for the whole flare (before shield)
    pub spike: f32,
    pub duration: f32,
}

#[derive(Clone, Debug)]
pub struct SolRecord {
    pub sol: u32,
    pub o2: f32,
    pub food: f32,
    pub dose: f32,
    pub science: f32,
    pub alloc: Alloc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Forecast {
    Sure(EventKind),
    Maybe(EventKind),
    Night,
}

#[derive(Resource)]
pub struct Sim {
    pub map: MapId,
    pub cadet: bool,
    pub coop: bool,
    pub live: bool,
    pub rng: Rng,
    pub sol: u32,
    pub sols: u32,
    /// progress inside the current sol 0..1
    pub t: f32,
    pub sol_len: f32,
    pub o2: f32,
    pub food: f32,
    pub dose: f32,
    pub morale: f32,
    pub science: f32,
    pub goal: f32,
    pub battery: f32,
    pub alloc: Alloc,
    /// generation this sol (blocks per sol)
    pub gen_now: f32,
    pub gen_broken: bool,
    pub leak: bool,
    pub dust_left: u32,
    pub panels_cleaned: bool,
    pub silence_left: u32,
    pub growth: f32,
    pub harvest_ready: bool,
    pub cargo: bool,
    pub emotion: Option<usize>,
    pub samples_total: u32,
    pub samples_delivered: u32,
    pub schedule: HashMap<u32, (EventKind, bool)>, // (kind, surprise)
    pub false_alarm: Option<u32>,
    pub forecast: Option<Forecast>,
    pub flare: Option<Flare>,
    pub status: Status,
    pub history: Vec<SolRecord>,
    pub players_out: f32,
    pub players_sheltered: f32,
    pub flags: Flags,
    pub paused: bool,
    pub elapsed: f32,
    pub pending: Vec<EventKind>,
}

#[derive(Default, Clone, Debug)]
pub struct Flags {
    pub sheltered_in_flare: bool,
    pub storm: bool,
    pub night: bool,
    pub helped: bool,
    pub flares: u32,
    pub repairs: u32,
    pub cleaned: bool,
}

impl Sim {
    pub fn new(map_id: MapId, cadet: bool, coop: bool, live: bool, weather: &Weather) -> Self {
        let m = map(map_id);
        let mut rng = Rng::seeded();
        let mut schedule = HashMap::new();
        let mut last: Option<EventKind> = None;
        let mut used_once: Vec<EventKind> = vec![];
        for sol in 2..=m.sols {
            if m.night_sols.contains(&sol) {
                continue;
            }
            if rng.chance(0.65) {
                let mut kind = m.events[rng.below(m.events.len())];
                let mut tries = 0;
                while (Some(kind) == last || used_once.contains(&kind)) && tries < 20 {
                    kind = m.events[rng.below(m.events.len())];
                    tries += 1;
                }
                if matches!(kind, EventKind::Dust | EventKind::Conjunction | EventKind::Supply) {
                    used_once.push(kind);
                }
                if kind == EventKind::Dust && sol > m.sols - 1 {
                    continue;
                }
                let surprise = !cadet && kind == EventKind::Flare && rng.chance(0.2);
                schedule.insert(sol, (kind, surprise));
                last = Some(kind);
            }
        }
        // live weather: a real high-risk week guarantees a flare early
        if live && map_id != MapId::Earth {
            let risk = weather.risk();
            let sol = 2 + rng.below(2.min(m.sols as usize - 1)) as u32;
            if risk >= 2 || !schedule.contains_key(&sol) {
                schedule.insert(sol, (EventKind::Flare, false));
            }
        }
        let mut false_alarm = None;
        if !cadet && map_id != MapId::Earth && rng.chance(0.5) {
            let free: Vec<u32> = (2..=m.sols).filter(|s| !schedule.contains_key(s) && !m.night_sols.contains(s)).collect();
            if !free.is_empty() {
                false_alarm = Some(free[rng.below(free.len())]);
            }
        }
        let goal = (m.sols as f32 * if cadet { 0.8 } else { 1.2 }).round();
        let mut s = Sim {
            map: map_id, cadet, coop, live, rng,
            sol: 1, sols: m.sols, t: 0.0, sol_len: SOL_LEN,
            o2: 65.0, food: if cadet { 100.0 } else { 65.0 }, dose: 0.0, morale: 70.0, science: 0.0, goal,
            battery: m.battery_start as f32, alloc: Alloc { ls: 2, green: if cadet { 0 } else { 2 }, shield: 0, sci: 0 },
            gen_now: m.gen as f32, gen_broken: false, leak: false, dust_left: 0, panels_cleaned: false, silence_left: 0,
            growth: 0.3, harvest_ready: false, cargo: false, emotion: None, samples_total: 0, samples_delivered: 0,
            schedule, false_alarm, forecast: None, flare: None, status: Status::Playing, history: vec![],
            players_out: 1.0, players_sheltered: 0.0, flags: Flags::default(), paused: false, elapsed: 0.0, pending: vec![],
        };
        s.begin_sol(weather);
        s
    }

    pub fn def(&self) -> &'static MapDef {
        map(self.map)
    }
    pub fn is_night(&self) -> bool {
        self.def().night_sols.contains(&self.sol)
    }
    pub fn available(&self) -> u32 {
        self.gen_now.max(0.0) as u32 + self.battery.floor() as u32
    }
    pub fn shield_red(&self) -> f32 {
        self.alloc.shield.min(SHIELD_MAX) as f32 * SHIELD_PER
    }
    /// 0..1 how much of the sun is up (for lighting)
    pub fn daylight(&self) -> f32 {
        if self.is_night() {
            return 0.0;
        }
        let d = (self.t * std::f32::consts::PI).sin();
        if self.dust_left > 0 { d * 0.35 } else { d }
    }

    fn begin_sol(&mut self, weather: &Weather) {
        let m = self.def();
        self.t = 0.0;
        self.panels_cleaned = false;
        // generation
        let mut g = m.gen as f32;
        if self.is_night() {
            g = 3.0;
            self.flags.night = true;
        }
        if self.dust_left > 0 {
            g *= 0.45;
        }
        if self.cadet {
            g = (g - 1.0).max(2.0);
        }
        self.gen_now = g.round();
        if self.silence_left > 0 {
            self.silence_left -= 1;
        }
        // event of this sol fires shortly after sunrise
        if let Some((kind, _)) = self.schedule.get(&self.sol).copied() {
            self.pending.push(kind);
        }
        // forecast for tomorrow
        self.forecast = None;
        let next = self.sol + 1;
        if self.silence_left == 0 && next <= self.sols {
            if m.night_sols.contains(&next) && !self.is_night() {
                self.forecast = Some(Forecast::Night);
            } else if self.false_alarm == Some(next) {
                self.forecast = Some(Forecast::Maybe(EventKind::Flare));
            } else if let Some((kind, surprise)) = self.schedule.get(&next).copied() {
                if !surprise && matches!(kind, EventKind::Flare | EventKind::Dust | EventKind::Failure | EventKind::Leak) {
                    self.forecast = Some(if self.cadet || kind != EventKind::Flare { Forecast::Sure(kind) } else { Forecast::Maybe(kind) });
                }
            }
        }
        let _ = weather;
    }

    /// Fire a pending event. Returns the kind so the game can spawn visuals/alerts.
    pub fn fire_pending(&mut self, weather: &Weather) -> Option<EventKind> {
        if self.t < 0.08 || self.pending.is_empty() {
            return None;
        }
        let kind = self.pending.remove(0);
        match kind {
            EventKind::Flare => {
                let (event, live) = weather.pick_flare(&mut self.rng);
                let factor = match self.map { MapId::Moon => 1.0, MapId::Mars => 0.8, MapId::Orbit => 1.25, MapId::Europa => 1.1, MapId::Earth => 0.0 };
                let spike = (12.0 + 18.0 * event.power) * factor;
                self.flare = Some(Flare { event, live, t_left: 0.4, spike, duration: 0.4 });
                self.flags.flares += 1;
            }
            EventKind::Dust => {
                self.dust_left = 2;
                self.gen_now = (self.gen_now * 0.45).round().max(1.0);
                self.flags.storm = true;
            }
            EventKind::Failure => self.gen_broken = true,
            EventKind::Leak => self.leak = true,
            EventKind::Harvest => {
                self.growth = 1.0;
                self.harvest_ready = true;
            }
            EventKind::Discovery => self.samples_total += 3,
            EventKind::Supply => self.cargo = true,
            EventKind::Emotion => self.emotion = Some(self.rng.below(MOODS.len())),
            EventKind::Conjunction => self.silence_left = 2,
        }
        Some(kind)
    }

    /// Advance simulation by `dt` real seconds. Returns true when a new sol started.
    pub fn tick(&mut self, dt: f32, weather: &Weather) -> bool {
        if self.status != Status::Playing || self.paused {
            return false;
        }
        self.elapsed += dt;
        let ds = dt / self.sol_len; // fraction of a sol
        let cadet = self.cadet;

        // power: allocation beyond generation drains the battery; surplus charges it
        let used = self.alloc.used() as f32;
        let gen = self.gen_now.max(0.0);
        let mut mult = 1.0;
        if used > gen {
            if self.battery <= 0.0 {
                mult = if used > 0.0 { gen / used } else { 1.0 };
            }
            self.battery = (self.battery - (used - gen) * ds).max(0.0);
        } else {
            self.battery = (self.battery + (gen - used) * ds).min(BATTERY_MAX);
        }

        // oxygen
        let ls_mult = if self.gen_broken { 0.5 } else { 1.0 };
        let leak = if self.leak { 4.0 } else { 0.0 };
        self.o2 += (self.alloc.ls as f32 * O2_PER * ls_mult * mult - O2_USE - leak) * ds;
        // food
        if !cadet {
            self.food += (self.alloc.green as f32 * FOOD_PER * mult - FOOD_USE) * ds;
        }
        self.growth = (self.growth + self.alloc.green as f32 * mult * 0.3 * ds).min(1.0);
        if self.growth >= 1.0 && !cadet {
            self.harvest_ready = true;
        }
        // radiation
        let red = self.shield_red();
        let m = self.def();
        let mut dose_rate = m.dose * (1.0 - red);
        // being outside in a suit adds exposure (thin walls)
        dose_rate += m.dose * 0.5 * self.players_out;
        if let Some(f) = &mut self.flare {
            let per_sol = f.spike / f.duration;
            let sheltered = self.players_sheltered;
            let exposure = (1.0 - sheltered) + sheltered * 0.15;
            dose_rate += per_sol * (1.0 - red) * exposure;
            f.t_left -= ds;
            if sheltered > 0.0 {
                self.flags.sheltered_in_flare = true;
            }
            if f.t_left <= 0.0 {
                self.flare = None;
            }
        }
        self.dose += dose_rate * ds;
        // morale & science
        self.morale -= 4.0 * ds;
        let sci_mult = if self.morale < 30.0 { 0.5 } else if self.morale >= 80.0 { 1.3 } else { 1.0 };
        self.science += self.alloc.sci as f32 * mult * sci_mult * ds;

        self.o2 = self.o2.clamp(0.0, 100.0);
        self.food = self.food.clamp(0.0, 100.0);
        self.dose = self.dose.clamp(0.0, 100.0);
        self.morale = self.morale.clamp(0.0, 100.0);

        // end conditions — the crew always evacuates safely
        if self.o2 <= 0.0 {
            self.status = Status::Lost(Cause::Oxygen);
        } else if !cadet && self.food <= 0.0 {
            self.status = Status::Lost(Cause::Food);
        } else if self.dose >= 100.0 {
            self.status = Status::Lost(Cause::Dose);
        }

        self.t += ds;
        if self.t >= 1.0 && self.status == Status::Playing {
            self.history.push(SolRecord { sol: self.sol, o2: self.o2, food: self.food, dose: self.dose, science: self.science, alloc: self.alloc });
            if self.sol >= self.sols {
                self.status = Status::Won;
                return false;
            }
            self.sol += 1;
            if self.dust_left > 0 {
                self.dust_left -= 1;
            }
            self.begin_sol(weather);
            return true;
        }
        false
    }

    // ---- player actions (each has a cost) ----
    pub fn repair_generator(&mut self) {
        self.gen_broken = false;
        self.battery = (self.battery - 2.0).max(0.0);
        self.flags.repairs += 1;
    }
    pub fn patch_leak(&mut self) {
        self.leak = false;
        self.battery = (self.battery - 1.0).max(0.0);
        self.flags.repairs += 1;
    }
    pub fn clean_panels(&mut self) {
        if self.dust_left > 0 && !self.panels_cleaned {
            self.panels_cleaned = true;
            self.flags.cleaned = true;
            self.gen_now = (self.def().gen as f32 * 0.75).round();
        }
    }
    pub fn harvest_eat(&mut self) {
        self.harvest_ready = false;
        self.growth = 0.0;
        self.food = (self.food + 12.0).min(100.0);
        self.morale = (self.morale + 10.0).min(100.0);
        self.o2 = (self.o2 + 3.0).min(100.0);
    }
    pub fn harvest_lab(&mut self) {
        self.harvest_ready = false;
        self.growth = 0.0;
        self.food = (self.food + 4.0).min(100.0);
        self.science += 2.0;
        self.o2 = (self.o2 + 3.0).min(100.0);
    }
    pub fn cargo_all(&mut self) {
        self.cargo = false;
        self.battery = (self.battery - 2.0).max(0.0);
        self.food = (self.food + 20.0).min(100.0);
        self.o2 = (self.o2 + 8.0).min(100.0);
    }
    pub fn cargo_some(&mut self) {
        self.cargo = false;
        self.food = (self.food + 8.0).min(100.0);
    }
    pub fn support_crew(&mut self) {
        self.emotion = None;
        self.battery = (self.battery - 1.0).max(0.0);
        self.morale = (self.morale + 20.0).min(100.0);
        self.flags.helped = true;
    }
    /// Deliver carried samples to the lab. Lab needs power. Returns science gained.
    pub fn deliver_samples(&mut self, n: u32) -> f32 {
        if n == 0 || self.alloc.sci == 0 {
            return 0.0;
        }
        self.samples_delivered += n;
        let gain = n as f32 * 1.5;
        self.science += gain;
        gain
    }

    pub fn stars(&self) -> u32 {
        if self.status != Status::Won {
            return 0;
        }
        1 + (self.science >= self.goal) as u32 + (self.dose < 50.0) as u32
    }
    pub fn points(&self) -> u32 {
        let per = [0, 20, 30, 40, 50, 60][self.def().tier as usize];
        match self.status {
            Status::Won => per * self.stars() + if self.live { 10 } else { 0 } + if self.coop { 10 } else { 0 },
            _ => 10 + (self.sol.saturating_sub(1) * 3).min(20),
        }
    }
}
