//! One continuous story: the flight of the ship "Aurora" (part A) → landing → the Moon (part B).
//!
//! The ship part is run by a small **director** (PLAN_V2, appendix 3):
//! - the crew always has a goal: story tasks of the day one by one, a calm "filler" task after 20 s idle;
//! - waves: calm → a problem with a timer → a breather (photo, call home, a crew line);
//! - tension grows by day; Junior gets at most one urgent problem and 1.5× longer timers;
//! - a failed problem has consequences, but the flight goes on. Only a lost hull or air is game over,
//!   and "start the day again" goes back to the day's checkpoint.

use crate::content::{Content, TaskKind, Trigger};
use crate::dialog::{self, DialogState};
use crate::expedition::{ExpeditionConfig, Level, PlayMode, Role};
use crate::minigame::{Mini, MiniInput, Outcome};
use crate::rng::Rng;
use crate::ship::{AsteroidField, ShipFail, ShipStats};
use serde::{Deserialize, Serialize};

/// seconds without a goal before the director gives a small task
pub const IDLE_GIVE: f32 = 20.0;
/// how close (cells) to a console to work at it
pub const REACH: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ship { day: u8 },
    /// the lander leaves the ship — part B's landing (`CampaignCommand::Landed` when it touches down)
    Landing,
    /// survival on the Moon — part B
    Moon { day: u8 },
    GameOver(ShipFail),
    Done,
}

