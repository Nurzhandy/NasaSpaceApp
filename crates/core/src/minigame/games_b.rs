//! On-the-way games: star sighting, course vectors, radio tuning with the light delay, the toilet
//! fan bolts, the flare shelter, the suit leak, the centrifuge, the antenna.

use super::*;

// ---------------------------------------------------------------- star sighting

const SKY: Btn = Btn(220.0, 40.0, 490.0, 270.0);

/// Find the star by its pattern on the map, then turn the sextant until it sits on the horizon.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StarSight {
    pub stars: Vec<(f32, f32, f32)>,
    pub target: usize,
    pub found: bool,
    pub angle: f32,
    pub want: f32,
    pub held: bool,
    pub rounds: u8,
    pub seed: u64,
}
const SX: f32 = 220.0;
const SW: f32 = 480.0;
impl StarSight {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let mut s = StarSight { stars: vec![], target: 0, found: false, angle: 0.1, want: 0.5, held: false, rounds: if hard { 2 } else { 1 }, seed: rng.next() };
        s.reroll();
        s
    }
    fn reroll(&mut self) {
        let mut seed = self.seed;
        self.stars.clear();
        while self.stars.len() < 26 {
            let p = (SKY.0 + 20.0 + lcg(&mut seed) * (SKY.2 - 40.0), SKY.1 + 20.0 + lcg(&mut seed) * (SKY.3 - 40.0), 3.0 + lcg(&mut seed) * 5.0);
            if self.stars.iter().all(|q| dist((p.0, p.1), (q.0, q.1)) > 36.0) {
                self.stars.push(p);
            }
        }
        self.target = (lcg(&mut seed) * 26.0) as usize % 26;
        self.want = 0.2 + lcg(&mut seed) * 0.6;
        self.angle = if self.want > 0.5 { 0.05 } else { 0.95 };
        self.seed = seed;
        self.found = false;
    }
    fn pattern(&self) -> Vec<usize> {
        let t = self.stars[self.target];
        let mut v: Vec<usize> = (0..self.stars.len()).filter(|k| *k != self.target).collect();
        v.sort_by(|a, b| dist((self.stars[*a].0, self.stars[*a].1), (t.0, t.1)).total_cmp(&dist((self.stars[*b].0, self.stars[*b].1), (t.0, t.1))));
        v.truncate(3);
        v
    }
    fn tol(hard: bool) -> f32 {
        if hard {
            0.015
        } else {
            0.03
        }
    }
}
fn lcg(s: &mut u64) -> f32 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*s >> 40) as f32 / (1u64 << 24) as f32
}
impl Play for StarSight {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) if !self.found => {
                if !SKY.hit(x, y) {
                    return Outcome::Fine;
                }
                let k = (0..self.stars.len()).min_by(|a, b| dist((x, y), (self.stars[*a].0, self.stars[*a].1)).total_cmp(&dist((x, y), (self.stars[*b].0, self.stars[*b].1))));
                match k {
                    Some(k) if dist((x, y), (self.stars[k].0, self.stars[k].1)) < 18.0 => {
                        if k == self.target {
                            self.found = true;
                            Outcome::Fine
                        } else {
                            Outcome::Mistake
                        }
                    }
                    _ => Outcome::Fine,
                }
            }
            MiniInput::Press(x, y) => {
                self.held = (x - (SX + self.angle * SW)).abs() < 30.0 && (y - 372.0).abs() < 30.0;
                Outcome::Fine
            }
            MiniInput::Drag(x, _) if self.held => {
                self.angle = ((x - SX) / SW).clamp(0.0, 1.0);
                Outcome::Fine
            }
            MiniInput::Release(..) if self.held => {
                self.held = false;
                if (self.angle - self.want).abs() < Self::tol(hard) {
                    self.rounds -= 1;
                    if self.rounds == 0 {
                        return Outcome::Done;
                    }
                    self.reroll();
                }
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        if self.found {
            0.5
        } else {
            0.0
        }
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if !self.found {
            let t = self.stars[self.target];
            click(&mut o, (t.0, t.1));
        } else {
            drag_to(&mut o, (SX + self.angle * SW, 372.0), (SX + self.want * SW, 372.0));
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        if self.found {
            p.hint("Двигай ползунок секстанта, пока звезда не сядет точно на линию горизонта Земли", "Slide the sextant until the star sits exactly on Earth's horizon line");
        } else {
            p.hint("Найди на небе звезду из карты (по её соседям) и нажми на неё", "Find the star from the map (by its neighbours) and press it");
        }
        // the star map card
        p.rect(14.0, 40.0, 196.0, 200.0, 0xf0ead8);
        p.text(24.0, 46.0, "Карта звёзд", "Star chart", 17.0, 0x333333);
        let t = self.stars[self.target];
        for k in self.pattern() {
            let s = self.stars[k];
            p.circle(112.0 + (s.0 - t.0) * 0.8, 145.0 + (s.1 - t.1) * 0.8, 4.0, 0x333333);
            p.line(112.0, 145.0, 112.0 + (s.0 - t.0) * 0.8, 145.0 + (s.1 - t.1) * 0.8, 2.0, 0x9a9080);
        }
        p.circle(112.0, 145.0, 7.0, 0xc03030);
        p.ring(112.0, 145.0, 13.0, 0xc03030, 2.0);
        p.rect(SKY.0, SKY.1, SKY.2, SKY.3, 0x070a24);
        for (k, s) in self.stars.iter().enumerate() {
            p.circle(s.0, s.1, s.2 * 0.6, 0xffffff);
            if self.found && k == self.target {
                p.ring(s.0, s.1, 14.0, 0x5aff8a, 3.0);
            }
        }
        if !hard && !self.found {
            p.text(14.0, 250.0, "Подсказка: смотри, как стоят 3 соседние звезды", "Tip: look at how its 3 neighbours stand", 14.0, 0xb0b8d8);
        }
        if self.found {
            // the sextant view: star vs horizon
            p.rect(14.0, 250.0, 196.0, 100.0, 0x0b0e22);
            let off = (self.angle - self.want) * 300.0;
            p.rect(24.0, 300.0, 176.0, 4.0, 0x3a8bff);
            p.circle(112.0, 302.0 - off, 7.0, 0xffffff);
            p.raw(SX, 334.0, format!("{:.1}°", 10.0 + self.angle * 50.0), 18.0, 0xffd35a, false);
            p.rect(SX, 366.0, SW, 12.0, 0x2a3048);
            p.circle(SX + self.angle * SW, 372.0, 18.0, 0xffd35a);
            p.text(SX + 90.0, 334.0, "Угол между звездой и горизонтом", "Angle between the star and the horizon", 16.0, 0xb0b8d8);
        }
    }
}

