//! A10 — encyclopedia: the Solar System map, all physics cards, and the crew's lines to read again.
//! Everything comes from `assets/content` (F5 reloads it).

use crate::art::Art;
use crate::kit::{self, Act, Action, UiAction, C_MUTED, C_TEXT};
use crate::ship::ContentRes;
use crate::{AppState, Prof};
use bevy::prelude::*;
use ja_core::content::{CardKind, Content};
use ja_core::{Lang, Level};

#[derive(Resource, Default)]
pub struct Enc {
    pub tab: u8,
    pub pick: usize,
}

#[derive(Component)]
struct EncRoot;

pub struct EncyclopediaPlugin;
impl Plugin for EncyclopediaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Enc>().add_systems(Update, (actions, build).chain().after(crate::Set::Input).run_if(in_state(AppState::Encyclopedia))).add_systems(Update, enc_test);
    }
}

fn actions(mut acts: EventReader<Act>, mut enc: ResMut<Enc>) {
    for Act(a) in acts.read() {
        match *a {
            Action::EncTab(t) => {
                enc.tab = t;
                enc.pick = 0;
            }
            Action::EncPick(i) => enc.pick = i as usize,
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build(mut commands: Commands, enc: Res<Enc>, content: Res<ContentRes>, art: Res<Art>, prof: Res<Prof>, old: Query<Entity, With<EncRoot>>, mut key: Local<String>) {
    let l = prof.lang();
    let level = if prof.0.cadet { Level::Junior } else { Level::Advanced };
    let k = format!("{}|{}|{:?}|{:?}|{}|{}", enc.tab, enc.pick, l, level, old.is_empty(), content.is_changed());
    if *key == k && !old.is_empty() {
        return;
    }
    *key = k;
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(c) = &content.0 else { return };
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, top: Val::Px(70.0), left: Val::Px(20.0), right: Val::Px(20.0), bottom: Val::Px(10.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), ..default() },
            GlobalZIndex(12),
            EncRoot,
            StateScoped(AppState::Encyclopedia),
        ))
        .with_children(|p| {
            p.spawn(kit::row(10.0)).with_children(|r| {
                kit::text(r, l.pick("Энциклопедия", "Encyclopedia"), &art.head, 30.0, C_TEXT);
                for (i, icon, ru, en) in [(0u8, "planet", "Планеты", "Planets"), (1, "test", "Физика полёта", "Flight physics"), (2, "speech", "Голоса экипажа", "Crew voices")] {
                    let bg = if enc.tab == i { kit::C_PRIMARY } else { kit::C_PANEL };
                    r.spawn((Button, UiAction(Action::EncTab(i)), kit::Focusable(1 + i as u16), Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), align_items: AlignItems::Center, padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(bg), kit::Base(bg), BorderColor(kit::C_LINE), BorderRadius::all(Val::Px(8.0))))
                        .with_children(|b| {
                            kit::icon(b, &art, icon, 28.0);
                            kit::text(b, l.pick(ru, en), &art.body, 18.0, if enc.tab == i { Color::srgb(0.18, 0.08, 0.0) } else { C_TEXT });
                        });
                }
            });
            match enc.tab {
                0 => crate::planets::map(p, &art, c, enc.pick, level, l),
                1 => cards(p, &art, c, enc.pick, level, l),
                _ => voices(p, &art, c, enc.pick, level, l),
            }
        });
}

fn list_button(p: &mut ChildSpawnerCommands, art: &Art, i: usize, on: bool, icon: &str, label: &str) {
    let bg = if on { kit::C_PRIMARY } else { kit::C_PANEL };
    p.spawn((Button, UiAction(Action::EncPick(i as u8)), kit::Focusable(10 + i as u16), Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), align_items: AlignItems::Center, padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(bg), kit::Base(bg), BorderColor(kit::C_LINE), BorderRadius::all(Val::Px(6.0))))
        .with_children(|b| {
            kit::icon(b, art, icon, 24.0);
            kit::text(b, label, &art.body, 16.0, if on { Color::srgb(0.18, 0.08, 0.0) } else { C_TEXT });
        });
}

