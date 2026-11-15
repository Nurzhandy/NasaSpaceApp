//! Launch day games: Go/No-Go poll, wires, the engine burn, the switch panel, keeping balance,
//! cleaning water, exercise rhythm, lunch.

use super::*;

// ---------------------------------------------------------------- Go / No-Go

const STATIONS: [(&str, &str); 7] = [("Погода", "Weather"), ("Топливо", "Fuel"), ("Связь", "Comms"), ("Двигатель", "Engines"), ("Экипаж", "Crew"), ("Траектория", "Trajectory"), ("Медицина", "Medical")];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub value: f32,
    pub lo: f32,
    pub hi: f32,
    /// Some(true) = said GO (or fixed), Some(false) = said NO-GO and must be fixed
    pub answer: Option<bool>,
}
impl Station {
    fn ok(&self) -> bool {
        self.value >= self.lo && self.value <= self.hi
    }
}

/// Each flight controller reports; say GO only if the needle is in the green. Then fix the NO-GOs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoNoGo {
    pub st: Vec<Station>,
    pub held: Option<usize>,
}
const GX: f32 = 170.0;
const GW: f32 = 300.0;
fn gy(i: usize) -> f32 {
    52.0 + i as f32 * 52.0
}
fn go_btn(i: usize) -> Btn {
    Btn(500.0, gy(i) - 8.0, 86.0, 40.0)
}
fn nogo_btn(i: usize) -> Btn {
    Btn(600.0, gy(i) - 8.0, 106.0, 40.0)
}

impl GoNoGo {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 7 } else { 5 };
        let bad = if hard { 2 + rng.below(2) } else { 1 + rng.below(2) };
        let mut idx: Vec<usize> = (0..n).collect();
        shuffle(&mut idx, rng);
        let st = (0..n)
            .map(|i| {
                let w = if hard { 0.18 } else { 0.3 };
                let lo = rng.range(0.15, 0.85 - w);
                let hi = lo + w;
                let value = if idx[..bad].contains(&i) { if rng.chance(0.5) && lo > 0.12 { rng.range(0.02, lo - 0.04) } else { rng.range((hi + 0.04).min(0.97), 0.98) } } else { rng.range(lo + 0.02, hi - 0.02) };
                Station { value, lo, hi, answer: None }
            })
            .collect();
        GoNoGo { st, held: None }
    }
    fn fixing(&self) -> bool {
        self.st.iter().all(|s| s.answer.is_some())
    }
}