/// Team budget (building, repairs) + each player's own coins (cosmetics).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wallet {
    pub team: u32,
    pub personal: [u32; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveTask {
    pub uid: u32,
    /// task id in the content
    pub id: String,
    pub kind: TaskKind,
    /// where to do it (a leak repair goes to the room that was hit)
    pub room: String,
    pub console: String,
    /// seconds left / total for problems
    pub t_left: Option<f32>,
    pub t_total: Option<f32>,
    pub scripted: bool,
    /// the mini-game while someone works on it
    pub mini: Option<Mini>,
    pub worker: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Breather {
    PhotoEarth,
    CallHome,
    CrewLine,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Crew {
    pub pos: (f32, f32),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub solved: u32,
    pub failed: u32,
    pub hits: u32,
    /// longest stretch with no task at all (director test)
    pub max_idle: f32,
    /// most urgent problems at once (director test)
    pub max_urgent: u32,
}

/// Everything the app draws (and the network host sends).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CampaignState {
    pub cfg: ExpeditionConfig,
    pub phase: Phase,
    pub ship: ShipStats,
    pub tasks: Vec<ActiveTask>,
    pub done: Vec<String>,
    pub wallet: Wallet,
    pub crew: Vec<Crew>,
    /// the asteroid map while a wave is on
    pub field: Option<AsteroidField>,
    pub waves: u32,
    pub dialog: Option<DialogState>,
    pub seen_dialogs: Vec<String>,
    pub flags: Vec<String>,
    /// the physics card on screen
    pub card: Option<String>,
    pub cards_seen: Vec<String>,
    pub breather: Option<Breather>,
    pub scripted_next: usize,
    pub elapsed: f32,
    pub paused: bool,
    pub score: Score,
    idle_t: f32,
    calm_t: f32,
    last_problem: Option<String>,
    next_uid: u32,
    breathers: u32,
    /// the "why it matters" window after a finished task
    #[serde(default)]
    pub lesson: Option<Lesson>,
    /// tasks whose law the crew has learned (ids, in order)
    #[serde(default)]
    pub laws: Vec<String>,
    /// the ship's pet, once adopted
    #[serde(default)]
    pub pet: Option<Pet>,
    /// emergencies that already happened this flight (each comes once, solved or not)
    #[serde(default)]
    pub problems_seen: Vec<String>,
}

/// A finished task: what was earned and why the job matters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub coins: u32,
    pub bonus: u32,
}

/// The ship's pet: `kind` indexes `minigame::PETS`; a happy pet (mood ≥ 50) gives the crew a bonus.
/// It follows its owner, or sits where it was told to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pet {
    pub kind: u8,
    pub mood: f32,
    pub owner: u8,
    #[serde(default = "yes")]
    pub follow: bool,
    /// where it sits (cells) when not following
    #[serde(default)]
    pub at: (f32, f32),
}
fn yes() -> bool {
    true
}
pub const PET_BONUS: u32 = 2;
/// how close you must stand to adopt or command the pet (cells)
pub const PET_REACH: f32 = 1.4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CampaignEvent {
    DayStarted(u8),
    TaskOffered { uid: u32, id: String },
    ProblemStarted { uid: u32, id: String },
    TaskDone { uid: u32, id: String, coins: u32 },
    /// the timer ran out: penalty applied, CapCom's advice in the task's `advice`
    TaskFailed { uid: u32, id: String },
    WaveStarted(u32),
    AsteroidHit { room: Option<String> },
    WaveOver { hits: u32 },
    Breather(Breather),
    Card(String),
    DialogStarted(String),
    DialogEnded,
    DayEnded(u8),
    Landing,
    GameOver(ShipFail),
    DayRestarted(u8),
    PetAdopted(u8),
    Mistake,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CampaignCommand {
    /// start working on a task (stand at its console)
    OpenTask { uid: u32 },
    Mini { uid: u32, input: MiniInput },
    /// walk away from a task (progress is kept)
    CloseTask { uid: u32 },
    /// touch the asteroid map (0..1 coordinates)
    Steer { x: f32, y: f32 },
    DialogNext,
    DialogChoose(usize),
    DialogSkip,
    CloseCard,
    CloseBreather,
    /// close the "why it matters" window
    CloseLesson,
    /// walk up to one of the pets in the living quarters and take it (once)
    AdoptPet { kind: u8 },
    /// the pet sits here / follows me again
    PetToggle,
    /// fill the pet's bowl now (offers the "feed the pet" job at the pet corner)
    FeedPet,
    /// talk to a crew member (index into `Content::npcs`)
    Talk { npc: u8 },
    /// after game over: back to the start of the day
    RestartDay,
    /// creative mode: skip to the next day
    NextDay,
    /// part B: the lander touched down
    Landed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignRefusal {
    NotNow,
    TooFar,
    Busy,
    UnknownTask,
    NotCreative,
}

/// The state at the start of a day (for "start the day again" and saving).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint(pub CampaignState);

pub struct Campaign {
    s: CampaignState,
    content: Content,
    rng: Rng,
    checkpoint: Checkpoint,
    /// network guest: `Some(seat)` — the host runs the rules, this copy only shows them
    mirror: Option<u8>,
    outbox: Vec<(u8, CampaignCommand)>,
}

impl Campaign {
    pub fn new(cfg: ExpeditionConfig, content: &Content) -> Self {
        let start = content.ship.as_ref().map(|s| s.start).unwrap_or((3.5, 7.5));
        let crew = (0..cfg.players.clamp(1, 4)).map(|i| Crew { pos: (start.0 + i as f32 * 0.8, start.1) }).collect();
        let s = CampaignState {
            cfg,
            phase: Phase::Ship { day: 1 },
            ship: ShipStats::default(),
            tasks: vec![],
            done: vec![],
            wallet: Wallet::default(),
            crew,
            field: None,
            waves: 0,
            dialog: None,
            seen_dialogs: vec![],
            flags: vec![],
            card: None,
            cards_seen: vec![],
            breather: None,
            scripted_next: 0,
            elapsed: 0.0,
            paused: false,
            score: Score::default(),
            idle_t: 0.0,
            calm_t: 0.0,
            last_problem: None,
            next_uid: 1,
            breathers: 0,
            lesson: None,
            laws: vec![],
            pet: None,
            problems_seen: vec![],
        };
        let mut c = Campaign { checkpoint: Checkpoint(s.clone()), s, content: content.clone(), rng: Rng::new(cfg.seed), mirror: None, outbox: vec![] };
        let _ = c.start_day(1);
        c
    }

    pub fn state(&self) -> &CampaignState {
        &self.s
    }
    pub fn content(&self) -> &Content {
        &self.content
    }
    /// Developer F5: swap in freshly loaded content (the running flight keeps going).
    pub fn reload_content(&mut self, c: &Content) {
        self.content = c.clone();
    }
    pub fn checkpoint(&self) -> Checkpoint {
        self.checkpoint.clone()
    }
    pub fn set_paused(&mut self, on: bool) {
        self.s.paused = on;
    }
    pub fn set_crew_pos(&mut self, player: u8, x: f32, y: f32) {
        if let Some(c) = self.s.crew.get_mut(player as usize) {
            c.pos = (x, y);
        }
    }
    /// Network guest: show the host's campaign.
    pub fn mirror(&mut self, s: CampaignState) {
        self.mirror_as(s, u8::MAX);
    }
    /// Network guest playing crew member `seat`: show the host's campaign but keep our own
    /// position (it walks here and is sent to the host). From now on `tick` does nothing and
    /// `apply` queues the command for the host (see `take_outbox`).
    pub fn mirror_as(&mut self, mut s: CampaignState, seat: u8) {
        if let (Some(mine), Some(theirs)) = (self.s.crew.get(seat as usize), s.crew.get_mut(seat as usize)) {
            if self.mirror == Some(seat) {
                *theirs = mine.clone();
            }
        }
        self.mirror = Some(seat);
        self.s = s;
    }
    pub fn is_mirror(&self) -> bool {
        self.mirror.is_some()
    }
    /// Commands a guest made since the last call, for the host.
    pub fn take_outbox(&mut self) -> Vec<(u8, CampaignCommand)> {
        std::mem::take(&mut self.outbox)
    }

    fn junior(&self) -> bool {
        self.s.cfg.level == Level::Junior
    }
    fn creative(&self) -> bool {
        self.s.cfg.mode == PlayMode::Creative
    }
    fn day(&self) -> u8 {
        match self.s.phase {
            Phase::Ship { day } => day,
            _ => 0,
        }
    }
    fn urgent(&self) -> usize {
        self.s.tasks.iter().filter(|t| t.kind == TaskKind::Problem).count()
    }
    /// Junior: one urgent problem at a time; Advanced: two from day 3.
    pub fn urgent_limit(&self) -> usize {
        if self.junior() || self.day() < 3 { 1 } else { 2 }
    }
    fn calm_gap(&self) -> f32 {
        let d = self.day() as f32;
        (32.0 - d * 4.0).max(16.0) * if self.junior() { 1.3 } else { 1.0 }
    }

    fn offer(&mut self, id: &str, scripted: bool, room: Option<String>) -> Option<CampaignEvent> {
        let t = self.content.task(id)?.clone();
        let uid = self.s.next_uid;
        self.s.next_uid += 1;
        let (room, console) = match room {
            Some(r) => {
                let console = self.content.room(&r).and_then(|x| x.consoles.first()).map(|k| k.id.clone()).unwrap_or_default();
                (r, console)
            }
            None => (t.room.clone(), t.console.clone()),
        };
        let mut timer = t.timer;
        if let Some(x) = timer.as_mut() {
            *x *= if self.junior() { 1.5 } else { 1.0 } * if self.s.cfg.has_role(Role::Engineer) { 1.25 } else { 1.0 };
        }
        self.s.tasks.push(ActiveTask { uid, id: t.id.clone(), kind: t.kind, room, console, t_left: timer, t_total: timer, scripted, mini: None, worker: None });
        self.s.idle_t = 0.0;
        Some(if t.kind == TaskKind::Problem { CampaignEvent::ProblemStarted { uid, id: t.id } } else { CampaignEvent::TaskOffered { uid, id: t.id } })
    }

    fn start_day(&mut self, day: u8) -> Vec<CampaignEvent> {
        self.s.phase = Phase::Ship { day };
        self.s.scripted_next = 0;
        self.s.calm_t = 0.0;
        self.s.tasks.retain(|t| !t.scripted);
        let mut ev = vec![CampaignEvent::DayStarted(day)];
        ev.extend(self.trigger(&Trigger::DayStart(day)));
        self.checkpoint = Checkpoint(self.s.clone());
        ev
    }

    fn trigger(&mut self, t: &Trigger) -> Vec<CampaignEvent> {
        if self.s.dialog.is_some() {
            return vec![];
        }
        match dialog::triggered(&self.content, t, &self.s.seen_dialogs, &self.s.flags) {
            Some(d) => {
                self.s.seen_dialogs.push(d.id.clone());
                self.s.dialog = Some(dialog::start(d));
                vec![CampaignEvent::DialogStarted(d.id.clone())]
            }
            None => vec![],
        }
    }

    fn episode_over(&mut self, episode: &str) -> Vec<CampaignEvent> {
        match crate::physics::next_card(&self.content, &self.s.cards_seen, episode) {
            Some(id) => {
                let id = id.to_string();
                self.s.cards_seen.push(id.clone());
                self.s.card = Some(id.clone());
                vec![CampaignEvent::Card(id)]
            }
            None => vec![],
        }
    }

    /// Modal things (dialog, card, breather) stop the clock.
    pub fn frozen(&self) -> bool {
        self.s.paused || self.s.dialog.is_some() || self.s.card.is_some() || self.s.breather.is_some() || self.s.lesson.is_some()
    }

    pub fn tick(&mut self, dt: f32) -> Vec<CampaignEvent> {
        let mut ev = vec![];
        if self.mirror.is_some() {
            return ev;
        }
        let Phase::Ship { day } = self.s.phase else { return ev };
        if self.frozen() || dt <= 0.0 {
            return ev;
        }
        self.s.elapsed += dt;
        let creative = self.creative();
        // ship systems drift (one game minute per real second / 60)
        let drift = if self.s.cfg.has_role(Role::LifeSupport) { 0.7 } else { 1.0 };
        self.s.ship.tick(dt / 60.0, drift, creative);
        // the pet gets hungry (about 6 minutes from full to sad)
        if let Some(p) = self.s.pet.as_mut() {
            p.mood = (p.mood - dt * 100.0 / 360.0).max(0.0);
        }
        // mini-game clocks (moving targets, the burn, the rhythm…)
        let mut finished = vec![];
        for t in self.s.tasks.iter_mut() {
            if let (Some(w), Some(m)) = (t.worker, t.mini.as_mut()) {
                match m.tick(dt) {
                    Outcome::Done => finished.push((w, t.uid)),
                    Outcome::Mistake => ev.push(CampaignEvent::Mistake),
                    Outcome::Fine => {}
                }
            }
        }
        for (w, uid) in finished {
            ev.extend(self.complete(w, uid, 0));
        }
        // problem timers
        let mut failed = vec![];
        for t in self.s.tasks.iter_mut() {
            if let Some(x) = t.t_left.as_mut() {
                *x -= dt;
                if *x <= 0.0 {
                    failed.push(t.uid);
                }
            }
        }
        for uid in failed {
            ev.extend(self.fail(uid));
        }
        // the asteroid map
        if let Some(mut f) = self.s.field.take() {
            let junior = self.junior();
            let hits = f.tick(dt, junior, &mut self.rng);
            for _ in 0..hits {
                ev.extend(self.asteroid_hit(&f));
            }
            if f.over() {
                let total = f.hits;
                self.s.waves += 1;
                ev.push(CampaignEvent::WaveOver { hits: total });
                if let Some(t) = self.s.tasks.iter().find(|t| t.kind == TaskKind::Asteroids).map(|t| t.uid) {
                    ev.extend(self.complete(uid_worker(&self.s, t), t, total));
                }
            } else {
                self.s.field = Some(f);
            }
        }
        // director: story tasks one by one
        let has_scripted = self.s.tasks.iter().any(|t| t.scripted);
        let scripted: Vec<String> = self.content.day(day).map(|d| d.scripted.clone()).unwrap_or_default();
        if !has_scripted && self.s.scripted_next < scripted.len() {
            let id = scripted[self.s.scripted_next].clone();
            ev.extend(self.offer(&id, true, None));
        }
        // director: problems after a calm stretch
        self.s.calm_t += dt;
        if !creative && self.s.field.is_none() && self.s.calm_t > self.calm_gap() && self.urgent() < self.urgent_limit() {
            let pool: Vec<String> = self.content.day(day).map(|d| d.problems.clone()).unwrap_or_default();
            let options: Vec<&String> = pool.iter().filter(|p| Some(*p) != self.s.last_problem.as_ref() && !self.s.tasks.iter().any(|t| &&t.id == p) && !self.s.done.contains(p) && !self.s.problems_seen.contains(p)).collect();
            if !options.is_empty() {
                let id = options[self.rng.below(options.len())].clone();
                self.s.last_problem = Some(id.clone());
                self.s.problems_seen.push(id.clone());
                self.s.calm_t = 0.0;
                ev.extend(self.offer(&id, false, None));
            }
        }
        // director: never more than IDLE_GIVE seconds without a goal
        if self.s.tasks.is_empty() {
            self.s.idle_t += dt;
            self.s.score.max_idle = self.s.score.max_idle.max(self.s.idle_t);
            if self.s.idle_t >= IDLE_GIVE {
                let all: Vec<String> = self.content.day(day).map(|d| d.fillers.clone()).unwrap_or_default();
                // fresh jobs first; the pet's jobs only when they make sense
                let ok = |id: &String, s: &CampaignState| match id.as_str() {
                    "pet_feed" => s.pet.as_ref().is_some_and(|p| p.mood < 70.0),
                    _ => true,
                };
                let fresh: Vec<String> = all.iter().filter(|f| ok(f, &self.s) && !self.s.done.contains(f)).cloned().collect();
                let fillers: Vec<String> = if fresh.is_empty() { all.iter().filter(|f| ok(f, &self.s)).cloned().collect() } else { fresh };
                if !fillers.is_empty() {
                    let id = fillers[self.rng.below(fillers.len())].clone();
                    ev.extend(self.offer(&id, false, None));
                }
            }
        } else {
            self.s.idle_t = 0.0;
        }
        self.s.score.max_urgent = self.s.score.max_urgent.max(self.urgent() as u32);
        // the day ends when its story is told and nothing urgent is left
        let story_done = self.s.scripted_next >= scripted.len() && !self.s.tasks.iter().any(|t| t.scripted);
        if story_done && self.urgent() == 0 && self.s.field.is_none() && !self.frozen() {
            ev.extend(self.end_day(day));
        }
        // the ship can be lost (never in creative mode)
        if !creative {
            if let Some(f) = self.s.ship.lost() {
                self.s.phase = Phase::GameOver(f);
                self.s.field = None;
                ev.push(CampaignEvent::GameOver(f));
            }
        }
        ev
    }

    fn end_day(&mut self, day: u8) -> Vec<CampaignEvent> {
        let mut ev = vec![CampaignEvent::DayEnded(day)];
        self.s.tasks.retain(|t| t.kind != TaskKind::Routine || t.scripted);
        if self.content.day(day + 1).is_some() {
            ev.extend(self.start_day(day + 1));
        } else {
            self.s.tasks.clear();
            self.s.phase = Phase::Landing;
            ev.push(CampaignEvent::Landing);
        }
        ev
    }

    fn asteroid_hit(&mut self, f: &AsteroidField) -> Vec<CampaignEvent> {
        let _ = f;
        self.s.score.hits += 1;
        if self.creative() {
            return vec![CampaignEvent::AsteroidHit { room: None }];
        }
        let pilot = if self.s.cfg.has_role(Role::Pilot) { 0.7 } else { 1.0 };
        let dmg = if self.junior() { 3.0 } else { 4.0 } * pilot;
        self.s.ship.hull = (self.s.ship.hull - dmg).max(0.0);
        self.s.ship.seal = (self.s.ship.seal - 2.0).max(0.0);
        // every third hit makes a hole in a random room (a repair task)
        let mut room = None;
        if self.s.score.hits % 3 == 0 && self.urgent() < self.urgent_limit() {
            let rooms: Vec<String> = self.content.ship.as_ref().map(|s| s.rooms.iter().map(|r| r.id.clone()).filter(|r| !self.s.ship.leaks.contains(r)).collect()).unwrap_or_default();
            if !rooms.is_empty() {
                let r = rooms[self.rng.below(rooms.len())].clone();
                self.s.ship.leaks.push(r.clone());
                room = Some(r);
            }
        }
        let mut ev = vec![CampaignEvent::AsteroidHit { room: room.clone() }];
        if let Some(r) = room {
            if self.content.task("leak_patch").is_some() {
                ev.extend(self.offer("leak_patch", false, Some(r)));
            }
        }
        ev
    }

    fn fail(&mut self, uid: u32) -> Vec<CampaignEvent> {
        let Some(i) = self.s.tasks.iter().position(|t| t.uid == uid) else { return vec![] };
        let t = self.s.tasks.remove(i);
        let def = self.content.task(&t.id).cloned();
        if let Some(d) = &def {
            self.s.ship.change(&d.penalty, -1.0);
        }
        if t.id == "leak_patch" {
            // Mission Control talks the crew through a temporary seal: the hole closes, the seal suffers
            self.s.ship.leaks.retain(|r| r != &t.room);
            self.s.ship.seal = (self.s.ship.seal - 10.0).max(0.0);
        }
        self.s.score.failed += 1;
        self.s.calm_t = 0.0;
        let mut ev = vec![CampaignEvent::TaskFailed { uid, id: t.id.clone() }];
        ev.extend(self.trigger(&Trigger::TaskFailed(t.id.clone())));
        ev.extend(self.episode_over(&t.id));
        ev
    }

    fn complete(&mut self, worker: u8, uid: u32, hits: u32) -> Vec<CampaignEvent> {
        let Some(i) = self.s.tasks.iter().position(|t| t.uid == uid) else { return vec![] };
        let t = self.s.tasks.remove(i);
        let def = self.content.task(&t.id).cloned();
        let mut coins = def.as_ref().map(|d| d.reward).unwrap_or(0);
        if t.kind == TaskKind::Asteroids {
            coins = coins.saturating_sub(hits * 2);
        }
        if t.room == "lab" && self.s.cfg.has_role(Role::Researcher) {
            coins += coins / 2;
        }
        // the pet: adopted here, fed here, and a happy pet cheers the crew up
        if t.mini.as_ref().is_some_and(|m| m.kind == crate::minigame::MiniKind::PetCare) {
            if let Some(p) = self.s.pet.as_mut() {
                p.mood = 100.0;
            }
        }
        let bonus = if self.s.pet.as_ref().is_some_and(|p| p.mood >= 50.0) && t.kind != TaskKind::Asteroids { PET_BONUS } else { 0 };
        coins += bonus;
        if def.as_ref().is_some_and(|d| d.why.is_some()) {
            self.s.lesson = Some(Lesson { id: t.id.clone(), coins, bonus });
        }
        if def.as_ref().is_some_and(|d| d.law.is_some()) && !self.s.laws.contains(&t.id) {
            self.s.laws.push(t.id.clone());
        }
        if let Some(d) = &def {
            self.s.ship.change(&d.fixes, 1.0);
        }
        if t.id == "leak_patch" {
            self.s.ship.leaks.retain(|r| r != &t.room);
        }
        self.s.wallet.team += coins;
        if let Some(p) = self.s.wallet.personal.get_mut(worker as usize) {
            *p += coins.div_ceil(4);
        }
        if t.scripted {
            self.s.scripted_next += 1;
        }
        self.s.done.push(t.id.clone());
        let mut ev = vec![CampaignEvent::TaskDone { uid, id: t.id.clone(), coins }];
        ev.extend(self.trigger(&Trigger::TaskDone(t.id.clone())));
        // the physics card that belongs to this very job (if there is one)
        ev.extend(self.episode_over(&t.id));
        if matches!(t.kind, TaskKind::Problem | TaskKind::Asteroids) {
            self.s.score.solved += 1;
            self.s.calm_t = 0.0;
            // a breather after every solved problem
            let b = [Breather::PhotoEarth, Breather::CallHome, Breather::CrewLine][self.s.breathers as usize % 3];
            self.s.breathers += 1;
            if self.s.card.is_none() {
                self.s.breather = Some(b);
                ev.push(CampaignEvent::Breather(b));
            }
        }
        ev
    }

    pub fn apply(&mut self, player: u8, cmd: CampaignCommand) -> Result<Vec<CampaignEvent>, CampaignRefusal> {
        if self.mirror.is_some() {
            self.outbox.push((player, cmd));
            return Ok(vec![]);
        }
        let p = (player as usize).min(self.s.crew.len().saturating_sub(1));
        match cmd {
            CampaignCommand::DialogNext | CampaignCommand::DialogChoose(_) => {
                let d = self.s.dialog.clone().ok_or(CampaignRefusal::NotNow)?;
                let choice = if let CampaignCommand::DialogChoose(i) = cmd { Some(i) } else { None };
                let (next, flag) = dialog::advance(&self.content, &d, choice);
                if let Some(f) = flag {
                    if !self.s.flags.contains(&f) {
                        self.s.flags.push(f);
                    }
                }
                self.s.dialog = next;
                return Ok(if self.s.dialog.is_none() { vec![CampaignEvent::DialogEnded] } else { vec![] });
            }
            CampaignCommand::DialogSkip => {
                self.s.dialog.take().ok_or(CampaignRefusal::NotNow)?;
                return Ok(vec![CampaignEvent::DialogEnded]);
            }
            CampaignCommand::CloseCard => {
                self.s.card.take().ok_or(CampaignRefusal::NotNow)?;
                return Ok(vec![]);
            }
            CampaignCommand::CloseBreather => {
                self.s.breather.take().ok_or(CampaignRefusal::NotNow)?;
                return Ok(vec![]);
            }
            CampaignCommand::CloseLesson => {
                self.s.lesson.take().ok_or(CampaignRefusal::NotNow)?;
                return Ok(vec![]);
            }
            CampaignCommand::AdoptPet { kind } => {
                if self.s.pet.is_some() || kind > 2 {
                    return Err(CampaignRefusal::NotNow);
                }
                let p = (player as usize).min(self.s.crew.len().saturating_sub(1));
                let spot = self.pet_spots()[kind as usize];
                let me = self.s.crew[p].pos;
                if (me.0 - spot.0).hypot(me.1 - spot.1) > PET_REACH {
                    return Err(CampaignRefusal::TooFar);
                }
                self.s.pet = Some(Pet { kind, mood: 100.0, owner: p as u8, follow: true, at: spot });
                return Ok(vec![CampaignEvent::PetAdopted(kind)]);
            }
            CampaignCommand::FeedPet => {
                if self.s.pet.is_none() || self.s.tasks.iter().any(|t| t.id == "pet_feed") || self.content.task("pet_feed").is_none() {
                    return Err(CampaignRefusal::NotNow);
                }
                return Ok(self.offer("pet_feed", false, None).into_iter().collect());
            }
            CampaignCommand::Talk { npc } => {
                if self.s.dialog.is_some() || self.s.field.is_some() || !matches!(self.s.phase, Phase::Ship { .. }) {
                    return Err(CampaignRefusal::NotNow);
                }
                let id = self.content.npcs.get(npc as usize).map(|n| n.id.clone()).ok_or(CampaignRefusal::NotNow)?;
                let talks: Vec<&crate::content::Dialog> = self.content.dialogs.iter().filter(|d| d.trigger == Trigger::Talk(id.clone()) && d.require_flag.as_ref().is_none_or(|f| self.s.flags.contains(f))).collect();
                if talks.is_empty() {
                    return Err(CampaignRefusal::NotNow);
                }
                // new conversations first, then the old ones again in turn
                let pick = talks.iter().find(|d| !self.s.seen_dialogs.contains(&d.id)).copied().unwrap_or_else(|| {
                    let n = self.s.seen_dialogs.iter().filter(|s| talks.iter().any(|d| &&d.id == s)).count();
                    talks[n % talks.len()]
                });
                let d = dialog::start(pick);
                self.s.seen_dialogs.push(pick.id.clone());
                self.s.dialog = Some(d);
                return Ok(vec![CampaignEvent::DialogStarted(pick.id.clone())]);
            }
            CampaignCommand::PetToggle => {
                let p = (player as usize).min(self.s.crew.len().saturating_sub(1));
                let me = self.s.crew[p].pos;
                let pet = self.s.pet.as_mut().ok_or(CampaignRefusal::NotNow)?;
                pet.follow = !pet.follow;
                if pet.follow {
                    pet.owner = p as u8;
                } else {
                    pet.at = me;
                }
                return Ok(vec![]);
            }
            CampaignCommand::RestartDay => {
                let Phase::GameOver(_) = self.s.phase else { return Err(CampaignRefusal::NotNow) };
                let cp = self.checkpoint.0.clone();
                let day = match cp.phase {
                    Phase::Ship { day } => day,
                    _ => 1,
                };
                self.s = cp;
                return Ok(vec![CampaignEvent::DayRestarted(day)]);
            }
            CampaignCommand::NextDay => {
                if !self.creative() {
                    return Err(CampaignRefusal::NotCreative);
                }
                let Phase::Ship { day } = self.s.phase else { return Err(CampaignRefusal::NotNow) };
                self.s.tasks.clear();
                self.s.field = None;
                return Ok(self.end_day(day));
            }
            CampaignCommand::Landed => {
                if self.s.phase != Phase::Landing {
                    return Err(CampaignRefusal::NotNow);
                }
                self.s.phase = Phase::Moon { day: 1 };
                return Ok(vec![]);
            }
            _ => {}
        }
        if !matches!(self.s.phase, Phase::Ship { .. }) || self.frozen() {
            return Err(CampaignRefusal::NotNow);
        }
        match cmd {
            CampaignCommand::OpenTask { uid } => {
                let t = self.s.tasks.iter().find(|t| t.uid == uid).ok_or(CampaignRefusal::UnknownTask)?;
                if t.worker.is_some_and(|w| w as usize != p) {
                    return Err(CampaignRefusal::Busy);
                }
                let at = self.console_pos(&t.room, &t.console).ok_or(CampaignRefusal::UnknownTask)?;
                let me = self.s.crew[p].pos;
                if (me.0 - at.0).hypot(me.1 - at.1) > REACH {
                    return Err(CampaignRefusal::TooFar);
                }
                let (kind, mini_kind) = (t.kind, self.content.task(&t.id).and_then(|d| d.minigame));
                let hard = self.s.cfg.level == Level::Advanced;
                let mut ev = vec![];
                if kind == TaskKind::Asteroids {
                    if self.s.field.is_none() {
                        self.s.field = Some(AsteroidField::new(self.s.waves));
                        ev.push(CampaignEvent::WaveStarted(self.s.waves + 1));
                    }
                }
                let new_mini = mini_kind.map(|k| Mini::new(k, hard, &mut self.rng));
                let t = self.s.tasks.iter_mut().find(|t| t.uid == uid).ok_or(CampaignRefusal::UnknownTask)?;
                t.worker = Some(p as u8);
                if t.mini.is_none() {
                    t.mini = new_mini;
                    if let (Some(m), Some(p)) = (t.mini.as_mut(), self.s.pet.as_ref()) {
                        m.set_pet(p.kind);
                    }
                }
                Ok(ev)
            }
            CampaignCommand::CloseTask { uid } => {
                let t = self.s.tasks.iter_mut().find(|t| t.uid == uid).ok_or(CampaignRefusal::UnknownTask)?;
                t.worker = None;
                Ok(vec![])
            }
            CampaignCommand::Mini { uid, input } => {
                let t = self.s.tasks.iter_mut().find(|t| t.uid == uid).ok_or(CampaignRefusal::UnknownTask)?;
                if t.worker != Some(p as u8) {
                    return Err(CampaignRefusal::NotNow);
                }
                let m = t.mini.as_mut().ok_or(CampaignRefusal::NotNow)?;
                match m.input(input) {
                    Outcome::Done => Ok(self.complete(p as u8, uid, 0)),
                    Outcome::Mistake => Ok(vec![CampaignEvent::Mistake]),
                    Outcome::Fine => Ok(vec![]),
                }
            }
            CampaignCommand::Steer { x, y } => {
                let f = self.s.field.as_mut().ok_or(CampaignRefusal::NotNow)?;
                f.steer(p as u8, x, y);
                Ok(vec![])
            }
            _ => Err(CampaignRefusal::NotNow),
        }
    }

    /// Developer / test shortcut: jump straight to lunar orbit (the flight is over).
    pub fn skip_to_landing(&mut self) {
        self.s.tasks.clear();
        self.s.field = None;
        self.s.dialog = None;
        self.s.card = None;
        self.s.lesson = None;
        self.s.breather = None;
        self.s.phase = Phase::Landing;
    }

    /// Where the three pets wait to be adopted (around the pet corner).
    pub fn pet_spots(&self) -> [(f32, f32); 3] {
        let c = self.console_pos("habitat", "petbed").unwrap_or((5.0, 10.5));
        [(c.0 - 2.4, c.1 + 1.3), (c.0 - 1.2, c.1 + 1.5), (c.0 + 0.0, c.1 + 1.3)]
    }

    /// Where a console is (cell centre).
    pub fn console_pos(&self, room: &str, console: &str) -> Option<(f32, f32)> {
        let r = self.content.room(room)?;
        let k = r.consoles.iter().find(|k| k.id == console)?;
        Some((k.x as f32 + 0.5, k.y as f32 + 0.5))
    }
}

fn uid_worker(s: &CampaignState, uid: u32) -> u8 {
    s.tasks.iter().find(|t| t.uid == uid).and_then(|t| t.worker).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{default_dir, load};

    fn content() -> Content {
        load(&default_dir()).expect("content")
    }

    #[derive(Clone, Copy)]
    struct Bot {
        /// seconds to notice and walk to a task
        react: f32,
        /// works on problems and dodges asteroids
        careful: bool,
    }

    /// Plays the whole flight; returns the final state.
    fn fly(cfg: ExpeditionConfig, bot: Bot, c: &Content) -> CampaignState {
        let mut g = Campaign::new(cfg, c);
        let dt = 0.25;
        let mut wait = 0.0f32;
        let mut guard = 0;
        while matches!(g.state().phase, Phase::Ship { .. }) && guard < 40_000 {
            guard += 1;
            g.tick(dt);
            // close dialogs, cards and breathers like a reader would
            if g.state().dialog.is_some() {
                let _ = g.apply(0, CampaignCommand::DialogChoose(0));
                let _ = g.apply(0, CampaignCommand::DialogNext);
                continue;
            }
            let _ = g.apply(0, CampaignCommand::CloseCard);
            let _ = g.apply(0, CampaignCommand::CloseBreather);
            let _ = g.apply(0, CampaignCommand::CloseLesson);
            if let Some(f) = &g.state().field {
                if bot.careful {
                    if let Some(r) = f.rocks.iter().filter(|r| r.x > f.ship.0).min_by(|a, b| a.x.total_cmp(&b.x)) {
                        let y = if r.y > f.ship.1 { r.y - 0.25 } else { r.y + 0.25 };
                        let _ = g.apply(0, CampaignCommand::Steer { x: 0.15, y });
                    }
                }
                continue;
            }
            wait -= dt;
            if wait > 0.0 {
                continue;
            }
            // pick a task: careful does problems first, careless ignores problems
            let pick = {
                let s = g.state();
                let mut ts: Vec<&ActiveTask> = s.tasks.iter().filter(|t| bot.careful || t.kind != TaskKind::Problem).collect();
                ts.sort_by_key(|t| (t.kind != TaskKind::Problem, t.uid));
                ts.first().map(|t| (t.uid, t.room.clone(), t.console.clone()))
            };
            let Some((uid, room, console)) = pick else { continue };
            if let Some((x, y)) = g.console_pos(&room, &console) {
                g.set_crew_pos(0, x, y);
            }
            g.apply(0, CampaignCommand::OpenTask { uid }).unwrap();
            if let Some(m) = g.state().tasks.iter().find(|t| t.uid == uid).and_then(|t| t.mini.clone()) {
                for i in m.solution() {
                    let _ = g.apply(0, CampaignCommand::Mini { uid, input: i });
                }
            }
            wait = bot.react;
        }
        g.state().clone()
    }

    #[test]
    fn the_director_keeps_the_crew_busy_and_fair() {
        let c = content();
        let (mut good_coins, mut bad_coins, mut good_hull, mut bad_hull) = (0u64, 0u64, 0.0f32, 0.0f32);
        for i in 0..300u64 {
            let level = if i % 2 == 0 { Level::Junior } else { Level::Advanced };
            let cfg = ExpeditionConfig { seed: 1000 + i, ..ExpeditionConfig::new(1, level) };
            let careful = fly(cfg, Bot { react: 6.0, careful: true }, &c);
            assert_eq!(careful.phase, Phase::Landing, "seed {i}: a careful crew always reaches the Moon");
            assert!(careful.score.max_idle < 30.0, "seed {i}: idle {}", careful.score.max_idle);
            let limit = if level == Level::Junior { 1 } else { 2 };
            assert!(careful.score.max_urgent <= limit, "seed {i}: {} urgent at once", careful.score.max_urgent);
            assert_eq!(careful.cards_seen.len(), careful.cards_seen.iter().collect::<std::collections::HashSet<_>>().len());
            let careless = fly(cfg, Bot { react: 6.0, careful: false }, &c);
            assert_eq!(careless.phase, Phase::Landing, "seed {i}: a careless crew still makes it");
            good_coins += careful.wallet.team as u64;
            bad_coins += careless.wallet.team as u64;
            good_hull += careful.ship.hull;
            bad_hull += careless.ship.hull;
        }
        println!("300 flights: careful {} coins, hull {:.0}% · careless {} coins, hull {:.0}%", good_coins / 300, good_hull / 300.0, bad_coins / 300, bad_hull / 300.0);
        assert!(bad_coins * 10 < good_coins * 9, "a careless crew earns clearly less");
        assert!(bad_hull < good_hull);
    }

    #[test]
    fn game_over_and_start_the_day_again() {
        let c = content();
        let mut g = Campaign::new(ExpeditionConfig::new(1, Level::Advanced), &c);
        let _ = g.apply(0, CampaignCommand::DialogSkip);
        g.tick(1.0);
        let day_coins = g.state().wallet.team;
        g.s.ship.hull = 0.0;
        let ev = g.tick(0.1);
        assert!(ev.contains(&CampaignEvent::GameOver(ShipFail::Hull)));
        assert_eq!(g.tick(1.0), vec![], "nothing moves after game over");
        let ev = g.apply(0, CampaignCommand::RestartDay).unwrap();
        assert_eq!(ev, vec![CampaignEvent::DayRestarted(1)]);
        assert_eq!(g.state().ship.hull, 100.0);
        assert_eq!(g.state().wallet.team, day_coins);
        assert!(matches!(g.state().phase, Phase::Ship { day: 1 }));
    }

    #[test]
    fn creative_mode_has_no_problems_and_can_skip_days() {
        let c = content();
        let cfg = ExpeditionConfig { mode: PlayMode::Creative, ..ExpeditionConfig::new(1, Level::Junior) };
        let mut g = Campaign::new(cfg, &c);
        let _ = g.apply(0, CampaignCommand::DialogSkip);
        for _ in 0..2000 {
            g.tick(0.25);
            let _ = g.apply(0, CampaignCommand::DialogSkip);
        }
        assert!(g.state().tasks.iter().all(|t| t.kind != TaskKind::Problem));
        assert_eq!(g.state().ship.hull, 100.0);
        for _ in 0..4 {
            let _ = g.apply(0, CampaignCommand::NextDay);
            let _ = g.apply(0, CampaignCommand::DialogSkip);
        }
        assert_eq!(g.state().phase, Phase::Landing);
        assert_eq!(g.apply(0, CampaignCommand::Landed), Ok(vec![]));
        assert_eq!(g.state().phase, Phase::Moon { day: 1 });
        let survival = Campaign::new(ExpeditionConfig::new(1, Level::Junior), &c);
        let mut survival = survival;
        assert_eq!(survival.apply(0, CampaignCommand::NextDay), Err(CampaignRefusal::NotCreative));
    }

    #[test]
    fn tasks_need_the_crew_at_the_console_and_dialogs_branch() {
        let c = content();
        let mut g = Campaign::new(ExpeditionConfig::new(2, Level::Junior), &c);
        // day 1 opens with a dialog; a choice may set a flag
        assert!(g.state().dialog.is_some(), "the flight starts with a briefing");
        while g.state().dialog.is_some() {
            g.apply(0, CampaignCommand::DialogChoose(0)).unwrap();
            let _ = g.apply(0, CampaignCommand::DialogNext);
        }
        g.tick(0.5);
        let t = g.state().tasks[0].clone();
        g.set_crew_pos(1, -50.0, -50.0);
        assert_eq!(g.apply(1, CampaignCommand::OpenTask { uid: t.uid }), Err(CampaignRefusal::TooFar));
        let (x, y) = g.console_pos(&t.room, &t.console).unwrap();
        g.set_crew_pos(1, x, y);
        g.apply(1, CampaignCommand::OpenTask { uid: t.uid }).unwrap();
        g.set_crew_pos(0, x, y);
        assert_eq!(g.apply(0, CampaignCommand::OpenTask { uid: t.uid }), Err(CampaignRefusal::Busy), "one player per console");
        let sol = g.state().tasks[0].mini.clone().unwrap().solution();
        let mut last = vec![];
        for i in sol {
            last = g.apply(1, CampaignCommand::Mini { uid: t.uid, input: i }).unwrap();
        }
        assert!(matches!(last[0], CampaignEvent::TaskDone { .. }));
        assert!(g.state().wallet.personal[1] > 0, "the worker gets personal coins");
        assert!(g.state().wallet.team > 0);
    }

    #[test]
    fn a_physics_card_after_each_episode_in_order() {
        let c = content();
        let mut seen = vec![];
        let mut shown = vec![];
        for ep in ["a", "b", "c"] {
            if let Some(id) = crate::physics::next_card(&c, &seen, ep) {
                seen.push(id.to_string());
                shown.push(id.to_string());
            }
        }
        // no card after an episode it has nothing to do with
        assert!(shown.is_empty(), "{shown:?}");
        let tied = c.cards.iter().find(|k| k.after.is_some()).unwrap();
        assert_eq!(crate::physics::next_card(&c, &[], tied.after.as_ref().unwrap()), Some(tied.id.as_str()));
    }

    #[test]
    fn campaign_state_travels_over_the_network() {
        let c = content();
        let g = Campaign::new(ExpeditionConfig::new(4, Level::Advanced), &c);
        let json = serde_json::to_string(g.state()).unwrap();
        let back: CampaignState = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, g.state());
        assert_eq!(back.crew.len(), 4);
    }

    /// Network guest: never simulates, queues commands, keeps its own position.
    #[test]
    fn mirror_queues_commands_and_keeps_own_position() {
        let c = content();
        let cfg = ExpeditionConfig::new(2, Level::Advanced);
        let mut host = Campaign::new(cfg, &c);
        let mut guest = Campaign::new(cfg, &c);
        guest.mirror_as(host.state().clone(), 1);
        assert!(guest.is_mirror());
        guest.set_crew_pos(1, 2.0, 3.0);
        let before = guest.state().elapsed;
        assert!(guest.tick(5.0).is_empty());
        assert_eq!(guest.state().elapsed, before);
        assert_eq!(guest.apply(1, CampaignCommand::DialogSkip), Ok(vec![]));
        let out = guest.take_outbox();
        assert_eq!(out, vec![(1, CampaignCommand::DialogSkip)]);
        for (p, cmd) in out {
            let _ = host.apply(p, cmd);
        }
        host.tick(1.0);
        guest.mirror_as(host.state().clone(), 1);
        assert_eq!(guest.state().crew[1].pos, (2.0, 3.0), "own position is ours");
        assert_eq!(guest.state().elapsed, host.state().elapsed);
    }

    /// The pet is optional: walk up and take it, it sits / follows on E; nobody has to adopt it.
    #[test]
    fn pet_is_adopted_by_walking_up_and_never_blocks_the_flight() {
        let c = content();
        let mut g = Campaign::new(ExpeditionConfig::new(1, Level::Junior), &c);
        let spot = g.pet_spots()[1];
        g.set_crew_pos(0, spot.0 + 5.0, spot.1);
        assert_eq!(g.apply(0, CampaignCommand::AdoptPet { kind: 1 }), Err(CampaignRefusal::TooFar));
        g.set_crew_pos(0, spot.0 + 0.5, spot.1);
        assert!(g.apply(0, CampaignCommand::AdoptPet { kind: 1 }).is_ok());
        assert_eq!(g.state().pet.as_ref().map(|p| (p.kind, p.follow)), Some((1, true)));
        assert!(g.apply(0, CampaignCommand::AdoptPet { kind: 2 }).is_err(), "only one pet");
        g.apply(0, CampaignCommand::PetToggle).unwrap();
        assert!(!g.state().pet.as_ref().unwrap().follow, "sits");
        g.apply(0, CampaignCommand::PetToggle).unwrap();
        assert!(g.state().pet.as_ref().unwrap().follow, "follows again");
        // no story task needs the pet
        assert!(c.days.iter().all(|d| d.scripted.iter().all(|t| !t.starts_with("pet"))));
    }

    /// Talking to a crew member: new conversations first, then they come round again.
    #[test]
    fn crew_talks_are_new_first_then_repeat() {
        let c = content();
        let mut g = Campaign::new(ExpeditionConfig::new(1, Level::Junior), &c);
        while g.state().dialog.is_some() {
            let _ = g.apply(0, CampaignCommand::DialogSkip);
        }
        let aya = c.npcs.iter().position(|n| n.id == "aya").unwrap() as u8;
        let mut ids = vec![];
        for _ in 0..4 {
            g.apply(0, CampaignCommand::Talk { npc: aya }).unwrap();
            ids.push(g.state().dialog.as_ref().unwrap().id.clone());
            g.apply(0, CampaignCommand::DialogSkip).unwrap();
        }
        assert_eq!(ids[..3].iter().collect::<std::collections::HashSet<_>>().len(), 3, "{ids:?}");
        assert_eq!(ids[3], ids[0]);
        assert!(c.npcs.iter().filter(|n| n.on_board).all(|n| !n.chatter.is_empty()));
    }
}
