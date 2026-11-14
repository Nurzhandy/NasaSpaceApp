//! STAND-IN implementation of the contract so part B can build the app before part A is done.
//! Simple, playable rules modelled on the 3D version. Part A replaces this with the real game
//! (building rules, per-module power, exploration, events, balance) behind the same `GameApi`.

use crate::catalog::{map, module_spec, EventKind, MapDef, MapId, MOODS};
use crate::*;

const SOL_LEN: f32 = 90.0;
const O2_USE: f32 = 12.0;
const FOOD_USE: f32 = 10.0;

#[derive(Clone)]
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
}

#[derive(Clone)]
struct ActiveFlare {
    info: FlareInfo,
    spike: f32,
}

#[derive(Clone)]
pub struct Game {
    cfg: MissionConfig,
    weather: Weather,
    rng: Rng,
    st: State,
    next_id: u32,
    schedule: Vec<(u32, EventKind)>,
    fired: Vec<u32>,
    flare: Option<ActiveFlare>,
    growth: f32,
    leak: bool,
    silence_left: u32,
    delivered: u32,
    sheltered_in_flare: bool,
    had_storm: bool,
    had_night: bool,
    helped: bool,
    crew_target: (f32, f32),
    pending_radio: Option<Radio>,
    ended: bool,
}

fn def(m: MapId) -> &'static MapDef {
    map(m)
}