impl Play for GoNoGo {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) if !self.fixing() => {
                for k in 0..self.st.len() {
                    if self.st[k].answer.is_some() {
                        continue;
                    }
                    let said_go = if go_btn(k).hit(x, y) {
                        true
                    } else if nogo_btn(k).hit(x, y) {
                        false
                    } else {
                        continue;
                    };
                    if said_go != self.st[k].ok() {
                        return Outcome::Mistake;
                    }
                    self.st[k].answer = Some(said_go);
                    if self.st.iter().all(|s| s.answer == Some(true)) {
                        return Outcome::Done;
                    }
                    return Outcome::Fine;
                }
                Outcome::Fine
            }
            MiniInput::Press(x, y) => {
                self.held = (0..self.st.len()).find(|k| self.st[*k].answer == Some(false) && (x - (GX + self.st[*k].value * GW)).abs() < 24.0 && (y - (gy(*k) + 8.0)).abs() < 22.0);
                Outcome::Fine
            }
            MiniInput::Drag(x, _) => {
                if let Some(k) = self.held {
                    self.st[k].value = ((x - GX) / GW).clamp(0.0, 1.0);
                }
                Outcome::Fine
            }
            MiniInput::Release(..) => {
                if let Some(k) = self.held.take() {
                    if self.st[k].ok() {
                        self.st[k].answer = Some(true);
                    }
                }
                if self.fixing() && self.st.iter().all(|s| s.answer == Some(true)) {
                    Outcome::Done
                } else {
                    Outcome::Fine
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        self.st.iter().filter(|s| s.answer == Some(true)).count() as f32 / self.st.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if !self.fixing() {
            if let Some(k) = self.st.iter().position(|s| s.answer.is_none()) {
                click(&mut o, if self.st[k].ok() { go_btn(k).mid() } else { nogo_btn(k).mid() });
            }
        } else if let Some(k) = self.st.iter().position(|s| s.answer == Some(false)) {
            let s = &self.st[k];
            drag_to(&mut o, (GX + s.value * GW, gy(k) + 8.0), (GX + (s.lo + s.hi) / 2.0 * GW, gy(k) + 8.0));
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        if self.fixing() {
            p.hint("Перетащи стрелки служб NO-GO в зелёную зону — тогда старт разрешён", "Drag the needles of the NO-GO stations into the green — then launch is GO");
        } else {
            p.hint("Стрелка в зелёной зоне — жми GO. Если нет — NO-GO!", "Needle in the green — press GO. If not — NO-GO!");
        }
        for (k, s) in self.st.iter().enumerate() {
            let y = gy(k);
            p.text(14.0, y - 4.0, STATIONS[k].0, STATIONS[k].1, 19.0, 0xffffff);
            p.gauge(GX, y, GW, s.value, s.lo, s.hi);
            match s.answer {
                None => {
                    p.button(go_btn(k), "GO", "GO", 0x2fbf6a, false);
                    p.button(nogo_btn(k), "NO-GO", "NO-GO", 0xff6a5a, false);
                }
                Some(true) => p.text(520.0, y - 4.0, "GO (ok)", "GO (ok)", 22.0, 0x5aff8a),
                Some(false) => {
                    p.text(520.0, y - 4.0, "NO-GO — тяни!", "NO-GO — drag!", 18.0, 0xff8a6a);
                    p.ring(GX + s.value * GW, y + 8.0, 16.0, 0xffd35a, 3.0);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- wires

/// Drag each wire from the left plug to the right socket with the same colour (and number).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wires {
    /// (colour, number) of left wire i
    pub left: Vec<(u8, u8)>,
    /// right socket j takes left wire `right[j]`
    pub right: Vec<usize>,
    pub links: Vec<Option<usize>>,
    pub held: Option<usize>,
    pub cur: (f32, f32),
}
const WIRE_C: [u32; 5] = [0xff4a4a, 0x3a8bff, 0xffd33a, 0x3adf6a, 0xc06aff];
fn wy(n: usize, i: usize) -> f32 {
    70.0 + i as f32 * (330.0 / (n - 1) as f32)
}
impl Wires {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let left: Vec<(u8, u8)> = if hard { (0..6).map(|i| ((i % 3) as u8, (i / 3) as u8 + 1)).collect() } else { (0..4).map(|i| (i as u8, 0)).collect() };
        let mut right: Vec<usize> = (0..left.len()).collect();
        shuffle(&mut right, rng);
        let mut left = left;
        shuffle(&mut left, rng);
        Wires { links: vec![None; left.len()], left, right, held: None, cur: (0.0, 0.0) }
    }
}
impl Play for Wires {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        let n = self.left.len();
        match i {
            MiniInput::Press(x, y) => {
                self.held = (0..n).find(|k| self.links[*k].is_none() && dist((x, y), (90.0, wy(n, *k))) < 30.0);
                self.cur = (x, y);
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                self.cur = (x, y);
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                let Some(a) = self.held.take() else { return Outcome::Fine };
                let Some(j) = (0..n).find(|j| dist((x, y), (630.0, wy(n, *j))) < 34.0) else { return Outcome::Fine };
                let b = self.right[j];
                if self.left[b] == self.left[a] && !self.links.contains(&Some(j)) {
                    self.links[a] = Some(j);
                    if self.links.iter().all(|l| l.is_some()) {
                        return Outcome::Done;
                    }
                    Outcome::Fine
                } else {
                    Outcome::Mistake
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        self.links.iter().filter(|l| l.is_some()).count() as f32 / self.links.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let n = self.left.len();
        let mut o = vec![];
        if let Some(a) = (0..n).find(|k| self.links[*k].is_none()) {
            let j = (0..n).find(|j| self.left[self.right[*j]] == self.left[a] && !self.links.contains(&Some(*j))).unwrap_or(0);
            drag_to(&mut o, (90.0, wy(n, a)), (630.0, wy(n, j)));
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint(
            if hard { "Тяни провод к гнезду того же цвета И с той же цифрой — замкни цепь" } else { "Тяни каждый провод к гнезду того же цвета — замкни цепь" },
            if hard { "Drag each wire to the socket of the same colour AND number — close the circuit" } else { "Drag each wire to the socket of the same colour — close the circuit" },
        );
        let n = self.left.len();
        p.rect(40.0, 40.0, 100.0, 380.0, 0x252b45);
        p.rect(580.0, 40.0, 100.0, 380.0, 0x252b45);
        p.ctext(360.0, 210.0, "блок питания", "power unit", 16.0, 0x6a7090);
        for (a, l) in self.links.iter().enumerate() {
            if let Some(j) = l {
                p.line(90.0, wy(n, a), 630.0, wy(n, *j), 9.0, WIRE_C[self.left[a].0 as usize]);
            }
        }
        if let Some(a) = self.held {
            p.line(90.0, wy(n, a), self.cur.0, self.cur.1, 9.0, WIRE_C[self.left[a].0 as usize]);
        }
        for k in 0..n {
            let (c, num) = self.left[k];
            p.circle(90.0, wy(n, k), 20.0, WIRE_C[c as usize]);
            let (c2, num2) = self.left[self.right[k]];
            p.rect(612.0, wy(n, k) - 18.0, 36.0, 36.0, WIRE_C[c2 as usize]);
            if num > 0 {
                p.raw(90.0, wy(n, k) - 12.0, num.to_string(), 20.0, 0x0b0e22, true);
                p.raw(630.0, wy(n, k) - 12.0, num2.to_string(), 20.0, 0x0b0e22, true);
            }
        }
        let lit = self.links.iter().all(|l| l.is_some());
        p.circle(360.0, 260.0, 18.0, if lit { 0x5aff8a } else { 0x553333 });
    }
}

// ---------------------------------------------------------------- the burn

/// Hold the engine button: speed grows faster and faster as fuel burns (a = F / m). Let go in the green.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Burn {
    pub v: f32,
    pub fuel: f32,
    pub firing: bool,
    pub lo: f32,
    pub hi: f32,
}
const DRY: f32 = 25.0;
const FUEL0: f32 = 28.0;
const V0: f32 = 7.8;
const THRUST: f32 = 300.0; // kN
const BURN_BTN: Btn = Btn(250.0, 330.0, 220.0, 80.0);
impl Burn {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let mid = rng.range(10.75, 10.95);
        let w = if hard { 0.07 } else { 0.18 };
        Burn { v: V0, fuel: FUEL0, firing: false, lo: mid - w, hi: mid + w }
    }
    fn accel(&self) -> f32 {
        THRUST / (DRY + self.fuel) // m/s² (kN / t)
    }
    fn stop(&mut self) -> Outcome {
        self.firing = false;
        if self.v >= self.lo && self.v <= self.hi {
            Outcome::Done
        } else if self.v > self.hi {
            self.v = V0;
            self.fuel = FUEL0;
            Outcome::Mistake
        } else {
            Outcome::Fine
        }
    }
}
impl Play for Burn {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) if BURN_BTN.hit(x, y) => {
                self.firing = true;
                Outcome::Fine
            }
            MiniInput::Space(true) => {
                self.firing = true;
                Outcome::Fine
            }
            MiniInput::Release(..) | MiniInput::Space(false) if self.firing => self.stop(),
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        if !self.firing {
            return Outcome::Fine;
        }
        // one real second = one minute of burn
        let a = self.accel();
        self.v += a * dt * 60.0 / 1000.0;
        self.fuel -= THRUST / (450.0 * 9.81) * dt * 60.0;
        if self.v > self.hi + 0.25 || self.fuel <= 0.0 {
            self.fuel = self.fuel.max(0.0);
            self.firing = false;
            self.v = V0;
            self.fuel = FUEL0;
            return Outcome::Mistake;
        }
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        (self.v - V0) / (self.lo - V0)
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mid = (self.lo + self.hi) / 2.0;
        if !self.firing {
            vec![MiniInput::Press(BURN_BTN.mid().0, BURN_BTN.mid().1)]
        } else if self.v >= mid - 0.004 {
            vec![MiniInput::Release(0.0, 0.0)]
        } else {
            let dv_per_s = self.accel() * 60.0 / 1000.0;
            vec![MiniInput::Wait(((mid - self.v) / dv_per_s * 0.6).clamp(0.005, 0.05))]
        }
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Держи кнопку (или Пробел): двигатель разгоняет корабль. Отпусти, когда скорость в зелёной зоне", "Hold the button (or Space) to fire. Let go when the speed is in the green");
        let f = |v: f32| (v - 7.5) / 4.0;
        p.text(30.0, 50.0, "Скорость, км/с", "Speed, km/s", 20.0, 0xffffff);
        p.gauge(30.0, 86.0, 660.0, f(self.v), f(self.lo), f(self.hi));
        p.raw(30.0, 112.0, format!("{:.2}", self.v), 34.0, 0xffd35a, false);
        p.text(30.0, 170.0, "Топливо", "Fuel", 18.0, 0xffffff);
        p.rect(130.0, 172.0, 300.0, 18.0, 0x2a3048);
        p.rect(130.0, 172.0, 300.0 * self.fuel / FUEL0, 18.0, 0xff8a3d);
        let m = DRY + self.fuel;
        p.raw(30.0, 212.0, format!("m = {m:.1} т   F = {THRUST:.0} кН   a = F / m = {:.1} м/с²", self.accel()), 20.0, 0x9fe8ff, false);
        p.text(30.0, 246.0, "Топливо сгорает -> масса меньше -> ускорение больше!", "Fuel burns -> less mass -> more acceleration!", 17.0, 0xb0b8d8);
        p.icon("rocket", 600.0, 230.0, 90.0);
        if self.firing {
            p.icon("flash", 600.0, 300.0, 50.0);
        }
        p.button(BURN_BTN, if self.firing { "ГОРИТ!" } else { "ДВИГАТЕЛЬ" }, if self.firing { "FIRING!" } else { "ENGINE" }, if self.firing { 0xff6a3d } else { 0xffd35a }, self.firing);
    }
}

// ---------------------------------------------------------------- switch panel (Apollo 12)

const SW_NAMES: [&str; 16] = ["SCE", "FC 1", "FC 2", "FC 3", "MAIN A", "MAIN B", "AUX", "BAT A", "BAT B", "INV 1", "INV 2", "CMC", "SPS", "RCS", "TLM", "G&N"];

/// Lightning hit: follow the checklist from Mission Control, switch by switch, in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Switches {
    pub on: Vec<bool>,
    pub steps: Vec<usize>,
    pub next: usize,
    pub t: f32,
}
fn sw_btn(n: usize, k: usize) -> Btn {
    let cols = 4;
    let rows = n / cols;
    let h = 300.0 / rows as f32;
    Btn(300.0 + (k % cols) as f32 * 102.0, 70.0 + (k / cols) as f32 * h, 92.0, h - 10.0)
}
impl Switches {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 16 } else { 12 };
        let on: Vec<bool> = (0..n).map(|_| rng.chance(0.5)).collect();
        let mut rest: Vec<usize> = (1..n).collect();
        shuffle(&mut rest, rng);
        let mut steps = vec![0];
        steps.extend(rest.into_iter().take(if hard { 4 } else { 2 }));
        Switches { on, steps, next: 0, t: 0.0 }
    }
}
impl Play for Switches {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        let MiniInput::Press(x, y) = i else { return Outcome::Fine };
        let n = self.on.len();
        let Some(k) = (0..n).find(|k| sw_btn(n, *k).hit(x, y)) else { return Outcome::Fine };
        if k != self.steps[self.next] {
            return Outcome::Mistake;
        }
        self.on[k] = !self.on[k];
        self.next += 1;
        if self.next >= self.steps.len() {
            Outcome::Done
        } else {
            Outcome::Fine
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        self.t += dt;
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.next as f32 / self.steps.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        click(&mut o, sw_btn(self.on.len(), self.steps[self.next]).mid());
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Читай список ЦУП и щёлкай переключатели строго по порядку", "Read Mission Control's list and flip the switches strictly in order");
        p.rect(14.0, 44.0, 270.0, 370.0, 0xf0ead8);
        p.text(26.0, 54.0, "ЦУП -> экипажу:", "Mission Control:", 18.0, 0x333333);
        for (i, k) in self.steps.iter().enumerate() {
            // the switch must end up the other way from how it was found
            let target = if i < self.next { self.on[*k] } else { !self.on[*k] };
            let what = if i == 0 { "AUX" } else if target { p.l.pick("ВКЛ", "ON") } else { p.l.pick("ВЫКЛ", "OFF") };
            let s = format!("{}. {} -> {}", i + 1, SW_NAMES[*k], what);
            let c = if i < self.next { 0x2a8a4a } else if i == self.next { 0xc03030 } else { 0x333333 };
            p.raw(26.0, 92.0 + i as f32 * 40.0, if i < self.next { format!("(ok) {s}") } else { s }, 20.0, c, false);
        }
        let n = self.on.len();
        for k in 0..n {
            let b = sw_btn(n, k);
            p.rect(b.0, b.1, b.2, b.3, 0x2a3048);
            p.raw(b.0 + b.2 / 2.0, b.1 + 4.0, SW_NAMES[k].to_string(), 15.0, 0xdfe5f2, true);
            let up = self.on[k];
            p.rect(b.0 + b.2 / 2.0 - 7.0, b.1 + if up { 24.0 } else { b.3 / 2.0 + 4.0 }, 14.0, b.3 / 2.0 - 20.0, if up { 0x5aff8a } else { 0x8a90a8 });
        }
        let blink = (self.t * 4.0).sin() > 0.0;
        for i in 0..(self.steps.len() - self.next) {
            p.circle(310.0 + i as f32 * 26.0, 52.0, 9.0, if blink { 0xff4a4a } else { 0x702020 });
        }
    }
}