// ---------------------------------------------------------------- course vector

const SHIP: (f32, f32) = (150.0, 230.0);
const FIRE: Btn = Btn(520.0, 370.0, 190.0, 50.0);

/// Drag the thrust arrow: speed now + push = new speed. Make the new speed point into the corridor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    pub v: (f32, f32),
    pub t: (f32, f32),
    pub aim: f32,
    pub tol: f32,
    pub budget: f32,
    pub held: bool,
}
impl Vector {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let aim: f32 = rng.range(-0.25, 0.25);
        let off = rng.range(0.3, 0.45) * if rng.chance(0.5) { 1.0 } else { -1.0 };
        let len = 230.0;
        let v = (len * (aim + off).cos(), len * (aim + off).sin());
        let need = len * off.sin().abs();
        Vector { v, t: (0.0, 0.0), aim, tol: if hard { 0.05 } else { 0.1 }, budget: need * if hard { 1.12 } else { 1.4 }, held: false }
    }
    fn result(&self) -> (f32, f32) {
        (self.v.0 + self.t.0, self.v.1 + self.t.1)
    }
    fn ok(&self) -> bool {
        let r = self.result();
        let a = r.1.atan2(r.0);
        (a - self.aim).abs() < self.tol && self.t.0.hypot(self.t.1) <= self.budget
    }
}
impl Play for Vector {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                if FIRE.hit(x, y) {
                    return if self.ok() { Outcome::Done } else { Outcome::Mistake };
                }
                let tip = (SHIP.0 + self.v.0 + self.t.0, SHIP.1 + self.v.1 + self.t.1);
                self.held = dist((x, y), tip) < 40.0 || dist((x, y), (SHIP.0 + self.v.0, SHIP.1 + self.v.1)) < 40.0;
                Outcome::Fine
            }
            MiniInput::Drag(x, y) if self.held => {
                let mut t = (x - SHIP.0 - self.v.0, y - SHIP.1 - self.v.1);
                let l = t.0.hypot(t.1);
                let max = self.budget * 1.5;
                if l > max {
                    t = (t.0 / l * max, t.1 / l * max);
                }
                self.t = t;
                Outcome::Fine
            }
            MiniInput::Release(..) => {
                self.held = false;
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        if self.ok() {
            0.8
        } else {
            0.0
        }
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if !self.ok() {
            let u = (self.aim.cos(), self.aim.sin());
            let proj = self.v.0 * u.0 + self.v.1 * u.1;
            let r = (u.0 * proj, u.1 * proj);
            let t = (r.0 - self.v.0, r.1 - self.v.1);
            let from = (SHIP.0 + self.v.0 + self.t.0, SHIP.1 + self.v.1 + self.t.1);
            drag_to(&mut o, from, (SHIP.0 + self.v.0 + t.0, SHIP.1 + self.v.1 + t.1));
        } else {
            click(&mut o, FIRE.mid());
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Тяни оранжевую стрелку толчка: зелёная (новая скорость) должна смотреть в коридор к Луне", "Drag the orange push arrow: the green (new speed) must point into the corridor to the Moon");
        let far = 560.0;
        for s in [-1.0, 1.0] {
            let a = self.aim + s * self.tol;
            p.line(SHIP.0, SHIP.1, SHIP.0 + far * a.cos(), SHIP.1 + far * a.sin(), 3.0, 0x2fbf6a);
        }
        p.icon("moon", SHIP.0 + far * self.aim.cos().min(1.0) - 10.0, SHIP.1 + (far - 10.0) * self.aim.sin(), 70.0);
        p.ring(SHIP.0 + self.v.0, SHIP.1 + self.v.1, self.budget, 0xff8a3d, 2.0);
        p.line(SHIP.0, SHIP.1, SHIP.0 + self.v.0, SHIP.1 + self.v.1, 6.0, 0x5ac8ff);
        let tip = (SHIP.0 + self.v.0 + self.t.0, SHIP.1 + self.v.1 + self.t.1);
        p.line(SHIP.0 + self.v.0, SHIP.1 + self.v.1, tip.0, tip.1, 6.0, 0xff8a3d);
        p.line(SHIP.0, SHIP.1, tip.0, tip.1, 4.0, if self.ok() { 0x5aff8a } else { 0xff5a5a });
        p.circle(tip.0, tip.1, 14.0, 0xff8a3d);
        p.icon("rocket", SHIP.0, SHIP.1, 50.0);
        p.text(14.0, 360.0, "синяя — скорость сейчас", "blue — speed now", 15.0, 0x5ac8ff);
        p.text(14.0, 380.0, "оранжевая — толчок (в круге топлива)", "orange — push (inside the fuel circle)", 15.0, 0xff8a3d);
        p.text(14.0, 400.0, "зелёная = синяя + оранжевая", "green = blue + orange", 15.0, 0x5aff8a);
        p.button(FIRE, "Включить", "Fire", 0xffd35a, false);
    }
}