impl Game {
    fn add_module(&mut self, kind: ModuleKind, x: i32, y: i32, power: u8) -> ModuleId {
        let id = self.next_id;
        self.next_id += 1;
        self.st.modules.push(Module { id, kind, x, y, power, broken: false, leaking: false, dusty: false, ripe: false });
        id
    }
    fn rect(m: &Module) -> (i32, i32, i32, i32) {
        let s = module_spec(m.kind);
        (m.x, m.y, s.w, s.h)
    }
    fn center(m: &Module) -> (f32, f32) {
        let (x, y, w, h) = Self::rect(m);
        (x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0)
    }
    fn dist_to_module(&self, player: u8, m: &Module) -> f32 {
        let p = &self.st.players[player as usize];
        let (x, y, w, h) = Self::rect(m);
        let dx = (x as f32 - p.x).max(p.x - (x + w) as f32).max(0.0);
        let dy = (y as f32 - p.y).max(p.y - (y + h) as f32).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
    fn module_at(&self, cx: i32, cy: i32) -> Option<usize> {
        self.st.modules.iter().position(|m| {
            let (x, y, w, h) = Self::rect(m);
            cx >= x && cx < x + w && cy >= y && cy < y + h
        })
    }
    fn nearest(&self, player: u8, kind: ModuleKind) -> Option<&Module> {
        self.st
            .modules
            .iter()
            .filter(|m| m.kind == kind)
            .min_by(|a, b| self.dist_to_module(player, a).total_cmp(&self.dist_to_module(player, b)))
    }
    fn battery_max(&self) -> f32 {
        4.0 + 6.0 * self.st.modules.iter().filter(|m| m.kind == ModuleKind::Battery).count() as f32
    }
    fn power_gen(&self) -> u32 {
        let light = self.st.daylight;
        let mut g = 0.0;
        for m in self.st.modules.iter().filter(|m| m.kind == ModuleKind::Solar && !m.broken) {
            g += if m.dusty { 1.0 } else { 3.0 } * light;
        }
        g.round() as u32
    }
    fn power_used(&self) -> u32 {
        self.st.modules.iter().filter(|m| !m.broken).map(|m| m.power as u32).sum()
    }
    fn powered(&self, kind: ModuleKind) -> u32 {
        self.st.modules.iter().filter(|m| m.kind == kind && !m.broken).map(|m| m.power as u32).sum()
    }
    fn daylight_at(&self, t: f32) -> f32 {
        if def(self.st.map).night_sols.contains(&self.st.sol) {
            return 0.0;
        }
        let base = ((t * std::f32::consts::PI).sin() * 1.5).clamp(0.0, 1.0);
        if self.st.dust_storm { base * 0.5 } else { base }
    }
    fn update_shield(&mut self) {
        let p = self.powered(ModuleKind::Shield);
        self.st.shield_radius = if p > 0 { 4.0 + 1.5 * p as f32 } else { 0.0 };
    }
    fn exposure(&self, p: &PlayerState) -> f32 {
        if p.in_shelter {
            return 0.15;
        }
        let mut e = 1.0;
        if self.st.shield_radius > 0.0 {
            if let Some(s) = self.st.modules.iter().find(|m| m.kind == ModuleKind::Shield && m.power > 0 && !m.broken) {
                let (cx, cy) = Self::center(s);
                if ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt() <= self.st.shield_radius {
                    e = 1.0 - 0.3 * self.powered(ModuleKind::Shield).min(3) as f32;
                }
            }
        }
        e
    }

    fn send_forecast(&mut self) {
        let next = self.st.sol + 1;
        if self.silence_left > 0 || next > self.st.sols {
            self.pending_radio = None;
            return;
        }
        let m = def(self.st.map);
        let kind = self.schedule.iter().find(|(s, _)| *s == next).map(|(_, k)| *k);
        let (icon, ru, en) = if m.night_sols.contains(&next) {
            ("night", "Завтра лунная ночь: копи энергию в батареях", "Lunar night tomorrow: store power in batteries")
        } else {
            match kind {
                Some(EventKind::Flare) => ("sun", "Завтра возможна вспышка! Готовь щит и убежище", "A flare may come tomorrow! Prepare shield and shelter"),
                Some(EventKind::Dust) => ("tornado", "Завтра пылевая буря: энергии будет мало", "Dust storm tomorrow: low power"),
                Some(EventKind::Failure) => ("wrench", "Датчики: модуль может сломаться", "Sensors: a module may break"),
                Some(EventKind::Leak) => ("comet", "Завтра поток микрометеоритов", "Micrometeorite shower tomorrow"),
                _ => ("check", "Завтра всё спокойно", "Tomorrow looks calm"),
            }
        };
        let antenna = self.powered(ModuleKind::Antenna) > 0;
        let delay = m.delay_s * if antenna { 0.5 } else { 1.0 };
        self.pending_radio = Some(Radio { icon, text_ru: ru.into(), text_en: en.into(), arrives_in: delay });
    }

    fn fire(&mut self, kind: EventKind, ev: &mut Vec<GameEvent>) {
        match kind {
            EventKind::Flare => {
                let strong: Vec<&WeatherEvent> = self.weather.events.iter().filter(|e| e.power >= 0.3).take(6).collect();
                let e = if strong.is_empty() { self.weather.events[0].clone() } else { strong[self.rng.below(strong.len())].clone() };
                let factor = match self.st.map { MapId::Moon => 1.0, MapId::Mars => 0.8, MapId::Orbit => 1.25, MapId::Europa => 1.1, MapId::Earth => 0.0 };
                let info = FlareInfo { label_ru: e.label_ru.clone(), label_en: e.label_en.clone(), date: e.date.clone(), live: self.weather.live, left: 1.0 };
                ev.push(GameEvent::Flare { label_ru: info.label_ru.clone(), label_en: info.label_en.clone(), date: info.date.clone(), live: info.live });
                self.flare = Some(ActiveFlare { info, spike: (12.0 + 18.0 * e.power) * factor });
            }
            EventKind::Dust => {
                self.st.dust_storm = true;
                self.had_storm = true;
                for m in self.st.modules.iter_mut().filter(|m| m.kind == ModuleKind::Solar) {
                    m.dusty = true;
                }
                ev.push(GameEvent::DustStorm);
            }
            EventKind::Failure => {
                let cands: Vec<usize> = (0..self.st.modules.len()).filter(|&i| module_spec(self.st.modules[i].kind).max_power > 0 && !self.st.modules[i].broken).collect();
                if !cands.is_empty() {
                    let i = cands[self.rng.below(cands.len())];
                    self.st.modules[i].broken = true;
                    ev.push(GameEvent::ModuleBroken(self.st.modules[i].id));
                }
            }
            EventKind::Leak => {
                if let Some(h) = self.st.modules.iter_mut().find(|m| m.kind == ModuleKind::Habitat) {
                    h.leaking = true;
                    self.leak = true;
                    ev.push(GameEvent::Leak(h.id));
                }
            }
            EventKind::Harvest => {
                if let Some(g) = self.st.modules.iter_mut().find(|m| m.kind == ModuleKind::Greenhouse) {
                    g.ripe = true;
                    self.st.harvest_ready = true;
                    ev.push(GameEvent::HarvestReady);
                }
            }
            EventKind::Discovery => {
                for _ in 0..3 {
                    self.spawn_site(SiteKind::Sample);
                }
                ev.push(GameEvent::Picked(SiteKind::Sample)); // alert reused: "new samples"
                ev.pop();
            }
            EventKind::Supply => {
                self.st.cargo = Some((self.st.base_x as f32 + 0.5, self.st.base_y as f32 + 7.0));
                ev.push(GameEvent::CargoLanding);
            }
            EventKind::Emotion => {
                let mood = self.rng.below(MOODS.len());
                self.st.crew_mood = Some(mood);
                ev.push(GameEvent::CrewNeedsSupport(mood));
            }
            EventKind::Conjunction => {
                self.silence_left = 2;
                self.st.silence = true;
            }
        }
    }

    fn spawn_site(&mut self, kind: SiteKind) {
        for _ in 0..40 {
            let x = self.rng.range(2.0, self.st.world_w as f32 - 2.0);
            let y = self.rng.range(2.0, self.st.world_h as f32 - 2.0);
            let d = ((x - self.st.base_x as f32).powi(2) + (y - self.st.base_y as f32).powi(2)).sqrt();
            if d > self.st.base_r as f32 + 1.0 {
                let id = self.next_id;
                self.next_id += 1;
                self.st.sites.push(Site { id, kind, x, y });
                return;
            }
        }
    }

    fn result(&self) -> MissionResult {
        let s = &self.st;
        let won = s.status == Status::Won;
        let stars = if won { 1 + (s.science >= s.goal) as u32 + (s.dose < 50.0) as u32 } else { 0 };
        let tier = def(s.map).tier as usize;
        let per = [0, 20, 30, 40, 50, 60][tier];
        let points = if won { per * stars + if self.cfg.live && self.weather.live { 10 } else { 0 } + if s.coop { 10 } else { 0 } } else { 10 + (s.sol.saturating_sub(1) * 3).min(20) };
        let mut badges = vec!["first"];
        if won {
            if s.science >= s.goal { badges.push("scientist"); }
            match s.map { MapId::Moon => badges.push("moon"), MapId::Mars => badges.push("mars"), MapId::Europa => badges.push("europa"), _ => {} }
            if self.had_storm { badges.push("storm"); }
            if self.had_night { badges.push("night"); }
            if s.coop { badges.push("coop"); }
        }
        if self.sheltered_in_flare { badges.push("shield"); }
        if self.helped { badges.push("helper"); }
        if self.delivered >= 6 { badges.push("collector"); }
        if self.cfg.live && self.weather.live { badges.push("live"); }
        MissionResult {
            map: s.map, won, cause: if let Status::Lost(c) = s.status { Some(c) } else { None }, stars, points, sol: s.sol, sols: s.sols,
            science: s.science, goal: s.goal, dose: s.dose, coop: s.coop, live: self.cfg.live && self.weather.live, history: s.history.clone(), badges,
        }
    }
}

impl GameApi for Game {
    fn new(cfg: MissionConfig, weather: &Weather) -> Self {
        let m = def(cfg.map);
        let weather = if weather.events.is_empty() { Weather::archive() } else { weather.clone() };
        let (w, h) = (44, 32);
        let (bx, by) = (w / 2, h / 2);
        let n = if cfg.coop { 2 } else { 1 };
        let players = (0..n).map(|i| PlayerState { x: bx as f32 - 0.5 + i as f32 * 1.5, y: by as f32 - 2.5, carried: 0, in_shelter: false }).collect();
        let st = State {
            map: cfg.map, cadet: cfg.cadet, coop: cfg.coop, sol: 1, sols: m.sols, day_t: 0.0, daylight: 0.0, night: false, dust_storm: false, silence: false,
            o2: 70.0, food: if cfg.cadet { 100.0 } else { 70.0 }, dose: 0.0, science: 0.0, goal: (m.sols as f32 * if cfg.cadet { 1.0 } else { 1.5 }).round(),
            morale: 70.0, battery: m.battery_start as f32, battery_max: 10.0, materials: 10, power_gen: 0, power_used: 0,
            world_w: w, world_h: h, base_x: bx, base_y: by, base_r: 9, modules: vec![], sites: vec![], players, shield_radius: 0.0,
            flare: None, radio: None, harvest_ready: false, cargo: None, crew_mood: None, crew_pos: (bx as f32 + 3.0, by as f32 + 3.0),
            status: Status::Playing, history: vec![],
        };
        let mut g = Game {
            cfg, weather, rng: Rng(cfg.seed | 1), st, next_id: 1, schedule: vec![], fired: vec![], flare: None, growth: 0.3, leak: false,
            silence_left: 0, delivered: 0, sheltered_in_flare: false, had_storm: false, had_night: false, helped: false,
            crew_target: (bx as f32 + 3.0, by as f32 + 3.0), pending_radio: None, ended: false,
        };
        // starter base
        g.add_module(ModuleKind::Habitat, bx - 1, by - 1, 0);
        g.add_module(ModuleKind::Solar, bx - 6, by + 1, 0);
        g.add_module(ModuleKind::Solar, bx - 6, by - 2, 0);
        g.add_module(ModuleKind::Oxygen, bx + 3, by + 2, 2);
        if !cfg.cadet {
            g.add_module(ModuleKind::Greenhouse, bx + 3, by - 2, 2);
        }
        g.add_module(ModuleKind::Battery, bx - 3, by + 3, 0);
        for _ in 0..6 { g.spawn_site(SiteKind::Sample); }
        for _ in 0..4 { g.spawn_site(SiteKind::Wreck); }
        if matches!(cfg.map, MapId::Moon | MapId::Mars | MapId::Europa) {
            for _ in 0..2 { g.spawn_site(SiteKind::Ice); }
        }
        // event schedule
        for sol in 2..=m.sols {
            if m.night_sols.contains(&sol) || g.rng.f32() > 0.7 {
                continue;
            }
            let k = m.events[g.rng.below(m.events.len())];
            g.schedule.push((sol, k));
        }
        if cfg.map != MapId::Earth && !g.schedule.iter().any(|(_, k)| *k == EventKind::Flare) {
            g.schedule.retain(|(s, _)| *s != 2);
            g.schedule.push((2, EventKind::Flare));
        }
        g.st.battery_max = g.battery_max();
        g.st.daylight = g.daylight_at(0.0);
        g.send_forecast();
        g
    }