// ---------------------------------------------------------------- balance (space sickness)

/// The balance ball drifts: keep the ring on it until the brain gets used to weightlessness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Steady {
    pub dot: (f32, f32),
    pub vel: (f32, f32),
    pub ring: (f32, f32),
    pub calm: f32,
    pub seed: u64,
    pub kick: f32,
}
fn lcg(s: &mut u64) -> f32 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*s >> 40) as f32 / (1u64 << 24) as f32
}
impl Steady {
    pub fn new(_hard: bool, rng: &mut Rng) -> Self {
        Steady { dot: (360.0, 230.0), vel: (0.0, 0.0), ring: (360.0, 380.0), calm: 0.0, seed: rng.next(), kick: 0.0 }
    }
    fn radius(hard: bool) -> f32 {
        if hard {
            30.0
        } else {
            42.0
        }
    }
}
impl Play for Steady {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        if let MiniInput::Press(x, y) | MiniInput::Drag(x, y) = i {
            self.ring = (x, y);
        }
        Outcome::Fine
    }
    fn tick(&mut self, dt: f32, hard: bool) -> Outcome {
        let sp = if hard { 150.0 } else { 90.0 };
        self.kick -= dt;
        if self.kick <= 0.0 {
            self.kick = 0.6;
            self.vel = ((lcg(&mut self.seed) - 0.5) * 2.0 * sp, (lcg(&mut self.seed) - 0.5) * 2.0 * sp);
        }
        self.dot.0 += self.vel.0 * dt;
        self.dot.1 += self.vel.1 * dt;
        if !(120.0..=600.0).contains(&self.dot.0) {
            self.vel.0 = -self.vel.0;
            self.dot.0 = self.dot.0.clamp(120.0, 600.0);
        }
        if !(80.0..=380.0).contains(&self.dot.1) {
            self.vel.1 = -self.vel.1;
            self.dot.1 = self.dot.1.clamp(80.0, 380.0);
        }
        if dist(self.ring, self.dot) < Self::radius(hard) {
            self.calm += dt / if hard { 9.0 } else { 6.0 };
        } else {
            self.calm = (self.calm - dt / 10.0).max(0.0);
        }
        if self.calm >= 1.0 {
            Outcome::Done
        } else {
            Outcome::Fine
        }
    }
    fn progress(&self) -> f32 {
        self.calm
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let ahead = (self.dot.0 + self.vel.0 * 0.03, self.dot.1 + self.vel.1 * 0.03);
        vec![MiniInput::Drag(ahead.0, ahead.1), MiniInput::Wait(0.05)]
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint("Держи кольцо на шарике равновесия — так мозг привыкает к невесомости", "Keep the ring on the balance ball — that's how the brain gets used to weightlessness");
        p.icon("astronaut", 360.0, 230.0, 260.0);
        p.circle_a(360.0, 230.0, 200.0, 0x000000, 0.25);
        p.circle(self.dot.0, self.dot.1, 14.0, 0xff8a3d);
        let inside = dist(self.ring, self.dot) < Self::radius(hard);
        p.ring(self.ring.0, self.ring.1, Self::radius(hard), if inside { 0x5aff8a } else { 0xffffff }, 5.0);
        p.text(20.0, 395.0, "Спокойствие", "Calm", 18.0, 0xffffff);
        p.rect(140.0, 398.0, 400.0, 16.0, 0x2a3048);
        p.rect(140.0, 398.0, 400.0 * self.calm.min(1.0), 16.0, 0x5aff8a);
    }
}

