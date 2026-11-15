//! A1 — all texts and data that methodologists edit, loaded from `assets/content/*.json`.
//! The code never needs to change for new texts. `load` reports every problem with the file
//! and key; `missing_kk` lists texts still waiting for a Kazakh translation.

use crate::expedition::Level;
use crate::i18n::Lang;
use crate::minigame::MiniKind;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// One text in three languages. No Kazakh yet → Russian is shown.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub ru: String,
    #[serde(default)]
    pub kk: Option<String>,
    pub en: String,
}

impl Text {
    pub fn get(&self, l: Lang) -> &str {
        match l {
            Lang::Ru => &self.ru,
            Lang::En => &self.en,
            Lang::Kk => self.kk.as_deref().filter(|s| !s.is_empty()).unwrap_or(&self.ru),
        }
    }
    pub fn new(ru: &str, en: &str) -> Self {
        Self { ru: ru.into(), kk: None, en: en.into() }
    }
}

/// A text with a simple (Junior) and an optional deeper (Advanced) version.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LevelText {
    pub junior: Text,
    #[serde(default)]
    pub advanced: Option<Text>,
}

impl LevelText {
    pub fn get(&self, level: Level, l: Lang) -> &str {
        match (level, &self.advanced) {
            (Level::Advanced, Some(a)) => a.get(l),
            _ => self.junior.get(l),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Npc {
    pub id: String,
    pub name: Text,
    pub role: Text,
    pub icon: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Console {
    pub id: String,
    /// cell inside the room
    pub x: i32,
    pub y: i32,
    pub name: Text,
    pub icon: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: Text,
    pub icon: String,
    /// rectangle in ship cells (x right, y up)
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub consoles: Vec<Console>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShipLayout {
    pub name: Text,
    pub rooms: Vec<Room>,
    /// door centres on the walls between rooms (cells)
    pub doors: Vec<(f32, f32)>,
    /// where the crew starts
    pub start: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// daily routine, calm
    Routine,
    /// urgent problem with a timer
    Problem,
    /// the asteroid map (Touch Panel)
    Asteroids,
    /// story step that moves the flight on
    Story,
}

/// What happens to the ship when a problem's timer runs out (the game goes on).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Penalty {
    #[serde(default)]
    pub o2: f32,
    #[serde(default)]
    pub co2: f32,
    #[serde(default)]
    pub hull: f32,
    #[serde(default)]
    pub seal: f32,
    #[serde(default)]
    pub energy: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShipTask {
    pub id: String,
    pub kind: TaskKind,
    pub room: String,
    pub console: String,
    #[serde(default)]
    pub minigame: Option<MiniKind>,
    pub title: Text,
    /// what to do and why (Junior / Advanced)
    pub brief: LevelText,
    /// "history: Apollo 13, 1970" — shown with the task
    #[serde(default)]
    pub history: Option<Text>,
    /// seconds for problems (Junior gets ×1.5)
    #[serde(default)]
    pub timer: Option<f32>,
    #[serde(default)]
    pub reward: u32,
    #[serde(default)]
    pub penalty: Penalty,
    /// CapCom's advice when the timer runs out
    #[serde(default)]
    pub advice: Option<Text>,
    /// when done, it fixes the ship (e.g. the CO₂ filter lowers CO₂)
    #[serde(default)]
    pub fixes: Penalty,
    #[serde(default)]
    pub draft: bool,
    /// shown when the task is done: why this matters for the ship or the planet
    #[serde(default)]
    pub why: Option<LevelText>,
    /// the law or rule this task shows (collected in the crew's notebook)
    #[serde(default)]
    pub law: Option<Text>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Day {
    pub day: u8,
    pub title: Text,
    /// story and routine tasks, in order — all must be done to end the day
    pub scripted: Vec<String>,
    /// problems the director may send this day
    pub problems: Vec<String>,
    /// small calm tasks when there is nothing to do
    #[serde(default)]
    pub fillers: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    /// a measurable quantity (v, a, F, m, p, Eк) — has a symbol and units
    Quantity,
    /// a phenomenon (inertia, gravity)
    Phenomenon,
    /// a concept (trajectory is a line, not a quantity)
    Concept,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsCard {
    pub id: String,
    pub kind: CardKind,
    pub title: Text,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    /// plain Unicode, e.g. "F = m·a"
    #[serde(default)]
    pub formula: Option<String>,
    pub junior: Text,
    #[serde(default)]
    pub advanced: Option<Text>,
    /// show it after this episode (task id); `None` = after any episode, in order
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub draft: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub text: Text,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub set_flag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DialogNode {
    pub id: String,
    pub npc: String,
    pub text: LevelText,
    /// next node when there are no choices (`None` = the end)
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "on", content = "id")]
pub enum Trigger {
    /// at the start of a flight day
    DayStart(u8),
    /// right after a task is done
    TaskDone(String),
    /// right after a task failed (its timer ran out)
    TaskFailed(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dialog {
    pub id: String,
    pub trigger: Trigger,
    /// shown only if this flag is set (by a choice in an earlier dialog)
    #[serde(default)]
    pub require_flag: Option<String>,
    pub nodes: Vec<DialogNode>,
    #[serde(default)]
    pub draft: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Planet {
    pub id: String,
    pub name: Text,
    pub icon: String,
    /// can an expedition go there now
    #[serde(default)]
    pub playable: bool,
    pub gravity: Text,
    pub temperature: Text,
    pub atmosphere: Text,
    pub surface: LevelText,
    pub history: LevelText,
    #[serde(default)]
    pub draft: bool,
}

/// Everything from `assets/content`. `ui`, `events`, `shop` belong to part B and are kept raw.
#[derive(Clone, Debug, Default)]
pub struct Content {
    pub ui: HashMap<String, Text>,
    pub npcs: Vec<Npc>,
    pub ship: Option<ShipLayout>,
    pub tasks: Vec<ShipTask>,
    pub days: Vec<Day>,
    pub cards: Vec<PhysicsCard>,
    pub dialogs: Vec<Dialog>,
    pub planets: Vec<Planet>,
    pub events: Option<serde_json::Value>,
    pub shop: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentError {
    pub file: String,
    pub key: String,
    pub problem: String,
}

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}: {}", self.file, self.key, self.problem)
    }
}

impl Content {
    pub fn task(&self, id: &str) -> Option<&ShipTask> {
        self.tasks.iter().find(|t| t.id == id)
    }
    pub fn npc(&self, id: &str) -> Option<&Npc> {
        self.npcs.iter().find(|n| n.id == id)
    }
    pub fn room(&self, id: &str) -> Option<&Room> {
        self.ship.as_ref()?.rooms.iter().find(|r| r.id == id)
    }
    pub fn day(&self, d: u8) -> Option<&Day> {
        self.days.iter().find(|x| x.day == d)
    }
    pub fn dialog(&self, id: &str) -> Option<&Dialog> {
        self.dialogs.iter().find(|x| x.id == id)
    }
    /// A UI string by key (part B's `ui.json`), falling back to the key itself.
    pub fn ui<'a>(&'a self, key: &'a str, l: Lang) -> &'a str {
        self.ui.get(key).map(|t| t.get(l)).unwrap_or(key)
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &Path, errs: &mut Vec<ContentError>) -> Option<T> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            errs.push(ContentError { file: name, key: "-".into(), problem: format!("can't read: {e}") });
            return None;
        }
    };
    match serde_json::from_str(&s) {
        Ok(v) => Some(v),
        Err(e) => {
            errs.push(ContentError { file: name, key: format!("line {}", e.line()), problem: e.to_string() });
            None
        }
    }
}

/// Load `dir` (usually `assets/content`). Optional files may be missing; broken ones are errors.
pub fn load(dir: &Path) -> Result<Content, Vec<ContentError>> {
    let mut errs = vec![];
    let mut c = Content::default();
    let opt = |name: &str| {
        let p = dir.join(name);
        p.exists().then_some(p)
    };
    if let Some(p) = opt("ui.json") {
        c.ui = read(&p, &mut errs).unwrap_or_default();
    }
    if let Some(p) = opt("npcs.json") {
        c.npcs = read(&p, &mut errs).unwrap_or_default();
    }
    if let Some(p) = opt("ship_rooms.json") {
        c.ship = read(&p, &mut errs);
    }
    if let Some(p) = opt("physics_cards.json") {
        c.cards = read(&p, &mut errs).unwrap_or_default();
    }
    if let Some(p) = opt("events.json") {
        c.events = read(&p, &mut errs);
    }
    if let Some(p) = opt("shop.json") {
        c.shop = read(&p, &mut errs);
    }
    // folders: every *.json inside, sorted by name
    let files = |sub: &str| -> Vec<std::path::PathBuf> {
        let mut v: Vec<_> = std::fs::read_dir(dir.join(sub)).map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "json")).collect()).unwrap_or_default();
        v.sort();
        v
    };
    #[derive(Deserialize)]
    struct Missions {
        tasks: Vec<ShipTask>,
        #[serde(default)]
        days: Vec<Day>,
    }
    for p in files("missions") {
        if let Some(m) = read::<Missions>(&p, &mut errs) {
            c.tasks.extend(m.tasks);
            c.days.extend(m.days);
        }
    }
    for p in files("dialogs") {
        if let Some(d) = read::<Vec<Dialog>>(&p, &mut errs) {
            c.dialogs.extend(d);
        }
    }
    for p in files("planets") {
        if let Some(pl) = read::<Planet>(&p, &mut errs) {
            c.planets.push(pl);
        }
    }
    errs.extend(validate(&c));
    if errs.is_empty() {
        Ok(c)
    } else {
        Err(errs)
    }
}

fn check_text(errs: &mut Vec<ContentError>, file: &str, key: &str, t: &Text) {
    if t.ru.trim().is_empty() || t.en.trim().is_empty() {
        errs.push(ContentError { file: file.into(), key: key.into(), problem: "needs both ru and en".into() });
    }
}

/// Every text has ru and en; every id that points somewhere points at something that exists.
pub fn validate(c: &Content) -> Vec<ContentError> {
    let mut e = vec![];
    for (k, t) in &c.ui {
        check_text(&mut e, "ui.json", k, t);
    }
    let npcs: HashSet<&str> = c.npcs.iter().map(|n| n.id.as_str()).collect();
    for n in &c.npcs {
        check_text(&mut e, "npcs.json", &format!("{}.name", n.id), &n.name);
        check_text(&mut e, "npcs.json", &format!("{}.role", n.id), &n.role);
    }
    let mut consoles: HashSet<(String, String)> = HashSet::new();
    if let Some(s) = &c.ship {
        check_text(&mut e, "ship_rooms.json", "name", &s.name);
        for r in &s.rooms {
            check_text(&mut e, "ship_rooms.json", &format!("{}.name", r.id), &r.name);
            for k in &r.consoles {
                check_text(&mut e, "ship_rooms.json", &format!("{}.{}.name", r.id, k.id), &k.name);
                if k.x < r.x || k.x >= r.x + r.w || k.y < r.y || k.y >= r.y + r.h {
                    e.push(ContentError { file: "ship_rooms.json".into(), key: format!("{}.{}", r.id, k.id), problem: "console is outside its room".into() });
                }
                consoles.insert((r.id.clone(), k.id.clone()));
            }
        }
    }
    let tasks: HashSet<&str> = c.tasks.iter().map(|t| t.id.as_str()).collect();
    for t in &c.tasks {
        let f = "missions/*.json";
        check_text(&mut e, f, &format!("{}.title", t.id), &t.title);
        check_text(&mut e, f, &format!("{}.brief", t.id), &t.brief.junior);
        if let Some(a) = &t.brief.advanced {
            check_text(&mut e, f, &format!("{}.brief.advanced", t.id), a);
        }
        if c.ship.is_some() && !consoles.contains(&(t.room.clone(), t.console.clone())) {
            e.push(ContentError { file: f.into(), key: t.id.clone(), problem: format!("no console {}.{}", t.room, t.console) });
        }
        if t.kind == TaskKind::Problem && t.timer.is_none() {
            e.push(ContentError { file: f.into(), key: t.id.clone(), problem: "a problem needs a timer".into() });
        }
        if t.kind != TaskKind::Asteroids && t.minigame.is_none() {
            e.push(ContentError { file: f.into(), key: t.id.clone(), problem: "needs a minigame".into() });
        }
    }
    for d in &c.days {
        check_text(&mut e, "missions/*.json", &format!("day{}.title", d.day), &d.title);
        for id in d.scripted.iter().chain(&d.problems).chain(&d.fillers) {
            if !tasks.contains(id.as_str()) {
                e.push(ContentError { file: "missions/*.json".into(), key: format!("day{}", d.day), problem: format!("unknown task {id}") });
            }
        }
    }
    let mut seen = HashSet::new();
    for card in &c.cards {
        check_text(&mut e, "physics_cards.json", &format!("{}.title", card.id), &card.title);
        check_text(&mut e, "physics_cards.json", &format!("{}.junior", card.id), &card.junior);
        if card.kind == CardKind::Quantity && card.symbol.is_none() {
            e.push(ContentError { file: "physics_cards.json".into(), key: card.id.clone(), problem: "a quantity needs a symbol".into() });
        }
        if card.kind != CardKind::Quantity && card.unit.is_some() {
            e.push(ContentError { file: "physics_cards.json".into(), key: card.id.clone(), problem: "only quantities have units".into() });
        }
        if let Some(a) = &card.after {
            if !tasks.contains(a.as_str()) {
                e.push(ContentError { file: "physics_cards.json".into(), key: card.id.clone(), problem: format!("unknown episode {a}") });
            }
        }
        if !seen.insert(card.id.as_str()) {
            e.push(ContentError { file: "physics_cards.json".into(), key: card.id.clone(), problem: "duplicate id".into() });
        }
    }
    for d in &c.dialogs {
        let f = "dialogs/*.json";
        let ids: HashSet<&str> = d.nodes.iter().map(|n| n.id.as_str()).collect();
        if d.nodes.is_empty() {
            e.push(ContentError { file: f.into(), key: d.id.clone(), problem: "no nodes".into() });
        }
        match &d.trigger {
            Trigger::TaskDone(t) | Trigger::TaskFailed(t) if !tasks.contains(t.as_str()) => e.push(ContentError { file: f.into(), key: d.id.clone(), problem: format!("unknown task {t}") }),
            _ => {}
        }
        for n in &d.nodes {
            check_text(&mut e, f, &format!("{}.{}", d.id, n.id), &n.text.junior);
            if !npcs.contains(n.npc.as_str()) {
                e.push(ContentError { file: f.into(), key: format!("{}.{}", d.id, n.id), problem: format!("unknown npc {}", n.npc) });
            }
            for nx in n.next.iter().chain(n.choices.iter().filter_map(|c| c.next.as_ref())) {
                if !ids.contains(nx.as_str()) {
                    e.push(ContentError { file: f.into(), key: format!("{}.{}", d.id, n.id), problem: format!("unknown node {nx}") });
                }
            }
            for ch in &n.choices {
                check_text(&mut e, f, &format!("{}.{}.choice", d.id, n.id), &ch.text);
            }
        }
    }
    for p in &c.planets {
        check_text(&mut e, "planets/*.json", &format!("{}.name", p.id), &p.name);
        check_text(&mut e, "planets/*.json", &format!("{}.surface", p.id), &p.surface.junior);
    }
    e
}

/// Keys of texts that still have no Kazakh translation (for the content report).
pub fn missing_kk(c: &Content) -> Vec<String> {
    let mut out = vec![];
    let mut t = |key: String, x: &Text| {
        if x.kk.as_deref().is_none_or(|s| s.trim().is_empty()) {
            out.push(key);
        }
    };
    for (k, x) in &c.ui {
        t(format!("ui.{k}"), x);
    }
    for n in &c.npcs {
        t(format!("npc.{}.name", n.id), &n.name);
    }
    for x in &c.tasks {
        t(format!("task.{}.title", x.id), &x.title);
        t(format!("task.{}.brief", x.id), &x.brief.junior);
    }
    for x in &c.cards {
        t(format!("card.{}", x.id), &x.junior);
    }
    for d in &c.dialogs {
        for n in &d.nodes {
            t(format!("dialog.{}.{}", d.id, n.id), &n.text.junior);
        }
    }
    out.sort();
    out
}

/// `assets/content` next to the workspace (for tests and `cargo run`).
pub fn default_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/content")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test -p ja-core content` — run it after every content edit.
    #[test]
    fn content_loads_and_every_link_is_valid() {
        let c = match load(&default_dir()) {
            Ok(c) => c,
            Err(errs) => panic!("content errors:\n{}", errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")),
        };
        assert!(c.ship.as_ref().is_some_and(|s| s.rooms.len() == 9), "nine ship rooms");
        assert_eq!(c.days.len(), 2, "two flight days");
        assert!(c.cards.len() >= 6);
        let kinds: HashSet<MiniKind> = c.tasks.iter().filter_map(|t| t.minigame).collect();
        assert_eq!(kinds.len(), crate::minigame::ALL_KINDS.len(), "every mini-game type is used");
        // every task has its own game (no repeats) and explains why it matters
        let with_game: Vec<_> = c.tasks.iter().filter_map(|t| t.minigame).collect();
        assert_eq!(with_game.len(), kinds.len(), "one game per task, no repeats");
        assert!(c.tasks.iter().all(|t| t.why.is_some() && t.law.is_some()), "every task says why it matters");
        let missing = missing_kk(&c);
        println!("content report: {} texts without Kazakh (shown in Russian)", missing.len());
    }

    #[test]
    fn broken_links_are_reported_with_file_and_key() {
        let mut c = load(&default_dir()).unwrap();
        c.days[0].scripted.push("no_such_task".into());
        c.cards[0].junior.en.clear();
        let errs = validate(&c);
        assert!(errs.iter().any(|e| e.key == "day1" && e.problem.contains("no_such_task")));
        assert!(errs.iter().any(|e| e.file == "physics_cards.json" && e.problem.contains("ru and en")));
    }

    #[test]
    fn kazakh_falls_back_to_russian() {
        let t = Text::new("Привет", "Hi");
        assert_eq!(t.get(Lang::Kk), "Привет");
        let t = Text { kk: Some("Сәлем".into()), ..t };
        assert_eq!(t.get(Lang::Kk), "Сәлем");
    }
}