    fn tick(&mut self, dt: f32) -> Vec<GameEvent> {
        let mut ev = vec![];
        if self.st.status != Status::Playing {
            if !self.ended {
                self.ended = true;
                ev.push(GameEvent::MissionEnded(self.result()));
            }
            return ev;
        }
        let ds = dt / SOL_LEN;
        let m = def(self.st.map);
        self.st.day_t += ds;
        self.st.night = m.night_sols.contains(&self.st.sol);
        if self.st.night {
            self.had_night = true;
        }
        self.st.daylight = self.daylight_at(self.st.day_t);
        self.update_shield();
        self.st.battery_max = self.battery_max();

        // radio delay
        if let Some(r) = &mut self.pending_radio {
            r.arrives_in -= dt;
            if r.arrives_in <= 0.0 {
                let r = self.pending_radio.take().unwrap();
                self.st.radio = Some(Radio { arrives_in: 0.0, ..r.clone() });
                ev.push(GameEvent::ForecastArrived(r));
            } else {
                self.st.radio = Some(r.clone());
            }
        }

        // power
        let gen = self.power_gen();
        let used = self.power_used();
        self.st.power_gen = gen;
        self.st.power_used = used;
        let mut eff = 1.0;
        if used > gen {
            if self.st.battery <= 0.0 {
                eff = gen as f32 / used as f32;
            }
            self.st.battery = (self.st.battery - (used - gen) as f32 * ds).max(0.0);
        } else {
            self.st.battery = (self.st.battery + (gen - used) as f32 * ds).min(self.st.battery_max);
        }

        // life support
        let o2_make = self.powered(ModuleKind::Oxygen) as f32 * 6.0 * eff;
        self.st.o2 += (o2_make - O2_USE - if self.leak { 4.0 } else { 0.0 }) * ds;
        if !self.st.cadet {
            let food_make = self.powered(ModuleKind::Greenhouse) as f32 * 5.0 * eff;
            self.st.food += (food_make - FOOD_USE) * ds;
        }
        self.growth += self.powered(ModuleKind::Greenhouse) as f32 * 0.35 * eff * ds;
        if self.growth >= 1.0 && !self.st.harvest_ready {
            if let Some(g) = self.st.modules.iter_mut().find(|m| m.kind == ModuleKind::Greenhouse) {
                g.ripe = true;
                self.st.harvest_ready = true;
                ev.push(GameEvent::HarvestReady);
            }
        }
        // radiation (average over players)
        let n = self.st.players.len().max(1) as f32;
        let exp: f32 = self.st.players.iter().map(|p| self.exposure(p)).sum::<f32>() / n;
        let mut dose = m.dose * exp;
        if let Some(f) = &mut self.flare {
            dose += f.spike / 0.4 * exp;
            f.info.left -= ds / 0.4;
            if self.st.players.iter().any(|p| p.in_shelter) {
                self.sheltered_in_flare = true;
            }
            if f.info.left <= 0.0 {
                self.flare = None;
                ev.push(GameEvent::FlareEnded);
            }
        }
        self.st.flare = self.flare.as_ref().map(|f| f.info.clone());
        self.st.dose += dose * ds;
        // science & morale
        let sci_mult = if self.st.morale < 30.0 { 0.5 } else if self.st.morale >= 80.0 { 1.3 } else { 1.0 };
        self.st.science += self.powered(ModuleKind::Lab) as f32 * 0.8 * eff * sci_mult * ds;
        self.st.morale = (self.st.morale - 4.0 * ds).clamp(0.0, 100.0);
        self.st.o2 = self.st.o2.clamp(0.0, 100.0);
        self.st.food = self.st.food.clamp(0.0, 100.0);
        self.st.dose = self.st.dose.clamp(0.0, 100.0);

        // crewmate wanders around the base
        let (cx, cy) = self.st.crew_pos;
        let (tx, ty) = self.crew_target;
        let d = ((tx - cx).powi(2) + (ty - cy).powi(2)).sqrt();
        if self.st.crew_mood.is_none() {
            if d < 0.2 {
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                let r = self.rng.range(2.5, 6.0);
                self.crew_target = (self.st.base_x as f32 + a.cos() * r, self.st.base_y as f32 + a.sin() * r);
            } else {
                let sp = 1.5 * dt / d;
                self.st.crew_pos = (cx + (tx - cx) * sp.min(1.0), cy + (ty - cy) * sp.min(1.0));
            }
        }

        // scheduled events fire early in the sol
        if self.st.day_t >= 0.15 && !self.fired.contains(&self.st.sol) {
            self.fired.push(self.st.sol);
            let kinds: Vec<EventKind> = self.schedule.iter().filter(|(s, _)| *s == self.st.sol).map(|(_, k)| *k).collect();
            for k in kinds {
                self.fire(k, &mut ev);
            }
        }

        // end conditions — the crew always evacuates safely
        if self.st.o2 <= 0.0 {
            self.st.status = Status::Lost(Cause::Oxygen);
        } else if !self.st.cadet && self.st.food <= 0.0 {
            self.st.status = Status::Lost(Cause::Food);
        } else if self.st.dose >= 100.0 {
            self.st.status = Status::Lost(Cause::Dose);
        }

        if self.st.day_t >= 1.0 && self.st.status == Status::Playing {
            let s = &self.st;
            self.st.history.push(SolRecord { sol: s.sol, o2: s.o2, food: s.food, dose: s.dose, science: s.science });
            if self.st.sol >= self.st.sols {
                self.st.status = Status::Won;
            } else {
                self.st.sol += 1;
                self.st.day_t = 0.0;
                if self.silence_left > 0 {
                    self.silence_left -= 1;
                }
                self.st.silence = self.silence_left > 0;
                if self.st.dust_storm && self.rng.f32() < 0.6 {
                    self.st.dust_storm = false;
                }
                self.send_forecast();
                ev.push(GameEvent::NewSol(self.st.sol));
            }
        }
        ev
    }

