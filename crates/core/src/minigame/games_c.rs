//! Near-the-Moon games: the Earth photo, the Apollo 13 adapter, the "barbecue roll", sealing a
//! crack, packing the lander, braking into lunar orbit — and the ship's pet.

use super::*;

fn lcg(s: &mut u64) -> f32 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*s >> 40) as f32 / (1u64 << 24) as f32
}

// ---------------------------------------------------------------- photo

const SHUTTER: Btn = Btn(560.0, 340.0, 150.0, 70.0);
const FRAME_X: f32 = 360.0;
fn photo_slider(k: usize) -> f32 {
    330.0 + k as f32 * 44.0
}
const PX0: f32 = 130.0;
const PW: f32 = 380.0;

/// Earth drifts past the window: set exposure and focus, press the shutter when it's in the frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Photo {
    pub x: f32,
    pub val: [f32; 2],
    pub want: [f32; 2],
    pub held: Option<usize>,
    pub shots: u8,
    pub need: u8,
}
impl Photo {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        Photo { x: 60.0, val: [0.1, 0.9], want: [rng.range(0.3, 0.8), rng.range(0.25, 0.75)], held: None, shots: 0, need: if hard { 2 } else { 1 } }
    }
    fn tol(hard: bool) -> (f32, f32) {
        if hard {
            (0.06, 45.0)
        } else {
            (0.1, 65.0)
        }
    }
    fn ready(&self, hard: bool) -> bool {
        let (t, _) = Self::tol(hard);
        (self.val[0] - self.want[0]).abs() < t && (self.val[1] - self.want[1]).abs() < t
    }
    fn shoot(&mut self, hard: bool) -> Outcome {
        let (_, fr) = Self::tol(hard);
        if self.ready(hard) && (self.x - FRAME_X).abs() < fr {
            self.shots += 1;
            if self.shots >= self.need {
                return Outcome::Done;
            }
            self.x = 40.0;
            Outcome::Fine
        } else {
            Outcome::Mistake
        }
    }
}
impl Play for Photo {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) if SHUTTER.hit(x, y) => self.shoot(hard),
            MiniInput::Space(true) => self.shoot(hard),
            MiniInput::Press(x, y) => {
                self.held = (0..2).find(|k| (y - photo_slider(*k)).abs() < 18.0 && (x - (PX0 + self.val[*k] * PW)).abs() < 30.0);
                Outcome::Fine
            }
            MiniInput::Drag(x, _) => {
                if let Some(k) = self.held {
                    self.val[k] = ((x - PX0) / PW).clamp(0.0, 1.0);
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
    fn tick(&mut self, dt: f32, hard: bool) -> Outcome {
        self.x += dt * if hard { 110.0 } else { 70.0 };
        if self.x > 700.0 {
            self.x = 20.0;
        }
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.shots as f32 / self.need as f32
    }
    fn bot(&self, hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if let Some(k) = (0..2).find(|k| (self.val[*k] - self.want[*k]).abs() >= Self::tol(hard).0 * 0.5) {
            drag_to(&mut o, (PX0 + self.val[k] * PW, photo_slider(k)), (PX0 + self.want[k] * PW, photo_slider(k)));
        } else if (self.x - FRAME_X).abs() < 8.0 {
            click(&mut o, SHUTTER.mid());
        } else {
            o.push(MiniInput::Wait(0.03));
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint("Настрой выдержку и резкость, жми «Снимок», когда Земля в рамке", "Set exposure and focus, press “Shoot” when Earth is in the frame");
        p.rect(20.0, 40.0, 680.0, 250.0, 0x05061a);
        let (t, fr) = Self::tol(hard);
        let blur = ((self.val[1] - self.want[1]).abs() / t).min(4.0);
        let bright = (self.val[0] - self.want[0]) / t;
        for k in 0..(blur as usize + 1) {
            p.circle_a(self.x + k as f32 * 6.0, 165.0, 60.0 + k as f32 * 5.0, 0x3a8bff, if k == 0 { 1.0 } else { 0.25 });
        }
        p.icon("earth", self.x, 165.0, 120.0);
        if bright.abs() > 1.0 {
            p.circle_a(self.x, 165.0, 70.0, if bright > 0.0 { 0xffffff } else { 0x000000 }, (bright.abs() * 0.2).min(0.8));
        }
        p.ring(FRAME_X, 165.0, 60.0 + fr, 0xffd35a, 3.0);
        for k in 0..2 {
            let y = photo_slider(k);
            p.text(14.0, y - 11.0, ["Выдержка", "Резкость"][k], ["Exposure", "Focus"][k], 18.0, 0xffffff);
            p.rect(PX0, y - 5.0, PW, 10.0, 0x2a3048);
            p.circle(PX0 + self.val[k] * PW, y, 15.0, 0xffd35a);
        }
        p.raw(600.0, 300.0, format!("{} / {}", self.shots, self.need), 24.0, 0x5aff8a, false);
        p.button(SHUTTER, "Снимок", "Shoot", 0xffd35a, false);
    }
}

// ---------------------------------------------------------------- Apollo 13 adapter

const PARTS: [(&str, &str, &str); 7] = [
    ("Квадратный фильтр", "Square filter", "box"),
    ("Пакет", "Bag", "shirt"),
    ("Картонка (обложка)", "Cardboard (cover)", "book"),
    ("Скотч", "Tape", "wrench"),
    ("Шланг", "Hose", "wind"),
    ("Носок", "Sock", "shirt"),
    ("Ножницы", "Scissors", "cross"),
];
const BUILD: Btn = Btn(250.0, 110.0, 230.0, 230.0);

/// The square filter doesn't fit the round hole: build the adapter in the right order, like Apollo 13.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Adapter {
    pub parts: Vec<Piece>,
    pub order: Vec<u8>,
    pub step: usize,
    pub held: Option<usize>,
}
impl Adapter {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let order: Vec<u8> = if hard { vec![0, 2, 1, 3, 4] } else { vec![0, 1, 3, 4] };
        let mut kinds: Vec<u8> = order.clone();
        kinds.push(5);
        if hard {
            kinds.push(6);
        }
        shuffle(&mut kinds, rng);
        let parts = kinds.iter().enumerate().map(|(i, k)| Piece::new(if i % 2 == 0 { 90.0 } else { 630.0 }, 80.0 + (i / 2) as f32 * 95.0, *k)).collect();
        Adapter { parts, order, step: 0, held: None }
    }
}
impl Play for Adapter {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.held = grab(&self.parts, x, y, 44.0);
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if let Some(k) = self.held {
                    (self.parts[k].x, self.parts[k].y) = (x, y);
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                let Some(k) = self.held.take() else { return Outcome::Fine };
                if !BUILD.hit(x, y) {
                    self.parts[k].home();
                    return Outcome::Fine;
                }
                if self.parts[k].kind != self.order[self.step] {
                    self.parts[k].home();
                    return Outcome::Mistake;
                }
                self.parts[k].placed = true;
                self.step += 1;
                if self.step == self.order.len() {
                    Outcome::Done
                } else {
                    Outcome::Fine
                }
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        self.step as f32 / self.order.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if let Some(k) = self.parts.iter().position(|p| !p.placed && p.kind == self.order[self.step]) {
            drag_to(&mut o, (self.parts[k].x, self.parts[k].y), BUILD.mid());
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("ЦУП читает инструкцию: клади детали на сборку строго по шагам. Лишнее не бери!", "Mission Control reads the steps: put the parts on in order. Leave the extra things!");
        p.rect(BUILD.0, BUILD.1, BUILD.2, BUILD.3, 0x2a3048);
        p.ring(BUILD.0 + BUILD.2 / 2.0, BUILD.1 + BUILD.3 / 2.0, 60.0, 0x9aa3b8, 6.0);
        p.ctext(BUILD.0 + BUILD.2 / 2.0, BUILD.1 + 6.0, "круглое гнездо", "round socket", 14.0, 0xb0b8d8);
        for (n, k) in self.order.iter().take(self.step).enumerate() {
            p.icon(PARTS[*k as usize].2, BUILD.0 + 40.0 + n as f32 * 38.0, BUILD.1 + BUILD.3 / 2.0, 44.0);
        }
        p.rect(250.0, 350.0, 230.0, 70.0, 0xf0ead8);
        for (n, k) in self.order.iter().enumerate() {
            let c = if n < self.step { 0x2a8a4a } else if n == self.step { 0xc03030 } else { 0x666666 };
            p.raw(258.0, 352.0 + n as f32 * 13.0, format!("{}. {}", n + 1, self.parts.iter().find(|q| q.kind == *k).map(|_| PARTS[*k as usize].0).unwrap_or("")), 12.0, c, false);
        }
        for it in self.parts.iter().filter(|q| !q.placed) {
            let (ru, en, ic) = PARTS[it.kind as usize];
            p.rect(it.x - 70.0, it.y - 36.0, 140.0, 72.0, 0xdfe5f2);
            p.icon(ic, it.x - 40.0, it.y, 44.0);
            p.text(it.x - 14.0, it.y - 18.0, ru, en, 13.0, 0x222222);
        }
    }
}

// ---------------------------------------------------------------- barbecue roll

const ROLL_X: f32 = 160.0;
const ROLL_W: f32 = 400.0;

/// One side faces the Sun and cooks, the other freezes: set a slow, even roll so both stay in the
/// green — but every spin costs thruster fuel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Barbecue {
    pub rate: f32,
    pub phi: f32,
    pub ta: f32,
    pub tb: f32,
    pub fuel: f32,
    pub good: f32,
    pub held: bool,
}
impl Barbecue {
    pub fn new(_hard: bool, _rng: &mut Rng) -> Self {
        Barbecue { rate: 0.0, phi: 0.0, ta: 90.0, tb: -70.0, fuel: 1.0, good: 0.0, held: false }
    }
    fn omega(&self) -> f32 {
        self.rate * 2.5
    }
    fn tau(hard: bool) -> f32 {
        if hard {
            2.0
        } else {
            3.0
        }
    }
    fn need(hard: bool) -> f32 {
        if hard {
            7.0
        } else {
            5.0
        }
    }
    fn zone(t: f32) -> bool {
        (-30.0..=50.0).contains(&t)
    }
    fn best(hard: bool) -> f32 {
        if hard {
            0.6
        } else {
            0.5
        }
    }
}
impl Play for Barbecue {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.held = (y - 380.0).abs() < 24.0 && (x - (ROLL_X + self.rate * ROLL_W)).abs() < 34.0;
                Outcome::Fine
            }
            MiniInput::Drag(x, _) if self.held => {
                self.rate = ((x - ROLL_X) / ROLL_W).clamp(0.0, 1.0);
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
        let w = self.omega();
        self.phi += w * dt;
        let tau = Self::tau(hard);
        let (ea, eb) = (10.0 + 110.0 * self.phi.cos(), 10.0 - 110.0 * self.phi.cos());
        self.ta += (ea - self.ta) * (dt / tau).min(1.0);
        self.tb += (eb - self.tb) * (dt / tau).min(1.0);
        // spinning costs fuel, more for fast spins
        self.fuel -= w * w * dt * if hard { 0.012 } else { 0.007 };
        if self.fuel <= 0.0 {
            *self = Barbecue::new(hard, &mut Rng::new(1));
            return Outcome::Mistake;
        }
        if Self::zone(self.ta) && Self::zone(self.tb) {
            self.good += dt;
        } else {
            self.good = 0.0;
        }
        if self.good >= Self::need(hard) {
            Outcome::Done
        } else {
            Outcome::Fine
        }
    }
    fn progress(&self) -> f32 {
        self.good / 7.0
    }
    fn bot(&self, hard: bool) -> Vec<MiniInput> {
        let b = Self::best(hard);
        if (self.rate - b).abs() > 0.02 {
            let mut o = vec![];
            drag_to(&mut o, (ROLL_X + self.rate * ROLL_W, 380.0), (ROLL_X + b * ROLL_W, 380.0));
            o
        } else {
            vec![MiniInput::Wait(0.1)]
        }
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Включи медленное вращение: обе стороны должны быть в зелёной зоне. Быстро крутить — тратить топливо!", "Start a slow roll: both sides in the green. Spinning fast burns fuel!");
        p.icon("sun", 70.0, 190.0, 110.0);
        p.circle(380.0, 190.0, 90.0, 0x8a93ab);
        let col = |t: f32| if t > 50.0 { 0xff5a3a } else if t < -30.0 { 0x5a9aff } else { 0x5aff8a };
        let (c, s) = (self.phi.cos(), self.phi.sin());
        p.circle(380.0 - 45.0 * c, 190.0 - 45.0 * s, 40.0, col(self.ta));
        p.circle(380.0 + 45.0 * c, 190.0 + 45.0 * s, 40.0, col(self.tb));
        p.raw(380.0 - 45.0 * c, 178.0 - 45.0 * s, "A".into(), 20.0, 0x0b0e22, true);
        p.raw(380.0 + 45.0 * c, 178.0 + 45.0 * s, "B".into(), 20.0, 0x0b0e22, true);
        p.raw(560.0, 120.0, format!("A: {:+.0} °C", self.ta), 22.0, col(self.ta), false);
        p.raw(560.0, 160.0, format!("B: {:+.0} °C", self.tb), 22.0, col(self.tb), false);
        p.text(560.0, 200.0, "норма -30…+50", "OK -30…+50", 15.0, 0xb0b8d8);
        p.text(560.0, 240.0, "Топливо", "Fuel", 16.0, 0xffffff);
        p.rect(560.0, 264.0, 140.0, 14.0, 0x2a3048);
        p.rect(560.0, 264.0, 140.0 * self.fuel.max(0.0), 14.0, 0xff8a3d);
        p.text(14.0, 340.0, "Скорость вращения", "Roll speed", 18.0, 0xffffff);
        p.rect(ROLL_X, 375.0, ROLL_W, 10.0, 0x2a3048);
        p.circle(ROLL_X + self.rate * ROLL_W, 380.0, 16.0, 0xffd35a);
        p.raw(600.0, 368.0, format!("{:.1} об/мин", self.omega() * 60.0 / std::f32::consts::TAU), 18.0, 0xffffff, false);
    }
}

