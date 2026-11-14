//! In-game HUD, modal windows, the shared button dispatcher and keyboard/gamepad navigation.
//! The UI is icon-first: big buttons, short bilingual text.

use crate::audio::{Ambient, Sfx, SfxLib};
use crate::data::*;
use crate::i18n::Lang;
use crate::player::{Inputs, Player};
use crate::save::Profile;
use crate::sim::{Sim, FOOD_PER, FOOD_USE, O2_PER, O2_USE, SHIELD_MAX, SHIELD_PER};
use crate::AppState;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

// ---------------------------------------------------------------- palette

pub const C_PANEL: Color = Color::srgba(0.04, 0.07, 0.16, 0.88);
pub const C_PANEL2: Color = Color::srgba(0.08, 0.12, 0.26, 0.97);
pub const C_TEXT: Color = Color::srgb(0.94, 0.96, 1.0);
pub const C_DIM: Color = Color::srgb(0.64, 0.72, 0.86);
pub const C_ACCENT: Color = Color::srgb(0.30, 0.78, 1.0);
pub const C_BTN: Color = Color::srgb(0.16, 0.30, 0.62);
pub const C_GO: Color = Color::srgb(0.14, 0.58, 0.34);
pub const C_LOCK: Color = Color::srgb(0.20, 0.22, 0.30);
pub const C_WARN: Color = Color::srgb(0.95, 0.58, 0.15);
pub const C_DANGER: Color = Color::srgb(0.88, 0.24, 0.22);
pub const C_GOLD: Color = Color::srgb(1.0, 0.80, 0.25);
pub const C_O2: Color = Color::srgb(0.35, 0.80, 1.0);
pub const C_FOOD: Color = Color::srgb(0.45, 0.85, 0.35);
pub const C_DOSE: Color = Color::srgb(1.0, 0.42, 0.30);
pub const C_SCI: Color = Color::srgb(0.75, 0.55, 1.0);
pub const C_MOOD: Color = Color::srgb(1.0, 0.55, 0.70);
pub const C_TRACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);

// ---------------------------------------------------------------- shared resources

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlertKind {
    Info,
    Good,
    Warn,
    Danger,
    Badge,
}

#[derive(Clone, Debug)]
pub struct Alert {
    pub icon: &'static str,
    pub ru: String,
    pub en: String,
    pub kind: AlertKind,
    pub age: f32,
}

impl Alert {
    pub fn new(kind: AlertKind, icon: &'static str, ru: impl Into<String>, en: impl Into<String>) -> Self {
        Self { icon, ru: ru.into(), en: en.into(), kind, age: 0.0 }
    }
    pub fn info(icon: &'static str, ru: &str, en: &str) -> Self {
        Self::new(AlertKind::Info, icon, ru, en)
    }
    pub fn good(icon: &'static str, ru: &str, en: &str) -> Self {
        Self::new(AlertKind::Good, icon, ru, en)
    }
    pub fn warn(icon: &'static str, ru: &str, en: &str) -> Self {
        Self::new(AlertKind::Warn, icon, ru, en)
    }
    pub fn badge(icon: &'static str, ru: &str, en: &str) -> Self {
        Self::new(AlertKind::Badge, icon, ru, en)
    }
    pub fn danger_owned(icon: &'static str, ru: String, en: String) -> Self {
        Self::new(AlertKind::Danger, icon, ru, en)
    }
    fn ttl(&self) -> f32 {
        match self.kind {
            AlertKind::Info | AlertKind::Good => 5.0,
            AlertKind::Badge => 6.0,
            AlertKind::Warn => 7.0,
            AlertKind::Danger => 9.0,
        }
    }
    fn color(&self) -> Color {
        match self.kind {
            AlertKind::Info => Color::srgba(0.12, 0.22, 0.45, 0.92),
            AlertKind::Good => Color::srgba(0.10, 0.42, 0.24, 0.92),
            AlertKind::Warn => Color::srgba(0.62, 0.36, 0.06, 0.94),
            AlertKind::Danger => Color::srgba(0.66, 0.12, 0.12, 0.95),
            AlertKind::Badge => Color::srgba(0.42, 0.24, 0.62, 0.94),
        }
    }
}

#[derive(Resource, Default)]
pub struct Alerts {
    pub items: Vec<Alert>,
    pub dirty: bool,
}

impl Alerts {
    pub fn push(&mut self, a: Alert) {
        self.items.push(a);
        if self.items.len() > 4 {
            self.items.remove(0);
        }
        self.dirty = true;
    }
}

