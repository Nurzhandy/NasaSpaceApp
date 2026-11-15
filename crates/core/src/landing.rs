//! Part B1 — the landing on the Moon: choose a site near the south pole, then fly the lander
//! down. Pure data like the rest of `ja-core`: the app calls `Lander::tick(dt, input)` every
//! frame and draws `Lander` (side view, metres).
//!
//! Real numbers: lunar gravity 1.62 m/s²; the descent engine can push about twice that, as on
//! the Apollo lunar module. The "1202" computer alarm and the "60 / 30 seconds" fuel calls
//! come from Apollo 11 (1969). A failed landing is never a crash: the autopilot aborts back to
//! orbit and the crew tries again.

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Lunar gravity, m/s².
pub const MOON_G: f32 = 1.62;
/// Full-throttle acceleration of the descent engine, m/s² (≈ 2 × lunar gravity).
pub const THRUST: f32 = 3.3;
/// Tilt speed, degrees per second.
pub const TURN_RATE: f32 = 45.0;
/// The level ground ends at ±`GROUND_HALF` metres; flying past it aborts.
pub const GROUND_HALF: f32 = 700.0;
/// Start: altitude, sink rate, drift (Junior / Advanced), horizontal position.
pub const START_ALT: f32 = 420.0;
pub const START_VY: f32 = -14.0;
pub const START_VX: [f32; 2] = [6.0, 10.0];
pub const START_X: f32 = -320.0;
/// The landing pad is at least this wide (metres of clear ground).
pub const PAD_MIN: f32 = 70.0;
/// The 1202 alarm goes off once, below this altitude.
pub const ALARM_ALT: f32 = 260.0;
/// How long the instruments stay frozen if nobody answers the alarm.
pub const ALARM_LEN: f32 = 4.0;

/// A landing site: three of NASA's candidate regions near the lunar south pole.
#[derive(Clone, Copy, Debug)]
pub struct SiteDef {
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    pub desc_ru: &'static str,
    pub desc_en: &'static str,
    /// 1..=3: sunlight for solar panels
    pub sun: u8,
    /// 1..=3: water ice close to the base
    pub ice: u8,
    /// 1..=3: boulders on the ground (harder landing)
    pub rough: u8,
}

pub const SITES: [SiteDef; 3] = [
    SiteDef {
        icon: "sun",
        name_ru: "Массив Малаперт",
        name_en: "Malapert Massif",
        desc_ru: "Высокая гора: солнце светит почти всегда, но вокруг много камней, а до льда далеко.",
        desc_en: "A tall mountain: the Sun shines almost all the time, but there are many boulders and the ice is far.",
        sun: 3,
        ice: 1,
        rough: 3,
    },
    SiteDef {
        icon: "flag",
        name_ru: "Плато Монс Мутон",
        name_en: "Mons Mouton Plateau",
        desc_ru: "Ровное плато: сесть проще всего. Солнца и льда — средне.",
        desc_en: "A flat plateau: the easiest landing. Some sun, some ice.",
        sun: 2,
        ice: 2,
        rough: 1,
    },
    SiteDef {
        icon: "ice",
        name_ru: "Край кратера Нобиле",
        name_en: "Nobile Rim",
        desc_ru: "Рядом тёмный кратер со льдом — вода и кислород под боком. Но солнца мало, и база мёрзнет.",
        desc_en: "Next to a dark crater with ice — water and oxygen nearby. But there is little sun and the base gets cold.",
        sun: 1,
        ice: 3,
        rough: 2,
    },
];