// ---------------------------------------------------------------- crack seal

/// Air hisses out of a crack: trace along it from start to end with the sealant, without slipping off.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Seal {
    pub path: Vec<(f32, f32)>,
    pub done: usize,
    pub tracing: bool,
    pub air: f32,
}
impl Seal {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let segs = if hard { 9 } else { 5 };
        let mut pts = vec![(80.0, rng.range(120.0, 320.0))];
        for i in 1..=segs {
            let x = 80.0 + i as f32 * (560.0 / segs as f32);
            let y = (pts.last().unwrap().1 + rng.range(-110.0, 110.0)).clamp(70.0, 380.0);
            pts.push((x, y));
        }
        // dense points every ~10 px
        let mut path = vec![];
        for w in pts.windows(2) {
            let n = (dist(w[0], w[1]) / 10.0).ceil() as usize;
            for k in 0..n {
                let f = k as f32 / n as f32;
                path.push((w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f));
            }
        }
        path.push(*pts.last().unwrap());
        Seal { path, done: 0, tracing: false, air: 1.0 }
    }
    fn tol(hard: bool) -> f32 {
        if hard {
            16.0
        } else {
            22.0
        }
    }
    fn near_path(&self, x: f32, y: f32) -> f32 {
        let lo = self.done.saturating_sub(4);
        let hi = (self.done + 6).min(self.path.len());
        self.path[lo..hi].iter().map(|q| dist((x, y), *q)).fold(f32::MAX, f32::min)
    }
}
impl Play for Seal {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        let tol = Self::tol(hard);
        match i {
            MiniInput::Press(x, y) => {
                let at = self.path[self.done.min(self.path.len() - 1)];
                self.tracing = dist((x, y), at) < tol * 1.6;
                Outcome::Fine
            }
            MiniInput::Drag(x, y) if self.tracing => {
                if self.near_path(x, y) > tol * 2.0 {
                    self.tracing = false;
                    return Outcome::Mistake; // slipped off the crack
                }
                while self.done < self.path.len() && dist((x, y), self.path[self.done]) < tol * 1.3 {
                    self.done += 1;
                }
                if self.done >= self.path.len() {
                    Outcome::Done
                } else {
                    Outcome::Fine
                }
            }
            MiniInput::Release(..) => {
                self.tracing = false;
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        let open = 1.0 - self.done as f32 / self.path.len() as f32;
        self.air = (self.air - dt * 0.01 * open).max(0.0);
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        self.done as f32 / self.path.len() as f32
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        let s = self.path[self.done];
        o.push(MiniInput::Press(s.0, s.1));
        for q in self.path.iter().skip(self.done) {
            o.push(MiniInput::Drag(q.0, q.1));
        }
        o.push(MiniInput::Release(0.0, 0.0));
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        p.hint("Зажми у начала трещины и веди герметиком точно по ней до конца — не соскальзывай!", "Press at the start of the crack and trace it with sealant to the end — don't slip off!");
        p.rect(20.0, 40.0, 680.0, 370.0, 0x9aa3b8);
        for k in 0..8 {
            p.rect(20.0 + k as f32 * 85.0, 40.0, 2.0, 370.0, 0x8a93ab);
        }
        let w = Self::tol(hard);
        for (k, q) in self.path.iter().enumerate() {
            if k < self.done {
                p.circle(q.0, q.1, w * 0.8, 0xffcc33);
            } else {
                p.circle(q.0, q.1, 3.5, 0x1a1a24);
            }
        }
        if self.done < self.path.len() {
            let s = self.path[self.done];
            p.ring(s.0, s.1, w * 1.4, 0x5aff8a, 3.0);
            let e = *self.path.last().unwrap();
            p.circle_a(e.0 + 12.0, e.1 - 14.0, 6.0, 0xffffff, 0.6);
            p.circle_a(e.0 + 24.0, e.1 - 26.0, 4.0, 0xffffff, 0.4);
        }
        p.raw(560.0, 380.0, format!("{} {:.0}%", p.l.pick("Воздух", "Air"), self.air * 100.0), 18.0, 0x0b0e22, false);
    }
}

// ---------------------------------------------------------------- packing the lander

const ITEMS: [(&str, &str, &str, u8, u8, bool); 12] = [
    // name, icon, mass kg, value, must
    ("Кислород", "Oxygen", "oxygen", 10, 0, true),
    ("Вода", "Water", "droplet", 8, 0, true),
    ("Еда", "Food", "food", 6, 0, true),
    ("Аптечка", "Medkit", "heart", 3, 4, false),
    ("Камера", "Camera", "s_photo", 2, 3, false),
    ("Флаг", "Flag", "flag", 1, 2, false),
    ("Инструменты", "Tools", "wrench", 7, 4, false),
    ("Гитара", "Guitar", "s_music", 6, 1, false),
    ("Научный прибор", "Science kit", "science", 9, 6, false),
    ("Запасной фильтр", "Spare filter", "wind", 3, 4, false),
    ("Гантели", "Dumbbells", "strong", 12, 0, false),
    ("Игрушка", "Toy", "cat", 1, 1, false),
];
const BAG: Btn = Btn(430.0, 130.0, 280.0, 220.0);
const READY: Btn = Btn(430.0, 362.0, 280.0, 56.0);

/// Every kilogram needs fuel to land: pack the must-haves and the most useful things under the limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub items: Vec<Piece>,
    pub limit: u32,
    pub need: u32,
    pub held: Option<usize>,
    pub why: Option<u8>,
}
fn pack_home(i: usize) -> (f32, f32) {
    (70.0 + (i % 3) as f32 * 120.0, 70.0 + (i / 3) as f32 * 88.0)
}
impl Pack {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let mut idx: Vec<u8> = (0..12).collect();
        shuffle(&mut idx, rng);
        let items: Vec<Piece> = idx.iter().enumerate().map(|(i, k)| Piece::new(pack_home(i).0, pack_home(i).1, *k)).collect();
        let limit = 40;
        let best = Self::best(limit);
        Pack { items, limit, need: if hard { best.1 } else { best.1.saturating_sub(4) }, held: None, why: None }
    }
    /// The best set (mask, value) under the limit with all must-haves.
    fn best(limit: u32) -> (u32, u32) {
        let mut best = (0, 0);
        for mask in 0u32..(1 << 12) {
            let (mut m, mut v, mut ok) = (0u32, 0u32, true);
            for (k, it) in ITEMS.iter().enumerate() {
                let on = mask & (1 << k) != 0;
                if it.5 && !on {
                    ok = false;
                }
                if on {
                    m += it.3 as u32;
                    v += it.4 as u32;
                }
            }
            if ok && m <= limit && v > best.1 {
                best = (mask, v);
            }
        }
        best
    }
    fn totals(&self) -> (u32, u32) {
        self.items.iter().filter(|p| p.placed).fold((0, 0), |a, p| (a.0 + ITEMS[p.kind as usize].3 as u32, a.1 + ITEMS[p.kind as usize].4 as u32))
    }
    fn arrange(&mut self) {
        let mut n = 0;
        for p in self.items.iter_mut() {
            if p.placed {
                p.x = BAG.0 + 40.0 + (n % 4) as f32 * 66.0;
                p.y = BAG.1 + 60.0 + (n / 4) as f32 * 60.0;
                n += 1;
            }
        }
    }
}
impl Play for Pack {
    fn input(&mut self, i: MiniInput, _hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) => {
                self.why = None;
                if READY.hit(x, y) {
                    let (m, v) = self.totals();
                    let must = self.items.iter().all(|p| !ITEMS[p.kind as usize].5 || p.placed);
                    return if !must {
                        self.why = Some(0);
                        Outcome::Mistake
                    } else if m > self.limit {
                        self.why = Some(1);
                        Outcome::Mistake
                    } else if v < self.need {
                        self.why = Some(2);
                        Outcome::Mistake
                    } else {
                        Outcome::Done
                    };
                }
                self.held = self.items.iter().enumerate().filter(|(_, p)| dist((x, y), (p.x, p.y)) < 34.0).map(|(i, _)| i).next();
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if let Some(k) = self.held {
                    (self.items[k].x, self.items[k].y) = (x, y);
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                let Some(k) = self.held.take() else { return Outcome::Fine };
                self.items[k].placed = BAG.hit(x, y);
                if !self.items[k].placed {
                    self.items[k].home();
                }
                self.arrange();
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn progress(&self) -> f32 {
        self.totals().1 as f32 / self.need.max(1) as f32 * 0.9
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let (mask, _) = Self::best(self.limit);
        let mut o = vec![];
        if let Some(k) = (0..self.items.len()).find(|k| {
            let p = self.items[*k];
            (mask & (1 << p.kind) != 0) != p.placed
        }) {
            let p = self.items[k];
            drag_to(&mut o, (p.x, p.y), if p.placed { (p.hx, p.hy) } else { BAG.mid() });
        } else {
            click(&mut o, READY.mid());
        }
        o
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Собери сумку в посадочный модуль: обязательное + самое полезное, но не тяжелее лимита", "Pack the lander bag: must-haves + the most useful things, under the mass limit");
        let (m, v) = self.totals();
        p.rect(BAG.0, BAG.1, BAG.2, BAG.3, 0x3a3f5a);
        p.ctext(BAG.0 + BAG.2 / 2.0, BAG.1 + 6.0, "Сумка модуля", "Lander bag", 17.0, 0xffffff);
        p.raw(BAG.0, 50.0, format!("{} {m} / {} кг", p.l.pick("Масса", "Mass"), self.limit), 22.0, if m > self.limit { 0xff5a5a } else { 0xffffff }, false);
        p.raw(BAG.0, 84.0, format!("{} {v} / {}", p.l.pick("Польза +", "Value +"), self.need), 22.0, if v >= self.need { 0x5aff8a } else { 0xffd35a }, false);
        for it in &self.items {
            let (ru, en, ic, kg, val, must) = ITEMS[it.kind as usize];
            p.rect(it.x - 32.0, it.y - 32.0, 64.0, 64.0, if must { 0xffe0a0 } else { 0xdfe5f2 });
            p.icon(ic, it.x, it.y - 8.0, 34.0);
            p.raw(it.x, it.y + 12.0, format!("{kg}кг +{val}"), 12.0, 0x222222, true);
            if !it.placed {
                p.ctext(it.x, it.y + 32.0, ru, en, 12.0, 0xffffff);
            }
        }
        p.button(READY, "Готово", "Done", 0xffd35a, false);
        if let Some(w) = self.why {
            let (ru, en) = [("Без кислорода, воды и еды лететь нельзя!", "No flying without oxygen, water and food!"), ("Слишком тяжело: не хватит топлива на посадку!", "Too heavy: not enough fuel to land!"), ("Можно взять полезнее — поменяй вещи", "You can pack more useful things — swap some")][w as usize];
            p.rect_a(10.0, 388.0, 410.0, 34.0, 0x601818, 0.9);
            p.text(18.0, 394.0, ru, en, 15.0, 0xffffff);
        }
    }
}

// ---------------------------------------------------------------- lunar orbit insertion

const MOON: (f32, f32) = (360.0, 220.0);
const BRAKE: Btn = Btn(560.0, 350.0, 150.0, 64.0);

/// The ship swings around the Moon: brake exactly at the closest point, again and again, until the
/// orbit is a circle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Orbit {
    pub th: f32,
    pub e: f32,
    pub left: u8,
}
impl Orbit {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        Orbit { th: rng.range(1.0, 3.0), e: 0.6, left: if hard { 3 } else { 2 } }
    }
    fn pos(&self, th: f32) -> (f32, f32) {
        let a = 150.0;
        let r = a * (1.0 - self.e * self.e) / (1.0 + self.e * th.cos());
        (MOON.0 + r * th.cos() * 1.4 - 60.0 * self.e, MOON.1 + r * th.sin())
    }
    fn tol(hard: bool) -> f32 {
        if hard {
            0.28
        } else {
            0.4
        }
    }
    fn brake(&mut self, hard: bool) -> Outcome {
        let d = wrap_angle(self.th);
        if d.abs() < Self::tol(hard) {
            self.e *= 0.35;
            self.left -= 1;
            if self.left == 0 {
                Outcome::Done
            } else {
                Outcome::Fine
            }
        } else {
            Outcome::Mistake
        }
    }
}
impl Play for Orbit {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        match i {
            MiniInput::Press(x, y) if BRAKE.hit(x, y) => self.brake(hard),
            MiniInput::Space(true) => self.brake(hard),
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, hard: bool) -> Outcome {
        // faster near the Moon, slower far away (Kepler)
        let k = (1.0 + self.e * self.th.cos()).powi(2) / (1.0 - self.e * self.e).powf(1.5);
        self.th += dt * if hard { 0.8 } else { 0.6 } * k.min(4.0);
        if self.th > std::f32::consts::PI {
            self.th -= std::f32::consts::TAU;
        }
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        1.0 - self.e / 0.6
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        if wrap_angle(self.th).abs() < 0.08 {
            let mut o = vec![];
            click(&mut o, BRAKE.mid());
            o
        } else {
            vec![MiniInput::Wait(0.02)]
        }
    }
    fn draw(&self, p: &mut Pen, _hard: bool) {
        p.hint("Жми «Тормоз» (или Пробел), когда корабль у жёлтой метки — ближе всего к Луне", "Press “Brake” (or Space) when the ship is at the yellow mark — closest to the Moon");
        for k in 0..72 {
            let q = self.pos(k as f32 / 72.0 * std::f32::consts::TAU);
            p.circle(q.0, q.1, 2.0, 0x6a7090);
        }
        p.circle(MOON.0, MOON.1, 56.0, 0xb8b8c0);
        p.circle(MOON.0 - 16.0, MOON.1 - 10.0, 10.0, 0x9a9aa4);
        p.circle(MOON.0 + 18.0, MOON.1 + 16.0, 7.0, 0x9a9aa4);
        let peri = self.pos(0.0);
        p.ring(peri.0, peri.1, 18.0, 0xffd35a, 4.0);
        let s = self.pos(self.th);
        p.icon("rocket", s.0, s.1, 34.0);
        p.icon("earth", 40.0, 220.0, 50.0);
        if s.0 > MOON.0 + 40.0 {
            p.text(470.0, 50.0, "За Луной: связи с Землёй нет", "Behind the Moon: no link to Earth", 17.0, 0xff8a6a);
        }
        p.raw(20.0, 380.0, format!("{} {}", p.l.pick("Торможений осталось:", "Burns left:"), self.left), 18.0, 0xffffff, false);
        p.button(BRAKE, "Тормоз", "Brake", 0xff8a3d, false);
    }
}

