# SPACE EXPLORERS — NASA Space Apps 2026 presentation

7 slides (the Space Apps limit, title included), in English.

- **[SPACE_EXPLORERS_SpaceApps2026.pdf](SPACE_EXPLORERS_SpaceApps2026.pdf)** — the deck
- [index.html](index.html) — the same slides as a web page (speaker notes are in the `<aside>` of each slide)
- `img/` — real in-game screenshots used in the slides

## Where each claim comes from

| Claim on the slides | Source in this repo |
|---|---|
| 23 mini-games, 2-day flight | `crates/core/src/minigame/mod.rs` (`MiniKind`), `assets/content/missions/ship.json` (`days`) |
| 1,200 test missions; careful bots 599/600, careless 0/600 | `cargo test -p ja-core --release survival_balance -- --ignored --nocapture` (`crates/core/src/balance.rs`) |
| +6 oxygen / crew breathes 12, +5 food / crew eats 10 | power console texts (`img/07_power_console.jpg`) |
| Dome 21 → 11 °C, 15 °C with insulation | evening report (`img/29_report.jpg`), `crates/core/src/survival.rs` |
| 1.3 s signal delay, 1.62 m/s² | `crates/core/src/data.rs`, `crates/core/src/game.rs` (forecast delay), landing screen |
| DONKI live API + labelled archive | `crates/core/src/weather.rs` |
| Dose 0.64 mSv/day (Mars), 1.369 mSv/day (Moon) | `crates/core/src/data.rs` → `SOURCES` (Hassler 2014; Zhang 2020) |
| Up to 4 computers on one Wi-Fi | `crates/game/src/net.rs` |
| Crew always evacuates safely | `crates/core/src/game.rs` (end conditions), `crates/core/src/i18n.rs` |
| Kazakh (beta) | `assets/content/ui.json` is a draft (107 of 121 strings) pending a native-speaker check |