/// What the pilot does this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LanderInput {
    /// 0..1 engine power
    pub throttle: f32,
    /// −1..1 tilt left / right
    pub turn: f32,
    /// the pilot answered the 1202 alarm ("Continue!")
    pub ack_alarm: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbortReason {
    TooFast,
    Tilted,
    Boulders,
    OffCourse,
    NoFuel,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LandingEvent {
    /// "1202!" — the computer is overloaded, the instruments freeze until the pilot answers
    Alarm1202,
    AlarmCleared,
    /// fuel call: hover seconds left (60 or 30)
    FuelCall(u8),
    /// "The Eagle has landed": stars 1..=3
    Touchdown { stars: u8, fuel_left: f32, attempts: u8 },
    /// not a crash: the autopilot takes the lander back up, try again
    Abort(AbortReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Flying,
    /// waiting for `retry()`
    Aborted(AbortReason),
    Landed,
}

/// What the lander was doing at the moment it touched the ground (for the "what went wrong" table).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Touch {
    /// sink speed, m/s (positive = down)
    pub down: f32,
    pub side: f32,
    pub tilt: f32,
    pub boulders: bool,
}

/// A boulder field on the ground, `from..to` metres.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Boulders {
    pub from: f32,
    pub to: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lander {
    pub site: u8,
    pub junior: bool,
    /// horizontal position and altitude, metres
    pub x: f32,
    pub alt: f32,
    /// m/s: + right, + up
    pub vx: f32,
    pub vy: f32,
    /// degrees, 0 = upright, + = tilted right (engine pushes right)
    pub angle: f32,
    /// seconds of full throttle left
    pub fuel: f32,
    pub fuel_max: f32,
    pub throttle: f32,
    pub boulders: Vec<Boulders>,
    /// seconds left of the frozen instruments (`Some` while the 1202 alarm is on)
    pub alarm: Option<f32>,
    alarm_done: bool,
    calls: u8,
    pub attempts: u8,
    pub phase: Phase,
    pub time: f32,
    /// the recommended landing pad: clear, flat ground `from..to` metres
    pub pad: (f32, f32),
    /// the last contact with the ground (set on touchdown or abort)
    pub touch: Option<Touch>,
}

impl Lander {
    pub fn new(site: u8, junior: bool, seed: u64) -> Lander {
        let site = site.min(2);
        let def = SITES[site as usize];
        let mut rng = Rng::new(seed ^ 0x1a4d);
        // boulder fields: more on rough sites, never right under the natural glide path's end
        let mut boulders = vec![];
        let fields = def.rough as usize * 2 + 1;
        let mut x = -GROUND_HALF + 60.0;
        for _ in 0..fields {
            x += rng.range(60.0, 180.0);
            let w = rng.range(30.0, 50.0 + 25.0 * def.rough as f32);
            if x + w > GROUND_HALF - 40.0 {
                break;
            }
            boulders.push(Boulders { from: x, to: x + w });
            x += w;
        }
        let fuel_max = if junior { 100.0 } else { 75.0 };
        let vx = START_VX[(!junior) as usize];
        let pad = pick_pad(&boulders, START_X + vx * 28.0);
        Lander {
            site,
            junior,
            x: START_X,
            alt: START_ALT,
            vx,
            vy: START_VY,
            angle: 0.0,
            fuel: fuel_max,
            fuel_max,
            throttle: 0.0,
            boulders,
            alarm: None,
            alarm_done: false,
            calls: 0,
            attempts: 1,
            phase: Phase::Flying,
            time: 0.0,
            pad,
            touch: None,
        }
    }

    pub fn site_def(&self) -> &'static SiteDef {
        &SITES[self.site as usize]
    }

    /// Hover seconds left: how long the engine could hold the lander still (what "60 seconds!" meant).
    /// The pilot's bonus: fuller tanks (more hover time). Call right after `new`.
    pub fn with_pilot(mut self, pilot: bool) -> Lander {
        if pilot {
            self.fuel_max *= crate::role::PILOT_FUEL;
            self.fuel = self.fuel_max;
        }
        self
    }

    pub fn hover_left(&self) -> f32 {
        self.fuel / (MOON_G / THRUST)
    }

    /// Safe limits for touchdown: (vertical m/s, horizontal m/s, tilt degrees).
    pub fn limits(&self) -> (f32, f32, f32) {
        if self.junior {
            (5.0, 3.5, 20.0)
        } else {
            (3.5, 2.5, 14.0)
        }
    }

    /// A good sink speed for this altitude: fast up high, slow near the ground (m/s, positive = down).
    pub fn safe_down(&self) -> f32 {
        let (lv, _, _) = self.limits();
        (self.alt * 0.1).clamp(lv * 0.7, 16.0)
    }

    /// Metres from the lander to the middle of the pad (+ = the pad is to the right).
    pub fn to_pad(&self) -> f32 {
        (self.pad.0 + self.pad.1) * 0.5 - self.x
    }

    pub fn over_pad(&self) -> bool {
        self.x >= self.pad.0 && self.x <= self.pad.1
    }

    /// The touchdown checklist right now: (down ok, sideways ok, tilt ok, flat ground below).
    pub fn checklist(&self) -> [bool; 4] {
        let (lv, lh, la) = self.limits();
        [-self.vy <= lv, self.vx.abs() <= lh, self.angle.abs() <= la, !self.boulders_at(self.x)]
    }

    /// Is there a boulder field under `x`?
    pub fn boulders_at(&self, x: f32) -> bool {
        self.boulders.iter().any(|b| x >= b.from && x <= b.to)
    }

    /// The instruments are readable (not frozen by the alarm).
    pub fn instruments_ok(&self) -> bool {
        self.alarm.is_none()
    }

    pub fn tick(&mut self, dt: f32, input: LanderInput) -> Vec<LandingEvent> {
        let mut ev = vec![];
        if self.phase != Phase::Flying || dt <= 0.0 {
            return ev;
        }
        let dt = dt.min(0.1);
        self.time += dt;
        // the alarm: the computer sheds work; instruments freeze until "Continue!" or a few seconds pass
        if let Some(t) = self.alarm {
            let t = t - dt;
            if input.ack_alarm || t <= 0.0 {
                self.alarm = None;
                ev.push(LandingEvent::AlarmCleared);
            } else {
                self.alarm = Some(t);
            }
        } else if !self.alarm_done && self.alt < ALARM_ALT {
            self.alarm_done = true;
            self.alarm = Some(ALARM_LEN);
            ev.push(LandingEvent::Alarm1202);
        }
        // attitude: the stabiliser levels the lander whenever the pilot lets go of A/D
        let turn = input.turn.clamp(-1.0, 1.0);
        if turn != 0.0 {
            self.angle += turn * TURN_RATE * dt;
        } else {
            let k = if self.junior { 1.0 } else { 0.6 };
            self.angle -= self.angle.signum() * (TURN_RATE * k * dt).min(self.angle.abs());
        }
        self.angle = self.angle.clamp(-45.0, 45.0);
        // engine
        self.throttle = if self.fuel > 0.0 { input.throttle.clamp(0.0, 1.0) } else { 0.0 };
        self.fuel = (self.fuel - self.throttle * dt).max(0.0);
        let a = self.throttle * THRUST;
        let rad = self.angle.to_radians();
        self.vx += a * rad.sin() * dt;
        self.vy += (a * rad.cos() - MOON_G) * dt;
        self.x += self.vx * dt;
        self.alt += self.vy * dt;
        // fuel calls, like Houston's "60 seconds", "30 seconds"
        let hover = self.hover_left();
        if self.calls == 0 && hover <= 60.0 {
            self.calls = 1;
            ev.push(LandingEvent::FuelCall(60));
        } else if self.calls == 1 && hover <= 30.0 {
            self.calls = 2;
            ev.push(LandingEvent::FuelCall(30));
        }
        if self.x.abs() > GROUND_HALF || self.alt > START_ALT + 250.0 {
            return self.abort(ev, AbortReason::OffCourse);
        }
        if self.alt <= 0.0 {
            self.alt = 0.0;
            let (lv, lh, la) = self.limits();
            self.touch = Some(Touch { down: -self.vy, side: self.vx, tilt: self.angle, boulders: self.boulders_at(self.x) });
            let reason = if self.boulders_at(self.x) {
                Some(AbortReason::Boulders)
            } else if -self.vy > lv || self.vx.abs() > lh {
                Some(if self.fuel <= 0.0 { AbortReason::NoFuel } else { AbortReason::TooFast })
            } else if self.angle.abs() > la {
                Some(AbortReason::Tilted)
            } else {
                None
            };
            if let Some(r) = reason {
                return self.abort(ev, r);
            }
            self.phase = Phase::Landed;
            let soft = -self.vy <= lv * 0.5;
            let thrifty = self.fuel >= self.fuel_max * 0.25 || self.over_pad();
            self.vx = 0.0;
            self.vy = 0.0;
            let stars = 1 + (soft || self.attempts == 1) as u8 + thrifty as u8;
            ev.push(LandingEvent::Touchdown { stars: stars.min(3), fuel_left: self.fuel, attempts: self.attempts });
        }
        ev
    }

    fn abort(&mut self, mut ev: Vec<LandingEvent>, r: AbortReason) -> Vec<LandingEvent> {
        self.phase = Phase::Aborted(r);
        self.alarm = None;
        ev.push(LandingEvent::Abort(r));
        ev
    }

    /// Back in orbit after an abort: same site, a fresh approach with a full tank.
    pub fn retry(&mut self) {
        let attempts = self.attempts.saturating_add(1);
        let boulders = std::mem::take(&mut self.boulders);
        let (fuel_max, pad) = (self.fuel_max, self.pad);
        *self = Lander { boulders, attempts, alarm_done: true, fuel: fuel_max, fuel_max, pad, ..Lander::new(self.site, self.junior, 0) };
    }

    /// Coins the landing brings to the base: stars and saved fuel.
    pub fn bonus_coins(&self) -> u32 {
        if self.phase != Phase::Landed {
            return 0;
        }
        let stars = match self.attempts {
            1 => 3,
            2 => 2,
            _ => 1,
        };
        stars * 3 + (self.fuel / self.fuel_max * 6.0).round() as u32
    }
}