/// Mission Control message travelling with the real (or labeled, sped-up) comms delay.
#[derive(Resource, Default)]
pub struct McMsg {
    pub icon: &'static str,
    pub ru: &'static str,
    pub en: &'static str,
    pub sent_at: f32,
    pub arrive_at: f32,
    pub active: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Prompt {
    pub icon: &'static str,
    pub ru: &'static str,
    pub en: &'static str,
    pub progress: Option<f32>,
    pub warn: bool,
}

/// What each player can do right now (filled by `interact_system`).
#[derive(Resource, Default)]
pub struct Prompts(pub [Option<Prompt>; 2]);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModalKind {
    #[default]
    None,
    Board,
    Harvest,
    Cargo,
    Emotion(usize),
    Tutorial(u8),
    Pause,
    Beacon,
}

impl ModalKind {
    /// Global windows stop both players; the others belong to one player (their half in co-op).
    pub fn global(self) -> bool {
        matches!(self, ModalKind::Pause | ModalKind::Tutorial(_) | ModalKind::Beacon)
    }
}

#[derive(Resource, Default)]
pub struct Modal {
    pub kind: ModalKind,
    pub owner: u8,
    pub step: u8,
    pub pick: Option<usize>,
}

impl Modal {
    pub fn open(&mut self, kind: ModalKind, owner: u8) {
        self.kind = kind;
        self.owner = owner;
        self.step = 0;
        self.pick = None;
    }
    pub fn close(&mut self) {
        self.kind = ModalKind::None;
    }
    pub fn blocks(&self, idx: u8) -> bool {
        self.kind != ModalKind::None && (self.kind.global() || self.owner == idx)
    }
}

/// Seconds spent in the Space Beacon breathing pause (never scored).
#[derive(Resource, Default)]
pub struct Breath(pub f32);

// ---------------------------------------------------------------- buttons & actions

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Block {
    Ls,
    Green,
    Shield,
    Sci,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    // any screen
    ToggleLang,
    ToggleSound,
    // menu & debrief
    Page(crate::menu::MenuPage),
    Solo,
    Coop,
    PickMap(MapId),
    ToggleLive,
    ToggleCadet,
    Start,
    Buy(u8, usize),
    Reflect(u8, u8),
    Again,
    Quit,
    // in game
    Alloc(Block, i32),
    Close,
    Harvest(bool),
    Cargo(bool),
    Feel(usize),
    Support(usize),
    Pause,
    Beacon,
    Help,
    TutNext,
    EndMission,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct UiAction(pub Action);

#[derive(Component)]
pub struct Btn {
    pub action: Action,
    pub color: Color,
}

/// Buttons reachable with arrows / WASD / D-pad. HUD buttons are mouse-only.
#[derive(Component)]
pub struct Focusable;

#[derive(Resource, Default)]
pub struct Focus {
    pub entity: Option<Entity>,
    pub action: Option<Action>,
    pub keyboard: bool,
}

// ---------------------------------------------------------------- building helpers

#[derive(Resource, Clone)]
pub struct UiAssets {
    pub title: Handle<Font>,
    pub body: Handle<Font>,
    pub server: AssetServer,
}

pub fn load_ui_assets(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(UiAssets {
        title: server.load("fonts/RussoOne-Regular.ttf"),
        body: server.load("fonts/PTSans-Bold.ttf"),
        server: server.clone(),
    });
}

/// Button description: `B::new(action, icon, label).color(..).size(..)`.
pub struct B<'s> {
    pub action: Action,
    pub icon: Option<&'static str>,
    pub label: &'s str,
    pub color: Color,
    pub size: f32,
    pub width: Val,
    pub focus: bool,
}

impl<'s> B<'s> {
    pub fn new(action: Action, icon: Option<&'static str>, label: &'s str) -> Self {
        Self { action, icon, label, color: C_BTN, size: 20.0, width: Val::Auto, focus: true }
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn width(mut self, w: Val) -> Self {
        self.width = w;
        self
    }
    pub fn mouse_only(mut self) -> Self {
        self.focus = false;
        self
    }
}

#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub ui: &'a UiAssets,
    pub lang: Lang,
}

fn clean(s: String) -> String {
    // the bundled fonts lack the non-breaking hyphen
    s.replace('\u{2011}', "-")
}

impl<'a> Ctx<'a> {
    pub fn t<'s>(&self, ru: &'s str, en: &'s str) -> &'s str {
        self.lang.t(ru, en)
    }
    pub fn en(&self) -> bool {
        self.lang == Lang::En
    }
    pub fn img(&self, name: &str) -> Handle<Image> {
        self.ui.server.load(format!("icons/{name}.png"))
    }
    pub fn text(&self, p: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, color: Color) -> Entity {
        p.spawn((Text::new(clean(s.into())), TextFont { font: self.ui.body.clone(), font_size: size, ..default() }, TextColor(color))).id()
    }
    pub fn head(&self, p: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, color: Color) -> Entity {
        p.spawn((Text::new(clean(s.into())), TextFont { font: self.ui.title.clone(), font_size: size, ..default() }, TextColor(color))).id()
    }
    pub fn icon(&self, p: &mut ChildSpawnerCommands, name: &str, size: f32) -> Entity {
        p.spawn((ImageNode::new(self.img(name)), Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() })).id()
    }
    pub fn icon_tint(&self, p: &mut ChildSpawnerCommands, name: &str, size: f32, tint: Color) -> Entity {
        p.spawn((ImageNode::new(self.img(name)).with_color(tint), Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() })).id()
    }
    /// A button without content; the caller adds children.
    pub fn btn_custom<'b>(&self, p: &'b mut ChildSpawnerCommands, action: Action, color: Color, width: Val, focus: bool) -> EntityCommands<'b> {
        let mut e = p.spawn((
            Button,
            Node {
                width,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(9.0)),
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor(color.mix(&Color::WHITE, 0.25)),
            BorderRadius::all(Val::Px(14.0)),
            Outline::new(Val::Px(3.0), Val::Px(2.0), Color::NONE),
            Btn { action, color },
        ));
        if focus {
            e.insert(Focusable);
        }
        e
    }
    pub fn btn<'b>(&self, p: &'b mut ChildSpawnerCommands, b: B) -> EntityCommands<'b> {
        let ctx = *self;
        let size = b.size;
        let label = b.label.to_string();
        let icon = b.icon;
        let mut e = self.btn_custom(p, b.action, b.color, b.width, b.focus);
        e.with_children(|c| {
            if let Some(i) = icon {
                ctx.icon(c, i, size * 1.6);
            }
            if !label.is_empty() {
                ctx.head(c, label, size, C_TEXT);
            }
        });
        e
    }
    pub fn panel<'b>(&self, p: &'b mut ChildSpawnerCommands, color: Color) -> EntityCommands<'b> {
        p.spawn((
            Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(12.0)), row_gap: Val::Px(8.0), ..default() },
            BackgroundColor(color),
            BorderRadius::all(Val::Px(16.0)),
        ))
    }
    pub fn row<'b>(&self, p: &'b mut ChildSpawnerCommands, gap: f32) -> EntityCommands<'b> {
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(gap), ..default() })
    }
    /// Icon + text line.
    pub fn line(&self, p: &mut ChildSpawnerCommands, icon: &str, s: impl Into<String>, size: f32, color: Color) {
        let ctx = *self;
        let s = s.into();
        self.row(p, 10.0).with_children(|r| {
            ctx.icon(r, icon, size * 1.6);
            ctx.text(r, s, size, color);
        });
    }
    /// A horizontal progress bar (0..1).
    pub fn bar(&self, p: &mut ChildSpawnerCommands, w: f32, h: f32, frac: f32, color: Color) {
        p.spawn((Node { width: Val::Px(w), height: Val::Px(h), ..default() }, BackgroundColor(C_TRACK), BorderRadius::all(Val::Px(h / 2.0))))
            .with_children(|b| {
                b.spawn((
                    Node { width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(color),
                    BorderRadius::all(Val::Px(h / 2.0)),
                ));
            });
    }
}

pub fn who_en(who: &str) -> &str {
    match who {
        "Ая" => "Aya",
        "Тимур" => "Timur",
        "БИП" => "BEEP",
        w => w,
    }
}

// ---------------------------------------------------------------- plugin

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UiAction>()
            .init_resource::<Alerts>()
            .init_resource::<McMsg>()
            .init_resource::<Modal>()
            .init_resource::<Prompts>()
            .init_resource::<Focus>()
            .init_resource::<Breath>()
            .add_systems(Startup, load_ui_assets)
            .add_systems(
                Update,
                (
                    sync_settings,
                    game_keys.run_if(resource_exists::<Sim>),
                    rebuild_modal.run_if(resource_exists::<Sim>),
                    nav_system,
                    click_system,
                    click_sound,
                    button_visuals,
                    handle_common,
                    handle_game_actions.run_if(resource_exists::<Sim>),
                    rebuild_hud.run_if(resource_exists::<Sim>),
                    update_hud.run_if(resource_exists::<Sim>),
                    update_alerts.run_if(resource_exists::<Sim>),
                    beacon_tick.run_if(resource_exists::<Sim>),
                )
                    .chain()
                    .in_set(crate::GameSet::Ui),
            );
    }
}

/// Mirror profile settings into small resources other systems read.
pub fn sync_settings(profile: Res<Profile>, mut lang: ResMut<Lang>, mut lib: Option<ResMut<SfxLib>>) {
    if !profile.is_changed() {
        return;
    }
    let l = if profile.lang_en { Lang::En } else { Lang::Ru };
    if *lang != l {
        *lang = l;
    }
    if let Some(lib) = lib.as_mut() {
        if lib.on != profile.sound {
            lib.on = profile.sound;
        }
    }
}

// ---------------------------------------------------------------- navigation & dispatch

pub fn nav_system(
    inputs: Res<Inputs>,
    modal: Res<Modal>,
    state: Res<State<AppState>>,
    mut focus: ResMut<Focus>,
    btns: Query<(Entity, &Btn, &GlobalTransform, &ComputedNode, &InheritedVisibility), With<Focusable>>,
    mut ev: EventWriter<UiAction>,
) {
    let playing = *state.get() == AppState::Playing;
    if playing && modal.kind == ModalKind::None {
        if focus.entity.is_some() {
            focus.entity = None;
            focus.action = None;
        }
        return;
    }
    let (nav, act) = if playing && !modal.kind.global() {
        let i = inputs.0[modal.owner.min(1) as usize];
        (i.nav, i.act)
    } else {
        let (a, b) = (inputs.0[0], inputs.0[1]);
        (a.nav + b.nav, a.act || b.act)
    };
    let items: Vec<(Entity, Action, Vec2)> = btns
        .iter()
        .filter(|(_, _, _, cn, vis)| vis.get() && cn.size().x > 0.0)
        .map(|(e, b, gt, _, _)| (e, b.action, gt.translation().truncate()))
        .collect();
    if items.is_empty() {
        return;
    }
    let cur = focus
        .entity
        .and_then(|e| items.iter().find(|i| i.0 == e).copied())
        .or_else(|| focus.action.and_then(|a| items.iter().find(|i| i.1 == a).copied()))
        .unwrap_or_else(|| {
            *items
                .iter()
                .min_by(|a, b| (a.2.y, a.2.x).partial_cmp(&(b.2.y, b.2.x)).unwrap_or(std::cmp::Ordering::Equal))
                .expect("non-empty")
        });
    let mut target = cur;
    if nav != Vec2::ZERO {
        // screen space has y pointing down
        let dir = Vec2::new(nav.x, -nav.y).normalize();
        let mut best_score = f32::MAX;
        for it in &items {
            if it.0 == cur.0 {
                continue;
            }
            let d = it.2 - cur.2;
            let along = d.dot(dir);
            if along < 4.0 {
                continue;
            }
            let score = along + (d - dir * along).length() * 2.0;
            if score < best_score {
                best_score = score;
                target = *it;
            }
        }
        focus.keyboard = true;
    }
    if focus.entity != Some(target.0) {
        focus.entity = Some(target.0);
        focus.action = Some(target.1);
    }
    if act {
        focus.keyboard = true;
        ev.write(UiAction(target.1));
    }
}