fn cards(p: &mut ChildSpawnerCommands, art: &Art, c: &Content, pick: usize, level: Level, l: Lang) {
    p.spawn(kit::row(16.0)).insert(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(16.0), align_items: AlignItems::FlexStart, ..default() }).with_children(|r| {
        r.spawn(kit::col(6.0)).with_children(|list| {
            for (i, k) in c.cards.iter().enumerate() {
                let icon = match k.kind {
                    CardKind::Quantity => "test",
                    CardKind::Phenomenon => "sparkles",
                    CardKind::Concept => "map",
                };
                list_button(list, art, i, i == pick, icon, k.title.get(l));
            }
        });
        let Some(k) = c.cards.get(pick) else { return };
        r.spawn(kit::panel()).insert(Node { width: Val::Px(700.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|b| {
            kit::text(b, k.title.get(l), &art.head, 28.0, kit::C_ACCENT);
            let kind = match k.kind {
                CardKind::Quantity => l.pick("величина", "quantity"),
                CardKind::Phenomenon => l.pick("явление", "phenomenon"),
                CardKind::Concept => l.pick("понятие", "concept"),
            };
            let mut line = vec![kind.to_string()];
            if let Some(s) = &k.symbol {
                line.push(format!("{} {s}", l.pick("обозначение", "symbol")));
            }
            if let Some(u) = &k.unit {
                line.push(u.clone());
            }
            kit::text(b, line.join(" · "), &art.body, 16.0, C_MUTED);
            if let (Level::Advanced, Some(f)) = (level, &k.formula) {
                kit::text(b, f.clone(), &art.head, 24.0, C_TEXT);
            }
            let t = match (level, &k.advanced) {
                (Level::Advanced, Some(a)) => a.get(l),
                _ => k.junior.get(l),
            };
            kit::wrap_text(b, t, &art.body, 18.0, C_TEXT, 660.0);
        });
    });
}

fn voices(p: &mut ChildSpawnerCommands, art: &Art, c: &Content, pick: usize, level: Level, l: Lang) {
    p.spawn(kit::row(16.0)).insert(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(16.0), align_items: AlignItems::FlexStart, ..default() }).with_children(|r| {
        r.spawn(kit::col(6.0)).with_children(|list| {
            for (i, d) in c.dialogs.iter().enumerate() {
                let first = d.nodes.first().and_then(|n| c.npc(&n.npc));
                let label = format!("{} {}", first.map(|n| n.name.get(l)).unwrap_or(""), i + 1);
                list_button(list, art, i, i == pick, first.map(|n| n.icon.as_str()).unwrap_or("speech"), &label);
            }
        });
        let Some(d) = c.dialogs.get(pick) else { return };
        r.spawn(kit::panel()).insert(Node { width: Val::Px(760.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|b| {
            for n in &d.nodes {
                let npc = c.npc(&n.npc);
                b.spawn(kit::row(10.0)).insert(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), align_items: AlignItems::FlexStart, ..default() }).with_children(|x| {
                    kit::icon(x, art, npc.map(|n| n.icon.as_str()).unwrap_or("speech"), 36.0);
                    x.spawn(kit::col(2.0)).with_children(|t| {
                        kit::text(t, npc.map(|n| n.name.get(l)).unwrap_or(""), &art.head, 15.0, kit::C_ACCENT);
                        kit::wrap_text(t, n.text.get(level, l), &art.body, 17.0, C_TEXT, 660.0);
                        for ch in &n.choices {
                            kit::text(t, format!("— {}", ch.text.get(l)), &art.body, 15.0, C_MUTED);
                        }
                    });
                });
            }
        });
    });
}

/// `JA_ENC_TEST=1 JA_SHOTS=dir`: opens each tab, takes a screenshot, quits.
pub fn enc_test(mut commands: Commands, time: Res<Time>, mut t: Local<f32>, mut step: Local<u32>, mut next: ResMut<NextState<AppState>>, mut acts: EventWriter<Act>, mut exit: EventWriter<AppExit>) {
    if std::env::var_os("JA_ENC_TEST").is_none() {
        return;
    }
    *t += time.delta_secs();
    let dir = std::env::var_os("JA_SHOTS").map(std::path::PathBuf::from).unwrap_or_default();
    let mut shot = |name: &str| {
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(dir.join(format!("{name}.png"))));
    };
    match (*step, *t) {
        (0, x) if x > 1.5 => {
            next.set(AppState::Encyclopedia);
            acts.write(Act(Action::EncPick(3)));
            *step = 1;
        }
        (1, x) if x > 3.0 => {
            shot("30_planets");
            *step = 2;
        }
        (2, x) if x > 3.8 => {
            acts.write(Act(Action::EncTab(1)));
            *step = 3;
        }
        (3, x) if x > 4.4 => {
            acts.write(Act(Action::EncPick(5)));
            *step = 4;
        }
        (4, x) if x > 5.4 => {
            shot("31_physics_cards");
            *step = 5;
        }
        (5, x) if x > 6.2 => {
            acts.write(Act(Action::EncTab(2)));
            *step = 6;
        }
        (6, x) if x > 7.2 => {
            shot("32_crew_voices");
            *step = 7;
        }
        (7, x) if x > 8.5 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
