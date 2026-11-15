//! Ship mini-games: every job on the ship has its own hands-on game that matches its console and
//! picture — drag the rubbish into the bin, trace a crack to seal it, hold the engine button until
//! the speed is right, balance a centrifuge… Each game teaches one law of physics or one real
//! spaceflight rule, and has an easier (Junior) and a harder (Advanced) version.
//!
//! Pure logic: the app sends pointer input in canvas pixels (`CW`×`CH`, y down) and draws the
//! shapes `Mini::scene` returns. The host checks every move, so network games can't cheat.

mod games_a;
mod games_b;
mod games_c;

pub use games_a::*;
pub use games_b::*;
pub use games_c::*;

use crate::i18n::Lang;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Canvas size in pixels.
pub const CW: f32 = 720.0;
pub const CH: f32 = 430.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiniKind {
    GoNoGo,
    Wires,
    Burn,
    Switches,
    Steady,
    Recycle,
    Rhythm,
    Meal,
    StarSight,
    Vector,
    Tune,
    Bolts,
    Shelter,
    FindLeak,
    Centrifuge,
    Antenna,
    Photo,
    Adapter,
    Barbecue,
    Seal,
    Pack,
    Orbit,
    PetCare,
}

pub const ALL_KINDS: [MiniKind; 23] = [
    MiniKind::GoNoGo,
    MiniKind::Wires,
    MiniKind::Burn,
    MiniKind::Switches,
    MiniKind::Steady,
    MiniKind::Recycle,
    MiniKind::Rhythm,
    MiniKind::Meal,
    MiniKind::StarSight,
    MiniKind::Vector,
    MiniKind::Tune,
    MiniKind::Bolts,
    MiniKind::Shelter,
    MiniKind::FindLeak,
    MiniKind::Centrifuge,
    MiniKind::Antenna,
    MiniKind::Photo,
    MiniKind::Adapter,
    MiniKind::Barbecue,
    MiniKind::Seal,
    MiniKind::Pack,
    MiniKind::Orbit,
    MiniKind::PetCare,
];

/// Pointer input in canvas pixels; `Space` is the keyboard's big button; `Wait` only for bots.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MiniInput {
    Press(f32, f32),
    Drag(f32, f32),
    Release(f32, f32),
    Space(bool),
    /// let time pass (tests and the bot; the real game ticks with the clock)
    Wait(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Fine,
    Mistake,
    Done,
}

/// What to draw (canvas pixels, y down). Colours are 0xRRGGBB.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// top-left corner
    Rect { x: f32, y: f32, w: f32, h: f32, c: u32, a: f32, r: f32 },
    Circle { x: f32, y: f32, r: f32, c: u32, a: f32 },
    Ring { x: f32, y: f32, r: f32, c: u32, w: f32 },
    /// centred picture from `assets/icons`
    Icon { n: &'static str, x: f32, y: f32, s: f32 },
    /// `center`: x is the middle of the text
    Text { x: f32, y: f32, t: String, s: f32, c: u32, center: bool },
    /// a dotted line
    Line { x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: u32 },
}

/// A little drawing kit for the games.
pub struct Pen<'a> {
    pub out: &'a mut Vec<Shape>,
    pub l: Lang,
}

impl Pen<'_> {
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32) {
        self.out.push(Shape::Rect { x, y, w, h, c, a: 1.0, r: 6.0 });
    }
    pub fn rect_a(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, a: f32) {
        self.out.push(Shape::Rect { x, y, w, h, c, a, r: 6.0 });
    }
    pub fn circle(&mut self, x: f32, y: f32, r: f32, c: u32) {
        self.out.push(Shape::Circle { x, y, r, c, a: 1.0 });
    }
    pub fn circle_a(&mut self, x: f32, y: f32, r: f32, c: u32, a: f32) {
        self.out.push(Shape::Circle { x, y, r, c, a });
    }
    pub fn ring(&mut self, x: f32, y: f32, r: f32, c: u32, w: f32) {
        self.out.push(Shape::Ring { x, y, r, c, w });
    }
    pub fn icon(&mut self, n: &'static str, x: f32, y: f32, s: f32) {
        self.out.push(Shape::Icon { n, x, y, s });
    }
    pub fn text(&mut self, x: f32, y: f32, ru: &str, en: &str, s: f32, c: u32) {
        let t = self.l.pick(ru, en).to_string();
        self.out.push(Shape::Text { x, y, t, s, c, center: false });
    }
    pub fn ctext(&mut self, x: f32, y: f32, ru: &str, en: &str, s: f32, c: u32) {
        let t = self.l.pick(ru, en).to_string();
        self.out.push(Shape::Text { x, y, t, s, c, center: true });
    }
    pub fn raw(&mut self, x: f32, y: f32, t: String, s: f32, c: u32, center: bool) {
        self.out.push(Shape::Text { x, y, t, s, c, center });
    }
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: u32) {
        self.out.push(Shape::Line { x0, y0, x1, y1, w, c });
    }
    /// A bar from `lo..hi` (0..1) with a green zone and a marker.
    pub fn gauge(&mut self, x: f32, y: f32, w: f32, v: f32, lo: f32, hi: f32) {
        self.rect(x, y, w, 16.0, 0x2a3048);
        self.rect(x + lo * w, y, (hi - lo) * w, 16.0, 0x2fbf6a);
        self.rect(x + v.clamp(0.0, 1.0) * w - 3.0, y - 6.0, 6.0, 28.0, 0xffffff);
    }
    /// A labelled button.
    pub fn button(&mut self, b: Btn, ru: &str, en: &str, c: u32, on: bool) {
        self.rect(b.0, b.1, b.2, b.3, if on { 0xffffff } else { 0x0b0e22 });
        self.rect(b.0 + 3.0, b.1 + 3.0, b.2 - 6.0, b.3 - 6.0, c);
        self.ctext(b.0 + b.2 / 2.0, b.1 + b.3 / 2.0 - 11.0, ru, en, 20.0, 0x0b0e22);
    }
    /// The hint line at the top of the canvas.
    pub fn hint(&mut self, ru: &str, en: &str) {
        let long = self.l.pick(ru, en).chars().count() > 78;
        self.rect_a(0.0, 0.0, CW, if long { 42.0 } else { 30.0 }, 0x000000, 0.55);
        self.text(10.0, 4.0, ru, en, if long { 15.0 } else { 17.0 }, 0xffd35a);
    }
}