/// The widest clear stretch near where the lander would naturally come down.
fn pick_pad(boulders: &[Boulders], near: f32) -> (f32, f32) {
    let mut edges = vec![-GROUND_HALF + 40.0];
    for b in boulders {
        edges.push(b.from);
        edges.push(b.to);
    }
    edges.push(GROUND_HALF - 40.0);
    let mut best: Option<(f32, f32, f32)> = None;
    for g in edges.chunks(2) {
        let (a, b) = (g[0] + 10.0, g[1] - 10.0);
        if b - a < PAD_MIN {
            continue;
        }
        // a pad no wider than 120 m, as close to `near` as the gap allows
        let w = (b - a).min(120.0);
        let c = near.clamp(a + w * 0.5, b - w * 0.5);
        let d = (c - near).abs();
        if best.map_or(true, |x| d < x.2) {
            best = Some((c - w * 0.5, c + w * 0.5, d));
        }
    }
    best.map(|b| (b.0, b.1)).unwrap_or((-60.0, 60.0))
}

/// A/D the stabiliser would press to drift towards the pad and stop over it (−1..1).
pub fn steer(l: &Lander) -> f32 {
    let want_vx = if l.alt < 25.0 { (l.to_pad() * 0.05).clamp(-1.0, 1.0) } else { (l.to_pad() * 0.08).clamp(-6.0, 6.0) };
    // near the ground never tilt past what a touchdown allows
    let max_tilt = if l.alt < 40.0 { l.limits().2 * 0.6 } else { 25.0 };
    let want_angle = ((want_vx - l.vx) * 6.0).clamp(-max_tilt, max_tilt);
    ((want_angle - l.angle) / 8.0).clamp(-1.0, 1.0)
}

