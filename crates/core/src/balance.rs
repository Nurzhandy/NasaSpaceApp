//! Monte-Carlo balance check (A7). Two simulated kids play through the public API only:
//! a careful one (builds a shield, follows forecasts, hides from flares) and a careless one.
//! Report: `cargo test -p ja-core --release balance -- --ignored --nocapture`

use crate::data::*;
use crate::game::*;
use crate::survival::{Threat, Upgrade};
use crate::weather::Weather;

#[derive(Clone, Copy)]
struct Policy {
    /// seconds to notice a problem
    react: f32,
    careful: bool,
}

const CAREFUL: Policy = Policy { react: 9.0, careful: true };
const CARELESS: Policy = Policy { react: 9.0, careful: false };
/// walking speed in cells per second
const SPEED: f32 = 4.0;

fn ids(s: &State, kind: ModuleKind) -> Vec<ModuleId> {
    s.modules.iter().filter(|m| m.kind == kind).map(|m| m.id).collect()
}

fn go_to_module(g: &mut Game, id: ModuleId) -> f32 {
    let (from, c) = {
        let s = g.state();
        (s.players[0].pos, s.module(id).map(|m| m.center()).unwrap_or((0.0, 0.0)))
    };
    let to = (c.0, c.1 + 2.0);
    g.set_player_pos(0, to.0, to.1);
    ((to.0 - from.0).hypot(to.1 - from.1)) / SPEED
}

/// Desired power per consumer, trimmed to what the base can afford.
fn allocate(g: &mut Game, pol: Policy) {
    let s = g.state();
    let d = s.def();
    let leak = if s.leak { 4.0 } else { 0.0 };
    let ox_ids = ids(s, ModuleKind::OxygenGen);
    let broken = s.modules.iter().any(|m| m.kind == ModuleKind::OxygenGen && m.broken);
    let mut ox = ((O2_USE + leak) / (O2_PER * if broken { 0.5 } else { 1.0 })).ceil() as u32 + (s.o2 < 45.0) as u32;
    let mut green = if s.cadet || s.count(ModuleKind::Greenhouse) == 0 { 0 } else { (FOOD_USE / FOOD_PER).ceil() as u32 + (s.food < 40.0) as u32 };
    let flare_soon = s.flare.is_some() || matches!(s.forecast, Some(Forecast::Sure(EventKind::Flare)) | Some(Forecast::Maybe(EventKind::Flare)));
    let base = if !pol.careful { 0 } else if d.dose >= 12.0 { 3 } else if d.dose >= 9.0 { 2 } else if d.dose >= 3.0 { 1 } else { 0 };
    let shields = ids(s, ModuleKind::Shield);
    let mut shield = if shields.is_empty() { 0 } else if pol.careful && flare_soon && d.dose > 1.0 { SHIELD_CAP } else { base };
    let antenna = 1u32;
    let budget = (s.supply.floor() as u32 + (s.battery > 3.0) as u32 + (s.battery > 6.0) as u32).min(s.available());
    ox = ox.min(ox_ids.len() as u32 * 3);
    let mut ant = antenna.min(ids(s, ModuleKind::Antenna).len() as u32);
    while ox + green + shield + ant > budget {
        if shield > 1 { shield -= 1 } else if green > 1 { green -= 1 } else if ant > 0 { ant -= 1 } else if shield > 0 { shield -= 1 } else if green > 0 { green -= 1 } else if ox > 1 { ox -= 1 } else { break }
    }
    let labs = ids(s, ModuleKind::Lab);
    let lab = budget.saturating_sub(ox + green + shield + ant).min(labs.len() as u32 * 3);
    let mut want: Vec<(ModuleId, u8)> = vec![];
    let mut spread = |list: &[ModuleId], mut n: u32, max: u32| {
        for id in list {
            let k = n.min(max);
            want.push((*id, k as u8));
            n -= k;
        }
    };
    spread(&ox_ids, ox, 3);
    spread(&ids(s, ModuleKind::Greenhouse), green, 3);
    spread(&shields, shield, 3);
    spread(&ids(s, ModuleKind::Antenna), ant, 1);
    spread(&labs, lab, 3);
    // survival: the heater when it is cold or a cold evening is coming
    if s.mode == Mode::Survival && pol.careful {
        let cold_soon = matches!(s.surv.tonight.or(match s.forecast { Some(Forecast::Tonight(t)) => Some(t), _ => None }), Some(t) if t.threat == Threat::Cold || t.second == Some(Threat::Cold)) || s.is_night();
        let slots = s.surv.level(Upgrade::Heater);
        let heat = if s.surv.temp < 17.0 || cold_soon { slots } else { 0 };
        want.push((s.habitat().id, heat));
    }
    let cur: Vec<(ModuleId, u8)> = want.iter().map(|(id, _)| (*id, s.module(*id).map(|m| m.power).unwrap_or(0))).collect();
    // lower first, then raise in priority order
    for ((id, w), (_, c)) in want.iter().zip(&cur) {
        if w < c {
            let _ = g.apply(0, Command::SetPower { module: *id, blocks: *w });
        }
    }
    for ((id, w), (_, c)) in want.iter().zip(&cur) {
        if w > c && g.apply(0, Command::SetPower { module: *id, blocks: *w }).is_err() {
            break;
        }
    }
}

