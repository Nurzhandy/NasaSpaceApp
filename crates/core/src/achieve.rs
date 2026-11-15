//! Achievements in six directions (spec §7): explorer, engineer, pilot, survival specialist,
//! scientist, team player. Each grows from lifetime counters in the profile and has three
//! tiers (bronze, silver, gold). Nothing is ever taken away.

use crate::progress::Stats;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Track {
    Explorer,
    Engineer,
    Pilot,
    Survivor,
    Scientist,
    Team,
}

pub const TRACKS: [Track; 6] = [Track::Explorer, Track::Engineer, Track::Pilot, Track::Survivor, Track::Scientist, Track::Team];

impl Track {
    pub fn icon(self) -> &'static str {
        match self {
            Track::Explorer => "rover",
            Track::Engineer => "wrench",
            Track::Pilot => "rocket",
            Track::Survivor => "shield",
            Track::Scientist => "science",
            Track::Team => "coop",
        }
    }
    /// Text keys in ui.json: `ach.<id>` (name) and `ach.<id>.what` (what counts).
    pub fn id(self) -> &'static str {
        match self {
            Track::Explorer => "explorer",
            Track::Engineer => "engineer",
            Track::Pilot => "pilot",
            Track::Survivor => "survivor",
            Track::Scientist => "scientist",
            Track::Team => "team",
        }
    }
    /// Counter needed for bronze, silver, gold.
    pub fn tiers(self) -> [u32; 3] {
        match self {
            Track::Explorer => [5, 20, 50],
            Track::Engineer => [10, 40, 100],
            Track::Pilot => [3, 12, 30],
            Track::Survivor => [3, 15, 40],
            Track::Scientist => [10, 50, 150],
            Track::Team => [1, 5, 15],
        }
    }
    /// The lifetime counter this direction grows from.
    pub fn value(self, s: &Stats) -> u32 {
        match self {
            // samples picked up + regions reached
            Track::Explorer => s.collected + s.regions,
            // repairs, modules built, upgrades bought
            Track::Engineer => s.repairs + s.built + s.upgrades,
            // stars from landings
            Track::Pilot => s.landing_stars,
            // sols lived through in survival + evening threats handled
            Track::Survivor => s.sols + s.countered,
            Track::Scientist => s.science,
            // missions together + times a crewmate was helped
            Track::Team => s.coop + s.helped,
        }
    }
    /// 0 = nothing yet, 1 bronze, 2 silver, 3 gold.
    pub fn tier(self, s: &Stats) -> u8 {
        let v = self.value(s);
        self.tiers().iter().filter(|t| v >= **t).count() as u8
    }
    /// Progress to the next tier: (have, need); `None` when gold is reached.
    pub fn next(self, s: &Stats) -> Option<(u32, u32)> {
        let t = self.tier(s) as usize;
        self.tiers().get(t).map(|need| (self.value(s), *need))
    }
}

/// Stars given when a tier is reached (spent on cosmetics, never on power).
pub const TIER_STARS: [u32; 3] = [10, 20, 40];

/// Tiers that `after` has and `before` did not.
pub fn new_tiers(before: &Stats, after: &Stats) -> Vec<(Track, u8)> {
    let mut out = vec![];
    for t in TRACKS {
        for k in t.tier(before)..t.tier(after) {
            out.push((t, k + 1));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_grow_and_report_once() {
        let a = Stats::default();
        assert!(TRACKS.iter().all(|t| t.tier(&a) == 0));
        let b = Stats { landing_stars: 13, coop: 1, ..Default::default() };
        assert_eq!(Track::Pilot.tier(&b), 2);
        assert_eq!(Track::Pilot.next(&b), Some((13, 30)));
        assert_eq!(new_tiers(&a, &b), vec![(Track::Pilot, 1), (Track::Pilot, 2), (Track::Team, 1)]);
        assert!(new_tiers(&b, &b).is_empty());
        let gold = Stats { science: 999, ..Default::default() };
        assert_eq!(Track::Scientist.tier(&gold), 3);
        assert_eq!(Track::Scientist.next(&gold), None);
    }
}