pub fn click_system(q: Query<(Entity, &Interaction, &Btn), Changed<Interaction>>, mut ev: EventWriter<UiAction>, mut focus: ResMut<Focus>) {
    for (e, i, b) in &q {
        match i {
            Interaction::Pressed => {
                ev.write(UiAction(b.action));
            }
            Interaction::Hovered => {
                focus.entity = Some(e);
                focus.action = Some(b.action);
                focus.keyboard = false;
            }
            Interaction::None => {}
        }
    }
}

pub fn click_sound(mut ev: EventReader<UiAction>, mut sfx: Sfx) {
    if ev.read().count() > 0 {
        sfx.play_vol("click", 0.6);
    }
}

pub fn button_visuals(focus: Res<Focus>, mut q: Query<(Entity, &Interaction, &Btn, &mut BackgroundColor, &mut Outline)>) {
    for (e, i, b, mut bg, mut ol) in &mut q {
        let focused = focus.keyboard && focus.entity == Some(e);
        let c = match i {
            Interaction::Pressed => b.color.mix(&Color::WHITE, 0.3),
            Interaction::Hovered => b.color.mix(&Color::WHITE, 0.15),
            Interaction::None if focused => b.color.mix(&Color::WHITE, 0.15),
            Interaction::None => b.color,
        };
        if bg.0 != c {
            bg.0 = c;
        }
        let oc = if focused { C_GOLD } else { Color::NONE };
        if ol.color != oc {
            ol.color = oc;
        }
    }
}

/// Settings that work everywhere (menu, pause, debrief).
pub fn handle_common(
    mut ev: EventReader<UiAction>,
    mut commands: Commands,
    mut profile: ResMut<Profile>,
    lib: Res<SfxLib>,
    ambient: Query<Entity, With<Ambient>>,
    state: Res<State<AppState>>,
) {
    for UiAction(a) in ev.read() {
        match a {
            Action::ToggleLang => {
                profile.lang_en = !profile.lang_en;
                crate::save::save(&profile);
            }
            Action::ToggleSound => {
                profile.sound = !profile.sound;
                crate::save::save(&profile);
                if !profile.sound {
                    for e in &ambient {
                        commands.entity(e).despawn();
                    }
                } else if *state.get() == AppState::Playing && ambient.is_empty() {
                    crate::audio::spawn_ambient(&mut commands, &lib);
                }
            }
            _ => {}
        }
    }
}

fn alloc(sim: &mut Sim, b: Block, d: i32) -> bool {
    let used = sim.alloc.used();
    let avail = sim.available();
    let max = if b == Block::Shield { SHIELD_MAX } else { 6 };
    let slot = match b {
        Block::Ls => &mut sim.alloc.ls,
        Block::Green => &mut sim.alloc.green,
        Block::Shield => &mut sim.alloc.shield,
        Block::Sci => &mut sim.alloc.sci,
    };
    if d > 0 {
        if used >= avail || *slot >= max {
            return false;
        }
        *slot += 1;
    } else {
        if *slot == 0 {
            return false;
        }
        *slot -= 1;
    }
    true
}

pub fn handle_game_actions(
    mut ev: EventReader<UiAction>,
    mut sim: ResMut<Sim>,
    mut modal: ResMut<Modal>,
    mut alerts: ResMut<Alerts>,
    mut breath: ResMut<Breath>,
    mut next: ResMut<NextState<AppState>>,
    mut page: ResMut<crate::menu::MenuPage>,
    mut sfx: Sfx,
) {
    for UiAction(a) in ev.read() {
        match *a {
            Action::Alloc(b, d) => {
                if alloc(&mut sim, b, d) {
                    modal.set_changed();
                } else {
                    sfx.play("bad");
                }
            }
            Action::Close => modal.close(),
            Action::Harvest(eat) => {
                if sim.harvest_ready {
                    if eat {
                        sim.harvest_eat();
                        alerts.push(Alert::good("plate", "Вкусный ужин! +12 еды, +10 настроения", "Tasty dinner! +12 food, +10 mood"));
                    } else {
                        sim.harvest_lab();
                        alerts.push(Alert::good("science", "Растения изучены: +2 науки", "Plants studied: +2 science"));
                    }
                    sfx.play("good");
                }
                modal.close();
            }
            Action::Cargo(all) => {
                if sim.cargo {
                    if all {
                        sim.cargo_all();
                        alerts.push(Alert::good("box", "Груз разгружен: +20 еды, +8 кислорода", "Cargo unloaded: +20 food, +8 oxygen"));
                    } else {
                        sim.cargo_some();
                        alerts.push(Alert::good("bread", "Взяли еду: +8", "Took food: +8"));
                    }
                    sfx.play("coin");
                }
                modal.close();
            }
            Action::Feel(i) => {
                if let ModalKind::Emotion(m) = modal.kind {
                    modal.pick = Some(i);
                    if MOODS[m].ok.contains(&FEELINGS[i].id) {
                        modal.step = 1;
                        sfx.play("good");
                    }
                }
            }
            Action::Support(i) => {
                if let (ModalKind::Emotion(m), true) = (modal.kind, sim.emotion.is_some()) {
                    sim.support_crew();
                    let (icon, ru, en) = MOODS[m].support[i.min(MOODS[m].support.len() - 1)];
                    alerts.push(Alert::new(AlertKind::Good, icon, format!("{ru} — экипажу легче · настроение +20"), format!("{en} — the crew feels better · mood +20")));
                    sfx.play("good");
                }
                modal.close();
            }
            Action::Pause => modal.open(ModalKind::Pause, 0),
            Action::Beacon => {
                breath.0 = 0.0;
                modal.open(ModalKind::Beacon, 0);
            }
            Action::Help => modal.open(ModalKind::Tutorial(0), 0),
            Action::TutNext => {
                if let ModalKind::Tutorial(n) = modal.kind {
                    if (n as usize + 1) < TUTORIAL.len() {
                        modal.kind = ModalKind::Tutorial(n + 1);
                    } else {
                        modal.close();
                    }
                }
            }
            Action::EndMission => {
                *page = crate::menu::MenuPage::Maps;
                next.set(AppState::Menu);
            }
            _ => {}
        }
    }
}

/// Esc / Start opens the pause window; Esc / B closes any window.
pub fn game_keys(inputs: Res<Inputs>, mut modal: ResMut<Modal>) {
    let [a, b] = inputs.0;
    if modal.kind != ModalKind::None {
        let close = if modal.kind.global() {
            a.back || b.back || a.pause || b.pause
        } else {
            let i = inputs.0[modal.owner.min(1) as usize];
            i.back || i.pause
        };
        if close {
            modal.close();
        }
    } else if a.pause || b.pause {
        modal.open(ModalKind::Pause, 0);
    }
}

