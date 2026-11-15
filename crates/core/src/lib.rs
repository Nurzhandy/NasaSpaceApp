//! # ja-core — rules of “Junior Astronaut: Outpost” (2D)
//!
//! Part A of `docs/PLAN_2D.md`. Pure Rust, no Bevy: the app (part B, `crates/game`) only
//!
//! 1. creates a mission: `Game::new(MissionConfig::new(MapId::Moon), &weather)`,
//! 2. calls `game.tick(dt)` every frame and `game.set_player_pos(i, x, y)` as astronauts walk,
//! 3. sends player actions: `game.apply(player, Command::Build { .. })` → `Ok(events)` or `Err(Refusal)`,
//! 4. draws `game.state()` and shows `i18n::event_msg(&event)` / `i18n::refusal_msg(refusal)`,
//! 5. on `GameEvent::MissionEnded(result)` calls `profile.finish(&result)` and `progress::save(&profile)`.
//!
//! Live NASA DONKI: `weather::spawn_fetch(weather::api_key(&profile.nasa_key))` returns a channel
//! that delivers one `Weather` (live or the honest archive). Poll it with `try_recv`.
//!
//! Tests: `cargo test -p ja-core` · balance report: `cargo test -p ja-core --release balance -- --ignored --nocapture`.

pub mod achieve;
pub mod data;
pub mod game;
pub mod i18n;
pub mod landing;
pub mod progress;
pub mod role;
pub mod rng;
pub mod survival;
pub mod ui_text;
pub mod weather;

#[cfg(test)]
mod balance;

pub use data::{EventKind, Goal, MapId, ModuleKind, SiteKind, SkinSlot, Unlock, Wave};
pub use game::{region_at, BossState, Cause, Command, Forecast, Game, GameEvent, MissionConfig, MissionResult, Mode, ModuleId, Refusal, SiteId, State, Status};
pub use i18n::{Lang, Msg, Tone};
pub use achieve::{Track, TRACKS};
pub use progress::{Book, Profile, Stats};
pub use role::{Role, ROLES};
pub use survival::{Body, EventReport, ReportLine, Survival, Threat, Tonight, Upgrade};
pub use landing::{Lander, LanderInput, LandingEvent};
pub use weather::Weather;
