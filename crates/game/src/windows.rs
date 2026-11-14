//! B5 — in-game windows: power console, harvest and cargo choices, crew support (name the
//! feeling → choose support), pause, Space Beacon breathing, first-mission tutorial.

use crate::art::Art;
use crate::audio::Play;
use crate::kit::{self, Act, Action, Inputs, UiBusy, C_BAD, C_GOOD, C_LINE, C_MUTED, C_TEXT};
use crate::world::{run, Happened, Paused, Refused};
use crate::{AppState, Prof, Set, Sim};
use ja_core::data::{module_def, FEELINGS, MOODS};
use ja_core::{i18n, Command, GameEvent};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Win {
    #[default]
    None,
    Power,
    Harvest,
    Cargo,
    Crew,
    Pause,
    Beacon,
    Tutorial(u8),
}

#[derive(Resource, Default)]
pub struct Modal {
    pub win: Win,
    pub who: u8,
    /// last feeling the child picked and whether it fits the story
    pub feeling: Option<(usize, bool)>,
    /// two calm breaths earn the beacon patch (once)
    pub beacon_awarded: bool,
    pub breathing: bool,
    pub t: f32,
    pub version: u32,
}
impl Modal {
    pub fn open(&mut self, win: Win, who: u8) {
        self.win = win;
        self.who = who;
        self.feeling = None;
        self.breathing = false;
        self.beacon_awarded = false;
        self.t = 0.0;
        self.version += 1;
    }
    pub fn close(&mut self) {
        self.open(Win::None, 0);
    }
    pub fn is_open(&self) -> bool {
        self.win != Win::None
    }
    /// Does this window stop player `idx` from walking?
    pub fn blocks(&self, idx: u8) -> bool {
        match self.win {
            Win::None => false,
            Win::Pause | Win::Beacon | Win::Tutorial(_) => true,
            _ => self.who == idx,
        }
    }
}

#[derive(Component)]
struct WinRoot;
#[derive(Component)]
struct Orb;
#[derive(Component)]
struct OrbText;

pub struct WindowsPlugin;
impl Plugin for WindowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Modal>()
            .add_systems(Update, first_tutorial.run_if(resource_added::<Sim>))
            .add_systems(Update, (keys, acts, feelings).chain().in_set(Set::Logic).run_if(in_state(AppState::Playing)))
            .add_systems(Update, (render, breathe, pause_world).in_set(Set::View).run_if(in_state(AppState::Playing)))
            .add_systems(OnExit(AppState::Playing), |mut m: ResMut<Modal>, mut p: ResMut<Paused>, mut b: ResMut<UiBusy>| {
                m.close();
                p.0 = false;
                b.0 = false;
            });
    }
}

fn first_tutorial(prof: Res<Prof>, mut modal: ResMut<Modal>) {
    if prof.0.missions == 0 {
        modal.open(Win::Tutorial(0), 0);
    } else {
        modal.close();
    }
}

fn keys(inputs: Res<Inputs>, mut modal: ResMut<Modal>, build: Res<crate::build::BuildMode>) {
    for (i, pad) in inputs.0.iter().enumerate() {
        if pad.pause && i == 0 {
            if modal.win == Win::Pause || modal.win == Win::Beacon {
                modal.close();
            } else if !matches!(modal.win, Win::Tutorial(_)) && !(build.active && build.player == 0) {
                modal.open(Win::Pause, 0);
            }
            return;
        }
        if pad.power && !modal.is_open() {
            modal.open(Win::Power, i as u8);
            return;
        }
        if pad.power && modal.win == Win::Power {
            modal.close();
            return;
        }
        if pad.back && modal.is_open() && modal.blocks(i as u8) && !matches!(modal.win, Win::Tutorial(_) | Win::Pause) {
            modal.close();
            return;
        }
    }
}