// ---------------------------------------------------------------- HUD

#[derive(Component)]
pub struct HudRoot;
#[derive(Component)]
pub struct AlertList;
#[derive(Component)]
pub struct AlertItem;
#[derive(Component)]
pub struct ModalRoot;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stat {
    O2,
    Food,
    Dose,
    Sci,
    Morale,
}

/// Text that is refreshed every frame.
#[derive(Component, Clone, Copy)]
pub enum Live {
    Bar(Stat),
    Sol,
    Power,
    Battery,
    Flare,
    Mc,
    Prompt(u8),
    Carry(u8),
    Breath,
}

/// Node whose width (or size) follows a value.
#[derive(Component, Clone, Copy)]
pub enum Fill {
    Bar(Stat),
    Sol,
    Battery,
    Flare,
    Mc,
    Prompt(u8),
    Breath,
}

/// Node shown only while something is true.
#[derive(Component, Clone, Copy)]
pub enum Show {
    Flare,
    Mc,
    Prompt(u8),
    PromptBar(u8),
    Carry(u8),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Day,
    Mc,
    Prompt(u8),
    Sound,
}

#[derive(Component)]
pub struct LiveIcon {
    pub kind: IconKind,
    pub cur: &'static str,
}

pub const TUTORIAL: &[(&str, &str, &str)] = &[
    ("walk", "Ходи клавишами WASD или стиком. Камера — Q и E или мышь с зажатой кнопкой. Прыжок — пробел.", "Walk with WASD or the stick. Camera: Q and E, or drag the mouse. Jump: Space."),
    ("power", "Подойди к пульту энергии (там светится луч) и нажми F. Раздели энергию между модулями.", "Walk to the power console (look for the light beam) and press F. Share the power between modules."),
    ("oxygen", "Следи за полосками слева: кислород, еда, радиация, наука. Ни одно решение не бесплатно.", "Watch the bars on the left: oxygen, food, radiation, science. No choice is free."),
    ("sun", "Вспышка на Солнце? Беги к двери купола и нажми F. В убежище радиации почти нет.", "Solar flare? Run to the dome door and press F. The shelter blocks most radiation."),
    ("wrench", "Что-то сломалось — подойди и держи F. Собирай светящиеся образцы и неси в лабораторию.", "Something broke? Walk up and hold F. Collect glowing samples and bring them to the lab."),
];

fn hud_stats(cadet: bool) -> Vec<(Stat, &'static str, Color, &'static str, &'static str)> {
    let mut v = vec![(Stat::O2, "oxygen", C_O2, "Кислород", "Oxygen")];
    if !cadet {
        v.push((Stat::Food, "food", C_FOOD, "Еда", "Food"));
    }
    v.push((Stat::Dose, "radiation", C_DOSE, "Радиация (доза)", "Radiation (dose)"));
    v.push((Stat::Sci, "science", C_SCI, "Наука", "Science"));
    v.push((Stat::Morale, "heart", C_MOOD, "Настроение", "Mood"));
    v
}