fn try_build(g: &mut Game, pol: Policy) -> bool {
    let s = g.state();
    let d = s.def();
    let wants: Vec<ModuleKind> = {
        let mut v = vec![];
        if pol.careful && d.dose >= 3.0 && s.count(ModuleKind::Shield) == 0 {
            v.push(ModuleKind::Shield);
        }
        if s.count(ModuleKind::Lab) == 0 {
            v.push(ModuleKind::Lab);
        }
        if pol.careful && !d.night_sols.is_empty() && s.count(ModuleKind::Battery) < 2 {
            v.push(ModuleKind::Battery);
        }
        if (s.demand as f32) > s.supply {
            v.push(if module_def(ModuleKind::Rtg).only.contains(&s.map) && d.panel_yield < 1.0 { ModuleKind::Rtg } else { ModuleKind::Solar });
        }
        v
    };
    for kind in wants {
        if g.state().coins < crate::survival::build_cost(kind, &g.state().surv) {
            return false; // save up for the most important one
        }
        if let Some((x, y)) = g.find_free_spot(kind) {
            return g.apply(0, Command::Build { kind, x, y }).is_ok();
        }
    }
    false
}

/// Survival: buy what tonight's forecast calls for (careful kids only).
fn shop(g: &mut Game) -> bool {
    let s = g.state();
    let tonight = match s.forecast {
        Some(Forecast::Tonight(t)) => Some(t),
        _ => None,
    };
    let mut wants = vec![Upgrade::Insulation, Upgrade::Heater];
    if let Some(t) = tonight {
        for th in [Some(t.threat), t.second].into_iter().flatten() {
            match th {
                Threat::Cold => wants.insert(0, Upgrade::Insulation),
                Threat::Meteors => wants.insert(0, Upgrade::Armor),
                Threat::Dust | Threat::Failure => wants.push(Upgrade::Service),
                _ => {}
            }
        }
    }
    if s.sol >= 3 {
        wants.push(Upgrade::Insulation);
        wants.push(Upgrade::Heater);
    }
    for u in wants {
        let Some(cost) = g.state().surv.next_cost(u) else { continue };
        if g.state().coins >= cost + 4 {
            return g.apply(0, Command::Upgrade(u)).is_ok();
        }
    }
    false
}

