//! Story panel: the current task, the finale ("boss") bar and which far regions are open.
//! Answers "what do I do next?" at a glance. All texts come from `ja_core::i18n`.

use crate::art::Art;
use crate::kit::{self, C_BG, C_GOOD, C_LINE, C_MUTED, C_TEXT, C_WARN};
use crate::{AppState, Prof, Set, Sim};
use bevy::prelude::*;
use ja_core::data::{boss, regions, unlock_def};
use ja_core::game::WAVE_LEN;
use ja_core::i18n;

#[derive(Component)]
struct QuestIcon;
#[derive(Component)]
struct QuestTitle;
#[derive(Component)]
struct QuestGoal;
#[derive(Component)]
struct BossRow;
#[derive(Component)]
struct BossText;
#[derive(Component)]
struct BossBar;
#[derive(Component)]
struct RegionIcon(u8);
#[derive(Component)]
struct RegionText(u8);

pub struct StoryPlugin;

impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn.run_if(resource_added::<Sim>))
            .add_systems(Update, update.in_set(Set::View).run_if(in_state(AppState::Playing)).run_if(resource_exists::<Sim>));
    }
}

fn spawn(mut commands: Commands, art: Res<Art>, sim: Res<Sim>) {
    let st = sim.0.state();
    let regs = regions(st.map);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(272.0),
                width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(C_BG),
            BorderColor(C_LINE),
            BorderRadius::all(Val::Px(8.0)),
            GlobalZIndex(10),
            StateScoped(AppState::Playing),
        ))
        .with_children(|p| {
            p.spawn(kit::row(8.0)).with_children(|r| {
                r.spawn((ImageNode::new(art.icon("book")), Node { width: Val::Px(30.0), height: Val::Px(30.0), flex_shrink: 0.0, ..default() }, QuestIcon));
                r.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 17.0, ..default() }, TextColor(kit::C_ACCENT), QuestTitle));
            });
            p.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 15.0, ..default() }, TextColor(C_TEXT), QuestGoal));
            // finale bar
            p.spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() }, Visibility::Hidden, BossRow)).with_children(|b| {
                b.spawn((Text::new(""), TextFont { font: art.head.clone(), font_size: 15.0, ..default() }, TextColor(C_WARN), BossText));
                b.spawn((Node { width: Val::Percent(100.0), height: Val::Px(8.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)))).with_children(|bar| {
                    bar.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(1.0, 0.45, 0.3)), BossBar));
                });
            });
            // far regions: open or what they need
            for (i, _) in regs.iter().enumerate() {
                p.spawn(kit::row(6.0)).with_children(|r| {
                    r.spawn((ImageNode::new(art.icon("lock")), Node { width: Val::Px(20.0), height: Val::Px(20.0), flex_shrink: 0.0, ..default() }, RegionIcon(i as u8)));
                    r.spawn((Text::new(""), TextFont { font: art.body.clone(), font_size: 13.0, ..default() }, TextColor(C_MUTED), RegionText(i as u8)));
                });
            }
        });
}

#[allow(clippy::type_complexity)]
fn update(
    sim: Res<Sim>,
    prof: Res<Prof>,
    art: Res<Art>,
    mut icon: Query<&mut ImageNode, (With<QuestIcon>, Without<RegionIcon>)>,
    mut title: Query<&mut Text, (With<QuestTitle>, Without<QuestGoal>, Without<BossText>, Without<RegionText>)>,
    mut goal: Query<&mut Text, (With<QuestGoal>, Without<QuestTitle>, Without<BossText>, Without<RegionText>)>,
    mut boss_row: Query<&mut Visibility, With<BossRow>>,
    mut boss_text: Query<&mut Text, (With<BossText>, Without<QuestTitle>, Without<QuestGoal>, Without<RegionText>)>,
    mut boss_bar: Query<&mut Node, With<BossBar>>,
    mut reg_icon: Query<(&RegionIcon, &mut ImageNode), Without<QuestIcon>>,
    mut reg_text: Query<(&RegionText, &mut Text, &mut TextColor), (Without<QuestTitle>, Without<QuestGoal>, Without<BossText>)>,
    mut last: Local<String>,
) {
    let st = sim.0.state();
    let l = prof.lang();
    // only rebuild texts when something changed
    let key = format!("{}{}{}{:?}{:?}{}", st.quest, st.quest_progress, st.boss_done, st.boss.map(|b| (b.wave, b.active, b.countered)), st.unlocks, l == ja_core::Lang::En);
    if let Some(b) = st.boss {
        for mut n in &mut boss_bar {
            n.width = Val::Percent(if b.active { (b.t_left / WAVE_LEN).clamp(0.0, 1.0) * 100.0 } else { 0.0 });
        }
    }
    if *last == key {
        return;
    }
    *last = key;
    let (t, g, ic) = match (i18n::quest_title(st.map, st.quest), i18n::quest_goal(st.map, st.quest, st.quest_progress)) {
        (Some(t), Some(g)) => (t.text(l).to_string(), g.text(l).to_string(), t.icon),
        _ => (l.pick("Все задания выполнены!", "All tasks done!").to_string(), l.pick("Исследуй карту и собирай находки", "Explore the map and collect finds").to_string(), "trophy"),
    };
    for mut x in &mut title {
        x.0 = t.clone();
    }
    for mut x in &mut goal {
        x.0 = g.clone();
    }
    for mut x in &mut icon {
        x.image = art.icon(ic);
    }
    for mut v in &mut boss_row {
        *v = if st.boss.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Some(b) = st.boss {
        let def = boss(st.map);
        for mut x in &mut boss_text {
            x.0 = format!("{} · {} {}/{}", l.pick(def.name_ru, def.name_en), l.pick("волна", "wave"), b.wave, b.of);
        }
    }
    let regs = regions(st.map);
    for (r, mut img) in &mut reg_icon {
        let open = st.region_open(r.0);
        img.image = art.icon(if open { regs[r.0 as usize].icon } else { "lock" });
    }
    for (r, mut x, mut c) in &mut reg_text {
        let reg = &regs[r.0 as usize];
        let open = st.region_open(r.0);
        x.0 = if open {
            l.pick(reg.name_ru, reg.name_en).to_string()
        } else {
            let u = unlock_def(reg.need);
            format!("{} — {} {}", l.pick(reg.name_ru, reg.name_en), l.pick("нужен", "needs"), l.pick(u.name_ru, u.name_en).to_lowercase())
        };
        c.0 = if open { C_GOOD } else { C_MUTED };
    }
}