// ---------------------------------------------------------------- water recycling

const JUNK: [(&str, &str, u32); 4] = [("грязь", "dirt", 0x8a5a3a), ("волос", "hair", 0x2a2a2a), ("пластик", "plastic", 0xd0d4e0), ("водоросль", "algae", 0x3a9a4a)];
const TANK: Btn = Btn(40.0, 50.0, 470.0, 360.0);
const BIN: Btn = Btn(550.0, 50.0, 150.0, 160.0);
const CLEAN: Btn = Btn(550.0, 240.0, 150.0, 170.0);

/// Floating stuff in the water tank: drag rubbish to the bin and clean drops to the clean tank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recycle {
    pub items: Vec<Piece>,
    pub vel: Vec<(f32, f32)>,
    pub held: Option<usize>,
}
impl Recycle {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 12 } else { 8 };
        let sp = if hard { 40.0 } else { 18.0 };
        let mut items = vec![];
        let mut vel = vec![];
        for i in 0..n {
            // kind 0 = clean water, 1.. = rubbish
            let kind = if i % 3 == 0 { 0 } else { 1 + rng.below(4) as u8 };
            items.push(Piece::new(rng.range(80.0, 470.0), rng.range(90.0, 380.0), kind));
            vel.push((rng.range(-sp, sp), rng.range(-sp, sp)));
        }
        Recycle { items, vel, held: None }
    }
}
impl Play for Recycle {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.held = grab(&self.items, x, y, 30.0);
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if let Some(k) = self.held {
                    self.items[k].x = x;
                    self.items[k].y = y;
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                let Some(k) = self.held.take() else { return Outcome::Fine };
                let water = self.items[k].kind == 0;
                let r = if BIN.hit(x, y) {
                    if water {
                        Outcome::Mistake // clean water thrown away!
                    } else {
                        self.items[k].placed = true;
                        Outcome::Fine
                    }
                } else if CLEAN.hit(x, y) {
                    if water {
                        self.items[k].placed = true;
                        Outcome::Fine
                    } else {
                        Outcome::Mistake // rubbish in the drinking water!
                    }
                } else {
                    Outcome::Fine
                };
                if r == Outcome::Mistake || !TANK.hit(self.items[k].x, self.items[k].y) && !self.items[k].placed {
                    self.items[k].x = 280.0;
                    self.items[k].y = 230.0;
                }
                if self.items.iter().all(|p| p.placed) {
                    Outcome::Done
                } else {
                    r
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        for (k, p) in self.items.iter_mut().enumerate() {
            if p.placed || self.held == Some(k) {
                continue;
            }
            p.x += self.vel[k].0 * dt;
            p.y += self.vel[k].1 * dt;
            if p.x < 70.0 || p.x > 480.0 {
                self.vel[k].0 = -self.vel[k].0;
                p.x = p.x.clamp(70.0, 480.0);
            }
            if p.y < 80.0 || p.y > 385.0 {
                self.vel[k].1 = -self.vel[k].1;
                p.y = p.y.clamp(80.0, 385.0);
            }
        }
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.items.iter().filter(|p| p.placed).count() as f32 / self.items.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if let Some(k) = self.items.iter().position(|p| !p.placed) {
            let p = self.items[k];
            drag_to(&mut o, (p.x, p.y), if p.kind == 0 { CLEAN.mid() } else { BIN.mid() });
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Мусор тащи в ведро для отходов, чистые капли — в бак чистой воды", "Drag rubbish into the waste bin and clean drops into the clean-water tank");
        p.rect(TANK.0, TANK.1, TANK.2, TANK.3, 0x1f6f9a);
        p.rect_a(TANK.0 + 6.0, TANK.1 + 6.0, TANK.2 - 12.0, TANK.3 - 12.0, 0x5ac8ff, 0.35);
        p.rect(BIN.0, BIN.1, BIN.2, BIN.3, 0x5a5048);
        p.icon("broom", BIN.0 + 75.0, BIN.1 + 70.0, 60.0);
        p.ctext(BIN.0 + 75.0, BIN.1 + 120.0, "Отходы", "Waste", 18.0, 0xffffff);
        p.rect(CLEAN.0, CLEAN.1, CLEAN.2, CLEAN.3, 0x2a8ad8);
        p.icon("droplet", CLEAN.0 + 75.0, CLEAN.1 + 70.0, 60.0);
        p.ctext(CLEAN.0 + 75.0, CLEAN.1 + 125.0, "Чистая вода", "Clean water", 18.0, 0xffffff);
        for it in self.items.iter().filter(|p| !p.placed) {
            if it.kind == 0 {
                p.icon("droplet", it.x, it.y, 40.0);
            } else {
                let (ru, en, c) = JUNK[(it.kind - 1) as usize];
                p.circle(it.x, it.y, 17.0, c);
                p.ctext(it.x, it.y + 16.0, ru, en, 13.0, 0xffffff);
            }
        }
    }
}

// ---------------------------------------------------------------- exercise rhythm

/// Treadmill steps to the beat: press when a note reaches the line. Bones and muscles need load.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rhythm {
    pub beats: Vec<f32>,
    pub hit: Vec<bool>,
    pub t: f32,
    pub hits: u32,
    pub need: u32,
}
const HIT_X: f32 = 140.0;
const NOTE_SPEED: f32 = 220.0;
impl Rhythm {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let need = if hard { 16 } else { 10 };
        let gap = if hard { 0.55 } else { 0.85 };
        let mut beats = vec![];
        let mut t = 2.0;
        for _ in 0..need + 2 {
            beats.push(t);
            t += gap * if hard && rng.chance(0.3) { 0.5 } else { 1.0 };
        }
        Rhythm { hit: vec![false; beats.len()], beats, t: 0.0, hits: 0, need }
    }
    fn window(hard: bool) -> f32 {
        if hard {
            0.13
        } else {
            0.2
        }
    }
    fn step(&mut self, hard: bool) -> Outcome {
        let w = Self::window(hard);
        let near = (0..self.beats.len()).find(|k| !self.hit[*k] && (self.beats[*k] - self.t).abs() < w);
        match near {
            Some(k) => {
                self.hit[k] = true;
                self.hits += 1;
                if self.hits >= self.need {
                    Outcome::Done
                } else {
                    Outcome::Fine
                }
            }
            None => Outcome::Mistake,
        }
    }
}
impl Play for Rhythm {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        match i {
            MiniInput::Press(..) | MiniInput::Space(true) => self.step(hard),
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, hard: bool) -> Outcome {
        self.t += dt;
        let w = Self::window(hard);
        let mut r = Outcome::Fine;
        for k in 0..self.beats.len() {
            if !self.hit[k] && self.t > self.beats[k] + w {
                // missed: that note is gone, a new one comes at the end
                self.hit[k] = true;
                let last = *self.beats.last().unwrap_or(&self.t);
                self.beats.push(last.max(self.t) + 0.9);
                self.hit.push(false);
                r = Outcome::Mistake;
            }
        }
        r
    }
    fn progress(&self) -> f32 {
        self.hits as f32 / self.need as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let Some(k) = (0..self.beats.len()).find(|k| !self.hit[*k]) else { return vec![MiniInput::Wait(0.05)] };
        let d = self.beats[k] - self.t;
        if d.abs() < 0.03 {
            vec![MiniInput::Press(360.0, 300.0), MiniInput::Release(360.0, 300.0)]
        } else {
            vec![MiniInput::Wait(d.clamp(0.005, 0.05))]
        }
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint("Жми (или Пробел), когда кружок на линии — шаг в ритм", "Press (or Space) when a circle is on the line — step to the beat");
        p.rect(20.0, 120.0, 680.0, 90.0, 0x22283f);
        p.rect(HIT_X - 4.0, 110.0, 8.0, 110.0, 0xffd35a);
        p.ring(HIT_X, 165.0, 40.0 * Self::window(hard) / 0.2, 0xffd35a, 3.0);
        for k in 0..self.beats.len() {
            if self.hit[k] {
                continue;
            }
            let x = HIT_X + (self.beats[k] - self.t) * NOTE_SPEED;
            if (20.0..700.0).contains(&x) {
                p.circle(x, 165.0, 22.0, 0x5ac8ff);
            }
        }
        p.icon("run", 360.0, 300.0, 110.0);
        p.raw(500.0, 260.0, format!("{} / {}", self.hits, self.need), 34.0, 0x5aff8a, false);
        p.icon("heart", 515.0, 326.0, 30.0);
        p.raw(540.0, 310.0, format!("{}", 90 + self.hits * 4), 26.0, 0xff6a8a, false);
        p.text(20.0, 390.0, "В невесомости кости и мышцы слабеют — на МКС тренируются 2 часа в день", "In weightlessness bones and muscles weaken — ISS crews train 2 hours a day", 16.0, 0xb0b8d8);
    }
}

