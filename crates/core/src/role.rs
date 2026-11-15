//! Crew roles: an optional specialty that gives a small bonus but never forbids anything.
//! Anyone can still repair, collect, fly and breathe — the role just makes one job easier.
//!
//! Part A's contract (`expedition.rs`) should reuse this enum rather than define its own.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    #[default]
    None,
    Pilot,
    Engineer,
    Researcher,
    LifeSupport,
}

pub const ROLES: [Role; 5] = [Role::None, Role::Pilot, Role::Engineer, Role::Researcher, Role::LifeSupport];

/// Landing: more fuel in the tanks.
pub const PILOT_FUEL: f32 = 1.2;
/// Repairs restore more strength / tightness.
pub const ENGINEER_REPAIR: f32 = 1.5;
/// Science from delivered samples.
pub const RESEARCHER_SCIENCE: f32 = 1.25;
/// Suit oxygen use outside.
pub const LIFE_SUPPORT_SUIT: f32 = 0.75;

impl Role {
    pub fn icon(self) -> &'static str {
        match self {
            Role::None => "astronaut",
            Role::Pilot => "rocket",
            Role::Engineer => "wrench",
            Role::Researcher => "science",
            Role::LifeSupport => "oxygen",
        }
    }
    /// Text key in `assets/content/ui.json`: `role.<id>` (name) and `role.<id>.bonus`.
    pub fn id(self) -> &'static str {
        match self {
            Role::None => "none",
            Role::Pilot => "pilot",
            Role::Engineer => "engineer",
            Role::Researcher => "researcher",
            Role::LifeSupport => "life_support",
        }
    }
    pub fn next(self) -> Role {
        let i = ROLES.iter().position(|r| *r == self).unwrap_or(0);
        ROLES[(i + 1) % ROLES.len()]
    }
}
