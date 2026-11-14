//! A3 — the six mini-game types, each with an easy (Junior) and a hard (Advanced) version.
//! Pure logic: the app draws `Mini::state` and sends `MiniInput`s; the host can check every move.
//! The same framework is used on the ship and (by part B) on the Moon.

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiniKind {
    /// connect wires of the same colour
    Wires,
    /// put each item into the right bin
    Sorting,
    /// hold to upload data, then tune the transmitter to the frequency
    Transmitter,
    /// type the coordinates shown
    Coordinates,
    /// clean the filter spots, then set the pressure into the green zone
    FilterPressure,
    /// patch every hole, then set the shield level
    PatchShield,
}

pub const ALL_KINDS: [MiniKind; 6] = [MiniKind::Wires, MiniKind::Sorting, MiniKind::Transmitter, MiniKind::Coordinates, MiniKind::FilterPressure, MiniKind::PatchShield];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MiniInput {
    /// pick a left wire / a filter spot / a hole
    Pick(usize),
    /// connect the picked wire to right socket N
    Link(usize),
    /// drop the current item into bin N
    Bin(u8),
    /// hold the button for this many seconds (upload)
    Hold(f32),
    Plus,
    Minus,
    Digit(u8),
    Erase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Fine,
    Mistake,
    Done,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MiniState {
    /// `left[i]` and `right[j]` are colour ids; `links[i]` is the socket wire i goes to
    Wires { left: Vec<u8>, right: Vec<u8>, links: Vec<Option<usize>>, picked: Option<usize> },
    /// `items[i]` is the bin item i belongs to; `next` is the item shown now
    Sorting { items: Vec<u8>, bins: u8, next: usize },
    /// stage 1: `upload` 0..1; stage 2: tune `freq` to `target`
    Transmitter { upload: f32, freq: i32, target: i32 },
    Coordinates { target: Vec<u8>, typed: Vec<u8> },
    /// clean every dirty spot, then bring `pressure` into `lo..=hi`
    FilterPressure { dirty: Vec<bool>, pressure: i32, lo: i32, hi: i32 },
    /// patch every hole, then set `shield` to `target`
    PatchShield { holes: Vec<bool>, shield: i32, target: i32 },
}

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
        let state = match kind {
            MiniKind::Wires => {
                let n = if hard { 5 } else { 3 };
                let left: Vec<u8> = (0..n as u8).collect();
                let mut right = left.clone();
                shuffle(&mut right, rng);
                MiniState::Wires { left, right, links: vec![None; n], picked: None }
            }
            MiniKind::Sorting => {
                let (n, bins) = if hard { (8, 4) } else { (5, 3) };
                MiniState::Sorting { items: (0..n).map(|_| rng.below(bins as usize) as u8).collect(), bins, next: 0 }
            }
            MiniKind::Transmitter => {
                let target = if hard { 3 + rng.below(13) as i32 } else { 3 + rng.below(5) as i32 };
                MiniState::Transmitter { upload: 0.0, freq: 0, target }
            }
            MiniKind::Coordinates => MiniState::Coordinates { target: (0..if hard { 6 } else { 4 }).map(|_| rng.below(10) as u8).collect(), typed: vec![] },
            MiniKind::FilterPressure => {
                let lo = if hard { 6 } else { 4 };
                MiniState::FilterPressure { dirty: vec![true; if hard { 6 } else { 4 }], pressure: 0, lo, hi: lo + if hard { 0 } else { 2 } }
            }
            MiniKind::PatchShield => MiniState::PatchShield { holes: vec![true; if hard { 5 } else { 3 }], shield: 0, target: if hard { 3 + rng.below(3) as i32 } else { 2 } },
        };
        Mini { kind, hard, state, mistakes: 0, done: false }
    }

    /// 0..1 for a progress bar.
    pub fn progress(&self) -> f32 {
        if self.done {
            return 1.0;
        }
        match &self.state {
            MiniState::Wires { links, .. } => links.iter().filter(|l| l.is_some()).count() as f32 / links.len() as f32,
            MiniState::Sorting { items, next, .. } => *next as f32 / items.len() as f32,
            MiniState::Transmitter { upload, freq, target } => upload * 0.5 + if *upload >= 1.0 { 0.5 * (1.0 - ((freq - target).abs() as f32 / (*target as f32).max(1.0)).min(1.0)) } else { 0.0 },
            MiniState::Coordinates { target, typed } => typed.len() as f32 / target.len() as f32,
            MiniState::FilterPressure { dirty, .. } => dirty.iter().filter(|d| !**d).count() as f32 / dirty.len() as f32 * 0.6,
            MiniState::PatchShield { holes, .. } => holes.iter().filter(|h| !**h).count() as f32 / holes.len() as f32 * 0.6,
        }
    }

    pub fn input(&mut self, i: MiniInput) -> Outcome {
        if self.done {
            return Outcome::Done;
        }
        let r = match (&mut self.state, i) {
            (MiniState::Wires { picked, links, .. }, MiniInput::Pick(a)) if a < links.len() => {
                *picked = Some(a);
                Outcome::Fine
            }
            (MiniState::Wires { left, right, links, picked }, MiniInput::Link(b)) if b < right.len() => match picked.take() {
                Some(a) if left[a] == right[b] && !links.contains(&Some(b)) => {
                    links[a] = Some(b);
                    if links.iter().all(|l| l.is_some()) { Outcome::Done } else { Outcome::Fine }
                }
                Some(_) => Outcome::Mistake,
                None => Outcome::Fine,
            },
            (MiniState::Sorting { items, next, .. }, MiniInput::Bin(b)) => {
                if items[*next] == b {
                    *next += 1;
                    if *next >= items.len() { Outcome::Done } else { Outcome::Fine }
                } else {
                    Outcome::Mistake
                }
            }
            (MiniState::Transmitter { upload, .. }, MiniInput::Hold(s)) if *upload < 1.0 => {
                *upload = (*upload + s / if self.hard { 3.0 } else { 2.0 }).min(1.0);
                Outcome::Fine
            }
            (MiniState::Transmitter { upload, freq, target }, d @ (MiniInput::Plus | MiniInput::Minus)) if *upload >= 1.0 => {
                *freq += if d == MiniInput::Plus { 1 } else { -1 };
                *freq = (*freq).clamp(0, 20);
                if freq == target { Outcome::Done } else { Outcome::Fine }
            }
            (MiniState::Coordinates { target, typed }, MiniInput::Digit(d)) if typed.len() < target.len() => {
                if target[typed.len()] == d {
                    typed.push(d);
                    if typed.len() == target.len() { Outcome::Done } else { Outcome::Fine }
                } else {
                    Outcome::Mistake
                }
            }
            (MiniState::Coordinates { typed, .. }, MiniInput::Erase) => {
                typed.pop();
                Outcome::Fine
            }
            (MiniState::FilterPressure { dirty, .. }, MiniInput::Pick(k)) if k < dirty.len() => {
                dirty[k] = false;
                Outcome::Fine
            }
            (MiniState::FilterPressure { dirty, pressure, lo, hi }, d @ (MiniInput::Plus | MiniInput::Minus)) => {
                if dirty.iter().any(|x| *x) {
                    Outcome::Mistake // clean the filter first
                } else {
                    *pressure = (*pressure + if d == MiniInput::Plus { 1 } else { -1 }).clamp(0, 10);
                    if (*lo..=*hi).contains(pressure) { Outcome::Done } else { Outcome::Fine }
                }
            }
            (MiniState::PatchShield { holes, .. }, MiniInput::Pick(k)) if k < holes.len() => {
                holes[k] = false;
                Outcome::Fine
            }
            (MiniState::PatchShield { holes, shield, target }, d @ (MiniInput::Plus | MiniInput::Minus)) => {
                if holes.iter().any(|x| *x) {
                    Outcome::Mistake // patch the holes first
                } else {
                    *shield = (*shield + if d == MiniInput::Plus { 1 } else { -1 }).clamp(0, 6);
                    if shield == target { Outcome::Done } else { Outcome::Fine }
                }
            }
            _ => Outcome::Fine,
        };
        match r {
            Outcome::Mistake => self.mistakes += 1,
            Outcome::Done => self.done = true,
            Outcome::Fine => {}
        }
        r
    }

    /// The moves that finish this mini-game (a perfect player). Used by tests and the bot.
    pub fn solution(&self) -> Vec<MiniInput> {
        let mut out = vec![];
        match &self.state {
            MiniState::Wires { left, right, .. } => {
                for (a, c) in left.iter().enumerate() {
                    out.push(MiniInput::Pick(a));
                    out.push(MiniInput::Link(right.iter().position(|x| x == c).unwrap_or(0)));
                }
            }
            MiniState::Sorting { items, next, .. } => out.extend(items[*next..].iter().map(|b| MiniInput::Bin(*b))),
            MiniState::Transmitter { target, .. } => {
                out.extend(std::iter::repeat_n(MiniInput::Hold(0.5), 7));
                out.extend(std::iter::repeat_n(MiniInput::Plus, *target as usize));
            }
            MiniState::Coordinates { target, .. } => out.extend(target.iter().map(|d| MiniInput::Digit(*d))),
            MiniState::FilterPressure { dirty, lo, .. } => {
                out.extend((0..dirty.len()).map(MiniInput::Pick));
                out.extend(std::iter::repeat_n(MiniInput::Plus, *lo as usize));
            }
            MiniState::PatchShield { holes, target, .. } => {
                out.extend((0..holes.len()).map(MiniInput::Pick));
                out.extend(std::iter::repeat_n(MiniInput::Plus, *target as usize));
            }
        }
        out
    }
}