// ---------------------------------------------------------------- lunch

const FOODS: [(&str, &str, &str, bool); 9] = [
    // name ru, en, icon, needs water
    ("Суп (сухой)", "Soup (dry)", "plate", true),
    ("Тортилья", "Tortilla", "food", false),
    ("Курица", "Chicken", "food", false),
    ("Сок (порошок)", "Juice (powder)", "droplet", true),
    ("Хлеб", "Bread", "bread", false),
    ("Соль (крупная)", "Salt (grains)", "rock", false),
    ("Каша (сухая)", "Porridge (dry)", "plate", true),
    ("Яблоко", "Apple", "seedling", false),
    ("Газировка", "Soda", "sparkles", false),
];
/// why some foods are banned in space
const TRAPS: [(u8, &str, &str); 3] = [
    (4, "Хлеб крошится: крошки летают и попадают в приборы и глаза!", "Bread makes crumbs: they float into gear and eyes!"),
    (5, "Крупинки соли разлетятся! В космосе соль — жидкая, в воде.", "Salt grains would fly away! In space salt is liquid, in water."),
    (8, "Пузырьки газа в невесомости не всплывают — живот раздует!", "Gas bubbles don't rise in weightlessness — tummy ache!"),
];
const TRAY: Btn = Btn(380.0, 230.0, 320.0, 180.0);
const TAP: (f32, f32) = (640.0, 110.0);