/// Engine lever that holds a safe sink speed for the altitude (0..1).
pub fn hold_throttle(l: &Lander) -> f32 {
    // far from the pad and low: slow the descent down and fly over first
    let far = ((l.to_pad().abs() - 15.0) / 80.0).clamp(0.0, 0.9) * ((150.0 - l.alt) / 150.0).clamp(0.0, 1.0);
    // a bit under the checklist's norm (`safe_down`), so the helper always keeps "Down" green
    let want_vy = -(l.alt * 0.08).clamp(0.8, 12.0) * (1.0 - far);
    let need = MOON_G + (want_vy - l.vy) * 1.5;
    (need / (THRUST * l.angle.to_radians().cos().max(0.3))).clamp(0.0, 1.0)
}

/// A simple autopilot (tests, the screenshot tour, the helper): fly to the pad, come down slowly.
pub fn autopilot(l: &Lander) -> LanderInput {
    LanderInput { throttle: hold_throttle(l), turn: steer(l), ack_alarm: l.alarm.is_some() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fly(l: &mut Lander) -> Vec<LandingEvent> {
        let mut all = vec![];
        for _ in 0..20_000 {
            let input = autopilot(l);
            all.extend(l.tick(1.0 / 60.0, input));
            if l.phase != Phase::Flying {
                break;
            }
        }
        all
    }

    #[test]
    fn the_autopilot_lands_on_every_site_for_both_levels() {
        for site in 0..3 {
            for junior in [true, false] {
                for seed in 0..40 {
                    let mut l = Lander::new(site, junior, seed);
                    let ev = fly(&mut l);
                    assert_eq!(l.phase, Phase::Landed, "site {site} junior {junior} seed {seed}: {ev:?} at x {:.0} vy {:.1} vx {:.1} angle {:.1}", l.x, l.vy, l.vx, l.angle);
                    assert!(l.over_pad(), "the autopilot lands on the pad: site {site} junior {junior} seed {seed} x {:.0} pad {:?}", l.x, l.pad);
                    assert!(l.pad.1 - l.pad.0 >= PAD_MIN && !l.boulders_at((l.pad.0 + l.pad.1) * 0.5));
                    assert!(ev.contains(&LandingEvent::Alarm1202));
                    assert!(l.bonus_coins() > 0);
                }
            }
        }
    }

    #[test]
    fn falling_without_the_engine_aborts_instead_of_crashing() {
        let mut l = Lander::new(1, false, 3);
        let mut ev = vec![];
        for _ in 0..5_000 {
            ev.extend(l.tick(1.0 / 60.0, LanderInput::default()));
            if l.phase != Phase::Flying {
                break;
            }
        }
        assert!(matches!(l.phase, Phase::Aborted(_)));
        let t = l.touch.expect("the touchdown is recorded for the debrief");
        assert!(t.down > l.limits().0);
        assert!(ev.iter().any(|e| matches!(e, LandingEvent::Abort(_))));
        l.retry();
        assert_eq!(l.phase, Phase::Flying);
        assert_eq!(l.attempts, 2);
        assert_eq!(l.fuel, l.fuel_max);
        // the alarm already happened once
        assert!(!fly(&mut l).contains(&LandingEvent::Alarm1202));
    }

    #[test]
    fn the_alarm_freezes_instruments_until_answered() {
        let mut l = Lander::new(1, false, 1);
        l.alt = ALARM_ALT + 1.0;
        let mut seen = false;
        for _ in 0..200 {
            let e = l.tick(1.0 / 60.0, LanderInput { throttle: 0.5, ..Default::default() });
            if e.contains(&LandingEvent::Alarm1202) {
                seen = true;
                break;
            }
        }
        assert!(seen && !l.instruments_ok());
        let e = l.tick(1.0 / 60.0, LanderInput { throttle: 0.5, turn: 0.0, ack_alarm: true });
        assert!(e.contains(&LandingEvent::AlarmCleared));
        assert!(l.instruments_ok());
    }

    #[test]
    fn fuel_calls_come_at_60_and_30_hover_seconds() {
        let mut l = Lander::new(1, false, 1);
        l.alarm_done = true;
        l.vx = 0.0;
        let mut calls = vec![];
        for _ in 0..10_000 {
            // hover in place until the tank runs dry
            let input = LanderInput { throttle: ((MOON_G - l.vy) / THRUST).clamp(0.0, 1.0), ..Default::default() };
            for e in l.tick(1.0 / 60.0, input) {
                if let LandingEvent::FuelCall(s) = e {
                    calls.push(s);
                }
            }
            if l.fuel <= 0.0 || l.phase != Phase::Flying {
                break;
            }
        }
        assert_eq!(calls, vec![60, 30]);
    }
}