/// A button: x, y, w, h.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Btn(pub f32, pub f32, pub f32, pub f32);
impl Btn {
    pub fn hit(&self, x: f32, y: f32) -> bool {
        x >= self.0 && x <= self.0 + self.2 && y >= self.1 && y <= self.1 + self.3
    }
    pub fn mid(&self) -> (f32, f32) {
        (self.0 + self.2 / 2.0, self.1 + self.3 / 2.0)
    }
}

/// Something you can drag: where it is, where it lives, what it is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub x: f32,
    pub y: f32,
    pub hx: f32,
    pub hy: f32,
    pub kind: u8,
    pub placed: bool,
}
impl Piece {
    pub fn new(x: f32, y: f32, kind: u8) -> Self {
        Piece { x, y, hx: x, hy: y, kind, placed: false }
    }
    pub fn home(&mut self) {
        self.x = self.hx;
        self.y = self.hy;
    }
}

/// The nearest free piece under the pointer.
pub fn grab(ps: &[Piece], x: f32, y: f32, r: f32) -> Option<usize> {
    ps.iter().enumerate().filter(|(_, p)| !p.placed).map(|(i, p)| (i, (p.x - x).hypot(p.y - y))).filter(|(_, d)| *d < r).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
}

/// An angle into -π..π.
pub fn wrap_angle(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI
}

pub fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Bot helper: press on a point, drag to another, let go.
pub fn drag_to(out: &mut Vec<MiniInput>, from: (f32, f32), to: (f32, f32)) {
    out.push(MiniInput::Press(from.0, from.1));
    out.push(MiniInput::Drag((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0));
    out.push(MiniInput::Drag(to.0, to.1));
    out.push(MiniInput::Release(to.0, to.1));
}
pub fn click(out: &mut Vec<MiniInput>, at: (f32, f32)) {
    out.push(MiniInput::Press(at.0, at.1));
    out.push(MiniInput::Release(at.0, at.1));
}

fn shuffle<T>(v: &mut [T], rng: &mut Rng) {
    for i in (1..v.len()).rev() {
        v.swap(i, rng.below(i + 1));
    }
}

/// One game: what every kind can do.
pub trait Play {
    fn input(&mut self, i: MiniInput, hard: bool) -> Outcome;
    fn tick(&mut self, _dt: f32, _hard: bool) -> Outcome {
        Outcome::Fine
    }
    fn progress(&self) -> f32;
    /// The next moves of a perfect player (the bot runs this until done).
    fn bot(&self, hard: bool) -> Vec<MiniInput>;
    fn draw(&self, p: &mut Pen, hard: bool);
}

macro_rules! games {
    ($($k:ident),*) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        pub enum MiniState { $($k($k)),* }
        impl MiniState {
            fn play(&mut self) -> &mut dyn Play { match self { $(MiniState::$k(g) => g),* } }
            fn view(&self) -> &dyn Play { match self { $(MiniState::$k(g) => g),* } }
            fn make(kind: MiniKind, hard: bool, rng: &mut Rng) -> Self { match kind { $(MiniKind::$k => MiniState::$k($k::new(hard, rng))),* } }
        }
    };
}
games!(GoNoGo, Wires, Burn, Switches, Steady, Recycle, Rhythm, Meal, StarSight, Vector, Tune, Bolts, Shelter, FindLeak, Centrifuge, Antenna, Photo, Adapter, Barbecue, Seal, Pack, Orbit, PetCare);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mini {
    pub kind: MiniKind,
    pub hard: bool,
    pub state: MiniState,
    pub mistakes: u32,
    pub done: bool,
}

