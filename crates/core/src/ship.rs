//! A2 / A4 — the ship "Aurora": its systems and the asteroid map (Touch Panel).
//!
//! Advanced players see real quantities: pressure in kPa, **energy** in the battery (kWh) and
//! **power** made and used (kW) — kept apart on purpose. Junior players see simple bars.
//! Asteroids between Earth and the Moon are a game convention; the real danger is micrometeoroids.

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

pub const PRESSURE_FULL: f32 = 101.3;
pub const ENERGY_CAP: f32 = 40.0;

/// Ship systems. 0 hull or 0 pressure = the ship is lost (the crew evacuates).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShipStats {
    /// hull strength, %
    pub hull: f32,
    /// how airtight the hull is, %
    pub seal: f32,
    /// cabin pressure, kPa
    pub pressure: f32,
    /// oxygen in the air, % of normal
    pub o2: f32,
    /// CO₂ in the air, % of the danger level
    pub co2: f32,
    /// energy stored in the battery, kWh
    pub energy: f32,
    /// power made by the solar arrays, kW
    pub power_in: f32,
    /// power used by the ship, kW
    pub power_out: f32,
    /// -1 cold … 0 fine … +1 hot (the "barbecue roll" keeps it near 0)
    pub heat: f32,
    /// rooms with a hole in them
    pub leaks: Vec<String>,
}

impl Default for ShipStats {
    fn default() -> Self {
        Self { hull: 100.0, seal: 100.0, pressure: PRESSURE_FULL, o2: 100.0, co2: 10.0, energy: 30.0, power_in: 4.0, power_out: 3.5, heat: 0.0, leaks: vec![] }
    }
}

impl ShipStats {
    /// Advance by `ds` minutes of flight (the campaign decides the rate). `drift` < 1 slows air drift
    /// (life-support role); `creative` stops all drain.
    pub fn tick(&mut self, ds: f32, drift: f32, creative: bool) {
        if creative {
            return;
        }
        // every hole lowers the seal and lets the air out
        let holes = self.leaks.len() as f32;
        self.seal = (self.seal - holes * 1.5 * ds).max(0.0);
        let leak_rate = holes * 4.0 + (100.0 - self.seal) * 0.02;
        self.pressure = (self.pressure - leak_rate * ds).max(0.0);
        // no holes: life support tops the air up from the tanks (like the ISS does)
        if holes == 0.0 {
            self.pressure = (self.pressure + 3.0 * ds).min(PRESSURE_FULL);
        }
        self.o2 = (self.o2 - (0.6 * drift + holes * 2.0) * ds).max(0.0);
        self.co2 = (self.co2 + 0.9 * drift * ds).min(100.0);
        self.energy = (self.energy + (self.power_in - self.power_out) * ds * 0.25).clamp(0.0, ENERGY_CAP);
        self.heat = (self.heat + 0.01 * ds).clamp(-1.0, 1.0);
    }
    pub fn lost(&self) -> Option<ShipFail> {
        if self.hull <= 0.0 {
            Some(ShipFail::Hull)
        } else if self.pressure <= 0.0 {
            Some(ShipFail::Pressure)
        } else if self.o2 <= 0.0 {
            Some(ShipFail::Oxygen)
        } else {
            None
        }
    }
    /// Apply a task's effect: positive numbers repair, negative numbers hurt.
    pub fn change(&mut self, p: &crate::content::Penalty, sign: f32) {
        self.o2 = (self.o2 + sign * p.o2).clamp(0.0, 100.0);
        self.co2 = (self.co2 - sign * p.co2).clamp(0.0, 100.0);
        self.hull = (self.hull + sign * p.hull).clamp(0.0, 100.0);
        self.seal = (self.seal + sign * p.seal).clamp(0.0, 100.0);
        self.energy = (self.energy + sign * p.energy).clamp(0.0, ENERGY_CAP);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipFail {
    Hull,
    Pressure,
    Oxygen,
}

// ---------------------------------------------------------------- Touch Panel

/// One asteroid on the panel. Coordinates are 0..1 across the panel (x right, y up).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rock {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub r: f32,
}

/// The asteroid map: rocks come from the right; the ship flies to where the players touch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AsteroidField {
    pub ship: (f32, f32),
    /// where each player last touched (co-op: the ship flies to the average)
    pub targets: [Option<(f32, f32)>; 4],
    pub rocks: Vec<Rock>,
    /// waves already played in this flight (difficulty grows)
    pub wave: u32,
    /// seconds left in this wave
    pub t_left: f32,
    pub spawn_t: f32,
    pub hits: u32,
}