pub fn rebuild_hud(
    mut commands: Commands,
    roots: Query<Entity, With<HudRoot>>,
    lang: Res<Lang>,
    ui: Res<UiAssets>,
    sim: Res<Sim>,
    mut alerts: ResMut<Alerts>,
) {
    if !roots.is_empty() && !lang.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    alerts.dirty = true;
    let ctx = Ctx { ui: &ui, lang: *lang };
    let players = if sim.coop { 2u8 } else { 1 };
    let abs = |l: Option<f32>, r: Option<f32>, t: Option<f32>, b: Option<f32>| Node {
        position_type: PositionType::Absolute,
        left: l.map(Val::Px).unwrap_or(Val::Auto),
        right: r.map(Val::Px).unwrap_or(Val::Auto),
        top: t.map(Val::Px).unwrap_or(Val::Auto),
        bottom: b.map(Val::Px).unwrap_or(Val::Auto),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(10.0),
        ..default()
    };
    commands
        .spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, HudRoot, StateScoped(AppState::Playing)))
        .with_children(|root| {
            if sim.coop {
                root.spawn((
                    Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), width: Val::Px(4.0), margin: UiRect::left(Val::Px(-2.0)), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(Color::srgb(0.02, 0.03, 0.08)),
                ));
            }
            // ---- left column: resources + Mission Control
            root.spawn(abs(Some(12.0), None, Some(12.0), None)).with_children(|col| {
                ctx.panel(col, C_PANEL).with_children(|p| {
                    for (stat, icon, color, ru, en) in hud_stats(sim.cadet) {
                        ctx.row(p, 8.0).with_children(|r| {
                            ctx.icon(r, icon, 32.0);
                            r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(2.0), ..default() }).with_children(|c| {
                                ctx.text(c, ctx.t(ru, en), 13.0, C_DIM);
                                c.spawn((Node { width: Val::Px(150.0), height: Val::Px(12.0), ..default() }, BackgroundColor(C_TRACK), BorderRadius::all(Val::Px(6.0))))
                                    .with_children(|b| {
                                        b.spawn((Node { width: Val::Percent(50.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(color), BorderRadius::all(Val::Px(6.0)), Fill::Bar(stat)));
                                    });
                            });
                            let t = ctx.text(r, "", 17.0, C_TEXT);
                            r.commands().entity(t).insert((Live::Bar(stat), Node { width: Val::Px(62.0), ..default() }));
                        });
                    }
                });
                // Mission Control message (arrives after the comms delay)
                col.spawn((
                    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(10.0)), width: Val::Px(300.0), display: Display::None, ..default() },
                    BackgroundColor(C_PANEL),
                    BorderRadius::all(Val::Px(14.0)),
                    Show::Mc,
                ))
                .with_children(|r| {
                    let i = ctx.icon(r, "radio", 38.0);
                    r.commands().entity(i).insert(LiveIcon { kind: IconKind::Mc, cur: "radio" });
                    r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), flex_shrink: 1.0, ..default() }).with_children(|c| {
                        ctx.text(c, ctx.t("ЦУП · Земля", "Mission Control · Earth"), 13.0, C_DIM);
                        let t = ctx.text(c, "", 16.0, C_TEXT);
                        c.commands().entity(t).insert(Live::Mc);
                        c.spawn((Node { width: Val::Px(220.0), height: Val::Px(5.0), ..default() }, BackgroundColor(C_TRACK))).with_children(|b| {
                            b.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_ACCENT), Fill::Mc));
                        });
                    });
                });
            });
            // ---- top centre: sol clock, flare banner, alerts
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(340.0),
                right: Val::Px(250.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|c| {
                c.spawn((
                    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() },
                    BackgroundColor(C_PANEL),
                    BorderRadius::all(Val::Px(16.0)),
                ))
                .with_children(|r| {
                    let i = ctx.icon(r, "sun", 36.0);
                    r.commands().entity(i).insert(LiveIcon { kind: IconKind::Day, cur: "sun" });
                    r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() }).with_children(|c| {
                        let t = ctx.head(c, "", 22.0, C_TEXT);
                        c.commands().entity(t).insert(Live::Sol);
                        c.spawn((Node { width: Val::Px(170.0), height: Val::Px(8.0), ..default() }, BackgroundColor(C_TRACK), BorderRadius::all(Val::Px(4.0)))).with_children(|b| {
                            b.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOLD), BorderRadius::all(Val::Px(4.0)), Fill::Sol));
                        });
                    });
                });
                c.spawn((
                    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(10.0)), max_width: Val::Px(620.0), display: Display::None, ..default() },
                    BackgroundColor(Color::srgba(0.75, 0.12, 0.10, 0.95)),
                    BorderRadius::all(Val::Px(16.0)),
                    Show::Flare,
                ))
                .with_children(|r| {
                    ctx.icon(r, "radiation", 46.0);
                    r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), flex_shrink: 1.0, ..default() }).with_children(|c| {
                        ctx.head(c, ctx.t("ВСПЫШКА! Беги к двери купола и нажми F", "FLARE! Run to the dome door and press F"), 19.0, C_TEXT);
                        let t = ctx.text(c, "", 15.0, C_TEXT);
                        c.commands().entity(t).insert(Live::Flare);
                        c.spawn((Node { width: Val::Px(260.0), height: Val::Px(7.0), ..default() }, BackgroundColor(C_TRACK))).with_children(|b| {
                            b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOLD), Fill::Flare));
                        });
                    });
                });
                c.spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), ..default() }, AlertList));
            });
            // ---- top right: buttons, power, carried samples
            root.spawn(Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|c| {
                ctx.row(c, 8.0).with_children(|r| {
                    ctx.btn(r, B::new(Action::Beacon, Some("beacon"), "").size(18.0).mouse_only());
                    ctx.btn_custom(r, Action::ToggleSound, C_BTN, Val::Auto, false).with_children(|b| {
                        let i = ctx.icon(b, "sound", 29.0);
                        b.commands().entity(i).insert(LiveIcon { kind: IconKind::Sound, cur: "sound" });
                    });
                    ctx.btn(r, B::new(Action::Pause, Some("gear"), "").size(18.0).mouse_only());
                });
                ctx.panel(c, C_PANEL).with_children(|p| {
                    ctx.row(p, 8.0).with_children(|r| {
                        ctx.icon(r, "power", 28.0);
                        let t = ctx.text(r, "", 17.0, C_TEXT);
                        r.commands().entity(t).insert(Live::Power);
                    });
                    ctx.row(p, 8.0).with_children(|r| {
                        ctx.icon(r, "battery", 28.0);
                        r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), ..default() }).with_children(|c| {
                            let t = ctx.text(c, "", 17.0, C_TEXT);
                            c.commands().entity(t).insert(Live::Battery);
                            c.spawn((Node { width: Val::Px(150.0), height: Val::Px(8.0), ..default() }, BackgroundColor(C_TRACK), BorderRadius::all(Val::Px(4.0)))).with_children(|b| {
                                b.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_FOOD), BorderRadius::all(Val::Px(4.0)), Fill::Battery));
                            });
                        });
                    });
                });
                for i in 0..players {
                    c.spawn((
                        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(8.0)), display: Display::None, ..default() },
                        BackgroundColor(C_PANEL),
                        BorderRadius::all(Val::Px(12.0)),
                        Show::Carry(i),
                    ))
                    .with_children(|r| {
                        ctx.icon(r, "gem", 26.0);
                        let t = ctx.text(r, "", 16.0, C_TEXT);
                        r.commands().entity(t).insert(Live::Carry(i));
                    });
                }
            });
            // ---- bottom: action prompt per player
            root.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), right: Val::Px(0.0), bottom: Val::Px(22.0), flex_direction: FlexDirection::Row, ..default() })
                .with_children(|row| {
                    for i in 0..players {
                        row.spawn(Node { width: Val::Percent(100.0 / players as f32), justify_content: JustifyContent::Center, ..default() }).with_children(|half| {
                            half.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(10.0),
                                    padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    display: Display::None,
                                    ..default()
                                },
                                BackgroundColor(C_PANEL2),
                                BorderColor(C_ACCENT),
                                BorderRadius::all(Val::Px(18.0)),
                                Show::Prompt(i),
                            ))
                            .with_children(|r| {
                                let ic = ctx.icon(r, "hint", 40.0);
                                r.commands().entity(ic).insert(LiveIcon { kind: IconKind::Prompt(i), cur: "hint" });
                                r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), ..default() }).with_children(|c| {
                                    let t = ctx.head(c, "", 20.0, C_TEXT);
                                    c.commands().entity(t).insert(Live::Prompt(i));
                                    c.spawn((Node { width: Val::Px(190.0), height: Val::Px(9.0), display: Display::None, ..default() }, BackgroundColor(C_TRACK), BorderRadius::all(Val::Px(4.0)), Show::PromptBar(i)))
                                        .with_children(|b| {
                                            b.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(C_GOLD), BorderRadius::all(Val::Px(4.0)), Fill::Prompt(i)));
                                        });
                                });
                            });
                        });
                    }
                });
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update_hud(
    time: Res<Time>,
    sim: Res<Sim>,
    lang: Res<Lang>,
    mc: Res<McMsg>,
    prompts: Res<Prompts>,
    profile: Res<Profile>,
    breath: Res<Breath>,
    ui: Res<UiAssets>,
    players: Query<&Player>,
    mut texts: Query<(&Live, &mut Text, &mut TextColor)>,
    mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor), Without<Show>>,
    mut shows: Query<(&Show, &mut Node, &mut BackgroundColor), Without<Fill>>,
    mut icons: Query<(&mut LiveIcon, &mut ImageNode)>,
) {
    let l = *lang;
    let now = time.elapsed_secs();
    let blink = (now * 5.0).sin() > 0.0;
    let mut carried = [0u32; 2];
    for p in &players {
        carried[p.idx.min(1) as usize] = p.carried;
    }
    let mc_wait = mc.arrive_at - now;
    let mc_frac = if mc.arrive_at > mc.sent_at { ((now - mc.sent_at) / (mc.arrive_at - mc.sent_at)).clamp(0.0, 1.0) } else { 1.0 };
    let stat = |s: Stat| -> f32 {
        match s {
            Stat::O2 => sim.o2,
            Stat::Food => sim.food,
            Stat::Dose => sim.dose,
            Stat::Sci => (sim.science / sim.goal.max(1.0) * 100.0).min(100.0),
            Stat::Morale => sim.morale,
        }
    };
    let breath_phase = breath.0 % 8.0;
    let breath_k = if breath_phase < 4.0 { breath_phase / 4.0 } else { 1.0 - (breath_phase - 4.0) / 4.0 };
    let breath_k = breath_k * breath_k * (3.0 - 2.0 * breath_k);

    for (live, mut text, mut color) in &mut texts {
        let (s, c): (String, Color) = match *live {
            Live::Bar(st) => {
                let v = stat(st);
                match st {
                    Stat::Sci => (format!("{:.1}/{:.0}", sim.science, sim.goal), C_TEXT),
                    Stat::Dose => (format!("{v:.0}%"), if v > 70.0 { C_DOSE } else { C_TEXT }),
                    _ => (format!("{v:.0}%"), if v < 25.0 && blink { C_DANGER } else { C_TEXT }),
                }
            }
            Live::Sol => (format!("{} {}/{}", l.t("Сол", "Sol"), sim.sol, sim.sols), C_TEXT),
            Live::Power => {
                let used = sim.alloc.used();
                let gen = sim.gen_now.max(0.0) as u32;
                (format!("{} {used} / {gen}", l.t("Занято", "Used")), if used > gen { C_WARN } else { C_TEXT })
            }
            Live::Battery => (format!("{} {:.1}/10", l.t("Батарея", "Battery"), sim.battery), if sim.battery < 1.0 && blink { C_DANGER } else { C_TEXT }),
            Live::Flare => match &sim.flare {
                Some(f) => {
                    let src = if f.live { "LIVE" } else { l.t("архив", "archive") };
                    (format!("NASA DONKI: {} · {} · {src}", l.t(&f.event.label_ru, &f.event.label_en), f.event.date), C_TEXT)
                }
                None => (String::new(), C_TEXT),
            },
            Live::Mc => {
                if mc_wait > 0.0 {
                    (format!("{} {mc_wait:.1} {}", l.t("Сигнал летит…", "Signal travelling…"), l.t("с", "s")), C_DIM)
                } else {
                    (l.t(mc.ru, mc.en).to_string(), C_TEXT)
                }
            }
            Live::Prompt(i) => match prompts.0[i as usize] {
                Some(p) => {
                    let key = if i == 0 { "F" } else { "RShift" };
                    (format!("{}  [{key}]", l.t(p.ru, p.en)), if p.warn { C_GOLD } else { C_TEXT })
                }
                None => (String::new(), C_TEXT),
            },
            Live::Carry(i) => (format!("{} {}", l.t("Образцы:", "Samples:"), carried[i as usize]), C_TEXT),
            Live::Breath => {
                if breath_phase < 4.0 {
                    (format!("{} {:.0}", l.t("Вдох…", "Breathe in…"), (4.0 - breath_phase).ceil()), C_TEXT)
                } else {
                    (format!("{} {:.0}", l.t("Выдох…", "Breathe out…"), (8.0 - breath_phase).ceil()), C_TEXT)
                }
            }
        };
        if text.0 != s {
            text.0 = s;
        }
        if color.0 != c {
            color.0 = c;
        }
    }

    for (fill, mut node, mut bg) in &mut fills {
        let pct = match *fill {
            Fill::Bar(st) => {
                let v = stat(st);
                if st == Stat::O2 {
                    bg.0 = if v < 25.0 && blink { C_DANGER } else { C_O2 };
                }
                v
            }
            Fill::Sol => sim.t * 100.0,
            Fill::Battery => sim.battery / crate::sim::BATTERY_MAX * 100.0,
            Fill::Flare => sim.flare.as_ref().map(|f| f.t_left / f.duration * 100.0).unwrap_or(0.0),
            Fill::Mc => mc_frac * 100.0,
            Fill::Prompt(i) => prompts.0[i as usize].and_then(|p| p.progress).unwrap_or(0.0) * 100.0,
            Fill::Breath => {
                let s = Val::Px(90.0 + 150.0 * breath_k);
                if node.width != s {
                    node.width = s;
                    node.height = s;
                }
                continue;
            }
        };
        let w = Val::Percent(pct.clamp(0.0, 100.0));
        if node.width != w {
            node.width = w;
        }
    }

    for (show, mut node, mut bg) in &mut shows {
        let on = match *show {
            Show::Flare => sim.flare.is_some(),
            Show::Mc => mc.active,
            Show::Prompt(i) => {
                if let Some(p) = prompts.0[i as usize] {
                    let c = if p.warn { Color::srgba(0.45, 0.25, 0.05, 0.96) } else { C_PANEL2 };
                    if bg.0 != c {
                        bg.0 = c;
                    }
                    true
                } else {
                    false
                }
            }
            Show::PromptBar(i) => prompts.0[i as usize].and_then(|p| p.progress).is_some(),
            Show::Carry(i) => carried[i as usize] > 0,
        };
        let d = if on { Display::Flex } else { Display::None };
        if node.display != d {
            node.display = d;
        }
    }

    for (mut li, mut img) in &mut icons {
        let name: &'static str = match li.kind {
            IconKind::Day => {
                if sim.is_night() {
                    "night"
                } else if sim.dust_left > 0 {
                    "tornado"
                } else {
                    "sun"
                }
            }
            IconKind::Mc => {
                if mc_wait > 0.0 || mc.icon.is_empty() {
                    "radio"
                } else {
                    mc.icon
                }
            }
            IconKind::Prompt(i) => prompts.0[i as usize].map(|p| p.icon).unwrap_or("hint"),
            IconKind::Sound => {
                if profile.sound {
                    "sound"
                } else {
                    "mute"
                }
            }
        };
        if li.cur != name {
            li.cur = name;
            img.image = ui.server.load(format!("icons/{name}.png"));
        }
    }
}