fn acts(mut acts: EventReader<Act>, mut modal: ResMut<Modal>, mut sim: ResMut<Sim>, mut next: ResMut<NextState<AppState>>, mut ok: EventWriter<Happened>, mut no: EventWriter<Refused>, mut play: EventWriter<Play>) {
    for Act(a) in acts.read() {
        let who = modal.who;
        match *a {
            Action::Power(id, d) => {
                if let Some(m) = sim.0.state().modules.iter().find(|m| m.id == id) {
                    let target = (m.power as i32 + d as i32).max(0) as u8;
                    if run(&mut sim, who, Command::SetPower { module: id, blocks: target }, &mut ok, &mut no) {
                        play.write(Play("click"));
                    } else {
                        play.write(Play("bad"));
                    }
                    modal.version += 1;
                }
            }
            Action::Close => modal.close(),
            Action::Harvest(eat) => {
                run(&mut sim, who, Command::ChooseHarvest(eat), &mut ok, &mut no);
                play.write(Play("good"));
                modal.close();
            }
            Action::Cargo(all) => {
                run(&mut sim, who, Command::ChooseCargo(all), &mut ok, &mut no);
                play.write(Play("good"));
                modal.close();
            }
            Action::Feel(i) => {
                run(&mut sim, who, Command::NameFeeling(i), &mut ok, &mut no);
                modal.feeling = Some((i, false));
            }
            Action::Support(i) => {
                if run(&mut sim, who, Command::Support(i), &mut ok, &mut no) {
                    play.write(Play("good"));
                }
                modal.close();
            }
            Action::Pause => {
                if modal.win == Win::Pause {
                    modal.close();
                } else {
                    modal.open(Win::Pause, 0);
                }
            }
            Action::Beacon => modal.open(Win::Beacon, 0),
            Action::Breathe => {
                modal.breathing = true;
                modal.t = 0.0;
                modal.version += 1;
            }
            Action::ExitMission => {
                modal.close();
                next.set(AppState::Menu);
            }
            Action::TutorialNext => {
                if let Win::Tutorial(s) = modal.win {
                    if s as usize + 1 >= i18n::TUTORIAL.len() {
                        modal.close();
                    } else {
                        modal.open(Win::Tutorial(s + 1), 0);
                    }
                }
            }
            Action::ToggleLang | Action::ToggleSound | Action::ToggleMusic => modal.version += 1,
            _ => {}
        }
    }
}

fn feelings(mut ev: EventReader<Happened>, mut modal: ResMut<Modal>) {
    for Happened(e) in ev.read() {
        match e {
            GameEvent::FeelingRight(i) => {
                modal.feeling = Some((*i, true));
                modal.version += 1;
            }
            GameEvent::FeelingTryAgain(i) => {
                modal.feeling = Some((*i, false));
                modal.version += 1;
            }
            _ => {}
        }
    }
}

fn pause_world(modal: Res<Modal>, mut sim: ResMut<Sim>, mut paused: ResMut<Paused>, mut busy: ResMut<UiBusy>) {
    let coop = sim.0.state().coop;
    let p = match modal.win {
        Win::None => false,
        Win::Pause | Win::Beacon | Win::Tutorial(_) => true,
        _ => !coop,
    };
    if paused.0 != p || sim.0.state().paused != p {
        paused.0 = p;
        sim.0.set_paused(p);
    }
    busy.0 = modal.is_open();
}