fn play_mode(map_id: MapId, cadet: bool, seed: u64, pol: Policy, survival: bool) -> State {
    let w = Weather::archive();
    let cfg = MissionConfig { map: map_id, cadet, coop: false, live: false, seed };
    let mut g = if survival { Game::survival(cfg, 1, 6, &w) } else { Game::new(cfg, &w) };
    let dt = 0.25;
    let (mut busy, mut t_flare, mut t_problem) = (0.0f32, 0.0f32, 0.0f32);
    while g.state().status == Status::Playing {
        g.tick(dt);
        busy = (busy - dt).max(0.0);
        allocate(&mut g, pol);
        let (flare, inside, problem, rest) = {
            let s = g.state();
            let surv = s.mode == Mode::Survival;
            let problem = s.modules.iter().any(|m| (m.kind == ModuleKind::OxygenGen && m.broken) || (surv && m.broken) || m.dusty) || s.leak || (surv && s.surv.seal < 100.0);
            // survival: careful kids sleep in the dome in the evening
            let rest = surv && pol.careful && s.t >= crate::survival::EVENING && !problem;
            (s.flare.is_some(), s.players[0].inside, problem, rest)
        };
        // flares: hide in the dome, come out afterwards
        t_flare = if flare { t_flare + dt } else { 0.0 };
        if pol.careful && flare && !inside && t_flare > pol.react * 0.6 {
            g.set_player_pos(0, DOOR.0, DOOR.1);
            let _ = g.apply(0, Command::EnterShelter);
            continue;
        }
        if inside {
            if !flare && !rest {
                let _ = g.apply(0, Command::ExitShelter);
            }
            continue;
        }
        if rest && busy <= 0.0 {
            g.set_player_pos(0, DOOR.0, DOOR.1);
            let _ = g.apply(0, Command::EnterShelter);
            continue;
        }
        // survival: refill the suit at the airlock
        if g.state().mode == Mode::Survival && g.state().players[0].body.suit_o2 < 35.0 {
            g.set_player_pos(0, DOOR.0, DOOR.1);
            busy = busy.max(1.0);
            continue;
        }
        if busy > 0.0 {
            continue;
        }
        // repairs and cleaning
        t_problem = if problem { t_problem + dt } else { 0.0 };
        if t_problem > pol.react {
            let target = {
                let s = g.state();
                let surv = s.mode == Mode::Survival;
                s.modules.iter().find(|m| (m.kind == ModuleKind::OxygenGen && m.broken) || (surv && m.broken) || (m.kind == ModuleKind::Habitat && (s.leak || (surv && s.surv.seal < 100.0))) || m.dusty).map(|m| (m.id, m.dusty && !m.broken))
            };
            if let Some((id, dusty)) = target {
                busy = go_to_module(&mut g, id) + 3.0;
                let _ = g.apply(0, if dusty { Command::Clean { module: id } } else { Command::Repair { module: id } });
                continue;
            }
        }
        // greenhouse, cargo, crew
        let (harvest, cargo, mood) = {
            let s = g.state();
            (s.harvest_ready, s.cargo, s.mood)
        };
        if harvest {
            if let Some(id) = ids(g.state(), ModuleKind::Greenhouse).first().copied() {
                busy = go_to_module(&mut g, id) + 1.0;
                let _ = g.apply(0, Command::ChooseHarvest(true));
                continue;
            }
        }
        if let Some(pad) = cargo {
            g.set_player_pos(0, pad.0, pad.1);
            busy = 3.0;
            let all = g.state().battery > 2.0;
            let _ = g.apply(0, Command::ChooseCargo(all));
            continue;
        }
        if let Some(m) = mood {
            if !m.named {
                let f = FEELINGS.iter().position(|f| MOODS[m.mood].ok.contains(&f.id)).unwrap_or(0);
                let _ = g.apply(0, Command::NameFeeling(f));
            } else if g.state().battery > 1.0 {
                let _ = g.apply(0, Command::Support(0));
            }
            busy = 4.0;
            continue;
        }
        // build, then deliver samples, then explore
        if survival && pol.careful && shop(&mut g) {
            busy = 1.0;
            continue;
        }
        if try_build(&mut g, pol) {
            busy = 4.0;
            continue;
        }
        let (carried, lab_on) = {
            let s = g.state();
            (s.players[0].carried, s.powered(ModuleKind::Lab) > 0)
        };
        if carried >= 2 && lab_on {
            if let Some(id) = ids(g.state(), ModuleKind::Lab).first().copied() {
                busy = go_to_module(&mut g, id) + 1.0;
                let _ = g.apply(0, Command::DeliverSamples);
                continue;
            }
        }
        let target = {
            let s = g.state();
            let p = s.players[0].pos;
            let want_ore = s.coins < 12;
            s.sites
                .iter()
                .filter(|st| st.region.map_or(true, |r| s.region_open(r)))
                .filter(|st| match st.kind {
                    SiteKind::Sample => carried < CARRY_MAX,
                    SiteKind::Ore | SiteKind::Wreck => want_ore,
                    SiteKind::Ice => s.o2 < 70.0,
                })
                .min_by(|a, b| (a.x - p.0).hypot(a.y - p.1).total_cmp(&(b.x - p.0).hypot(b.y - p.1)))
                .map(|st| (st.id, st.x, st.y, (st.x - p.0).hypot(st.y - p.1)))
        };
        if let Some((site, x, y, dist)) = target {
            g.set_player_pos(0, x, y);
            let _ = g.apply(0, Command::Collect { site });
            busy = dist / SPEED + 1.0;
        }
    }
    g.state().clone()
}