pub const SHIP_R: f32 = 0.035;
pub const WAVE_SECONDS: f32 = 25.0;

impl AsteroidField {
    pub fn new(wave: u32) -> Self {
        Self { ship: (0.15, 0.5), targets: [None; 4], rocks: vec![], wave, t_left: WAVE_SECONDS, spawn_t: 0.5, hits: 0 }
    }
    pub fn steer(&mut self, player: u8, x: f32, y: f32) {
        if let Some(t) = self.targets.get_mut(player as usize) {
            *t = Some((x.clamp(0.0, 1.0), y.clamp(0.05, 0.95)));
        }
    }
    /// Advance `dt` seconds. Returns how many rocks hit the ship. Junior rocks are slower and fewer.
    pub fn tick(&mut self, dt: f32, junior: bool, rng: &mut Rng) -> u32 {
        self.t_left -= dt;
        // fly toward the average touch point
        let pts: Vec<(f32, f32)> = self.targets.iter().flatten().copied().collect();
        if !pts.is_empty() {
            let n = pts.len() as f32;
            let goal = (pts.iter().map(|p| p.0).sum::<f32>() / n, pts.iter().map(|p| p.1).sum::<f32>() / n);
            let (dx, dy) = (goal.0 - self.ship.0, goal.1 - self.ship.1);
            let d = (dx * dx + dy * dy).sqrt();
            let step = 0.55 * dt;
            if d > 0.001 {
                let k = (step / d).min(1.0);
                self.ship.0 = (self.ship.0 + dx * k).clamp(0.03, 0.6);
                self.ship.1 = (self.ship.1 + dy * k).clamp(0.05, 0.95);
            }
        }
        // spawn: more and faster rocks every wave
        let pace = 1.0 + self.wave as f32 * 0.35;
        let slow = if junior { 0.7 } else { 1.0 };
        self.spawn_t -= dt;
        if self.spawn_t <= 0.0 && self.t_left > 2.0 {
            self.spawn_t = (0.9 / pace) * if junior { 1.4 } else { 1.0 };
            let speed = (0.25 + 0.05 * self.wave as f32) * slow * rng.range(0.8, 1.3);
            self.rocks.push(Rock { x: 1.05, y: rng.range(0.05, 0.95), vx: -speed, vy: rng.range(-0.05, 0.05), r: rng.range(0.025, 0.05) });
        }
        let mut hits = 0;
        for r in &mut self.rocks {
            r.x += r.vx * dt;
            r.y += r.vy * dt;
        }
        let ship = self.ship;
        self.rocks.retain(|r| {
            let d = ((r.x - ship.0).powi(2) + (r.y - ship.1).powi(2)).sqrt();
            if d < r.r + SHIP_R {
                hits += 1;
                false
            } else {
                r.x > -0.1 && r.y > -0.1 && r.y < 1.1
            }
        });
        self.hits += hits;
        hits
    }
    pub fn over(&self) -> bool {
        self.t_left <= 0.0
    }
}

// ---------------------------------------------------------------- walking inside the ship

/// Can a crew member stand here? Inside a room (a little away from the walls) or in a doorway.
pub fn walkable(l: &crate::content::ShipLayout, x: f32, y: f32) -> bool {
    const M: f32 = 0.3;
    // consoles are solid: you stand in front of them
    let console = l.rooms.iter().flat_map(|r| r.consoles.iter()).any(|k| x > k.x as f32 - 0.2 && x < k.x as f32 + 1.2 && y > k.y as f32 - 0.2 && y < k.y as f32 + 1.2);
    !console
        && (l.rooms.iter().any(|r| x > r.x as f32 + M && x < (r.x + r.w) as f32 - M && y > r.y as f32 + M && y < (r.y + r.h) as f32 - M)
            || l.doors.iter().any(|&(dx, dy)| (x - dx).abs() < DOOR_HALF && (y - dy).abs() < DOOR_HALF))
}

/// Half the width of a doorway (cells).
pub const DOOR_HALF: f32 = 1.0;