#[allow(clippy::too_many_arguments)]
fn render(mut commands: Commands, music: Res<crate::Music>, modal: Res<Modal>, mut last: Local<u32>, mut timer: Local<f32>, time: Res<Time>, sim: Res<Sim>, art: Res<Art>, prof: Res<Prof>, old: Query<Entity, With<WinRoot>>, mut focus: ResMut<kit::Focus>) {
    // live-refresh the power console once a second (generation changes during the day)
    *timer += time.delta_secs();
    let tick_refresh = modal.win == Win::Power && *timer >= 1.0;
    if tick_refresh {
        *timer = 0.0;
    }
    if modal.version == *last && !tick_refresh {
        return;
    }
    let new_window = modal.version != *last;
    *last = modal.version;
    for e in &old {
        commands.entity(e).despawn();
    }
    if new_window && !matches!(modal.win, Win::Power) {
        focus.idx = 0;
    }
    if !modal.is_open() {
        return;
    }
    let l = prof.lang();
    let st = sim.0.state();
    commands.spawn((kit::overlay(true, 30), WinRoot, StateScoped(AppState::Playing))).with_children(|o| {
        o.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(22.0)), row_gap: Val::Px(12.0), border: UiRect::all(Val::Px(3.0)), min_width: Val::Px(520.0), max_width: Val::Px(800.0), ..default() }).with_children(|p| {
            let title = |p: &mut ChildSpawnerCommands, ic: &str, t: &str| {
                p.spawn(kit::row(12.0)).with_children(|r| {
                    kit::icon(r, &art, ic, 46.0);
                    kit::text(r, t, &art.head, 28.0, C_TEXT);
                });
            };
            match modal.win {
                Win::Power => {
                    title(p, "power", l.pick("Пульт энергии", "Power console"));
                    let free = st.available() as i32 - st.demand as i32;
                    p.spawn(kit::row(18.0)).with_children(|r| {
                        for (ic, v, c) in [("sun", format!("{:.0}", st.supply.max(0.0)), C_TEXT), ("battery", format!("{:.0}/{:.0}", st.battery, st.battery_cap), C_TEXT), ("flash", format!("{} {}", free.max(0), l.pick("свободно", "free")), if free > 0 { kit::C_ACCENT } else { C_BAD })] {
                            r.spawn(kit::row(6.0)).with_children(|x| {
                                kit::icon(x, &art, ic, 28.0);
                                kit::text(x, v, &art.head, 22.0, c);
                            });
                        }
                    });
                    kit::text(p, l.pick("Энергии всегда меньше, чем хочется. Ночью работает только батарея.", "There’s never enough power. At night only the battery works."), &art.body, 17.0, C_MUTED);
                    let mut order = 0u16;
                    for m in st.modules.iter().filter(|m| module_def(m.kind).max_power > 0) {
                        let s = module_def(m.kind);
                        p.spawn((kit::row(10.0), BorderColor(C_LINE))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::vertical(Val::Px(4.0)), border: UiRect::top(Val::Px(1.0)), ..default() }).with_children(|r| {
                            kit::icon(r, &art, s.icon, 36.0);
                            r.spawn(Node { width: Val::Px(300.0), flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                                kit::text(c, l.pick(s.name_ru, s.name_en), &art.body, 20.0, if m.broken { C_BAD } else { C_TEXT });
                                kit::wrap_text(c, if m.broken { l.pick("сломан — подойди и держи F", "broken — go and hold F") } else { l.pick(s.desc_ru, s.desc_en) }, &art.body, 13.0, C_MUTED, 300.0);
                            });
                            kit::button(r, &art, Action::Power(m.id, -1), order, "", "−", false);
                            kit::pips(r, m.power as u32, s.max_power as u32, kit::C_WARN, 26.0);
                            kit::button(r, &art, Action::Power(m.id, 1), order + 1, "", "+", false);
                        });
                        order += 2;
                    }
                    p.spawn(kit::row(10.0)).insert(Node { justify_content: JustifyContent::FlexEnd, flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
                        kit::button(r, &art, Action::Close, 100, "check", l.pick("Готово", "Done"), true);
                    });
                }
                Win::Harvest | Win::Cargo => {
                    let harvest = modal.win == Win::Harvest;
                    title(p, if harvest { "plant" } else { "box" }, if harvest { l.pick("Урожай готов!", "Harvest ready!") } else { l.pick("Грузовой корабль", "Cargo ship") });
                    kit::text(p, l.pick("Бесплатного выбора нет: у каждого варианта есть цена.", "No choice is free: every option has a cost."), &art.body, 17.0, C_MUTED);
                    let opts: [(Action, &str, &str, [(&str, &str); 3]); 2] = if harvest {
                        [
                            (Action::Harvest(true), "plate", l.pick("Съесть", "Eat"), [("food", "+12"), ("heart", "+10"), ("science", "0")]),
                            (Action::Harvest(false), "test", l.pick("В лабораторию", "To the lab"), [("science", "+2"), ("food", "+4"), ("plate", "−8")]),
                        ]
                    } else {
                        [
                            (Action::Cargo(true), "strong", l.pick("Разгрузить всё", "Unload all"), [("food", "+20"), ("wrench", "+6"), ("battery", "−2")]),
                            (Action::Cargo(false), "ok_hand", l.pick("Только нужное", "Essentials"), [("food", "+8"), ("wrench", "+2"), ("battery", "0")]),
                        ]
                    };
                    p.spawn(kit::row(14.0)).with_children(|r| {
                        for (i, (act, ic, label, eff)) in opts.iter().enumerate() {
                            r.spawn((Button, kit::UiAction(*act), kit::Focusable(i as u16), Node { width: Val::Px(250.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(3.0)), ..default() }, BackgroundColor(kit::C_PANEL), kit::Base(kit::C_PANEL), BorderColor(C_LINE), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
                                kit::icon(b, &art, ic, 60.0);
                                kit::text(b, *label, &art.body, 22.0, C_TEXT);
                                for (ei, v) in eff {
                                    b.spawn(kit::row(6.0)).with_children(|e| {
                                        kit::icon(e, &art, ei, 24.0);
                                        kit::text(e, *v, &art.head, 20.0, if v.starts_with('−') { C_BAD } else if *v == "0" { C_MUTED } else { C_GOOD });
                                    });
                                }
                            });
                        }
                    });
                }
                Win::Crew => {
                    let Some(mood) = st.mood.map(|m| m.mood) else {
                        kit::button(p, &art, Action::Close, 0, "check", "OK", true);
                        return;
                    };
                    let md = &MOODS[mood];
                    title(p, "speech", l.pick(md.who_ru, md.who_en));
                    kit::wrap_text(p, l.pick(md.text_ru, md.text_en), &art.body, 22.0, C_TEXT, 680.0);
                    match modal.feeling {
                        None | Some((_, false)) => {
                            if let Some((i, false)) = modal.feeling {
                                p.spawn(kit::row(10.0)).with_children(|r| {
                                    kit::icon(r, &art, FEELINGS[i].icon, 32.0);
                                    kit::wrap_text(r, l.pick("Может быть! Подсказки в истории говорят о другом. Люди чувствуют по-разному — попробуй ещё.", "Maybe! The story hints at something else. People feel differently — try again."), &art.body, 18.0, C_TEXT, 600.0);
                                });
                            }
                            kit::text(p, l.pick("Что, по-твоему, он(а) чувствует?", "What do you think they feel?"), &art.body, 19.0, C_MUTED);
                            p.spawn(kit::row(8.0)).insert(Node { flex_wrap: FlexWrap::Wrap, row_gap: Val::Px(8.0), column_gap: Val::Px(8.0), flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
                                for (i, f) in FEELINGS.iter().enumerate() {
                                    kit::button(r, &art, Action::Feel(i), i as u16, f.icon, l.pick(f.name_ru, f.name_en), false);
                                }
                            });
                        }
                        Some((i, true)) => {
                            let f = &FEELINGS[i];
                            p.spawn(kit::row(10.0)).with_children(|r| {
                                kit::icon(r, &art, f.icon, 36.0);
                                kit::wrap_text(r, l.pick("Да, похоже на то. Ты связал чувство с тем, что случилось.", "Yes, it looks like it. You linked the feeling to what happened."), &art.body, 19.0, C_TEXT, 600.0);
                            });
                            kit::text(p, l.pick("Как поддержать? (−1 ⚡ на связь с Землёй)", "How to help? (−1 ⚡ for the call to Earth)"), &art.body, 19.0, C_MUTED);
                            for (k, (ic, ru, en)) in md.support.iter().enumerate() {
                                kit::button(p, &art, Action::Support(k), k as u16, ic, l.pick(ru, en), false);
                            }
                        }
                    }
                }
                Win::Pause => {
                    title(p, "pause", l.pick("Пауза", "Paused"));
                    kit::button(p, &art, Action::Pause, 0, "play", l.pick("Продолжить", "Resume"), true);
                    kit::button(p, &art, Action::Beacon, 1, "beacon", l.pick("Космический маяк — подышать", "Space Beacon — breathe"), false);
                    p.spawn(kit::row(8.0)).with_children(|r| {
                        kit::button(r, &art, Action::ToggleSound, 2, if prof.0.sound { "sound" } else { "mute" }, l.pick("Звуки", "Sounds"), false);
                        kit::button(r, &art, Action::ToggleMusic, 3, if music.0 { "s_music" } else { "mute" }, l.pick("Музыка", "Music"), false);
                        kit::button(r, &art, Action::ToggleLang, 4, "globe", if prof.0.lang == ja_core::Lang::En { "English" } else { "Русский" }, false);
                    });
                    kit::button(p, &art, Action::ExitMission, 5, "exit", l.pick("Выйти в меню", "Exit to menu"), false);
                    p.spawn(kit::row(14.0)).insert(Node { flex_wrap: FlexWrap::Wrap, flex_direction: FlexDirection::Row, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
                        for (ic, ru, en) in [("walk", "WASD — ходить", "WASD — walk"), ("ok_hand", "F — действие", "F — action"), ("hammer", "B — стройка", "B — build"), ("power", "Tab — энергия", "Tab — power"), ("gamepad", "Геймпад: A/Y/Select", "Gamepad: A/Y/Select")] {
                            r.spawn(kit::row(5.0)).with_children(|x| {
                                kit::icon(x, &art, ic, 20.0);
                                kit::text(x, l.pick(ru, en), &art.body, 15.0, C_MUTED);
                            });
                        }
                    });
                }
                Win::Beacon => {
                    title(p, "beacon", l.pick("Космический маяк", "Space Beacon"));
                    kit::wrap_text(p, l.pick("Подышим вместе с маяком. Это по желанию — остановись в любой момент. Маяк не измеряет волнение и не ставит баллы.", "Breathe with the beacon. It’s optional — stop any time. It doesn’t measure feelings or give scores."), &art.body, 18.0, C_MUTED, 600.0);
                    p.spawn(Node { width: Val::Percent(100.0), height: Val::Px(220.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|c| {
                        c.spawn((Node { width: Val::Px(90.0), height: Val::Px(90.0), ..default() }, BackgroundColor(Color::srgb(1.0, 0.85, 0.4)), BorderRadius::MAX, Orb));
                    });
                    p.spawn((Text::new(if modal.breathing { "" } else { l.pick("Нажми «Начать»", "Press “Start”") }), TextFont { font: art.head.clone(), font_size: 26.0, ..default() }, TextColor(C_TEXT), TextLayout::new_with_justify(JustifyText::Center), OrbText));
                    p.spawn(kit::row(10.0)).with_children(|r| {
                        if !modal.breathing {
                            kit::button(r, &art, Action::Breathe, 0, "play", l.pick("Начать", "Start"), true);
                        }
                        kit::button(r, &art, Action::Close, 1, "back", l.pick("К миссии", "Back to mission"), false);
                    });
                }
                Win::Tutorial(step) => {
                    title(p, "teacher", l.pick("Инструктор", "Instructor"));
                    let steps = i18n::TUTORIAL;
                    let n = steps.len();
                    let (ic, ru, en) = steps[(step as usize).min(n - 1)];
                    p.spawn(kit::row(14.0)).with_children(|r| {
                        kit::icon(r, &art, ic, 80.0);
                        kit::wrap_text(r, l.pick(ru, en), &art.body, 22.0, C_TEXT, 520.0);
                    });
                    p.spawn(kit::row(12.0)).insert(Node { justify_content: JustifyContent::FlexEnd, flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
                        let last = step as usize + 1 >= n;
                        kit::text(r, format!("{}/{}", step + 1, n), &art.body, 18.0, C_MUTED);
                        kit::button(r, &art, Action::TutorialNext, 0, if last { "rocket" } else { "play" }, if last { l.pick("Поехали!", "Let’s go!") } else { l.pick("Дальше", "Next") }, true);
                    });
                }
                Win::None => {}
            }
            let _ = C_GOOD;
        });
    });
}