    fn apply(&mut self, player: u8, cmd: Command) -> Result<Vec<GameEvent>, Refusal> {
        let pi = player as usize;
        if pi >= self.st.players.len() {
            return Err(Refusal::NothingToDo);
        }
        let mut ev = vec![];
        match cmd {
            Command::Build { kind, x, y } => {
                let s = module_spec(kind);
                if x < 1 || y < 1 || x + s.w >= self.st.world_w || y + s.h >= self.st.world_h {
                    return Err(Refusal::OutsideBase);
                }
                let (cx, cy) = (x as f32 + s.w as f32 / 2.0, y as f32 + s.h as f32 / 2.0);
                if ((cx - self.st.base_x as f32).powi(2) + (cy - self.st.base_y as f32).powi(2)).sqrt() > self.st.base_r as f32 {
                    return Err(Refusal::OutsideBase);
                }
                for dx in 0..s.w {
                    for dy in 0..s.h {
                        if self.module_at(x + dx, y + dy).is_some() {
                            return Err(Refusal::CellBusy);
                        }
                    }
                }
                let p = &self.st.players[pi];
                if ((cx - p.x).powi(2) + (cy - p.y).powi(2)).sqrt() > 8.0 {
                    return Err(Refusal::TooFar);
                }
                if self.st.materials < s.materials {
                    return Err(Refusal::NotEnoughMaterials);
                }
                self.st.materials -= s.materials;
                let id = self.add_module(kind, x, y, 0);
                ev.push(GameEvent::Built(id));
            }
            Command::Demolish { x, y } => {
                let Some(i) = self.module_at(x, y) else { return Err(Refusal::NothingToDo) };
                if self.st.modules[i].kind == ModuleKind::Habitat {
                    return Err(Refusal::CannotDemolish);
                }
                if self.dist_to_module(player, &self.st.modules[i]) > 8.0 {
                    return Err(Refusal::TooFar);
                }
                let m = self.st.modules.remove(i);
                self.st.materials += module_spec(m.kind).materials / 2;
                ev.push(GameEvent::Demolished);
            }
            Command::SetPower { module, blocks } => {
                let gen = self.power_gen();
                let avail = gen + self.st.battery.floor() as u32;
                let used = self.power_used();
                let Some(m) = self.st.modules.iter_mut().find(|m| m.id == module) else { return Err(Refusal::NothingToDo) };
                let max = module_spec(m.kind).max_power;
                if max == 0 {
                    return Err(Refusal::NothingToDo);
                }
                if blocks > max {
                    return Err(Refusal::MaxPower);
                }
                if blocks > m.power && used + (blocks - m.power) as u32 > avail.max(used) {
                    return Err(Refusal::NotEnoughPower);
                }
                m.power = blocks;
            }
            Command::Repair { module } => {
                let Some(i) = self.st.modules.iter().position(|m| m.id == module) else { return Err(Refusal::NothingToDo) };
                if self.dist_to_module(player, &self.st.modules[i]) > 2.5 {
                    return Err(Refusal::TooFar);
                }
                let m = &mut self.st.modules[i];
                if !(m.broken || m.leaking || m.dusty) {
                    return Err(Refusal::NothingToDo);
                }
                if m.broken {
                    self.st.battery = (self.st.battery - 1.0).max(0.0);
                }
                m.broken = false;
                m.dusty = false;
                if m.leaking {
                    m.leaking = false;
                    self.leak = false;
                }
                ev.push(GameEvent::Repaired(module));
            }
            Command::PickSample { site } => {
                let Some(i) = self.st.sites.iter().position(|s| s.id == site) else { return Err(Refusal::NothingToDo) };
                let s = self.st.sites[i].clone();
                let p = &self.st.players[pi];
                if ((s.x - p.x).powi(2) + (s.y - p.y).powi(2)).sqrt() > 2.0 {
                    return Err(Refusal::TooFar);
                }
                self.st.sites.remove(i);
                match s.kind {
                    SiteKind::Sample => self.st.players[pi].carried += 1,
                    SiteKind::Ice => self.st.o2 = (self.st.o2 + 10.0).min(100.0),
                    SiteKind::Wreck => self.st.materials += 4,
                }
                ev.push(GameEvent::Picked(s.kind));
            }
            Command::DeliverSamples => {
                let Some(lab) = self.nearest(player, ModuleKind::Lab).cloned() else { return Err(Refusal::NeedsModule(ModuleKind::Lab)) };
                if self.dist_to_module(player, &lab) > 2.5 {
                    return Err(Refusal::TooFar);
                }
                if lab.power == 0 || lab.broken {
                    return Err(Refusal::LabNeedsPower);
                }
                let n = self.st.players[pi].carried;
                if n == 0 {
                    return Err(Refusal::NothingToDo);
                }
                self.st.players[pi].carried = 0;
                self.delivered += n;
                let gain = n as f32 * 1.5;
                self.st.science += gain;
                ev.push(GameEvent::SamplesDelivered { science: gain });
            }
            Command::EnterShelter => {
                let Some(h) = self.nearest(player, ModuleKind::Habitat).cloned() else { return Err(Refusal::NothingToDo) };
                if self.dist_to_module(player, &h) > 2.0 {
                    return Err(Refusal::TooFar);
                }
                self.st.players[pi].in_shelter = true;
                if self.flare.is_some() {
                    self.sheltered_in_flare = true;
                }
            }
            Command::ExitShelter => self.st.players[pi].in_shelter = false,
            Command::ChooseHarvest(eat) => {
                if !self.st.harvest_ready {
                    return Err(Refusal::NothingToDo);
                }
                self.st.harvest_ready = false;
                self.growth = 0.0;
                for g in self.st.modules.iter_mut().filter(|m| m.kind == ModuleKind::Greenhouse) {
                    g.ripe = false;
                }
                self.st.o2 = (self.st.o2 + 3.0).min(100.0);
                if eat {
                    self.st.food = (self.st.food + 12.0).min(100.0);
                    self.st.morale = (self.st.morale + 10.0).min(100.0);
                } else {
                    self.st.food = (self.st.food + 4.0).min(100.0);
                    self.st.science += 2.0;
                }
            }
            Command::ChooseCargo(all) => {
                if self.st.cargo.is_none() {
                    return Err(Refusal::NothingToDo);
                }
                self.st.cargo = None;
                if all {
                    self.st.battery = (self.st.battery - 2.0).max(0.0);
                    self.st.food = (self.st.food + 20.0).min(100.0);
                    self.st.o2 = (self.st.o2 + 8.0).min(100.0);
                    self.st.materials += 6;
                } else {
                    self.st.food = (self.st.food + 8.0).min(100.0);
                    self.st.materials += 2;
                }
            }
            Command::NameFeeling(f) => {
                let Some(mood) = self.st.crew_mood else { return Err(Refusal::NothingToDo) };
                let ok = crate::catalog::FEELINGS.get(f).map(|x| MOODS[mood].ok.contains(&x.id)).unwrap_or(false);
                ev.push(GameEvent::FeelingChecked { ok });
            }
            Command::Support(_) => {
                if self.st.crew_mood.is_none() {
                    return Err(Refusal::NothingToDo);
                }
                self.st.crew_mood = None;
                self.st.battery = (self.st.battery - 1.0).max(0.0);
                self.st.morale = (self.st.morale + 20.0).min(100.0);
                self.helped = true;
                ev.push(GameEvent::Supported);
            }
        }
        self.st.power_used = self.power_used();
        self.update_shield();
        Ok(ev)
    }