fn shuffle<T>(v: &mut [T], rng: &mut Rng) {
    for i in (1..v.len()).rev() {
        v.swap(i, rng.below(i + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_minigame_can_be_finished_on_both_levels() {
        let mut rng = Rng::new(11);
        for kind in ALL_KINDS {
            for hard in [false, true] {
                let mut m = Mini::new(kind, hard, &mut rng);
                let mut last = Outcome::Fine;
                for i in m.solution() {
                    last = m.input(i);
                }
                assert_eq!(last, Outcome::Done, "{kind:?} hard={hard}");
                assert!(m.done && m.mistakes == 0 && m.progress() == 1.0);
            }
        }
    }

    #[test]
    fn mistakes_are_counted_and_order_matters() {
        let mut rng = Rng::new(3);
        let mut m = Mini::new(MiniKind::FilterPressure, false, &mut rng);
        assert_eq!(m.input(MiniInput::Plus), Outcome::Mistake, "pressure before cleaning");
        let mut w = Mini::new(MiniKind::Wires, false, &mut rng);
        let MiniState::Wires { left, right, .. } = w.state.clone() else { unreachable!() };
        let wrong = right.iter().position(|c| *c != left[0]).unwrap();
        w.input(MiniInput::Pick(0));
        assert_eq!(w.input(MiniInput::Link(wrong)), Outcome::Mistake);
        assert_eq!(w.mistakes, 1);
    }

    #[test]
    fn hard_versions_are_bigger() {
        let mut rng = Rng::new(5);
        let len = |m: &Mini| m.solution().len();
        for kind in ALL_KINDS {
            let (e, h) = (Mini::new(kind, false, &mut rng), Mini::new(kind, true, &mut rng));
            if kind != MiniKind::Transmitter {
                assert!(len(&h) > len(&e), "{kind:?}");
            }
        }
    }
}