#[derive(Default)]
struct Stats {
    runs: u32,
    wins: u32,
    stars: u32,
    dose: f32,
    lost: [u32; 4],
}

fn run(map_id: MapId, cadet: bool, pol: Policy, n: u32) -> Stats {
    run_mode(map_id, cadet, pol, n, false)
}

fn run_mode(map_id: MapId, cadet: bool, pol: Policy, n: u32, survival: bool) -> Stats {
    let mut st = Stats::default();
    for i in 0..n {
        let s = play_mode(map_id, cadet, 0x9E37_79B9 ^ (i as u64).wrapping_mul(0x2545_F491_4F6C_DD1D), pol, survival);
        st.runs += 1;
        st.dose += s.dose;
        match s.status {
            Status::Won => {
                st.wins += 1;
                st.stars += s.stars();
            }
            Status::Lost(Cause::Oxygen) => st.lost[0] += 1,
            Status::Lost(Cause::Cold) => st.lost[3] += 1,
            Status::Lost(Cause::Food) => st.lost[1] += 1,
            Status::Lost(Cause::Dose) => st.lost[2] += 1,
            Status::Playing => {}
        }
    }
    st
}

#[test]
fn a_careful_kid_can_win_the_moon() {
    let st = run(MapId::Moon, false, CAREFUL, 20);
    assert!(st.wins >= 14, "{} / 20", st.wins);
}

#[test]
#[ignore]
fn balance() {
    let n = 300;
    println!("{:<7} {:<6} {:<9} {:>5} {:>6} {:>6}  lost O2/food/dose", "map", "age", "player", "win%", "stars", "dose");
    let mut careful = vec![];
    let mut careless = vec![];
    for m in MAPS {
        for cadet in [true, false] {
            for (name, pol) in [("careful", CAREFUL), ("careless", CARELESS)] {
                let st = run(m.id, cadet, pol, n);
                let win = st.wins as f32 / st.runs as f32 * 100.0;
                println!(
                    "{:<7} {:<6} {:<9} {:>5.0} {:>6.2} {:>6.1}  {}/{}/{}",
                    format!("{:?}", m.id), if cadet { "7-9" } else { "10-12" }, name, win,
                    st.stars as f32 / st.wins.max(1) as f32, st.dose / st.runs as f32, st.lost[0], st.lost[1], st.lost[2]
                );
                if pol.careful { careful.push((m.id, cadet, win)) } else { careless.push((m.id, cadet, win)) }
            }
        }
    }
    for ((id, cadet, win), (_, _, bad)) in careful.iter().zip(&careless) {
        assert!(*win >= 70.0, "{id:?} cadet={cadet}: careful kid wins only {win:.0}%");
        if map(*id).dose >= 9.0 {
            assert!(bad + 20.0 <= *win, "{id:?} cadet={cadet}: being careful must matter ({bad:.0}% vs {win:.0}%)");
        }
    }
}

#[test]
fn a_careful_crew_survives_the_moon() {
    for cadet in [true, false] {
        let st = run_mode(MapId::Moon, cadet, CAREFUL, 20, true);
        assert!(st.wins >= 15, "cadet={cadet}: {} / 20, lost O2/food/dose {:?}", st.wins, st.lost);
    }
}

/// Survival report: `cargo test -p ja-core --release survival_balance -- --ignored --nocapture`
#[test]
#[ignore]
fn survival_balance() {
    let n = 300;
    println!("survival on the Moon ({n} runs each)");
    println!("{:<8} {:<9} {:>5} {:>6}  lost O2/food/dose/cold", "level", "player", "win%", "dose");
    let mut res = vec![];
    for cadet in [true, false] {
        for (name, pol) in [("careful", CAREFUL), ("careless", CARELESS)] {
            let st = run_mode(MapId::Moon, cadet, pol, n, true);
            let win = st.wins as f32 / st.runs as f32 * 100.0;
            println!("{:<8} {:<9} {:>5.0} {:>6.1}  {}/{}/{}/{}", if cadet { "Junior" } else { "Advanced" }, name, win, st.dose / st.runs as f32, st.lost[0], st.lost[1], st.lost[2], st.lost[3]);
            res.push(win);
        }
    }
    assert!(res[0] >= 80.0 && res[2] >= 75.0, "careful crews must survive: {res:?}");
    assert!(res[3] + 15.0 <= res[2], "preparing must matter for Advanced: {res:?}");
}