impl Mini {
    pub fn new(kind: MiniKind, hard: bool, rng: &mut Rng) -> Self {
        Mini { kind, hard, state: MiniState::make(kind, hard, rng), mistakes: 0, done: false }
    }

    fn count(&mut self, r: Outcome) -> Outcome {
        match r {
            Outcome::Mistake => self.mistakes += 1,
            Outcome::Done => self.done = true,
            Outcome::Fine => {}
        }
        r
    }

    pub fn input(&mut self, i: MiniInput) -> Outcome {
        if self.done {
            return Outcome::Done;
        }
        let hard = self.hard;
        let r = match i {
            MiniInput::Wait(dt) => {
                // bots: time passes in small steps, like frames
                let mut left = dt;
                let mut r = Outcome::Fine;
                while left > 0.0 && r != Outcome::Done {
                    let s = left.min(0.05);
                    left -= s;
                    let o = self.state.play().tick(s, hard);
                    if o != Outcome::Fine {
                        r = o;
                    }
                }
                r
            }
            i => self.state.play().input(i, hard),
        };
        self.count(r)
    }

    /// The clock (the campaign calls this while someone works on the task).
    pub fn tick(&mut self, dt: f32) -> Outcome {
        if self.done {
            return Outcome::Done;
        }
        let hard = self.hard;
        let r = self.state.play().tick(dt, hard);
        self.count(r)
    }

    /// 0..1 for a progress bar.
    pub fn progress(&self) -> f32 {
        if self.done {
            1.0
        } else {
            self.state.view().progress().clamp(0.0, 0.99)
        }
    }

    /// What to draw now.
    pub fn scene(&self, l: Lang) -> Vec<Shape> {
        let mut out = vec![];
        let mut p = Pen { out: &mut out, l };
        p.rect(0.0, 0.0, CW, CH, 0x141a33);
        self.state.view().draw(&mut p, self.hard);
        out
    }

    /// The moves that finish this game (a perfect player). Used by tests and the bot.
    pub fn solution(&self) -> Vec<MiniInput> {
        let mut sim = self.clone();
        let mut out = vec![];
        for _ in 0..4000 {
            if sim.done {
                break;
            }
            let steps = sim.state.view().bot(sim.hard);
            if steps.is_empty() {
                out.push(MiniInput::Wait(0.05));
                sim.input(MiniInput::Wait(0.05));
                continue;
            }
            for s in steps {
                out.push(s);
                sim.input(s);
                if sim.done {
                    break;
                }
            }
        }
        out
    }

    /// The pet's kind for `PetCare` (set by the campaign).
    pub fn set_pet(&mut self, kind: u8) {
        if let MiniState::PetCare(g) = &mut self.state {
            g.pet = kind;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_minigame_can_be_finished_on_both_levels() {
        for seed in 0..20 {
            let mut rng = Rng::new(11 + seed);
            for kind in ALL_KINDS {
                for hard in [false, true] {
                    let m = Mini::new(kind, hard, &mut rng);
                    let sol = m.solution();
                    let mut m2 = m.clone();
                    let mut last = Outcome::Fine;
                    for i in sol {
                        last = m2.input(i);
                    }
                    assert_eq!(last, Outcome::Done, "{kind:?} hard={hard} seed={seed}");
                    assert!(m2.done && m2.mistakes == 0, "{kind:?} hard={hard}: {} mistakes", m2.mistakes);
                    let l = m2.scene(Lang::Ru);
                    assert!(l.len() > 3, "{kind:?} draws something");
                }
            }
        }
    }

    #[test]
    fn hard_versions_take_more_work() {
        let mut rng = Rng::new(5);
        let effort = |m: &Mini| m.solution().iter().filter(|i| !matches!(i, MiniInput::Wait(_))).count() as f32 + m.solution().iter().map(|i| if let MiniInput::Wait(t) = i { *t } else { 0.0 }).sum::<f32>();
        let mut harder = 0;
        for kind in ALL_KINDS {
            let (e, h) = (Mini::new(kind, false, &mut rng), Mini::new(kind, true, &mut rng));
            if effort(&h) >= effort(&e) {
                harder += 1;
            }
        }
        assert!(harder >= 19, "most hard versions are bigger: {harder}/23");
    }

    #[test]
    fn random_clicking_makes_mistakes_somewhere() {
        // a game you can't win by mashing: wrong moves are counted
        let mut rng = Rng::new(9);
        let mut any = 0;
        for kind in ALL_KINDS {
            let mut m = Mini::new(kind, true, &mut rng);
            let mut r = Rng::new(1);
            for _ in 0..80 {
                let (x, y) = (r.f32() * CW, r.f32() * CH);
                m.input(MiniInput::Press(x, y));
                m.input(MiniInput::Drag(r.f32() * CW, r.f32() * CH));
                m.input(MiniInput::Release(r.f32() * CW, r.f32() * CH));
                m.input(MiniInput::Wait(0.1));
            }
            if m.mistakes > 0 || !m.done {
                any += 1;
            }
        }
        assert!(any >= 21, "random clicking doesn't win: {any}/23");
    }
}
