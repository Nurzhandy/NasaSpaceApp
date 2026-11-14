//! Proximity interactions: walk up to a module and press the action key (hold for repairs).

use crate::player::{Inputs, Player};
use crate::sim::Sim;
use crate::ui::{Alert, Alerts, Modal, ModalKind, Prompt, Prompts};
use crate::world::{Station, StationKind};
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct Hold {
    pub kind: Option<StationKind>,
    pub t: f32,
}

const HOLD_TIME: f32 = 3.0;

pub fn interact_system(
    mut commands: Commands,
    time: Res<Time>,
    inputs: Res<Inputs>,
    mut sim: ResMut<Sim>,
    mut modal: ResMut<Modal>,
    mut prompts: ResMut<Prompts>,
    mut alerts: ResMut<Alerts>,
    mut profile: ResMut<crate::save::Profile>,
    stations: Query<(Entity, &GlobalTransform, &Station)>,
    mut players: Query<(&mut Player, &Transform, &mut Hold)>,
    mut sfx: crate::audio::Sfx,
) {
    let dt = time.delta_secs();
    for (mut p, tr, mut hold) in &mut players {
        let idx = p.idx as usize;
        let inp = inputs.0[idx];
        prompts.0[idx] = None;
        if modal.blocks(p.idx) || p.frozen {
            hold.t = 0.0;
            continue;
        }
        // inside the shelter: only "exit"
        if p.in_shelter {
            prompts.0[idx] = Some(Prompt { icon: "exit", ru: "Выйти", en: "Exit", progress: None, warn: false });
            if inp.act {
                p.in_shelter = false;
                sfx.play("click");
            }
            continue;
        }
        // nearest station in range
        let pos = tr.translation;
        let mut best: Option<(Entity, StationKind, f32)> = None;
        for (e, gt, st) in &stations {
            let d = gt.translation().xz().distance(pos.xz());
            if d <= st.radius && best.map_or(true, |b| d < b.2) {
                best = Some((e, st.kind, d));
            }
        }
        let Some((entity, kind, _)) = best else {
            hold.t = 0.0;
            continue;
        };
        // what can be done here right now?
        let (prompt, holdable): (Option<Prompt>, bool) = match kind {
            StationKind::Board => (Some(Prompt { icon: "power", ru: "Энергия", en: "Power", progress: None, warn: false }), false),
            StationKind::Airlock => (Some(Prompt { icon: "house", ru: "В убежище", en: "Shelter", progress: None, warn: sim.flare.is_some() }), false),
            StationKind::Generator if sim.gen_broken => (Some(Prompt { icon: "wrench", ru: "Чинить (держи)", en: "Repair (hold)", progress: Some(hold.t / HOLD_TIME), warn: true }), true),
            StationKind::Panels if sim.dust_left > 0 && !sim.panels_cleaned => (Some(Prompt { icon: "broom", ru: "Чистить (держи)", en: "Clean (hold)", progress: Some(hold.t / HOLD_TIME), warn: true }), true),
            StationKind::Leak if sim.leak => (Some(Prompt { icon: "hammer", ru: "Заделать (держи)", en: "Patch (hold)", progress: Some(hold.t / HOLD_TIME), warn: true }), true),
            StationKind::Greenhouse if sim.harvest_ready => (Some(Prompt { icon: "plant", ru: "Урожай", en: "Harvest", progress: None, warn: false }), false),
            StationKind::Lab if p.carried > 0 => {
                if sim.alloc.sci > 0 {
                    (Some(Prompt { icon: "science", ru: "Сдать образцы", en: "Deliver samples", progress: None, warn: false }), false)
                } else {
                    (Some(Prompt { icon: "power", ru: "Лаборатории нужна энергия", en: "Lab needs power", progress: None, warn: true }), false)
                }
            }
            StationKind::Crewmate if sim.emotion.is_some() => (Some(Prompt { icon: "speech", ru: "Поговорить", en: "Talk", progress: None, warn: false }), false),
            StationKind::Cargo if sim.cargo => (Some(Prompt { icon: "box", ru: "Разгрузить", en: "Unload", progress: None, warn: false }), false),
            StationKind::Sample => (Some(Prompt { icon: "gem", ru: "Взять образец", en: "Pick up", progress: None, warn: false }), false),
            _ => (None, false),
        };
        let Some(prompt) = prompt else {
            hold.t = 0.0;
            continue;
        };
        prompts.0[idx] = Some(prompt);

        if holdable {
            if inp.act_held {
                if hold.kind != Some(kind) {
                    hold.kind = Some(kind);
                    hold.t = 0.0;
                }
                let before = hold.t;
                hold.t += dt;
                if (hold.t * 3.0).floor() > (before * 3.0).floor() {
                    sfx.play_vol("repair", 0.5);
                }
                if hold.t >= HOLD_TIME {
                    hold.t = 0.0;
                    hold.kind = None;
                    match kind {
                        StationKind::Generator => {
                            sim.repair_generator();
                            alerts.push(Alert::good("check", "Генератор починен · −2 ⚡", "Generator fixed · −2 ⚡"));
                        }
                        StationKind::Panels => {
                            sim.clean_panels();
                            alerts.push(Alert::good("check", "Панели чистые · энергии больше", "Panels clean · more power"));
                        }
                        StationKind::Leak => {
                            sim.patch_leak();
                            alerts.push(Alert::good("check", "Утечка заделана · −1 ⚡", "Leak patched · −1 ⚡"));
                        }
                        _ => {}
                    }
                    sfx.play("good");
                }
            } else {
                hold.t = 0.0;
                hold.kind = None;
            }
            continue;
        }

        if !inp.act {
            continue;
        }
        sfx.play("click");
        match kind {
            StationKind::Board => modal.open(ModalKind::Board, p.idx),
            StationKind::Airlock => {
                p.in_shelter = true;
                if sim.flare.is_some() {
                    sim.flags.sheltered_in_flare = true;
                }
            }
            StationKind::Greenhouse => modal.open(ModalKind::Harvest, p.idx),
            StationKind::Lab => {
                let gain = sim.deliver_samples(p.carried);
                if gain > 0.0 {
                    alerts.push(Alert::good("science", "Образцы в лаборатории", "Samples delivered"));
                    sfx.play("coin");
                    p.carried = 0;
                    if sim.samples_delivered >= 6 && profile.award("collector") {
                        alerts.push(Alert::badge("gem", "Нашивка: Коллекционер", "Patch: Collector"));
                        sfx.play("badge");
                    }
                }
            }
            StationKind::Crewmate => {
                if let Some(mood) = sim.emotion {
                    modal.open(ModalKind::Emotion(mood), p.idx);
                }
            }
            StationKind::Cargo => modal.open(ModalKind::Cargo, p.idx),
            StationKind::Sample => {
                commands.entity(entity).despawn();
                p.carried += 1;
                sfx.play("pickup");
            }
            _ => {}
        }
    }
}