// ---------------------------------------------------------------- radio tuning

const SEND: Btn = Btn(580.0, 290.0, 130.0, 70.0);
fn slider_y(k: usize) -> f32 {
    300.0 + k as f32 * 42.0
}

/// Match our wave to Earth's (frequency, loudness, and on hard — phase), send, wait for the reply.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tune {
    pub val: Vec<f32>,
    pub want: Vec<f32>,
    pub held: Option<usize>,
    pub sent: Option<f32>,
}
const TX: f32 = 150.0;
const TW: f32 = 400.0;
const DELAY: f32 = 1.3;
impl Tune {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 3 } else { 2 };
        Tune { val: (0..n).map(|_| rng.range(0.05, 0.3)).collect(), want: (0..n).map(|_| rng.range(0.45, 0.95)).collect(), held: None, sent: None }
    }
    fn ok(&self, hard: bool) -> bool {
        let tol = if hard { 0.035 } else { 0.06 };
        self.val.iter().zip(&self.want).all(|(a, b)| (a - b).abs() < tol)
    }
    fn wave(p: &mut Pen, v: &[f32], c: u32, y0: f32) {
        let f = 1.0 + v[0] * 5.0;
        let a = 20.0 + v[1] * 50.0;
        let ph = v.get(2).copied().unwrap_or(0.5) * 6.28;
        for i in 0..90 {
            let x = 30.0 + i as f32 * 7.4;
            p.circle(x, y0 + a * (x / 120.0 * f + ph).sin(), 3.0, c);
        }
    }
}
impl Play for Tune {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        if self.sent.is_some() {
            return Outcome::Fine;
        }
        match i {
            MiniInput::Press(x, y) => {
                if SEND.hit(x, y) {
                    if self.ok(hard) {
                        self.sent = Some(0.0);
                        return Outcome::Fine;
                    }
                    return Outcome::Mistake;
                }
                self.held = (0..self.val.len()).find(|k| (y - slider_y(*k)).abs() < 18.0 && (x - (TX + self.val[*k] * TW)).abs() < 30.0);
                Outcome::Fine
            }
            MiniInput::Drag(x, _) => {
                if let Some(k) = self.held {
                    self.val[k] = ((x - TX) / TW).clamp(0.0, 1.0);
                }
                Outcome::Fine
            }
            MiniInput::Release(..) => {
                self.held = None;
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        if let Some(t) = self.sent.as_mut() {
            *t += dt;
            if *t >= DELAY * 2.0 + 0.4 {
                return Outcome::Done;
            }
        }
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        match self.sent {
            Some(t) => 0.5 + t / (DELAY * 4.0),
            None => 0.0,
        }
    }
    fn bot(&self, hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if self.sent.is_some() {
            return vec![MiniInput::Wait(0.1)];
        }
        if let Some(k) = (0..self.val.len()).find(|k| (self.val[*k] - self.want[*k]).abs() >= if hard { 0.02 } else { 0.03 }) {
            drag_to(&mut o, (TX + self.val[k] * TW, slider_y(k)), (TX + self.want[k] * TW, slider_y(k)));
        } else {
            click(&mut o, SEND.mid());
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint("Подстрой свою волну (оранжевую) под волну Земли (зелёную) и отправь сообщение", "Tune your wave (orange) to Earth's (green) and send the message");
        p.rect(14.0, 40.0, 692.0, 230.0, 0x0b1a14);
        Self::wave(p, &self.want, 0x2fbf6a, 150.0);
        Self::wave(p, &self.val, 0xff8a3d, 150.0);
        let names = [("Частота", "Frequency"), ("Громкость", "Loudness"), ("Фаза", "Phase")];
        for k in 0..self.val.len() {
            let y = slider_y(k);
            p.text(14.0, y - 11.0, names[k].0, names[k].1, 18.0, 0xffffff);
            p.rect(TX, y - 5.0, TW, 10.0, 0x2a3048);
            p.circle(TX + self.val[k] * TW, y, 15.0, 0xff8a3d);
        }
        if let Some(t) = self.sent {
            // the message flies to Earth at the speed of light and back
            let go = (t / DELAY).min(1.0);
            let back = ((t - DELAY) / DELAY).clamp(0.0, 1.0);
            p.rect_a(14.0, 40.0, 692.0, 230.0, 0x000000, 0.6);
            p.icon("rocket", 70.0, 150.0, 50.0);
            p.icon("earth", 650.0, 150.0, 60.0);
            if t < DELAY {
                p.circle(90.0 + go * 520.0, 150.0, 8.0, 0xffd35a);
            } else {
                p.circle(610.0 - back * 520.0, 150.0, 8.0, 0x5aff8a);
            }
            p.raw(360.0, 60.0, format!("{:.1} с", t.min(DELAY * 2.0)), 26.0, 0xffffff, true);
            p.ctext(360.0, 220.0, "Свет и радио летят до Луны 1,3 секунды", "Light and radio take 1.3 seconds to reach the Moon", 17.0, 0xb0b8d8);
            if t >= DELAY * 2.0 {
                p.ctext(360.0, 95.0, "Земля: «Слышим вас отлично!»", "Earth: “We hear you loud and clear!”", 22.0, 0x5aff8a);
            }
        } else {
            p.button(SEND, "Отправить", "Send", if self.ok(hard) { 0x5aff8a } else { 0x8a90a8 }, false);
        }
    }
}

// ---------------------------------------------------------------- toilet fan bolts

const BOLTS: [(f32, f32); 4] = [(270.0, 120.0), (450.0, 120.0), (270.0, 300.0), (450.0, 300.0)];
const FAN_SLOT: (f32, f32) = (360.0, 210.0);
const FAN_BIN: Btn = Btn(20.0, 280.0, 150.0, 130.0);
const NEW_FAN: (f32, f32) = (620.0, 330.0);

/// Unscrew four bolts (turn around each one), swap the clogged fan, screw them back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bolts {
    /// 0 unscrew, 1 swap the fan, 2 screw back
    pub phase: u8,
    pub turn: Vec<f32>,
    pub held: Option<usize>,
    pub last: f32,
    pub old_fan: Piece,
    pub new_fan: Piece,
    pub fan_held: Option<u8>,
}
impl Bolts {
    pub fn new(_hard: bool, _rng: &mut Rng) -> Self {
        Bolts { phase: 0, turn: vec![0.0; 4], held: None, last: 0.0, old_fan: Piece::new(FAN_SLOT.0, FAN_SLOT.1, 0), new_fan: Piece::new(NEW_FAN.0, NEW_FAN.1, 1), fan_held: None }
    }
    fn need(hard: bool) -> f32 {
        if hard {
            4.0 * std::f32::consts::PI
        } else {
            2.0 * std::f32::consts::PI
        }
    }
}
fn wrap(a: f32) -> f32 {
    let pi = std::f32::consts::PI;
    let mut a = a;
    while a > pi {
        a -= 2.0 * pi;
    }
    while a < -pi {
        a += 2.0 * pi;
    }
    a
}
impl Play for Bolts {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        let need = Self::need(hard);
        match (self.phase, i) {
            (0 | 2, MiniInput::Press(x, y)) => {
                self.held = (0..4).find(|k| self.turn[*k] < need && dist((x, y), BOLTS[*k]) < 60.0);
                if let Some(k) = self.held {
                    self.last = (y - BOLTS[k].1).atan2(x - BOLTS[k].0);
                }
                Outcome::Fine
            }
            (0 | 2, MiniInput::Drag(x, y)) => {
                let Some(k) = self.held else { return Outcome::Fine };
                let a = (y - BOLTS[k].1).atan2(x - BOLTS[k].0);
                let d = wrap(a - self.last);
                self.last = a;
                // unscrew = against the clock (angle goes down on screen), screw = with the clock
                let good = if self.phase == 0 { -d } else { d };
                if good > 0.0 && good < 1.2 {
                    self.turn[k] += good;
                }
                if self.turn.iter().all(|t| *t >= need) {
                    self.held = None;
                    if self.phase == 0 {
                        self.phase = 1;
                    } else {
                        return Outcome::Done;
                    }
                }
                Outcome::Fine
            }
            (0 | 2, MiniInput::Release(..)) => {
                self.held = None;
                Outcome::Fine
            }
            (1, MiniInput::Press(x, y)) => {
                self.fan_held = if !self.old_fan.placed && dist((x, y), (self.old_fan.x, self.old_fan.y)) < 60.0 {
                    Some(0)
                } else if !self.new_fan.placed && dist((x, y), (self.new_fan.x, self.new_fan.y)) < 60.0 {
                    Some(1)
                } else {
                    None
                };
                Outcome::Fine
            }
            (1, MiniInput::Drag(x, y)) => {
                match self.fan_held {
                    Some(0) => (self.old_fan.x, self.old_fan.y) = (x, y),
                    Some(_) => (self.new_fan.x, self.new_fan.y) = (x, y),
                    None => {}
                }
                Outcome::Fine
            }
            (1, MiniInput::Release(x, y)) => {
                let Some(f) = self.fan_held.take() else { return Outcome::Fine };
                let mut r = Outcome::Fine;
                if f == 0 {
                    if FAN_BIN.hit(x, y) {
                        self.old_fan.placed = true;
                    } else {
                        self.old_fan.home();
                    }
                } else if dist((x, y), FAN_SLOT) < 50.0 && self.old_fan.placed {
                    self.new_fan.placed = true;
                    self.new_fan.x = FAN_SLOT.0;
                    self.new_fan.y = FAN_SLOT.1;
                } else {
                    if FAN_BIN.hit(x, y) || dist((x, y), FAN_SLOT) < 50.0 {
                        r = Outcome::Mistake; // the new fan in the bin, or the slot still has the old one
                    }
                    self.new_fan.home();
                }
                if self.old_fan.placed && self.new_fan.placed {
                    self.phase = 2;
                    self.turn = vec![0.0; 4];
                }
                r
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        self.phase as f32 / 3.0
    }
    fn bot(&self, hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        let need = Self::need(hard);
        match self.phase {
            1 => {
                if !self.old_fan.placed {
                    drag_to(&mut o, (self.old_fan.x, self.old_fan.y), FAN_BIN.mid());
                } else {
                    drag_to(&mut o, (self.new_fan.x, self.new_fan.y), FAN_SLOT);
                }
            }
            _ => {
                if let Some(k) = (0..4).find(|k| self.turn[*k] < need) {
                    let dir = if self.phase == 0 { -1.0 } else { 1.0 };
                    let (cx, cy) = BOLTS[k];
                    let mut a: f32 = 0.0;
                    o.push(MiniInput::Press(cx + 40.0, cy));
                    let steps = (need / 0.5).ceil() as usize + 2;
                    for _ in 0..steps {
                        a += 0.5 * dir;
                        o.push(MiniInput::Drag(cx + 40.0 * a.cos(), cy + 40.0 * a.sin()));
                    }
                    o.push(MiniInput::Release(cx, cy));
                }
            }
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        match self.phase {
            0 => p.hint("Открути 4 болта: зажми у болта и крути по кругу ПРОТИВ часовой стрелки (<-)", "Unscrew 4 bolts: press by a bolt and circle AGAINST the clock (<-)"),
            1 => p.hint("Забитый вентилятор — в мусор, новый — на место", "Clogged fan into the bin, the new one into the slot"),
            _ => p.hint("Закрути болты обратно — ПО часовой стрелке (->)", "Screw the bolts back — WITH the clock (->)"),
        }
        let need = Self::need(hard);
        if self.phase != 1 {
            p.rect(240.0, 90.0, 240.0, 240.0, 0xb8c0d4);
            p.rect(256.0, 106.0, 208.0, 208.0, 0x9aa3b8);
        } else {
            p.circle(FAN_SLOT.0, FAN_SLOT.1, 80.0, 0x2a2f45);
        }
        for (k, b) in BOLTS.iter().enumerate() {
            if self.phase == 1 {
                continue;
            }
            let frac = (self.turn[k] / need).min(1.0);
            p.circle(b.0, b.1, 22.0, 0x5a6178);
            p.circle(b.0, b.1, 14.0, if frac >= 1.0 { 0x5aff8a } else { 0xdfe5f2 });
            let dots = (frac * 16.0) as usize;
            for d in 0..dots {
                let a = d as f32 / 16.0 * std::f32::consts::TAU;
                p.circle(b.0 + 34.0 * a.cos(), b.1 + 34.0 * a.sin(), 3.0, 0xffd35a);
            }
            p.raw(b.0, b.1 - 52.0, if self.phase == 0 { "<-".into() } else { "->".into() }, 22.0, 0xffd35a, true);
        }
        if self.phase == 1 || self.phase == 2 && self.new_fan.placed {
            if !self.old_fan.placed {
                p.circle(self.old_fan.x, self.old_fan.y, 58.0, 0x6a5a3a);
                p.icon("gear", self.old_fan.x, self.old_fan.y, 70.0);
                p.ctext(self.old_fan.x, self.old_fan.y + 50.0, "забит пылью", "clogged", 14.0, 0xffffff);
            }
        }
        if self.phase >= 1 {
            p.circle(self.new_fan.x, self.new_fan.y, 58.0, 0x3a8bff);
            p.icon("gear", self.new_fan.x, self.new_fan.y, 70.0);
            if !self.new_fan.placed {
                p.ctext(self.new_fan.x, self.new_fan.y + 50.0, "новый", "new", 14.0, 0xffffff);
            }
        }
        p.rect(FAN_BIN.0, FAN_BIN.1, FAN_BIN.2, FAN_BIN.3, 0x5a5048);
        p.ctext(FAN_BIN.0 + 75.0, FAN_BIN.1 + 50.0, "Мусор", "Bin", 18.0, 0xffffff);
        p.text(500.0, 60.0, "В космосе нет силы тяжести —", "No gravity in space —", 15.0, 0xb0b8d8);
        p.text(500.0, 80.0, "туалет работает потоком воздуха", "the toilet works by airflow", 15.0, 0xb0b8d8);
    }
}

// ---------------------------------------------------------------- flare shelter

const BAGS: [(&str, &str, u32); 4] = [("Вода", "Water", 0x3a8bff), ("Еда", "Food", 0xd9883a), ("Пустой мешок", "Empty bag", 0xc8c8c8), ("Железный ящик", "Iron box", 0x6a6a7a)];

/// A solar flare is coming: build a wall of water and food bags around the sleeping place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shelter {
    pub gaps: Vec<(f32, f32, bool)>,
    pub bags: Vec<Piece>,
    pub held: Option<usize>,
    pub t: f32,
    pub why: Option<u8>,
}
impl Shelter {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 10 } else { 6 };
        let gaps = (0..n).map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            (360.0 + 120.0 * a.cos(), 225.0 + 105.0 * a.sin(), false)
        });
        let mut kinds: Vec<u8> = (0..n).map(|i| if i % 3 == 2 { 1 } else { 0 }).collect();
        kinds.push(2);
        kinds.push(2);
        if hard {
            kinds.push(3);
            kinds.push(3);
        }
        shuffle(&mut kinds, rng);
        let bags = kinds.iter().enumerate().map(|(i, k)| Piece::new(if i % 2 == 0 { 60.0 } else { 660.0 }, 60.0 + (i / 2) as f32 * (340.0 / ((kinds.len() as f32 / 2.0).ceil())), *k)).collect();
        Shelter { gaps: gaps.collect(), bags, held: None, t: 0.0, why: None }
    }
}
impl Play for Shelter {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.why = None;
                self.held = grab(&self.bags, x, y, 34.0);
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if let Some(k) = self.held {
                    (self.bags[k].x, self.bags[k].y) = (x, y);
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                let Some(k) = self.held.take() else { return Outcome::Fine };
                let Some(g) = (0..self.gaps.len()).find(|g| !self.gaps[*g].2 && dist((x, y), (self.gaps[*g].0, self.gaps[*g].1)) < 36.0) else {
                    self.bags[k].home();
                    return Outcome::Fine;
                };
                let kind = self.bags[k].kind;
                if kind >= 2 {
                    self.why = Some(kind);
                    self.bags[k].home();
                    return Outcome::Mistake;
                }
                self.gaps[g].2 = true;
                self.bags[k].placed = true;
                (self.bags[k].x, self.bags[k].y) = (self.gaps[g].0, self.gaps[g].1);
                if self.gaps.iter().all(|g| g.2) {
                    Outcome::Done
                } else {
                    Outcome::Fine
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        self.t += dt;
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.gaps.iter().filter(|g| g.2).count() as f32 / self.gaps.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if let (Some(b), Some(g)) = (self.bags.iter().position(|b| !b.placed && b.kind < 2), self.gaps.iter().position(|g| !g.2)) {
            drag_to(&mut o, (self.bags[b].x, self.bags[b].y), (self.gaps[g].0, self.gaps[g].1));
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Закрой все щели вокруг спальника мешками, которые задерживают излучение", "Fill every gap around the sleeping bag with bags that stop radiation");
        // flare particles flying in
        for i in 0..14 {
            let f = (self.t * 0.6 + i as f32 * 0.071) % 1.0;
            let a = i as f32 * 0.45;
            let r = 340.0 * (1.0 - f);
            p.circle(360.0 + r * a.cos(), 225.0 + r * a.sin() * 0.7, 4.0, 0xff4a4a);
        }
        p.rect(310.0, 185.0, 100.0, 80.0, 0x3a3f5a);
        p.icon("astronaut", 360.0, 225.0, 60.0);
        for g in &self.gaps {
            if !g.2 {
                p.ring(g.0, g.1, 26.0, 0xffd35a, 3.0);
            }
        }
        for b in &self.bags {
            let (ru, en, c) = BAGS[b.kind as usize];
            p.rect(b.x - 34.0, b.y - 24.0, 68.0, 48.0, c);
            p.ctext(b.x, b.y - 10.0, ru, en, 13.0, 0x0b0e22);
        }
        if let Some(k) = self.why {
            let (ru, en) = if k == 2 { ("Пустой мешок ничего не задержит — нужно вещество!", "An empty bag stops nothing — you need stuff inside!") } else { ("Железо от частиц даёт вторичное излучение. Лучше всего — вода!", "Iron hit by particles makes secondary radiation. Water is best!") };
            p.rect_a(10.0, 384.0, 700.0, 38.0, 0x601818, 0.9);
            p.text(20.0, 392.0, ru, en, 17.0, 0xffffff);
        }
    }
}

// ---------------------------------------------------------------- suit leak

const PUMP: Btn = Btn(540.0, 330.0, 170.0, 80.0);

/// Move the leak sensor over the suit until it hisses loudest, patch it; then pump to the right pressure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FindLeak {
    pub leaks: Vec<(f32, f32, bool)>,
    pub sensor: (f32, f32),
    pub pressure: f32,
    pub pumping: bool,
}
impl FindLeak {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let spots = [(330.0, 120.0), (390.0, 150.0), (300.0, 240.0), (420.0, 250.0), (340.0, 330.0), (380.0, 360.0), (260.0, 180.0), (460.0, 190.0)];
        let mut idx: Vec<usize> = (0..spots.len()).collect();
        shuffle(&mut idx, rng);
        let leaks = idx.iter().take(if hard { 2 } else { 1 }).map(|k| (spots[*k].0 + rng.range(-10.0, 10.0), spots[*k].1 + rng.range(-10.0, 10.0), false)).collect();
        FindLeak { leaks, sensor: (600.0, 120.0), pressure: 0.0, pumping: false }
    }
    fn hiss(&self) -> f32 {
        self.leaks.iter().filter(|l| !l.2).map(|l| 1.0 - (dist(self.sensor, (l.0, l.1)) / 260.0).min(1.0)).fold(0.0, f32::max)
    }
    fn found_all(&self) -> bool {
        self.leaks.iter().all(|l| l.2)
    }
    fn zone(hard: bool) -> (f32, f32) {
        if hard {
            (0.72, 0.8)
        } else {
            (0.68, 0.84)
        }
    }
}
impl Play for FindLeak {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        if !self.found_all() {
            match i {
                MiniInput::Press(x, y) | MiniInput::Drag(x, y) => self.sensor = (x, y),
                MiniInput::Release(x, y) => {
                    self.sensor = (x, y);
                    let r = if hard { 16.0 } else { 24.0 };
                    if let Some(l) = self.leaks.iter_mut().find(|l| !l.2 && dist((x, y), (l.0, l.1)) < r) {
                        l.2 = true;
                    }
                }
                _ => {}
            }
            return Outcome::Fine;
        }
        match i {
            MiniInput::Press(x, y) if PUMP.hit(x, y) => {
                self.pumping = true;
                Outcome::Fine
            }
            MiniInput::Space(true) => {
                self.pumping = true;
                Outcome::Fine
            }
            MiniInput::Release(..) | MiniInput::Space(false) if self.pumping => {
                self.pumping = false;
                let (lo, hi) = Self::zone(hard);
                if self.pressure >= lo && self.pressure <= hi {
                    Outcome::Done
                } else if self.pressure > hi {
                    self.pressure = 0.3;
                    Outcome::Mistake
                } else {
                    Outcome::Fine
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        if self.pumping {
            self.pressure += dt * 0.22;
            if self.pressure > 1.0 {
                self.pressure = 0.3;
                self.pumping = false;
                return Outcome::Mistake;
            }
        }
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.leaks.iter().filter(|l| l.2).count() as f32 / self.leaks.len() as f32 * 0.6 + self.pressure * 0.4
    }
    fn bot(&self, hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if let Some(l) = self.leaks.iter().find(|l| !l.2) {
            o.push(MiniInput::Press(l.0 + 3.0, l.1));
            o.push(MiniInput::Release(l.0, l.1));
            return o;
        }
        let (lo, hi) = Self::zone(hard);
        if !self.pumping {
            o.push(MiniInput::Press(PUMP.mid().0, PUMP.mid().1));
        } else if self.pressure >= (lo + hi) / 2.0 {
            o.push(MiniInput::Release(0.0, 0.0));
        } else {
            o.push(MiniInput::Wait(0.02));
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        if self.found_all() {
            p.hint("Держи «Наддув» — отпусти, когда давление в зелёной зоне", "Hold “Pump” — let go when the pressure is in the green");
        } else {
            p.hint("Води датчиком по скафандру: чем громче шипит — тем ближе дырка. Отпусти прямо на ней", "Move the sensor over the suit: the louder the hiss, the closer the hole. Let go right on it");
        }
        p.icon("astronaut", 360.0, 240.0, 340.0);
        for l in &self.leaks {
            if l.2 {
                p.rect(l.0 - 16.0, l.1 - 12.0, 32.0, 24.0, 0xffcc33);
            }
        }
        if !self.found_all() {
            let h = self.hiss();
            if h > 0.85 {
                for l in self.leaks.iter().filter(|l| !l.2) {
                    p.circle_a(l.0 + 10.0, l.1 - 10.0, 5.0, 0xffffff, 0.7);
                    p.circle_a(l.0 + 18.0, l.1 - 22.0, 4.0, 0xffffff, 0.5);
                }
            }
            p.ring(self.sensor.0, self.sensor.1, 22.0, 0x5aff8a, 4.0);
            p.text(20.0, 60.0, "Шипение", "Hiss", 18.0, 0xffffff);
            p.rect(20.0, 90.0, 26.0, 280.0, 0x2a3048);
            p.rect(20.0, 90.0 + 280.0 * (1.0 - h), 26.0, 280.0 * h, if h > 0.85 { 0xff5a5a } else { 0xffd35a });
        } else {
            let (lo, hi) = Self::zone(hard);
            p.text(20.0, 60.0, "Давление", "Pressure", 18.0, 0xffffff);
            p.rect(20.0, 90.0, 30.0, 280.0, 0x2a3048);
            p.rect(20.0, 90.0 + 280.0 * (1.0 - hi), 30.0, 280.0 * (hi - lo), 0x2fbf6a);
            p.rect(14.0, 90.0 + 280.0 * (1.0 - self.pressure) - 3.0, 42.0, 6.0, 0xffffff);
            p.button(PUMP, "Наддув", "Pump", if self.pumping { 0xff8a3d } else { 0xffd35a }, self.pumping);
        }
    }
}

// ---------------------------------------------------------------- centrifuge

const CEN: (f32, f32) = (430.0, 220.0);
const SPIN: Btn = Btn(14.0, 360.0, 200.0, 56.0);

/// Put the sample tubes into the centrifuge so it's balanced: equal masses opposite each other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Centrifuge {
    pub slots: usize,
    pub tubes: Vec<Piece>,
    /// slot -> tube
    pub slot_of: Vec<Option<usize>>,
    pub held: Option<usize>,
    pub need: usize,
    pub shake: f32,
}
fn slot_pos(n: usize, k: usize) -> (f32, f32) {
    let a = k as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    (CEN.0 + 130.0 * a.cos(), CEN.1 + 130.0 * a.sin())
}
impl Centrifuge {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let (slots, mut masses): (usize, Vec<u8>) = if hard { (8, vec![1, 1, 2, 2, 3, 3, 4]) } else { (6, vec![1, 1, 2, 2, 3, 3]) };
        shuffle(&mut masses, rng);
        let tubes = masses.iter().enumerate().map(|(i, m)| Piece::new(40.0 + (i % 4) as f32 * 52.0, 80.0 + (i / 4) as f32 * 120.0, *m)).collect();
        Centrifuge { slots, tubes, slot_of: vec![None; slots], held: None, need: 6, shake: 0.0 }
    }
    fn balanced(&self) -> bool {
        let (mut sx, mut sy) = (0.0f32, 0.0f32);
        for (k, t) in self.slot_of.iter().enumerate() {
            if let Some(t) = t {
                let a = k as f32 / self.slots as f32 * std::f32::consts::TAU;
                let m = self.tubes[*t].kind as f32;
                sx += m * a.cos();
                sy += m * a.sin();
            }
        }
        sx.hypot(sy) < 0.05
    }
    fn placed(&self) -> usize {
        self.slot_of.iter().filter(|s| s.is_some()).count()
    }
}
impl Play for Centrifuge {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                if SPIN.hit(x, y) {
                    return if self.placed() >= self.need && self.balanced() {
                        Outcome::Done
                    } else {
                        self.shake = 1.0;
                        Outcome::Mistake
                    };
                }
                self.held = self.tubes.iter().enumerate().filter(|(_, t)| dist((x, y), (t.x, t.y)) < 30.0).map(|(i, _)| i).next();
                if let Some(t) = self.held {
                    for s in self.slot_of.iter_mut() {
                        if *s == Some(t) {
                            *s = None;
                        }
                    }
                    self.tubes[t].placed = false;
                }
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if let Some(t) = self.held {
                    (self.tubes[t].x, self.tubes[t].y) = (x, y);
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                let Some(t) = self.held.take() else { return Outcome::Fine };
                match (0..self.slots).find(|k| self.slot_of[*k].is_none() && dist((x, y), slot_pos(self.slots, *k)) < 32.0) {
                    Some(k) => {
                        self.slot_of[k] = Some(t);
                        self.tubes[t].placed = true;
                        (self.tubes[t].x, self.tubes[t].y) = slot_pos(self.slots, k);
                    }
                    None => self.tubes[t].home(),
                }
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        self.shake = (self.shake - dt).max(0.0);
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.placed() as f32 / self.need as f32 * 0.9
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if self.placed() >= self.need {
            click(&mut o, SPIN.mid());
            return o;
        }
        // pair equal masses opposite each other
        let n = self.slots;
        let free_tubes: Vec<usize> = (0..self.tubes.len()).filter(|t| !self.tubes[*t].placed).collect();
        for &a in &free_tubes {
            if let Some(&b) = free_tubes.iter().find(|b| **b != a && self.tubes[**b].kind == self.tubes[a].kind) {
                if let Some(k) = (0..n / 2).find(|k| self.slot_of[*k].is_none() && self.slot_of[k + n / 2].is_none()) {
                    drag_to(&mut o, (self.tubes[a].x, self.tubes[a].y), slot_pos(n, k));
                    drag_to(&mut o, (self.tubes[b].x, self.tubes[b].y), slot_pos(n, k + n / 2));
                    return o;
                }
            }
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint(
            if hard { "Расставь 6 пробирок так, чтобы центрифуга не шаталась. Одна пробирка — лишняя" } else { "Расставь все пробирки так, чтобы центрифуга не шаталась" },
            if hard { "Place 6 tubes so the centrifuge doesn't wobble. One tube is extra" } else { "Place all tubes so the centrifuge doesn't wobble" },
        );
        let sh = (self.shake * 40.0).sin() * 8.0 * self.shake;
        p.circle(CEN.0 + sh, CEN.1, 165.0, 0xc8d0dc);
        p.circle(CEN.0 + sh, CEN.1, 150.0, 0xe8eef4);
        p.circle(CEN.0 + sh, CEN.1, 20.0, 0x5a6178);
        for k in 0..self.slots {
            let s = slot_pos(self.slots, k);
            p.ring(s.0 + sh, s.1, 24.0, 0x5a6178, 3.0);
        }
        for t in &self.tubes {
            let c = [0, 0x7ad0ff, 0x5aff8a, 0xffd35a, 0xff6a8a][t.kind as usize];
            p.rect(t.x - 14.0, t.y - 26.0, 28.0, 52.0, c);
            p.raw(t.x, t.y - 12.0, format!("{}г", t.kind), 16.0, 0x0b0e22, true);
        }
        let b = self.balanced() && self.placed() > 0;
        p.text(14.0, 300.0, if b { "Равновесие (ok)" } else { "Перекос!" }, if b { "Balanced (ok)" } else { "Off balance!" }, 20.0, if b { 0x5aff8a } else { 0xff8a6a });
        p.button(SPIN, "Запустить", "Spin", 0xffd35a, false);
    }
}

// ---------------------------------------------------------------- antenna

const DISH: (f32, f32) = (360.0, 330.0);

/// Turn the dish (drag around it) and keep it pointed at Earth while it moves across the sky.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Antenna {
    pub a: f32,
    pub t: f32,
    pub lock: f32,
    pub held: bool,
    pub base: f32,
}
impl Antenna {
    pub fn new(_hard: bool, rng: &mut Rng) -> Self {
        Antenna { a: -0.3, t: rng.range(0.0, 6.0), lock: 0.0, held: false, base: -std::f32::consts::FRAC_PI_2 }
    }
    fn earth(&self, hard: bool) -> f32 {
        self.base + (self.t * if hard { 0.5 } else { 0.3 }).sin() * 1.0
    }
    fn signal(&self, hard: bool) -> f32 {
        (self.a - self.earth(hard)).cos().max(0.0)
    }
    fn need(hard: bool) -> f32 {
        if hard {
            5.0
        } else {
            3.0
        }
    }
}
impl Play for Antenna {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.held = dist((x, y), DISH) < 200.0;
                if self.held {
                    self.a = (y - DISH.1).atan2(x - DISH.0);
                }
                Outcome::Fine
            }
            MiniInput::Drag(x, y) if self.held => {
                self.a = (y - DISH.1).atan2(x - DISH.0);
                Outcome::Fine
            }
            MiniInput::Release(..) => {
                self.held = false;
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, hard: bool) -> Outcome {
        self.t += dt;
        let need = if hard { 0.985 } else { 0.97 };
        if self.signal(hard) > need {
            self.lock += dt;
        } else {
            self.lock = (self.lock - dt * 0.5).max(0.0);
        }
        if self.lock >= Self::need(hard) {
            Outcome::Done
        } else {
            Outcome::Fine
        }
    }
    fn progress(&self) -> f32 {
        self.lock / 5.0
    }
    fn bot(&self, hard: bool) -> Vec<MiniInput> {
        let e = self.earth(hard) + (if hard { 0.5 } else { 0.3 }) * (self.t * if hard { 0.5 } else { 0.3 }).cos() * 0.05;
        let to = (DISH.0 + 150.0 * e.cos(), DISH.1 + 150.0 * e.sin());
        vec![MiniInput::Press(to.0, to.1), MiniInput::Wait(0.05)]
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint("Крути антенну (тяни вокруг неё) и держи её на Земле, пока идёт звонок", "Turn the dish (drag around it) and keep it on Earth during the call");
        let e = self.earth(hard);
        p.icon("earth", DISH.0 + 270.0 * e.cos(), DISH.1 + 270.0 * e.sin(), 60.0);
        for i in 1..12 {
            let r = i as f32 * 20.0;
            p.circle(DISH.0 + r * self.a.cos(), DISH.1 + r * self.a.sin(), 3.0, 0xffd35a);
        }
        p.circle(DISH.0, DISH.1, 40.0, 0xb8c0d4);
        p.circle(DISH.0 + 26.0 * self.a.cos(), DISH.1 + 26.0 * self.a.sin(), 16.0, 0xdfe5f2);
        let s = self.signal(hard);
        p.text(14.0, 50.0, "Сигнал", "Signal", 18.0, 0xffffff);
        for b in 0..5 {
            let on = s > 0.8 + b as f32 * 0.045;
            p.rect(14.0 + b as f32 * 22.0, 110.0 - b as f32 * 10.0, 16.0, 14.0 + b as f32 * 10.0, if on { 0x5aff8a } else { 0x2a3048 });
        }
        p.text(14.0, 140.0, "Связь держится", "Call connected", 16.0, 0xffffff);
        p.rect(14.0, 166.0, 160.0, 14.0, 0x2a3048);
        p.rect(14.0, 166.0, 160.0 * (self.lock / Self::need(hard)).min(1.0), 14.0, 0x5aff8a);
        if self.lock > 1.0 {
            p.text(14.0, 200.0, "Мама: «Мы так тобой гордимся!»", "Mom: “We're so proud of you!”", 17.0, 0xffd35a);
        }
    }
}
