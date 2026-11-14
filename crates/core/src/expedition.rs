//! Settings chosen before an expedition starts (the "Create expedition" screen). They never
//! change during play.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum PlayMode {
    /// resources run down, problems have timers, the ship can be lost
    #[default]
    Survival,
    /// free play: no drain, no accidents, tasks are optional
    Creative,
}

/// Two audiences in one game. Replaces the old `cadet: bool`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Level {
    /// 7–9: simple words, longer timers, one urgent problem at a time
    #[default]
    Junior,
    /// 10–12+: quantities with units, harder mini-games
    Advanced,
}

/// A role gives a small bonus but never forbids anything.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Role {
    /// asteroid hits do less damage
    Pilot,
    /// repairs: longer problem timers
    Engineer,
    /// more coins for science tasks
    Researcher,
    /// air and CO₂ drift slower
    LifeSupport,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ExpeditionConfig {
    pub mode: PlayMode,
    pub level: Level,
    /// 1..=4
    pub players: u8,
    pub roles: [Option<Role>; 4],
    pub seed: u64,
}

impl ExpeditionConfig {
    pub fn new(players: u8, level: Level) -> Self {
        Self { mode: PlayMode::Survival, level, players: players.clamp(1, 4), roles: [None; 4], seed: crate::rng::Rng::time_seed() }
    }
    /// Every crew member gets a role in seat order: pilot, engineer, researcher, life support.
    /// A solo player is the pilot. (A role is a small team bonus, never a limit.)
    pub fn with_auto_roles(mut self) -> Self {
        const ORDER: [Role; 4] = [Role::Pilot, Role::Engineer, Role::Researcher, Role::LifeSupport];
        for i in 0..4 {
            self.roles[i] = (i < self.players as usize).then_some(ORDER[i]);
        }
        self
    }
    pub fn has_role(&self, r: Role) -> bool {
        self.roles.iter().take(self.players as usize).any(|x| *x == Some(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_roles_follow_seats() {
        let c = ExpeditionConfig::new(2, Level::Junior).with_auto_roles();
        assert_eq!(c.roles, [Some(Role::Pilot), Some(Role::Engineer), None, None]);
        assert!(c.has_role(Role::Engineer) && !c.has_role(Role::Researcher));
        let four = ExpeditionConfig::new(4, Level::Advanced).with_auto_roles();
        assert!(four.has_role(Role::LifeSupport));
    }
}