pub fn update_alerts(
    mut commands: Commands,
    time: Res<Time>,
    mut alerts: ResMut<Alerts>,
    lang: Res<Lang>,
    ui: Res<UiAssets>,
    list: Query<Entity, With<AlertList>>,
    items: Query<Entity, With<AlertItem>>,
) {
    let dt = time.delta_secs();
    let before = alerts.items.len();
    for a in alerts.items.iter_mut() {
        a.age += dt;
    }
    alerts.items.retain(|a| a.age < a.ttl());
    if alerts.items.len() != before {
        alerts.dirty = true;
    }
    if !alerts.dirty {
        return;
    }
    let Ok(list) = list.single() else { return };
    alerts.dirty = false;
    for e in &items {
        commands.entity(e).despawn();
    }
    let ctx = Ctx { ui: &ui, lang: *lang };
    commands.entity(list).with_children(|p| {
        for a in alerts.items.iter().rev() {
            p.spawn((
                Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)), max_width: Val::Px(600.0), ..default() },
                BackgroundColor(a.color()),
                BorderRadius::all(Val::Px(12.0)),
                AlertItem,
            ))
            .with_children(|r| {
                ctx.icon(r, a.icon, 28.0);
                ctx.text(r, ctx.t(&a.ru, &a.en), 17.0, C_TEXT);
            });
        }
    });
}

pub fn beacon_tick(time: Res<Time>, modal: Res<Modal>, mut breath: ResMut<Breath>, mut profile: ResMut<Profile>, mut alerts: ResMut<Alerts>, mut sfx: Sfx) {
    if modal.kind != ModalKind::Beacon {
        return;
    }
    let before = breath.0;
    breath.0 += time.delta_secs();
    // two full calm breaths; the beacon itself is never scored
    if before < 16.0 && breath.0 >= 16.0 && profile.award("beacon") {
        crate::save::save(&profile);
        alerts.push(Alert::badge("beacon", "Нашивка: Знаю, как успокоиться", "Patch: I know how to calm down"));
        sfx.play("badge");
    }
}

// ---------------------------------------------------------------- modal windows

#[allow(clippy::too_many_arguments)]
pub fn rebuild_modal(
    mut commands: Commands,
    modal: Res<Modal>,
    sim: Res<Sim>,
    lang: Res<Lang>,
    ui: Res<UiAssets>,
    profile: Res<Profile>,
    roots: Query<Entity, With<ModalRoot>>,
    time: Res<Time>,
    mut refresh: Local<f32>,
) {
    *refresh += time.delta_secs();
    // in co-op the world keeps running behind the power console: refresh its numbers
    let periodic = modal.kind == ModalKind::Board && sim.coop && *refresh > 1.0;
    if !(modal.is_changed() || lang.is_changed() || profile.is_changed() && modal.kind == ModalKind::Pause || periodic) {
        return;
    }
    *refresh = 0.0;
    for e in &roots {
        commands.entity(e).despawn();
    }
    if modal.kind == ModalKind::None {
        return;
    }
    let ctx = Ctx { ui: &ui, lang: *lang };
    let half = sim.coop && !modal.kind.global();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(if half { 50.0 * modal.owner.min(1) as f32 } else { 0.0 }),
                top: Val::Px(0.0),
                width: Val::Percent(if half { 50.0 } else { 100.0 }),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.02, 0.06, 0.55)),
            FocusPolicy::Block,
            GlobalZIndex(20),
            ModalRoot,
            StateScoped(AppState::Playing),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(22.0)),
                    row_gap: Val::Px(12.0),
                    max_width: Val::Px(if half { 620.0 } else { 780.0 }),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(C_PANEL2),
                BorderColor(C_ACCENT),
                BorderRadius::all(Val::Px(22.0)),
            ))
            .with_children(|p| match modal.kind {
                ModalKind::Board => board(p, &ctx, &sim, half),
                ModalKind::Harvest => harvest(p, &ctx),
                ModalKind::Cargo => cargo(p, &ctx),
                ModalKind::Emotion(m) => emotion(p, &ctx, m, &modal),
                ModalKind::Tutorial(n) => tutorial(p, &ctx, n, sim.coop),
                ModalKind::Pause => pause(p, &ctx, &profile, sim.coop),
                ModalKind::Beacon => beacon(p, &ctx),
                ModalKind::None => {}
            });
        });
}