fn breathe(time: Res<Time>, mut modal: ResMut<Modal>, mut prof: ResMut<Prof>, mut play: EventWriter<Play>, mut orb: Query<&mut Node, With<Orb>>, mut txt: Query<&mut Text, With<OrbText>>) {
    if modal.win != Win::Beacon || !modal.breathing {
        return;
    }
    let l = prof.lang();
    modal.t += time.delta_secs();
    let cycle = 9.0;
    let n = (modal.t / cycle) as u32;
    let ph = modal.t % cycle;
    if n >= 2 && !modal.beacon_awarded {
        modal.beacon_awarded = true;
        if prof.0.award("beacon") {
            prof.save();
            play.write(Play("badge"));
        }
    }
    let (size, label) = if n >= 4 {
        (90.0, l.pick("Молодец, что попробовал!", "Nice job trying it!").to_string())
    } else if ph < 4.0 {
        (90.0 + 110.0 * ph / 4.0, format!("{} {}", l.pick("Вдох…", "Breathe in…"), ph as u32 + 1))
    } else {
        (200.0 - 110.0 * (ph - 4.0) / 5.0, format!("{} {}", l.pick("Выдох…", "Breathe out…"), (ph - 4.0) as u32 + 1))
    };
    for mut n in &mut orb {
        n.width = Val::Px(size);
        n.height = Val::Px(size);
    }
    for mut t in &mut txt {
        t.0 = label.clone();
    }
}