// ---------------------------------------------------------------- the pet

pub const PETS: [(&str, &str, &str); 3] = [("robot", "Робот Бип", "Robot Beep"), ("cat", "Кот Пломбир", "Cat Sundae"), ("alien", "Инопланетик Зу", "Little alien Zoo")];
const BOWL: (f32, f32) = (560.0, 340.0);
const VAC: Btn = Btn(20.0, 300.0, 140.0, 110.0);

/// Feed the pet: catch the floating crumbs with the vacuum, fill the bowl, pet it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PetCare {
    pub pet: u8,
    pub crumbs: Vec<Piece>,
    pub vel: Vec<(f32, f32)>,
    pub food: Piece,
    pub fed: bool,
    pub pats: u8,
    pub held: Option<usize>,
    pub food_held: bool,
    pub seed: u64,
}
impl PetCare {
    pub fn new(hard: bool, rng: &mut Rng) -> Self {
        let n = if hard { 7 } else { 4 };
        let crumbs = (0..n).map(|_| Piece::new(rng.range(200.0, 650.0), rng.range(60.0, 260.0), 0)).collect();
        let vel = (0..n).map(|_| (rng.range(-30.0, 30.0), rng.range(-30.0, 30.0))).collect();
        PetCare { pet: 0, crumbs, vel, food: Piece::new(90.0, 120.0, 1), fed: false, pats: 0, held: None, food_held: false, seed: rng.next() }
    }
    fn need_pats(hard: bool) -> u8 {
        if hard {
            5
        } else {
            3
        }
    }
}
impl Play for PetCare {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome {
        let clean = self.crumbs.iter().all(|c| c.placed);
        match i {
            MiniInput::Press(x, y) => {
                if clean && self.fed && dist((x, y), (360.0, 330.0)) < 60.0 {
                    self.pats += 1;
                    return if self.pats >= Self::need_pats(hard) { Outcome::Done } else { Outcome::Fine };
                }
                if dist((x, y), (self.food.x, self.food.y)) < 40.0 && !self.fed {
                    self.food_held = true;
                } else {
                    self.held = grab(&self.crumbs, x, y, 26.0);
                }
                Outcome::Fine
            }
            MiniInput::Drag(x, y) => {
                if self.food_held {
                    (self.food.x, self.food.y) = (x, y);
                } else if let Some(k) = self.held {
                    (self.crumbs[k].x, self.crumbs[k].y) = (x, y);
                }
                Outcome::Fine
            }
            MiniInput::Release(x, y) => {
                if self.food_held {
                    self.food_held = false;
                    if dist((x, y), BOWL) < 50.0 {
                        if !clean {
                            self.food.home();
                            return Outcome::Mistake; // crumbs first, or they float into the food
                        }
                        self.fed = true;
                    } else {
                        self.food.home();
                    }
                    return Outcome::Fine;
                }
                let Some(k) = self.held.take() else { return Outcome::Fine };
                if VAC.hit(x, y) {
                    self.crumbs[k].placed = true;
                }
                Outcome::Fine
            }
            _ => Outcome::Fine,
        }
    }
    fn tick(&mut self, dt: f32, _hard: bool) -> Outcome {
        for (k, c) in self.crumbs.iter_mut().enumerate() {
            if c.placed || self.held == Some(k) {
                continue;
            }
            c.x += self.vel[k].0 * dt;
            c.y += self.vel[k].1 * dt;
            if c.x < 190.0 || c.x > 700.0 {
                self.vel[k].0 = -self.vel[k].0;
                c.x = c.x.clamp(190.0, 700.0);
            }
            if c.y < 50.0 || c.y > 280.0 {
                self.vel[k].1 = -self.vel[k].1;
                c.y = c.y.clamp(50.0, 280.0);
            }
        }
        let _ = lcg(&mut self.seed);
        Outcome::Fine
    }
    fn progress(&self) -> f32 {
        (self.crumbs.iter().filter(|c| c.placed).count() as f32 / self.crumbs.len() as f32) * 0.5 + if self.fed { 0.3 } else { 0.0 } + self.pats as f32 * 0.04
    }
    fn bot(&self, _hard: bool) -> Vec<MiniInput> {
        let mut o = vec![];
        if let Some(c) = self.crumbs.iter().find(|c| !c.placed) {
            drag_to(&mut o, (c.x, c.y), VAC.mid());
        } else if !self.fed {
            drag_to(&mut o, (self.food.x, self.food.y), BOWL);
        } else {
            click(&mut o, (360.0, 330.0));
        }
        o
    }
    fn draw(&self, p: &mut Pen, hard: bool) {
        let clean = self.crumbs.iter().all(|c| c.placed);
        if !clean {
            p.hint("Сначала поймай летающие крошки — тащи их в пылесос", "First catch the floating crumbs — drag them into the vacuum");
        } else if !self.fed {
            p.hint("Теперь положи корм в миску", "Now put the food into the bowl");
        } else {
            p.hint("Погладь питомца — нажимай на него!", "Pet your pet — press it!");
        }
        p.rect(VAC.0, VAC.1, VAC.2, VAC.3, 0x5a6178);
        p.icon("broom", VAC.0 + 70.0, VAC.1 + 46.0, 50.0);
        p.ctext(VAC.0 + 70.0, VAC.1 + 80.0, "пылесос", "vacuum", 15.0, 0xffffff);
        for c in self.crumbs.iter().filter(|c| !c.placed) {
            p.circle(c.x, c.y, 10.0, 0xd9a35a);
        }
        p.icon(PETS[self.pet as usize % 3].0, 360.0, 330.0, 110.0);
        if self.fed {
            for k in 0..=self.pats {
                p.icon("heart", 330.0 + k as f32 * 30.0, 262.0, 26.0);
            }
            p.raw(470.0, 300.0, format!("{} / {}", self.pats, Self::need_pats(hard)), 20.0, 0xffffff, false);
        }
        p.circle(BOWL.0, BOWL.1, 40.0, 0xdfe5f2);
        if !self.fed {
            p.circle(self.food.x, self.food.y, 26.0, 0xd9883a);
            p.icon("food", self.food.x, self.food.y, 34.0);
        } else {
            p.icon("food", BOWL.0, BOWL.1, 40.0);
        }
    }
}