fn board(p: &mut ChildSpawnerCommands, ctx: &Ctx, sim: &Sim, half: bool) {
    let ctx = *ctx;
    let used = sim.alloc.used();
    let gen = sim.gen_now.max(0.0) as u32;
    ctx.row(p, 12.0).with_children(|r| {
        ctx.icon(r, "power", 50.0);
        ctx.head(r, ctx.t("Пульт энергии", "Power console"), 30.0, C_TEXT);
    });
    let gen_ru = if sim.is_night() { "Ночь: солнце не светит" } else if sim.dust_left > 0 && !sim.panels_cleaned { "Пыль на панелях" } else { "Солнечные панели" };
    let gen_en = if sim.is_night() { "Night: no sunlight" } else if sim.dust_left > 0 && !sim.panels_cleaned { "Dust on the panels" } else { "Solar panels" };
    ctx.text(p, format!("{}: {gen} {} · {} {:.1}", ctx.t(gen_ru, gen_en), ctx.t("блоков за сол", "blocks per sol"), ctx.t("Батарея", "Battery"), sim.battery), 17.0, C_DIM);
    // blocks: yellow = sunlight, orange = taken from the battery
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), flex_wrap: FlexWrap::Wrap, max_width: Val::Px(600.0), ..default() }).with_children(|r| {
        for i in 0..used.max(gen) {
            let c = if i < used {
                if i < gen { C_GOLD } else { C_WARN }
            } else {
                C_TRACK
            };
            r.spawn((Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() }, BackgroundColor(c), BorderRadius::all(Val::Px(5.0))));
        }
    });
    let w = if half { 540.0 } else { 680.0 };
    let goal = sim.goal;
    let o2_hint = format!("{} +{O2_PER:.0} {} · {} {O2_USE:.0}", ctx.t("1 блок =", "1 block ="), ctx.t("кислорода за сол", "oxygen per sol"), ctx.t("экипаж дышит", "the crew breathes"));
    let food_hint = format!("{} +{FOOD_PER:.0} {} · {} {FOOD_USE:.0}", ctx.t("1 блок =", "1 block ="), ctx.t("еды за сол", "food per sol"), ctx.t("экипаж ест", "the crew eats"));
    let shield_hint = format!("{} −{:.0}% {} ({} {SHIELD_MAX})", ctx.t("1 блок =", "1 block ="), SHIELD_PER * 100.0, ctx.t("радиации", "radiation"), ctx.t("макс.", "max"));
    let sci_hint = format!("{} +1 {} · {} {goal:.0}", ctx.t("1 блок =", "1 block ="), ctx.t("науки за сол", "science per sol"), ctx.t("цель", "goal"));
    board_row(p, &ctx, w, "oxygen", C_O2, ctx.t("Кислород", "Oxygen"), &o2_hint, sim.alloc.ls, 6, Block::Ls);
    if !sim.cadet {
        board_row(p, &ctx, w, "plant", C_FOOD, ctx.t("Оранжерея", "Greenhouse"), &food_hint, sim.alloc.green, 6, Block::Green);
    }
    board_row(p, &ctx, w, "shield", C_ACCENT, ctx.t("Щит от радиации", "Radiation shield"), &shield_hint, sim.alloc.shield, SHIELD_MAX, Block::Shield);
    board_row(p, &ctx, w, "science", C_SCI, ctx.t("Наука", "Science"), &sci_hint, sim.alloc.sci, 6, Block::Sci);
    if used > gen {
        let (ru, en, c) = if sim.battery <= 0.0 {
            ("Батарея пуста — модули работают слабее!", "Battery empty — modules run weaker!", C_DANGER)
        } else {
            ("Ты берёшь больше, чем даёт солнце — батарея разряжается", "You use more than the sun gives — the battery drains", C_WARN)
        };
        ctx.line(p, "warning", ctx.t(ru, en), 16.0, c);
    } else if used < gen {
        ctx.line(p, "battery", ctx.t("Лишняя энергия заряжает батарею", "Spare power charges the battery"), 16.0, C_FOOD);
    }
    ctx.btn(p, B::new(Action::Close, Some("check"), ctx.t("Готово", "Done")).color(C_GO));
}

#[allow(clippy::too_many_arguments)]
fn board_row(p: &mut ChildSpawnerCommands, ctx: &Ctx, w: f32, icon: &str, color: Color, name: &str, hint: &str, value: u32, max: u32, block: Block) {
    let ctx = *ctx;
    let (name, hint) = (name.to_string(), hint.to_string());
    p.spawn((
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), width: Val::Px(w), padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)), ..default() },
        BackgroundColor(C_PANEL),
        BorderRadius::all(Val::Px(14.0)),
    ))
    .with_children(|r| {
        ctx.icon(r, icon, 44.0);
        r.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, flex_shrink: 1.0, row_gap: Val::Px(2.0), ..default() }).with_children(|c| {
            ctx.head(c, name, 19.0, C_TEXT);
            ctx.text(c, hint, 14.0, C_DIM);
        });
        ctx.btn(r, B::new(Action::Alloc(block, -1), None, "-").size(26.0).width(Val::Px(52.0)));
        r.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(3.0), width: Val::Px(112.0), justify_content: JustifyContent::Center, ..default() }).with_children(|pp| {
            for i in 0..max {
                pp.spawn((Node { width: Val::Px(14.0), height: Val::Px(26.0), ..default() }, BackgroundColor(if i < value { color } else { C_TRACK }), BorderRadius::all(Val::Px(4.0))));
            }
        });
        ctx.btn(r, B::new(Action::Alloc(block, 1), None, "+").size(26.0).width(Val::Px(52.0)));
    });
}

fn choice(p: &mut ChildSpawnerCommands, ctx: &Ctx, action: Action, icon: &'static str, title: &str, sub: &str, color: Color) {
    let ctx = *ctx;
    let (title, sub) = (title.to_string(), sub.to_string());
    ctx.btn_custom(p, action, color, Val::Px(520.0), true).with_children(|b| {
        ctx.icon(b, icon, 48.0);
        b.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, row_gap: Val::Px(2.0), ..default() }).with_children(|c| {
            ctx.head(c, title, 20.0, C_TEXT);
            ctx.text(c, sub, 15.0, C_TEXT);
        });
    });
}

fn harvest(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    ctx.icon(p, "plant", 64.0);
    ctx.head(p, ctx.t("Урожай готов!", "Harvest is ready!"), 28.0, C_TEXT);
    ctx.text(p, ctx.t("В 2015 году астронавты МКС впервые съели салат, выращенный в космосе (NASA Veggie).", "In 2015 ISS astronauts first ate lettuce grown in space (NASA Veggie)."), 16.0, C_DIM);
    choice(p, ctx, Action::Harvest(true), "plate", ctx.t("Съесть вместе", "Eat together"), ctx.t("+12 еды, +10 настроения", "+12 food, +10 mood"), C_GO);
    choice(p, ctx, Action::Harvest(false), "science", ctx.t("Отдать в лабораторию", "Send to the lab"), ctx.t("+2 науки, +4 еды", "+2 science, +4 food"), C_BTN);
    ctx.btn(p, B::new(Action::Close, Some("back"), ctx.t("Позже", "Later")).size(16.0).color(C_LOCK));
}