/// Move from `from` toward `to`, sliding along walls.
pub fn step(l: &crate::content::ShipLayout, from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
    for p in [to, (to.0, from.1), (from.0, to.1)] {
        if walkable(l, p.0, p.1) {
            return p;
        }
    }
    from
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walls_stop_you_and_doors_let_you_through() {
        let c = crate::content::load(&crate::content::default_dir()).unwrap();
        let l = c.ship.unwrap();
        assert!(walkable(&l, l.start.0, l.start.1), "the crew starts on the floor");
        for &(dx, dy) in &l.doors {
            assert!(walkable(&l, dx, dy), "door {dx},{dy}");
        }
        // every console can be reached from the start by walking through doors
        for r in &l.rooms {
            for k in &r.consoles {
                // somewhere in front of the console is floor
                let c = (k.x as f32 + 0.5, k.y as f32 + 0.5);
                let goal = [(c.0, c.1 - 1.0), (c.0, c.1 + 1.0), (c.0 - 1.0, c.1), (c.0 + 1.0, c.1)].into_iter().find(|p| walkable(&l, p.0, p.1)).unwrap_or_else(|| panic!("{}.{} has no floor in front", r.id, k.id));
                assert!(!walkable(&l, c.0, c.1), "{}.{} is solid", r.id, k.id);
                let mut seen = std::collections::HashSet::new();
                let mut q = vec![((l.start.0 * 2.0) as i32, (l.start.1 * 2.0) as i32)];
                let target = ((goal.0 * 2.0) as i32, (goal.1 * 2.0) as i32);
                let mut ok = false;
                while let Some(p) = q.pop() {
                    if (p.0 - target.0).abs() <= 1 && (p.1 - target.1).abs() <= 1 {
                        ok = true;
                        break;
                    }
                    for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let n = (p.0 + d.0, p.1 + d.1);
                        if seen.insert(n) && walkable(&l, n.0 as f32 / 2.0, n.1 as f32 / 2.0) {
                            q.push(n);
                        }
                    }
                }
                assert!(ok, "can't walk to {}.{}", r.id, k.id);
            }
        }
    }

    #[test]
    fn holes_let_the_air_out_until_the_ship_is_lost() {
        let mut s = ShipStats::default();
        s.tick(10.0, 1.0, false);
        assert!(s.lost().is_none());
        s.leaks = vec!["lab".into(), "gym".into()];
        let mut t = 0.0;
        while s.lost().is_none() && t < 500.0 {
            s.tick(1.0, 1.0, false);
            t += 1.0;
        }
        assert_eq!(s.lost(), Some(ShipFail::Pressure));
        let mut c = ShipStats { leaks: vec!["lab".into()], ..Default::default() };
        c.tick(100.0, 1.0, true);
        assert_eq!(c, ShipStats { leaks: vec!["lab".into()], ..Default::default() }, "creative mode drains nothing");
    }

    #[test]
    fn steering_away_avoids_more_rocks_than_sitting_still() {
        let run = |dodge: bool| {
            let mut rng = Rng::new(9);
            let mut f = AsteroidField::new(1);
            let mut t = 0.0;
            while !f.over() {
                if dodge {
                    // move away from the nearest rock in front of the ship
                    if let Some(r) = f.rocks.iter().filter(|r| r.x > f.ship.0).min_by(|a, b| a.x.total_cmp(&b.x)) {
                        let y = if r.y > f.ship.1 { r.y - 0.25 } else { r.y + 0.25 };
                        f.steer(0, 0.15, y);
                    }
                }
                f.tick(0.05, false, &mut rng);
                t += 0.05;
            }
            assert!(t >= WAVE_SECONDS - 0.1);
            f.hits
        };
        assert!(run(true) < run(false), "dodging helps");
    }

    #[test]
    fn coop_flies_to_the_average_touch() {
        let mut f = AsteroidField::new(0);
        f.steer(0, 0.2, 0.2);
        f.steer(1, 0.2, 0.8);
        let mut rng = Rng::new(1);
        for _ in 0..100 {
            f.tick(0.05, true, &mut rng);
            f.rocks.clear();
        }
        assert!((f.ship.1 - 0.5).abs() < 0.02, "{:?}", f.ship);
    }
}