/// Make lunch by the menu: right foods into the tray, dry ones need water from the tap. No crumbs!
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Meal {
    pub menu: Vec<u8>,
    pub pantry: Vec<Piece>,
    pub wet: Vec<bool>,
    pub tap: Piece,
    pub held: Option<usize>,
    pub holding_tap: bool,
    pub why: Option<u8>,
}
fn pantry_pos(i: usize) -> (f32, f32) {
    (90.0 + (i % 2) as f32 * 150.0, 70.0 + (i / 2) as f32 * 72.0)
}
fn tray_slot(i: usize) -> (f32, f32) {
    (TRAY.0 + 70.0 + (i % 2) as f32 * 160.0, TRAY.1 + 50.0 + (i / 2) as f32 * 80.0)
}
impl Meal {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 4 } else { 3 };
        let good: Vec<u8> = vec![0, 1, 2, 3, 6, 7];
        let mut g = good.clone();
        shuffle(&mut g, rng);
        let mut menu: Vec<u8> = g.iter().copied().take(n).collect();
        if !menu.iter().any(|m| FOODS[*m as usize].3) {
            menu[0] = 0;
        }
        let mut stock: Vec<u8> = menu.clone();
        stock.extend(g.iter().skip(n).take(1));
        stock.extend([4u8, 5, 8].iter().take(if hard { 3 } else { 2 }));
        shuffle(&mut stock, rng);
        let pantry = stock.iter().enumerate().map(|(i, k)| Piece::new(pantry_pos(i).0, pantry_pos(i).1, *k)).collect::<Vec<_>>();
        Meal { menu, wet: vec![false; pantry.len()], pantry, tap: Piece::new(TAP.0, TAP.1, 99), held: None, holding_tap: false, why: None }
    }
    fn in_tray(&self) -> Vec<usize> {
        (0..self.pantry.len()).filter(|k| self.pantry[*k].placed).collect()
    }
    fn finished(&self) -> bool {
        let tray: Vec<u8> = self.in_tray().iter().map(|k| self.pantry[*k].kind).collect();
        self.menu.iter().all(|m| tray.contains(m)) && self.in_tray().iter().all(|k| !FOODS[self.pantry[*k].kind as usize].3 || self.wet[*k])
    }
}
impl Play for Meal {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.why = None;
                if dist((x, y), (self.tap.x, self.tap.y)) < 34.0 {
                    self.holding_tap = true;
                } else {
                    self.held = grab(&self.pantry, x, y, 40.0);
                }
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if self.holding_tap {
                    self.tap.x = x;
                    self.tap.y = y;
                } else if let Some(k) = self.held {
                    self.pantry[k].x = x;
                    self.pantry[k].y = y;
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                if self.holding_tap {
                    self.holding_tap = false;
                    self.tap.home();
                    let target = self.in_tray().into_iter().find(|k| dist((x, y), (self.pantry[*k].x, self.pantry[*k].y)) < 50.0);
                    return match target {
                        Some(k) if FOODS[self.pantry[k].kind as usize].3 && !self.wet[k] => {
                            self.wet[k] = true;
                            if self.finished() {
                                Outcome::Done
                            } else {
                                Outcome::Fine
                            }
                        }
                        Some(_) => Outcome::Mistake, // this one doesn't need water
                        None => Outcome::Fine,
                    };
                }
                let Some(k) = self.held.take() else { return Outcome::Fine };
                if !TRAY.hit(x, y) {
                    self.pantry[k].home();
                    return Outcome::Fine;
                }
                let kind = self.pantry[k].kind;
                if let Some(t) = TRAPS.iter().position(|t| t.0 == kind) {
                    self.why = Some(t as u8);
                    self.pantry[k].home();
                    return Outcome::Mistake;
                }
                let tray: Vec<u8> = self.in_tray().iter().map(|j| self.pantry[*j].kind).collect();
                if !self.menu.contains(&kind) || tray.contains(&kind) {
                    self.pantry[k].home();
                    return Outcome::Mistake;
                }
                let slot = tray_slot(tray.len());
                self.pantry[k].placed = true;
                self.pantry[k].x = slot.0;
                self.pantry[k].y = slot.1;
                if self.finished() {
                    Outcome::Done
                } else {
                    Outcome::Fine
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        let tray = self.in_tray();
        (tray.len() as f32 + tray.iter().filter(|k| self.wet[**k]).count() as f32) / (self.menu.len() as f32 + self.menu.iter().filter(|m| FOODS[**m as usize].3).count() as f32)
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        let tray: Vec<u8> = self.in_tray().iter().map(|k| self.pantry[*k].kind).collect();
        if let Some(k) = (0..self.pantry.len()).find(|k| !self.pantry[*k].placed && self.menu.contains(&self.pantry[*k].kind) && !tray.contains(&self.pantry[*k].kind)) {
            drag_to(&mut o, (self.pantry[k].x, self.pantry[k].y), TRAY.mid());
        } else if let Some(k) = self.in_tray().into_iter().find(|k| FOODS[self.pantry[*k].kind as usize].3 && !self.wet[*k]) {
            drag_to(&mut o, (self.tap.x, self.tap.y), (self.pantry[k].x, self.pantry[k].y));
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Собери обед по меню на поднос. Сухую еду полей водой из крана. Осторожно с запрещённой едой!", "Put the menu on the tray. Pour tap water on dry food. Careful with banned food!");
        p.rect(380.0, 44.0, 200.0, 170.0, 0xf6f0de);
        p.text(392.0, 50.0, "Меню:", "Menu:", 19.0, 0x333333);
        let tray: Vec<u8> = self.in_tray().iter().map(|k| self.pantry[*k].kind).collect();
        for (i, m) in self.menu.iter().enumerate() {
            let f = FOODS[*m as usize];
            let ok = tray.contains(m);
            p.text(392.0, 80.0 + i as f32 * 30.0, f.0, f.1, 17.0, if ok { 0x2a8a4a } else { 0x333333 });
        }
        p.rect(TRAY.0, TRAY.1, TRAY.2, TRAY.3, 0xb8c0d4);
        p.ctext(TRAY.0 + TRAY.2 / 2.0, TRAY.1 + 4.0, "Поднос (на липучках)", "Tray (velcro)", 15.0, 0x333333);
        for (k, it) in self.pantry.iter().enumerate() {
            let f = FOODS[it.kind as usize];
            p.rect(it.x - 64.0, it.y - 28.0, 128.0, 56.0, if self.wet[k] { 0x7ad0ff } else { 0xf0f0f0 });
            p.icon(f.2, it.x - 42.0, it.y, 36.0);
            p.text(it.x - 20.0, it.y - 20.0, f.0, f.1, 14.0, 0x222222);
            if f.3 {
                p.text(it.x - 20.0, it.y + 2.0, if self.wet[k] { "с водой (ok)" } else { "нужна вода" }, if self.wet[k] { "watered (ok)" } else { "needs water" }, 12.0, 0x2a5a9a);
            }
        }
        p.rect(TAP.0 - 30.0, TAP.1 - 70.0, 60.0, 40.0, 0x8a90a8);
        p.circle(self.tap.x, self.tap.y, 26.0, 0x3a8bff);
        p.icon("droplet", self.tap.x, self.tap.y, 34.0);
        p.ctext(TAP.0, TAP.1 + 30.0, "кран", "tap", 15.0, 0xffffff);
        if let Some(t) = self.why {
            let (_, ru, en) = TRAPS[t as usize];
            p.rect_a(10.0, 380.0, 700.0, 40.0, 0x601818, 0.9);
            p.text(20.0, 388.0, ru, en, 17.0, 0xffffff);
        }
    }
}