fn cargo(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    ctx.icon(p, "box", 64.0);
    ctx.head(p, ctx.t("Грузовой корабль", "Cargo ship"), 28.0, C_TEXT);
    ctx.text(p, ctx.t("Большой груз разгружает манипулятор — ему нужна энергия батареи.", "The robotic arm unloads big crates — it needs battery power."), 16.0, C_DIM);
    choice(p, ctx, Action::Cargo(true), "box", ctx.t("Разгрузить всё", "Unload everything"), ctx.t("+20 еды, +8 кислорода, −2 батареи", "+20 food, +8 oxygen, −2 battery"), C_BTN);
    choice(p, ctx, Action::Cargo(false), "bread", ctx.t("Взять только еду", "Take only food"), ctx.t("+8 еды, энергия не нужна", "+8 food, no power needed"), C_GO);
    ctx.btn(p, B::new(Action::Close, Some("back"), ctx.t("Позже", "Later")).size(16.0).color(C_LOCK));
}

fn emotion(p: &mut ChildSpawnerCommands, ctx: &Ctx, m: usize, modal: &Modal) {
    let ctx = *ctx;
    let mood = &MOODS[m.min(MOODS.len() - 1)];
    let who = if ctx.en() { who_en(mood.who) } else { mood.who };
    ctx.icon(p, "speech", 56.0);
    ctx.text(p, ctx.t(mood.text_ru, mood.text_en), 19.0, C_TEXT);
    if modal.step == 0 {
        ctx.head(p, format!("{} {who}?", ctx.t("Что чувствует", "How does it feel for")), 22.0, C_ACCENT);
        p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, justify_content: JustifyContent::Center, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), max_width: Val::Px(600.0), ..default() })
            .with_children(|g| {
                for (i, f) in FEELINGS.iter().enumerate() {
                    let wrong = modal.pick == Some(i);
                    ctx.btn(g, B::new(Action::Feel(i), Some(f.icon), ctx.t(f.name_ru, f.name_en)).size(17.0).width(Val::Px(185.0)).color(if wrong { C_LOCK } else { C_BTN }));
                }
            });
        if modal.pick.is_some() {
            ctx.line(p, "hint", ctx.t("Может быть. А что ещё? Посмотри на подсказки в истории.", "Maybe. What else could it be? Look at the clues in the story."), 16.0, C_GOLD);
        }
    } else {
        let f = &FEELINGS[modal.pick.unwrap_or(0).min(FEELINGS.len() - 1)];
        ctx.head(p, format!("{} {}. {}", ctx.t("Да, похоже на", "Yes, it looks like"), ctx.t(f.name_ru, f.name_en), ctx.t("Как поддержать?", "How can you help?")), 21.0, C_FOOD);
        for (i, (icon, ru, en)) in mood.support.iter().enumerate() {
            ctx.btn(p, B::new(Action::Support(i), Some(icon), ctx.t(ru, en)).size(18.0).width(Val::Px(480.0)).color(C_GO));
        }
        ctx.text(p, ctx.t("Поддержка занимает время: −1 батареи, зато +20 настроения.", "Support takes time: −1 battery, but +20 mood."), 15.0, C_DIM);
    }
    ctx.btn(p, B::new(Action::Close, Some("back"), ctx.t("Позже", "Later")).size(16.0).color(C_LOCK));
}

fn tutorial(p: &mut ChildSpawnerCommands, ctx: &Ctx, n: u8, coop: bool) {
    let (icon, ru, en) = TUTORIAL[(n as usize).min(TUTORIAL.len() - 1)];
    ctx.text(p, format!("{} {}/{}", ctx.t("Обучение", "Training"), n + 1, TUTORIAL.len()), 15.0, C_DIM);
    ctx.icon(p, icon, 72.0);
    ctx.text(p, ctx.t(ru, en), 21.0, C_TEXT);
    if n == 0 && coop {
        ctx.text(p, ctx.t("Игрок 2: стрелки, правый Shift — действие, правый Ctrl — прыжок, клавиши , и . — камера.", "Player 2: arrows, Right Shift — action, Right Ctrl — jump, , and . — camera."), 16.0, C_ACCENT);
    }
    let last = n as usize + 1 >= TUTORIAL.len();
    ctx.row(p, 12.0).with_children(|r| {
        ctx.btn(r, B::new(Action::TutNext, Some(if last { "check" } else { "play" }), if last { ctx.t("Понятно!", "Got it!") } else { ctx.t("Дальше", "Next") }).color(C_GO));
        if !last {
            ctx.btn(r, B::new(Action::Close, None, ctx.t("Пропустить", "Skip")).size(16.0).color(C_LOCK));
        }
    });
}

fn pause(p: &mut ChildSpawnerCommands, ctx: &Ctx, profile: &Profile, coop: bool) {
    let ctx = *ctx;
    ctx.icon(p, "gear", 56.0);
    ctx.head(p, ctx.t("Пауза", "Paused"), 30.0, C_TEXT);
    let w = Val::Px(440.0);
    ctx.btn(p, B::new(Action::Close, Some("play"), ctx.t("Продолжить", "Resume")).color(C_GO).width(w));
    ctx.btn(p, B::new(Action::Beacon, Some("beacon"), ctx.t("Космический маяк: подышать", "Space Beacon: breathe")).width(w));
    ctx.btn(p, B::new(Action::Help, Some("book"), ctx.t("Как играть", "How to play")).width(w));
    ctx.row(p, 10.0).with_children(|r| {
        ctx.btn(r, B::new(Action::ToggleSound, Some(if profile.sound { "sound" } else { "mute" }), if profile.sound { ctx.t("Звук вкл", "Sound on") } else { ctx.t("Звук выкл", "Sound off") }).size(17.0));
        ctx.btn(r, B::new(Action::ToggleLang, Some("globe"), if ctx.en() { "English" } else { "Русский" }).size(17.0));
    });
    ctx.btn(p, B::new(Action::EndMission, Some("exit"), ctx.t("Выйти в меню (без награды)", "Back to menu (no reward)")).size(17.0).color(C_LOCK).width(w));
    let keys = if coop {
        ctx.t(
            "Игрок 1: WASD, F — действие, пробел — прыжок, Q/E — камера.\nИгрок 2: стрелки, правый Shift, правый Ctrl, клавиши , и .",
            "Player 1: WASD, F — action, Space — jump, Q/E — camera.\nPlayer 2: arrows, Right Shift, Right Ctrl, , and . keys",
        )
    } else {
        ctx.t("WASD — идти, F — действие (держи для ремонта), пробел — прыжок, Q/E или мышь — камера, Esc — пауза.", "WASD — walk, F — action (hold to repair), Space — jump, Q/E or mouse — camera, Esc — pause.")
    };
    ctx.text(p, keys, 15.0, C_DIM);
}

fn beacon(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    let ctx = *ctx;
    ctx.head(p, ctx.t("Космический маяк", "Space Beacon"), 28.0, C_TEXT);
    ctx.text(p, ctx.t("Следи за кругом: он растёт — вдыхай, уменьшается — выдыхай.", "Follow the circle: it grows — breathe in, it shrinks — breathe out."), 17.0, C_DIM);
    p.spawn(Node { width: Val::Px(250.0), height: Val::Px(250.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|c| {
        c.spawn((
            Node { width: Val::Px(90.0), height: Val::Px(90.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            BackgroundColor(Color::srgba(0.35, 0.8, 1.0, 0.35)),
            BorderRadius::MAX,
            Fill::Breath,
        ))
        .with_children(|i| {
            ctx.icon(i, "beacon", 56.0);
        });
    });
    let t = ctx.head(p, "", 24.0, C_TEXT);
    p.commands().entity(t).insert(Live::Breath);
    ctx.text(p, ctx.t("Маяк не даёт очков — он просто помогает успокоиться. Игра на паузе.", "The beacon gives no points — it just helps you calm down. The game is paused."), 15.0, C_DIM);
    ctx.btn(p, B::new(Action::Close, Some("check"), ctx.t("Я готов(а)", "I'm ready")).color(C_GO));
}