    fn check(&self, player: u8, cmd: &Command) -> Result<(), Refusal> {
        self.clone().apply(player, cmd.clone()).map(|_| ())
    }

    fn set_player(&mut self, player: u8, x: f32, y: f32) {
        if let Some(p) = self.st.players.get_mut(player as usize) {
            p.x = x;
            p.y = y;
        }
    }

    fn state(&self) -> &State {
        &self.st
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(map: MapId) -> Game {
        Game::new(MissionConfig { map, cadet: false, coop: false, live: false, seed: 42 }, &Weather::archive())
    }
    #[test]
    fn cannot_build_on_busy_cell() {
        let mut g = game(MapId::Moon);
        let (bx, by) = (g.st.base_x, g.st.base_y);
        g.set_player(0, bx as f32, by as f32 - 3.0);
        assert_eq!(g.check(0, &Command::Build { kind: ModuleKind::Battery, x: bx, y: by }), Err(Refusal::CellBusy));
    }
    #[test]
    fn mission_finishes() {
        let mut g = game(MapId::Earth);
        let mut ended = false;
        for _ in 0..(90 * 10 * 10) {
            if g.tick(0.1).iter().any(|e| matches!(e, GameEvent::MissionEnded(_))) {
                ended = true;
                break;
            }
        }
        assert!(ended);
    }
}
