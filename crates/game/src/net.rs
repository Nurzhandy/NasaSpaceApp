//! A7–A9 — co-op over the local network: up to 4 computers on the same Wi‑Fi, plain UDP, no server.
//!
//! The **host** runs the rules (`ja_core::Game` on the Moon, `ja_core::Campaign` on the ship) and
//! its keyboard drives astronaut 0. Each **guest** gets a seat (1–3) and drives that astronaut: it
//! sends its position and numbered messages (commands, quick phrases) that are re-sent until the
//! host acknowledges them, and shows the host's state, broadcast ~15 times per second. The guest's
//! copy is a mirror (`Game::mirror`, `Campaign::mirror_as`): it never simulates, so the computers
//! can't drift apart.
//!
//! Finding a game: the host gets a 4-digit **room code** and announces it every second on UDP port
//! `BEACON_PORT`; friends type the code (or the host's IP, if the school Wi‑Fi blocks broadcasts).
//! The room goes `Waiting → Preparing → Active ⇄ Paused → Completed / Failed`. A guest who drops
//! out keeps their seat: the host pauses and saves, the guest reconnects on its own and play goes on.

use crate::art::Art;
use crate::kit::{self, Act, Action, UiAction, UiBusy, C_GOOD, C_MUTED, C_TEXT, C_WARN};
use crate::player::Player;
use crate::ship::{Camp, ContentRes};
use crate::world::{Happened, Refused};
use crate::{AppState, Prof, Set, Setup, Sim};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use ja_core::campaign::{Campaign, CampaignCommand, CampaignState, Phase};
use ja_core::{Command, GameEvent, Lang, MissionConfig, Refusal, State, Status};
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

pub const PORT: u16 = 7777;
pub const BEACON_PORT: u16 = 7778;
const PROTO: u32 = 2;
/// the host and up to three friends
pub const SEATS: usize = 4;
const SEND_EVERY: f32 = 1.0 / 15.0;
/// no packets for this long = the other computer dropped out
const TIMEOUT: f32 = 5.0;
/// a guest that dropped out keeps trying to come back this long
const RECONNECT: f32 = 60.0;
const MAX_PACKET: usize = 65_000;
/// one quick phrase per player per this many seconds (no spamming)
const SAY_EVERY: f32 = 1.5;

/// Where the room is (shown to everyone and announced on the LAN).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Room {
    /// the lobby is open, friends are joining
    #[default]
    Waiting,
    /// the host is choosing the base / expedition
    Preparing,
    Active,
    /// someone dropped out: the game waits for them
    Paused,
    Completed,
    Failed,
}

impl Room {
    pub fn label(self, l: Lang) -> &'static str {
        match self {
            Room::Waiting => l.pick("ждём игроков", "waiting for players"),
            Room::Preparing => l.pick("готовимся", "preparing"),
            Room::Active => l.pick("идёт игра", "playing"),
            Room::Paused => l.pick("пауза — ждём игрока", "paused — waiting for a player"),
            Room::Completed => l.pick("миссия выполнена", "mission complete"),
            Room::Failed => l.pick("миссия провалена", "mission failed"),
        }
    }
    /// A mission is running: only players already in it may come (back).
    pub fn locked(self) -> bool {
        matches!(self, Room::Active | Room::Paused)
    }
}

/// Quick phrases (A9): icon, Russian, English.
pub const PHRASES: [(&str, &str, &str); 6] = [
    ("thumbs", "Отлично!", "Great!"),
    ("sos", "Помоги!", "Help!"),
    ("walk", "Иду!", "On my way!"),
    ("check", "Готово!", "Done!"),
    ("hourglass", "Подожди!", "Wait!"),
    ("battery", "Нужна энергия!", "Need power!"),
];

/// What a guest asks the host to do, numbered and re-sent until acknowledged.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Msg {
    /// on the Moon
    Cmd(Command),
    /// on the ship
    Ship(CampaignCommand),
    /// a quick phrase
    Say(u8),
}

/// A quick phrase someone said (numbered, so re-sent copies are shown once).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Said {
    pub n: u32,
    pub seat: u8,
    pub phrase: u8,
}

/// Host → one guest.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Down {
    pub proto: u32,
    /// your seat (astronaut number)
    pub seat: u8,
    pub code: u16,
    pub room: Room,
    /// who is here, by seat
    pub present: [bool; SEATS],
    /// the Moon mission, if one is running
    pub cfg: Option<MissionConfig>,
    /// astronauts in that mission
    pub crew: u8,
    /// the last of your messages the host has applied
    pub ack: u32,
    pub state: Option<State>,
    /// the ship, while flying
    pub ship: Option<CampaignState>,
    pub events: Vec<GameEvent>,
    pub refusals: Vec<Refusal>,
    pub said: Vec<Said>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Packet {
    /// host → everyone on the LAN, once a second
    Beacon { proto: u32, code: u16, room: Room, players: u8 },
    /// guest → host until answered (also to come back after a drop-out: `id` finds the old seat)
    Hello { proto: u32, id: u64 },
    /// guest → host: where my astronaut stands + messages not yet acknowledged
    Up { pos: (f32, f32), msgs: Vec<(u32, Msg)> },
    Down(Box<Down>),
    /// host → guest: no free seat (or a mission is running and you weren't in it)
    Full,
    /// polite goodbye
    Bye,
}

fn encode(p: &Packet) -> Vec<u8> {
    let mut b = serde_json::to_vec(p).unwrap_or_default();
    if b.len() > MAX_PACKET {
        // a very long sandbox mission: drop the per-sol history rather than the packet
        if let Packet::Down(d) = p {
            let mut d = d.clone();
            if let Some(s) = d.state.as_mut() {
                s.history.clear();
            }
            b = serde_json::to_vec(&Packet::Down(d)).unwrap_or_default();
        }
    }
    b
}

/// Reads everything waiting on a non-blocking socket.
fn drain(sock: &UdpSocket) -> Vec<(Packet, SocketAddr)> {
    let mut out = vec![];
    let mut buf = vec![0u8; 65_536];
    for _ in 0..512 {
        match sock.recv_from(&mut buf) {
            Ok((n, from)) => {
                if let Ok(p) = serde_json::from_slice::<Packet>(&buf[..n]) {
                    out.push((p, from));
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
            // Windows reports "connection reset" for an earlier packet nobody received: ignore
            Err(_) => continue,
        }
    }
    out
}

/// This computer's address on the local network (no packets are sent to find it).
pub fn local_ip() -> IpAddr {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|a| a.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

fn random_u64() -> u64 {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    // splitmix64: the clock alone is too regular
    let mut z = t ^ (std::process::id() as u64).rotate_left(32);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

// ---------------------------------------------------------------- host

pub struct Guest {
    pub addr: SocketAddr,
    pub id: u64,
    pub here: bool,
    silence: f32,
    applied: u32,
    pos: Option<(f32, f32)>,
    msgs: Vec<Msg>,
    refusals: Vec<Refusal>,
    say_t: f32,
}

pub struct HostLink {
    sock: UdpSocket,
    beacon: Option<UdpSocket>,
    beacon_t: f32,
    /// the UDP port actually bound (tests bind port 0)
    #[allow(dead_code)]
    pub port: u16,
    pub code: u16,
    /// seat 0 is the host itself
    pub seats: [Option<Guest>; SEATS],
    pub room: Room,
    send_t: f32,
    pub out_events: Vec<GameEvent>,
    said: Vec<(Said, f32)>,
    next_said: u32,
    /// phrases said since the last `take_said` (for the host's own screen)
    fresh: Vec<Said>,
}

impl HostLink {
    pub fn bind(port: u16) -> std::io::Result<Self> {
        let sock = UdpSocket::bind(("0.0.0.0", port))?;
        sock.set_nonblocking(true)?;
        let port = sock.local_addr()?.port();
        let beacon = UdpSocket::bind("0.0.0.0:0").ok().filter(|b| b.set_broadcast(true).is_ok() && b.set_nonblocking(true).is_ok());
        let code = 1000 + (random_u64() % 9000) as u16;
        Ok(Self { sock, beacon, beacon_t: 0.0, port, code, seats: Default::default(), room: Room::Waiting, send_t: 0.0, out_events: vec![], said: vec![], next_said: 1, fresh: vec![] })
    }
    /// Guests connected right now.
    pub fn connected(&self) -> usize {
        self.seats.iter().flatten().filter(|g| g.here).count()
    }
    /// Seats taken (including guests who dropped out of a running mission).
    pub fn taken(&self) -> usize {
        self.seats.iter().flatten().count()
    }
    /// Someone in the mission dropped out.
    pub fn missing(&self) -> bool {
        self.seats.iter().flatten().any(|g| !g.here)
    }
    pub fn present(&self) -> [bool; SEATS] {
        let mut p = [false; SEATS];
        p[0] = true;
        for (i, g) in self.seats.iter().enumerate() {
            if let Some(g) = g {
                p[i] = g.here;
            }
        }
        p
    }
    /// Before a mission starts: close the gaps so the seats are 1, 2, 3 in a row.
    fn compact(&mut self) {
        let mut gs: Vec<Guest> = self.seats.iter_mut().skip(1).filter_map(|s| s.take()).collect();
        for i in 1..SEATS {
            if !gs.is_empty() {
                self.seats[i] = Some(gs.remove(0));
            }
        }
    }
    /// Receive what the guests sent; tell the LAN we exist.
    pub fn poll(&mut self, dt: f32) {
        let locked = self.room.locked();
        for s in self.seats.iter_mut() {
            if let Some(g) = s {
                g.silence += dt;
                g.say_t -= dt;
                if g.here && g.silence > TIMEOUT {
                    g.here = false;
                }
                // between missions a seat isn't kept for long
                if !locked && !g.here && g.silence > TIMEOUT * 2.0 {
                    *s = None;
                }
            }
        }
        if !locked {
            self.compact();
        }
        for a in self.said.iter_mut() {
            a.1 += dt;
        }
        self.said.retain(|a| a.1 < 3.0);
        for (p, from) in drain(&self.sock) {
            match p {
                Packet::Hello { proto, id } if proto == PROTO => self.hello(id, from),
                Packet::Up { pos, msgs } => {
                    let Some(i) = self.seat_of(from) else { continue };
                    let Some(g) = self.seats[i].as_mut() else { continue };
                    g.silence = 0.0;
                    g.here = true;
                    g.pos = Some(pos);
                    let mut msgs = msgs;
                    msgs.sort_by_key(|c| c.0);
                    let mut say = vec![];
                    for (seq, m) in msgs {
                        if seq == g.applied + 1 {
                            g.applied = seq;
                            match m {
                                Msg::Say(ph) => {
                                    if g.say_t <= 0.0 {
                                        g.say_t = SAY_EVERY;
                                        say.push(ph);
                                    }
                                }
                                m => g.msgs.push(m),
                            }
                        }
                    }
                    for ph in say {
                        self.say(i as u8, ph);
                    }
                }
                Packet::Bye => {
                    if let Some(i) = self.seat_of(from) {
                        if locked {
                            if let Some(g) = self.seats[i].as_mut() {
                                g.here = false;
                            }
                        } else {
                            self.seats[i] = None;
                        }
                    }
                }
                _ => {}
            }
        }
        self.beacon_t -= dt;
        if self.beacon_t <= 0.0 {
            self.beacon_t = 1.0;
            if let Some(b) = &self.beacon {
                let p = Packet::Beacon { proto: PROTO, code: self.code, room: self.room, players: 1 + self.connected() as u8 };
                let _ = b.send_to(&encode(&p), (Ipv4Addr::BROADCAST, BEACON_PORT));
            }
        }
    }
    fn seat_of(&self, from: SocketAddr) -> Option<usize> {
        self.seats.iter().position(|g| g.as_ref().is_some_and(|g| g.addr == from))
    }
    fn hello(&mut self, id: u64, from: SocketAddr) {
        // back after a drop-out (or a re-sent hello): the same seat
        if let Some(g) = self.seats.iter_mut().flatten().find(|g| g.id == id) {
            g.addr = from;
            g.here = true;
            g.silence = 0.0;
            self.send_t = SEND_EVERY;
            return;
        }
        let free = (1..SEATS).find(|i| self.seats[*i].is_none());
        match free {
            Some(i) if !self.room.locked() => {
                self.seats[i] = Some(Guest { addr: from, id, here: true, silence: 0.0, applied: 0, pos: None, msgs: vec![], refusals: vec![], say_t: 0.0 });
                self.send_t = SEND_EVERY; // answer right away
            }
            _ => {
                let _ = self.sock.send_to(&encode(&Packet::Full), from);
            }
        }
    }
    /// Each guest's latest position and the messages to run for their astronaut, in order.
    pub fn take_input(&mut self) -> Vec<(u8, Option<(f32, f32)>, Vec<Msg>)> {
        self.seats.iter_mut().enumerate().filter_map(|(i, g)| g.as_mut().map(|g| (i as u8, g.pos.take(), std::mem::take(&mut g.msgs)))).collect()
    }
    /// Tell this guest why their command didn't work (the host's screen stays calm).
    pub fn refuse(&mut self, seat: u8, r: Refusal) {
        if let Some(Some(g)) = self.seats.get_mut(seat as usize) {
            g.refusals.push(r);
        }
    }
    /// A quick phrase from `seat` (the host itself is seat 0) for everyone.
    pub fn say(&mut self, seat: u8, phrase: u8) -> Said {
        let s = Said { n: self.next_said, seat, phrase: phrase.min(PHRASES.len() as u8 - 1) };
        self.next_said += 1;
        self.said.push((s, 0.0));
        self.fresh.push(s);
        s
    }
    pub fn take_said(&mut self) -> Vec<Said> {
        std::mem::take(&mut self.fresh)
    }
    /// Send the current state to every guest (rate-limited).
    pub fn send(&mut self, dt: f32, cfg: Option<MissionConfig>, crew: u8, state: Option<&State>, ship: Option<&CampaignState>) {
        self.send_t += dt;
        if self.connected() == 0 {
            self.out_events.clear();
            return;
        }
        if self.send_t < SEND_EVERY {
            return;
        }
        self.send_t = 0.0;
        let present = self.present();
        let events = std::mem::take(&mut self.out_events);
        let said: Vec<Said> = self.said.iter().map(|a| a.0).collect();
        let mut out = vec![];
        for (i, g) in self.seats.iter_mut().enumerate() {
            let Some(g) = g.as_mut().filter(|g| g.here) else { continue };
            let d = Down { proto: PROTO, seat: i as u8, code: self.code, room: self.room, present, cfg, crew, ack: g.applied, state: state.cloned(), ship: ship.cloned(), events: events.clone(), refusals: std::mem::take(&mut g.refusals), said: said.clone() };
            out.push((g.addr, encode(&Packet::Down(Box::new(d)))));
        }
        for (to, b) in out {
            let _ = self.sock.send_to(&b, to);
        }
    }
    pub fn bye(&self) {
        for g in self.seats.iter().flatten().filter(|g| g.here) {
            let _ = self.sock.send_to(&encode(&Packet::Bye), g.addr);
        }
    }
}

// ---------------------------------------------------------------- guest

pub struct GuestLink {
    sock: UdpSocket,
    pub host: SocketAddr,
    /// who we are to the host (keeps our seat after a drop-out)
    pub id: u64,
    next: u32,
    unacked: Vec<(u32, Msg)>,
    silence: f32,
    pub heard: bool,
    /// the host said no: the room is full or a mission is running
    pub full: bool,
    send_t: f32,
    pub seat: u8,
    pub code: u16,
    pub room: Room,
    pub present: [bool; SEATS],
    pub crew: u8,
    pub cfg: Option<MissionConfig>,
    pub state: Option<State>,
    pub ship: Option<CampaignState>,
    pub events: Vec<GameEvent>,
    pub refusals: Vec<Refusal>,
    pub said: Vec<Said>,
    last_said: u32,
}

impl GuestLink {
    pub fn connect(host: SocketAddr, id: u64) -> std::io::Result<Self> {
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.set_nonblocking(true)?;
        Ok(Self {
            sock,
            host,
            id,
            next: 1,
            unacked: vec![],
            silence: 0.0,
            heard: false,
            full: false,
            send_t: 1.0,
            seat: 1,
            code: 0,
            room: Room::Waiting,
            present: [false; SEATS],
            crew: 2,
            cfg: None,
            state: None,
            ship: None,
            events: vec![],
            refusals: vec![],
            said: vec![],
            last_said: 0,
        })
    }
    /// The host went quiet: we keep our seat and keep knocking.
    pub fn reconnecting(&self) -> bool {
        self.heard && self.silence > TIMEOUT
    }
    /// Give up: the host is gone for good (or never answered, or said no).
    pub fn lost(&self) -> bool {
        self.full || self.silence > if self.heard { RECONNECT } else { 8.0 }
    }
    pub fn queue(&mut self, m: Msg) {
        self.unacked.push((self.next, m));
        self.next += 1;
    }
    pub fn poll(&mut self, dt: f32) {
        self.silence += dt;
        for (p, from) in drain(&self.sock) {
            if from.ip() != self.host.ip() {
                continue;
            }
            match p {
                Packet::Down(d) if d.proto == PROTO => {
                    let d = *d;
                    self.heard = true;
                    self.silence = 0.0;
                    self.unacked.retain(|(s, _)| *s > d.ack);
                    self.seat = d.seat;
                    self.code = d.code;
                    self.room = d.room;
                    self.present = d.present;
                    self.crew = d.crew;
                    self.cfg = d.cfg;
                    if d.state.is_some() {
                        self.state = d.state;
                    }
                    self.ship = d.ship;
                    self.events.extend(d.events);
                    self.refusals.extend(d.refusals);
                    for s in d.said {
                        if s.n > self.last_said {
                            self.last_said = s.n;
                            self.said.push(s);
                        }
                    }
                }
                Packet::Full => self.full = true,
                Packet::Bye => {
                    self.silence = RECONNECT + 1.0;
                    self.heard = true;
                }
                _ => {}
            }
        }
    }
    /// Tell the host where our astronaut is and what we want (rate-limited, messages re-sent until acknowledged).
    pub fn send(&mut self, dt: f32, pos: (f32, f32)) {
        self.send_t += dt;
        if self.send_t < SEND_EVERY && self.unacked.is_empty() {
            return;
        }
        if self.send_t < SEND_EVERY / 2.0 {
            return;
        }
        self.send_t = 0.0;
        // not heard yet, or the host went quiet: knock again (the id gets our seat back)
        let p = if self.heard && self.silence < 1.0 { Packet::Up { pos, msgs: self.unacked.clone() } } else { Packet::Hello { proto: PROTO, id: self.id } };
        let _ = self.sock.send_to(&encode(&p), self.host);
        if !(self.heard && self.silence < 1.0) {
            // and the messages right after, so a quick drop-out loses nothing
            if self.heard {
                let _ = self.sock.send_to(&encode(&Packet::Up { pos, msgs: self.unacked.clone() }), self.host);
            }
        }
    }
    pub fn bye(&self) {
        let _ = self.sock.send_to(&encode(&Packet::Bye), self.host);
    }
}

/// A game announced on the LAN.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Found {
    pub ip: IpAddr,
    pub code: u16,
    pub room: Room,
    pub players: u8,
    age: f32,
}

/// Listens for hosts announcing themselves on the LAN.
pub struct Finder {
    sock: Option<UdpSocket>,
    pub found: Vec<Found>,
}

impl Finder {
    pub fn new() -> Self {
        let sock = UdpSocket::bind(("0.0.0.0", BEACON_PORT)).ok().filter(|s| s.set_nonblocking(true).is_ok());
        Self { sock, found: vec![] }
    }
    /// Returns true when the list changed.
    pub fn poll(&mut self, dt: f32) -> bool {
        let key = |v: &[Found]| v.iter().map(|f| (f.ip, f.code, f.room, f.players)).collect::<Vec<_>>();
        let before = key(&self.found);
        for f in &mut self.found {
            f.age += dt;
        }
        self.found.retain(|f| f.age < 4.0);
        if let Some(s) = &self.sock {
            for (p, from) in drain(s) {
                if let Packet::Beacon { proto: PROTO, code, room, players } = p {
                    let f = Found { ip: from.ip(), code, room, players, age: 0.0 };
                    match self.found.iter_mut().find(|x| x.ip == f.ip) {
                        Some(x) => *x = f,
                        None => self.found.push(f),
                    }
                }
            }
        }
        before != key(&self.found)
    }
}

// ---------------------------------------------------------------- Bevy side

#[derive(Default)]
pub enum Role {
    #[default]
    Off,
    Host(HostLink),
    Guest(GuestLink),
}

#[derive(Resource)]
pub struct Net {
    pub role: Role,
    /// seed of the host mission the guest is playing (or played last)
    joined: Option<u64>,
    ended: bool,
    /// who this computer is to a host (the same while the game runs: reconnection finds the seat)
    pub id: u64,
    /// the host paused the game: someone dropped out
    pub hold: bool,
    /// "mute player": their phrases are not shown here
    pub muted: [bool; SEATS],
    /// phrases on screen: (seat, phrase, age)
    pub chat: Vec<(u8, u8, f32)>,
}

impl Default for Net {
    fn default() -> Self {
        Self { role: Role::Off, joined: None, ended: false, id: random_u64(), hold: false, muted: [false; SEATS], chat: vec![] }
    }
}

impl Net {
    pub fn active(&self) -> bool {
        !matches!(self.role, Role::Off)
    }
    /// The astronaut this computer plays (None when not in a network game).
    pub fn local_idx(&self) -> Option<u8> {
        match &self.role {
            Role::Off => None,
            Role::Host(_) => Some(0),
            Role::Guest(g) => Some(g.seat),
        }
    }
    /// This astronaut is moved by another computer.
    pub fn is_remote(&self, idx: u8) -> bool {
        self.active() && self.local_idx() != Some(idx)
    }
    /// Astronauts for a new mission (0 = decide from `coop`).
    pub fn crew(&self) -> u8 {
        match &self.role {
            Role::Off => 0,
            Role::Host(h) => (1 + h.taken() as u8).min(SEATS as u8),
            Role::Guest(g) => g.crew,
        }
    }
    fn stop(&mut self) {
        match &self.role {
            Role::Host(h) => h.bye(),
            Role::Guest(g) => g.bye(),
            Role::Off => {}
        }
        self.role = Role::Off;
        self.joined = None;
        self.hold = false;
        self.chat.clear();
    }
    fn hear(&mut self, s: Said) {
        if !self.muted.get(s.seat as usize).copied().unwrap_or(false) {
            self.chat.push((s.seat, s.phrase, 0.0));
            if self.chat.len() > 4 {
                self.chat.remove(0);
            }
        }
    }
}

/// Lobby page state: found games, the typed code / IP, the last message.
#[derive(Resource, Default)]
pub struct Lobby {
    finder: Option<Finder>,
    pub hosts: Vec<Found>,
    pub ip_text: String,
    pub note: Option<(&'static str, &'static str)>,
    /// bump to rebuild the lobby page
    pub rev: u32,
}

/// The host's mission the guest should start, and how many astronauts it has (read by `main::start_mission`).
#[derive(Resource)]
pub struct NetStart(pub MissionConfig, pub u8);

#[derive(Component)]
struct Badge;
#[derive(Component)]
struct BadgeText;
#[derive(Component)]
struct ChatRoot;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Net>()
            .init_resource::<Lobby>()
            .add_systems(Startup, spawn_badge)
            .add_systems(Update, (lobby_actions, say_actions, type_ip.run_if(in_state(AppState::Lobby)), find_hosts.run_if(in_state(AppState::Lobby))).after(Set::Input))
            .add_systems(Update, (host_in, guest_in).in_set(Set::Logic).before(crate::world::TickRules))
            .add_systems(Update, (host_out, guest_out, badge, chat_view).in_set(Set::View))
            .add_systems(Update, autotest.run_if(|| std::env::var_os("JA_NET_TEST").is_some()))
            .add_systems(Last, goodbye);
    }
}

fn lobby_actions(mut acts: EventReader<Act>, mut net: ResMut<Net>, mut lobby: ResMut<Lobby>, mut setup: ResMut<Setup>) {
    for Act(a) in acts.read() {
        match *a {
            Action::NetHost => {
                net.stop();
                match HostLink::bind(PORT) {
                    Ok(h) => {
                        net.role = Role::Host(h);
                        setup.coop = true;
                        setup.refresh += 1;
                        lobby.note = None;
                    }
                    Err(_) => lobby.note = Some(("Не получилось открыть порт 7777. Может, игра уже запущена?", "Couldn't open port 7777. Is the game already running?")),
                }
                lobby.rev += 1;
            }
            Action::NetJoin(i) => {
                if let Some(f) = lobby.hosts.get(i as usize).copied() {
                    join(&mut net, &mut lobby, SocketAddr::new(f.ip, PORT));
                }
            }
            Action::NetJoinTyped => {
                let t = lobby.ip_text.trim().to_string();
                if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) {
                    // a room code: find it among the games announced on this network
                    let code: u16 = t.parse().unwrap_or(0);
                    match lobby.hosts.iter().find(|f| f.code == code).copied() {
                        Some(f) => join(&mut net, &mut lobby, SocketAddr::new(f.ip, PORT)),
                        None => {
                            lobby.note = Some(("Комнату с таким кодом не видно в этой сети. Проверь код или набери IP хоста.", "No room with that code on this network. Check the code or type the host's IP."));
                            lobby.rev += 1;
                        }
                    }
                } else {
                    match t.parse::<IpAddr>() {
                        Ok(ip) => join(&mut net, &mut lobby, SocketAddr::new(ip, PORT)),
                        Err(_) => {
                            lobby.note = Some(("Набери код комнаты (4 цифры) или IP-адрес, например 192.168.1.5", "Type the room code (4 digits) or an IP address, e.g. 192.168.1.5"));
                            lobby.rev += 1;
                        }
                    }
                }
            }
            Action::NetStop => {
                net.stop();
                lobby.note = None;
                lobby.rev += 1;
                setup.refresh += 1;
            }
            Action::NetMute(seat) => {
                if let Some(m) = net.muted.get_mut(seat as usize) {
                    *m = !*m;
                    let muted = *m;
                    if muted {
                        net.chat.retain(|c| c.0 != seat);
                    }
                }
                lobby.rev += 1;
                setup.refresh += 1;
            }
            _ => {}
        }
    }
}

fn join(net: &mut Net, lobby: &mut Lobby, addr: SocketAddr) {
    net.stop();
    match GuestLink::connect(addr, net.id) {
        Ok(g) => {
            net.role = Role::Guest(g);
            lobby.note = None;
        }
        Err(_) => lobby.note = Some(("Не получилось подключиться.", "Couldn't connect.")),
    }
    lobby.rev += 1;
}

/// Digits and dots typed on the lobby page go into the code / IP field.
fn type_ip(mut keys: EventReader<KeyboardInput>, mut lobby: ResMut<Lobby>) {
    for k in keys.read() {
        if !k.state.is_pressed() {
            continue;
        }
        match &k.logical_key {
            Key::Character(c) if c.chars().all(|ch| ch.is_ascii_digit() || ch == '.') && lobby.ip_text.len() < 15 => {
                lobby.ip_text.push_str(c);
                lobby.rev += 1;
            }
            Key::Backspace | Key::Delete => {
                lobby.ip_text.pop();
                lobby.rev += 1;
            }
            _ => {}
        }
    }
}

fn find_hosts(time: Res<Time>, mut lobby: ResMut<Lobby>, net: Res<Net>) {
    if net.active() {
        lobby.finder = None;
        return;
    }
    let dt = time.delta_secs();
    let lobby = &mut *lobby;
    let finder = lobby.finder.get_or_insert_with(Finder::new);
    if finder.poll(dt) {
        lobby.hosts = finder.found.clone();
        lobby.rev += 1;
    }
}

// ---- quick phrases (A9): keys 1–6 or the buttons under the chat

#[allow(clippy::too_many_arguments)]
fn say_actions(mut acts: EventReader<Act>, keys: Res<ButtonInput<KeyCode>>, mut net: ResMut<Net>, state: Res<bevy::state::state::State<AppState>>, busy: Res<UiBusy>, time: Res<Time>, mut wait: Local<f32>) {
    *wait -= time.delta_secs();
    let mut said = acts.read().filter_map(|Act(a)| if let Action::Say(i) = *a { Some(i) } else { None }).last();
    let in_game = matches!(*state.get(), AppState::Playing | AppState::Ship);
    if net.active() && in_game && !busy.0 {
        const KEYS: [KeyCode; 6] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6];
        if let Some(i) = KEYS.iter().position(|k| keys.just_pressed(*k)) {
            said = Some(i as u8);
        }
    }
    let Some(i) = said else { return };
    if *wait > 0.0 {
        return;
    }
    *wait = SAY_EVERY;
    match &mut net.role {
        Role::Host(h) => {
            h.say(0, i);
        }
        Role::Guest(g) => g.queue(Msg::Say(i)),
        Role::Off => {}
    }
}

// ---- host

#[allow(clippy::too_many_arguments)]
fn host_in(
    time: Res<Time>,
    mut net: ResMut<Net>,
    sim: Option<ResMut<Sim>>,
    camp: Option<ResMut<Camp>>,
    ship_ui: Option<Res<crate::ship::ShipUi>>,
    state: Res<bevy::state::state::State<AppState>>,
    mut ok: EventWriter<Happened>,
    mut setup: ResMut<Setup>,
) {
    let net = &mut *net;
    let Role::Host(h) = &mut net.role else {
        net.hold = false;
        return;
    };
    let st = *state.get();
    let before = (h.connected(), h.taken(), h.room);
    // where the room is: decides who may join and whether the game waits
    h.room = match st {
        AppState::Lobby => Room::Waiting,
        AppState::Playing => match sim.as_ref().map(|s| s.0.state().status.clone()) {
            Some(Status::Won) => Room::Completed,
            Some(Status::Lost(_)) => Room::Failed,
            Some(_) if h.missing() => Room::Paused,
            Some(_) => Room::Active,
            None => Room::Preparing,
        },
        AppState::Ship => match camp.as_ref().map(|c| c.0.state().phase) {
            Some(Phase::GameOver(_)) => Room::Failed,
            Some(Phase::Ship { .. }) if h.missing() => Room::Paused,
            Some(Phase::Ship { .. }) => Room::Active,
            _ => Room::Preparing,
        },
        _ => Room::Preparing,
    };
    h.poll(time.delta_secs());
    if before != (h.connected(), h.taken(), h.room) {
        setup.refresh += 1;
    }
    let hold = h.room == Room::Paused;
    let said = h.take_said();
    let input = h.take_input();
    let mut refusals = vec![];
    match (st, sim, camp) {
        (AppState::Playing, Some(mut sim), _) => {
            if hold && !net.hold {
                // someone dropped out: keep the base safe on disk while we wait
                crate::save::write(&sim);
            }
            let n = sim.0.state().players.len() as u8;
            for (seat, pos, msgs) in input {
                if seat >= n {
                    continue;
                }
                if let Some((x, y)) = pos {
                    sim.set_player_pos(seat, x, y);
                }
                for m in msgs {
                    if let Msg::Cmd(c) = m {
                        match sim.apply(seat, c) {
                            Ok(evs) => {
                                for e in evs {
                                    ok.write(Happened(e));
                                }
                            }
                            // the guest sees why; the host's screen stays calm
                            Err(r) => refusals.push((seat, r)),
                        }
                    }
                }
            }
        }
        (AppState::Ship, _, Some(mut camp)) => {
            if hold != net.hold {
                let user_paused = ship_ui.as_ref().is_some_and(|u| u.paused);
                camp.0.set_paused(hold || user_paused);
            }
            let n = camp.0.state().crew.len() as u8;
            for (seat, pos, msgs) in input {
                if seat >= n {
                    continue;
                }
                if let Some((x, y)) = pos {
                    camp.0.set_crew_pos(seat, x, y);
                }
                for m in msgs {
                    if let Msg::Ship(c) = m {
                        let _ = camp.0.apply(seat, c);
                    }
                }
            }
        }
        _ => {}
    }
    for (seat, r) in refusals {
        h.refuse(seat, r);
    }
    net.hold = hold;
    for s in said {
        net.hear(s);
    }
}

fn host_out(time: Res<Time>, mut net: ResMut<Net>, sim: Option<Res<Sim>>, camp: Option<Res<Camp>>, mut ev: EventReader<Happened>, state: Res<bevy::state::state::State<AppState>>) {
    let Role::Host(h) = &mut net.role else {
        ev.clear();
        return;
    };
    h.out_events.extend(ev.read().map(|Happened(e)| e.clone()));
    let dt = time.delta_secs();
    match (*state.get(), sim, camp) {
        (AppState::Playing, Some(sim), _) => {
            let st = sim.0.state();
            let cfg = MissionConfig { map: st.map, cadet: st.cadet, coop: true, live: st.live, seed: sim.1.seed() };
            h.send(dt, Some(cfg), st.players.len() as u8, Some(st), None);
        }
        (AppState::Ship, _, Some(camp)) => h.send(dt, None, 0, None, Some(camp.0.state())),
        _ => h.send(dt, None, 0, None, None),
    }
}

// ---- guest

#[allow(clippy::too_many_arguments)]
fn guest_in(
    mut commands: Commands,
    time: Res<Time>,
    mut net: ResMut<Net>,
    sim: Option<ResMut<Sim>>,
    camp: Option<ResMut<Camp>>,
    content: Res<ContentRes>,
    state: Res<bevy::state::state::State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut setup: ResMut<Setup>,
    mut lobby: ResMut<Lobby>,
    mut ok: EventWriter<Happened>,
    mut no: EventWriter<Refused>,
) {
    let net = &mut *net;
    let Role::Guest(g) = &mut net.role else { return };
    g.poll(time.delta_secs());
    let said: Vec<Said> = g.said.drain(..).collect();
    for s in said {
        net.hear(s);
    }
    let Role::Guest(g) = &mut net.role else { return };
    let st = *state.get();
    if g.lost() {
        let (heard, full) = (g.heard, g.full);
        net.stop();
        lobby.note = Some(if full {
            ("В этой комнате нет мест или игра уже идёт без тебя.", "This room is full, or the game is already on without you.")
        } else if heard {
            ("Связь с хостом потеряна.", "Lost the connection to the host.")
        } else {
            ("Хост не отвечает. Проверь код / IP и что компьютеры в одной Wi‑Fi.", "The host doesn't answer. Check the code / IP and that the computers are on the same Wi‑Fi.")
        });
        lobby.rev += 1;
        // leaving `Playing` drops the mission (save.rs), never remove it mid-frame
        if st != AppState::Lobby {
            next.set(AppState::Lobby);
        }
        return;
    }
    // the host is flying the ship: fly with them
    if let Some(ship) = g.ship.take() {
        match (st, camp) {
            (AppState::Ship, Some(mut camp)) => {
                for (_, c) in camp.0.take_outbox() {
                    g.queue(Msg::Ship(c));
                }
                camp.0.mirror_as(ship, g.seat);
            }
            (s, _) if s != AppState::Ship && s != AppState::Playing => {
                if let Some(c) = &content.0 {
                    let mut cp = Campaign::new(ship.cfg, c);
                    cp.mirror_as(ship, g.seat);
                    commands.insert_resource(Camp(cp));
                    setup.coop = true;
                    next.set(AppState::Ship);
                }
            }
            _ => {}
        }
        return;
    }
    // the host started a mission we are not in yet → join it
    let host_playing = g.state.as_ref().is_some_and(|s| s.status == Status::Playing);
    let playing = st == AppState::Playing;
    if let (Some(cfg), true) = (g.cfg, host_playing) {
        if !playing && net.joined != Some(cfg.seed) {
            net.joined = Some(cfg.seed);
            net.ended = false;
            setup.map = cfg.map;
            setup.coop = true;
            commands.insert_resource(NetStart(cfg, g.crew));
            next.set(AppState::Playing);
            return;
        }
    }
    if g.cfg.is_none() {
        lobby.rev += (!g.events.is_empty()) as u32;
    }
    let Some(mut sim) = sim.filter(|_| playing) else {
        g.events.clear();
        g.refusals.clear();
        return;
    };
    for (_, c) in sim.0.take_outbox() {
        g.queue(Msg::Cmd(c));
    }
    if let Some(s) = g.state.take() {
        sim.0.mirror(s);
    }
    for e in g.events.drain(..) {
        // the end of the mission is detected from the state below (packets can get lost)
        if !matches!(e, GameEvent::MissionEnded(_)) {
            ok.write(Happened(e));
        }
    }
    for r in g.refusals.drain(..) {
        no.write(Refused(r));
    }
    if sim.0.is_mirror() && sim.0.state().status != Status::Playing && !net.ended {
        net.ended = true;
        ok.write(Happened(GameEvent::MissionEnded(sim.0.result())));
    }
}

fn guest_out(time: Res<Time>, mut net: ResMut<Net>, players: Query<&Player>, camp: Option<Res<Camp>>, state: Res<bevy::state::state::State<AppState>>) {
    let Role::Guest(g) = &mut net.role else { return };
    let seat = g.seat;
    let on_ship = *state.get() == AppState::Ship;
    let pos = match camp.filter(|_| on_ship) {
        Some(c) => c.0.state().crew.get(seat as usize).map(|c| c.pos),
        None => players.iter().find(|p| p.idx == seat).map(|p| (p.pos.x, p.pos.y)),
    };
    g.send(time.delta_secs(), pos.unwrap_or(ja_core::game::DOOR));
}

fn goodbye(mut exit: EventReader<AppExit>, mut net: ResMut<Net>) {
    if exit.read().count() > 0 {
        net.stop();
    }
}

// ---- status badge (bottom left, every screen) and the phrases

pub fn seat_color(i: u8) -> Color {
    match i {
        0 => Color::srgb(1.0, 0.75, 0.3),
        1 => Color::srgb(0.5, 0.8, 1.0),
        2 => Color::srgb(0.55, 0.95, 0.55),
        _ => Color::srgb(1.0, 0.6, 0.85),
    }
}

fn spawn_badge(mut commands: Commands) {
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(12.0), bottom: Val::Px(90.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.16, 0.85)),
            BorderRadius::all(Val::Px(6.0)),
            GlobalZIndex(40),
            Visibility::Hidden,
            Badge,
        ))
        .with_children(|p| {
            p.spawn((Text::new(""), TextFont { font_size: 15.0, ..default() }, TextColor(C_TEXT), BadgeText));
        });
}

fn badge(net: Res<Net>, prof: Res<Prof>, art: Option<Res<Art>>, mut b: Query<&mut Visibility, With<Badge>>, mut t: Query<(&mut Text, &mut TextColor, &mut TextFont), With<BadgeText>>, mut last: Local<String>) {
    let l = prof.lang();
    let (s, c) = status_line(&net, l);
    if *last == s {
        return;
    }
    *last = s.clone();
    for mut v in &mut b {
        *v = if s.is_empty() { Visibility::Hidden } else { Visibility::Visible };
    }
    for (mut x, mut col, mut f) in &mut t {
        x.0 = s.clone();
        col.0 = c;
        if let Some(a) = &art {
            f.font = a.body.clone();
        }
    }
}

pub fn status_line(net: &Net, l: Lang) -> (String, Color) {
    let room = l.pick("Комната", "Room");
    match &net.role {
        Role::Off => (String::new(), C_TEXT),
        Role::Host(h) => {
            let c = if h.room == Room::Paused || h.connected() == 0 { C_WARN } else { C_GOOD };
            (format!("{room} {} · {} {}/{SEATS} · {}", h.code, l.pick("игроков", "players"), 1 + h.connected(), h.room.label(l)), c)
        }
        Role::Guest(g) if g.reconnecting() => (l.pick("Связь пропала — переподключаемся… Твоё место ждёт тебя.", "Connection lost — reconnecting… Your seat is kept.").to_string(), C_WARN),
        Role::Guest(g) if g.heard => (format!("{room} {} · {} {} · {}", g.code, l.pick("ты игрок", "you are player"), g.seat + 1, g.room.label(l)), C_GOOD),
        Role::Guest(g) => (format!("{} {}…", l.pick("Подключаемся к", "Connecting to"), g.host.ip()), C_WARN),
    }
}

fn player_name(l: Lang, seat: u8) -> String {
    format!("{} {}", l.pick("Игрок", "Player"), seat + 1)
}

/// Phrases on screen + the phrase buttons (network game, on the Moon and on the ship).
#[allow(clippy::too_many_arguments)]
fn chat_view(mut commands: Commands, time: Res<Time>, mut net: ResMut<Net>, art: Option<Res<Art>>, prof: Res<Prof>, state: Res<bevy::state::state::State<AppState>>, old: Query<Entity, With<ChatRoot>>, mut key: Local<String>) {
    let dt = time.delta_secs();
    for c in net.chat.iter_mut() {
        c.2 += dt;
    }
    net.chat.retain(|c| c.2 < 6.0);
    let show = net.active() && matches!(*state.get(), AppState::Playing | AppState::Ship);
    let l = prof.lang();
    let k = format!("{show}{:?}{:?}{:?}", net.chat.iter().map(|c| (c.0, c.1)).collect::<Vec<_>>(), net.muted, l);
    if *key == k && (old.is_empty() != show) {
        return;
    }
    *key = k;
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(art) = art.filter(|_| show) else { return };
    let me = net.local_idx().unwrap_or(0);
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(12.0), bottom: Val::Px(124.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
            GlobalZIndex(41),
            ChatRoot,
        ))
        .with_children(|p| {
            for &(seat, ph, _) in &net.chat {
                let (icon, ru, en) = PHRASES[ph as usize % PHRASES.len()];
                p.spawn((Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), align_items: AlignItems::Center, padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)), ..default() }, BackgroundColor(Color::srgba(0.05, 0.06, 0.16, 0.85)), BorderRadius::all(Val::Px(6.0)))).with_children(|r| {
                    kit::text(r, format!("{}:", player_name(l, seat)), &art.head, 15.0, seat_color(seat));
                    kit::icon(r, &art, icon, 22.0);
                    kit::text(r, l.pick(ru, en), &art.body, 16.0, C_TEXT);
                    if seat != me {
                        // mute this player (their phrases stop showing here)
                        r.spawn((Button, UiAction(Action::NetMute(seat)), Node { padding: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(Color::NONE))).with_children(|b| {
                            kit::icon(b, &art, "mute", 18.0);
                        });
                    }
                });
            }
            p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), ..default() }).with_children(|r| {
                for (i, (icon, ru, en)) in PHRASES.iter().enumerate() {
                    let bg = Color::srgba(0.05, 0.06, 0.16, 0.85);
                    r.spawn((Button, UiAction(Action::Say(i as u8)), kit::Base(bg), Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)), ..default() }, BackgroundColor(bg), BorderRadius::all(Val::Px(6.0))))
                        .with_children(|b| {
                            kit::icon(b, &art, icon, 22.0);
                            kit::text(b, format!("{} {}", i + 1, l.pick(ru, en)), &art.body, 10.0, C_MUTED);
                        });
                }
            });
        });
}

/// The lobby page (built by `menu::build_page`).
pub fn lobby_page(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, net: &Net, lobby: &Lobby) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(26.0)), row_gap: Val::Px(12.0), ..default() }).with_children(|c| {
        kit::text(c, l.pick("Игра по сети: до 4 компьютеров", "Network game: up to 4 computers"), &art.head, 32.0, C_TEXT);
        kit::wrap_text(c, l.pick("Все компьютеры должны быть в одной Wi‑Fi. Один создаёт комнату, остальные вводят её код. Если Windows спросит про сеть — нажми «Разрешить».", "All computers must be on the same Wi‑Fi. One creates a room, the others type its code. If Windows asks about the network, press “Allow”."), &art.body, 17.0, C_MUTED, 900.0);
        if let Role::Guest(g) = &net.role {
            let (s, col) = status_line(net, l);
            kit::text(c, s, &art.head, 22.0, col);
            if g.heard {
                roster(c, art, l, net, &g.present, g.seat);
                kit::text(c, l.pick("Ждём, когда хост начнёт экспедицию или выберет базу…", "Waiting for the host to start the expedition or pick a base…"), &art.body, 18.0, C_TEXT);
            }
            kit::button(c, art, Action::NetStop, 1, "cross", l.pick("Отключиться", "Disconnect"), false);
            return;
        }
        if let Role::Host(h) = &net.role {
            c.spawn(kit::row(14.0)).with_children(|r| {
                kit::text(r, l.pick("Код комнаты:", "Room code:"), &art.head, 26.0, C_TEXT);
                kit::text(r, h.code.to_string(), &art.head, 48.0, kit::C_ACCENT);
                kit::text(r, format!("{} {}", l.pick("или IP", "or IP"), local_ip()), &art.body, 17.0, C_MUTED);
            });
            let (s, col) = status_line(net, l);
            kit::text(c, s, &art.body, 18.0, col);
            roster(c, art, l, net, &h.present(), 0);
            c.spawn(kit::row(10.0)).with_children(|r| {
                kit::button(r, art, Action::Go(AppState::Expedition), 1, "rocket", l.pick("Экспедиция", "Expedition"), true);
                kit::button(r, art, Action::Go(AppState::Maps), 2, "map", l.pick("Выбрать базу", "Pick a base"), false);
                kit::button(r, art, Action::NetStop, 3, "cross", l.pick("Закрыть комнату", "Close the room"), false);
            });
            return;
        }
        kit::tile(c, art, Action::NetHost, 1, "satellite", l.pick("Создать комнату", "Create a room"), 260.0, Color::srgb(0.9, 0.42, 0.22));
        kit::text(c, l.pick("Или подключись к комнате друга:", "Or join a friend's room:"), &art.head, 22.0, C_TEXT);
        if lobby.hosts.is_empty() {
            kit::text(c, l.pick("Ищем комнаты в этой сети…", "Looking for rooms on this network…"), &art.body, 17.0, C_MUTED);
        }
        c.spawn(kit::row(10.0)).with_children(|r| {
            for (i, f) in lobby.hosts.iter().enumerate().take(4) {
                let label = format!("{} {} · {}/{SEATS} · {}", l.pick("Комната", "Room"), f.code, f.players, f.room.label(l));
                kit::button(r, art, Action::NetJoin(i as u8), 2 + i as u16, "coop", &label, !f.room.locked());
            }
        });
        c.spawn(kit::row(10.0)).with_children(|r| {
            kit::text(r, l.pick("Код комнаты или IP:", "Room code or IP:"), &art.body, 18.0, C_TEXT);
            let shown = if lobby.ip_text.is_empty() { l.pick("набери цифры…", "type digits…").to_string() } else { lobby.ip_text.clone() };
            r.spawn((Node { min_width: Val::Px(220.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)), BorderRadius::all(Val::Px(6.0)))).with_children(|f| {
                kit::text(f, shown, &art.head, 22.0, if lobby.ip_text.is_empty() { C_MUTED } else { C_TEXT });
            });
            kit::button(r, art, Action::NetJoinTyped, 10, "play", l.pick("Подключиться", "Join"), false);
        });
        if let Some((ru, en)) = lobby.note {
            kit::text(c, l.pick(ru, en), &art.body, 18.0, C_WARN);
        }
    });
}

/// Who is in the room; the others can be muted.
fn roster(c: &mut ChildSpawnerCommands, art: &Art, l: Lang, net: &Net, present: &[bool; SEATS], me: u8) {
    c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(8.0), ..default() }).with_children(|r| {
        for i in 0..SEATS as u8 {
            let here = present[i as usize];
            r.spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), padding: UiRect::all(Val::Px(10.0)), min_width: Val::Px(170.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, if here { 0.10 } else { 0.03 })), BorderRadius::all(Val::Px(8.0)))).with_children(|b| {
                kit::icon(b, art, if here { "astronaut" } else { "hourglass" }, 34.0);
                kit::text(b, player_name(l, i), &art.head, 17.0, if here { seat_color(i) } else { C_MUTED });
                let what = if i == me {
                    l.pick("это ты", "that's you")
                } else if i == 0 {
                    l.pick("хост", "host")
                } else if here {
                    l.pick("в игре", "joined")
                } else {
                    l.pick("свободно", "free")
                };
                kit::text(b, what, &art.body, 14.0, C_MUTED);
                if here && i != me {
                    let m = net.muted[i as usize];
                    kit::button(b, art, Action::NetMute(i), 20 + i as u16, if m { "mute" } else { "sound" }, if m { l.pick("заглушён", "muted") } else { l.pick("заглушить", "mute") }, false);
                }
            });
        }
    });
}

// ---- end-to-end check on one computer: JA_NET_TEST=host / JA_NET_TEST=guest
// (JA_NET_GUESTS=n: the host waits for n guests before starting; JA_NET_TAG: screenshot suffix)

#[allow(clippy::too_many_arguments)]
fn autotest(
    mut commands: Commands,
    time: Res<Time>,
    mut t: Local<f32>,
    mut step: Local<u32>,
    mut acts: EventWriter<Act>,
    mut net: ResMut<Net>,
    mut lobby: ResMut<Lobby>,
    mut setup: ResMut<Setup>,
    state: Res<bevy::state::state::State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    sim: Option<ResMut<Sim>>,
    mut players: Query<&mut Player>,
    mut exit: EventWriter<AppExit>,
) {
    let role = std::env::var("JA_NET_TEST").unwrap_or_default();
    let want: usize = std::env::var("JA_NET_GUESTS").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    *t += time.delta_secs();
    let shots = std::env::var_os("JA_SHOTS").map(std::path::PathBuf::from);
    let tag = std::env::var("JA_NET_TAG").unwrap_or_default();
    let shot = |commands: &mut Commands, name: &str| {
        if let Some(d) = &shots {
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(d.join(format!("{name}{tag}.png"))));
        }
    };
    let playing = *state.get() == AppState::Playing;
    if role == "host" {
        match *step {
            0 if *t > 1.0 => {
                next.set(AppState::Lobby);
                acts.write(Act(Action::NetHost));
                *step = 1;
            }
            1 if matches!(&net.role, Role::Host(h) if h.connected() >= want) => {
                *step = 2;
                *t = 0.0;
            }
            2 if *t > 1.5 => {
                shot(&mut commands, "net_room");
                setup.map = ja_core::MapId::Earth;
                next.set(AppState::Playing);
                *step = 3;
                *t = 0.0;
            }
            3 if playing && *t > 10.0 => {
                if let Some(sim) = &sim {
                    let st = sim.0.state();
                    let pos: Vec<_> = st.players.iter().map(|p| p.pos).collect();
                    info!("NET-TEST host: crew {}, positions {pos:?}, solar panels {}, modules {}, chat {}", st.players.len(), st.count(ja_core::ModuleKind::Solar), st.modules.len(), net.chat.len());
                }
                shot(&mut commands, "net_host");
                *step = 4;
                *t = 0.0;
            }
            4 if *t > 2.0 => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    } else {
        // no host answered: the lobby must explain it; take a picture and stop
        if *step == 1 && *state.get() == AppState::Lobby && *t > 14.0 && !net.active() {
            shot(&mut commands, "net_lobby");
            *step = 9;
            *t = 0.0;
        }
        if *step == 9 && *t > 1.5 {
            exit.write(AppExit::Success);
        }
        let seat = net.local_idx().unwrap_or(1);
        match *step {
            0 if *t > 2.0 => {
                next.set(AppState::Lobby);
                lobby.ip_text = "127.0.0.1".into();
                acts.write(Act(Action::NetJoinTyped));
                *step = 1;
            }
            1 if playing && sim.is_some() && *t > 2.0 => {
                *step = 2;
                *t = 0.0;
            }
            2 if *t > 2.0 => {
                // walk our astronaut a few cells, say something and ask the host to build a solar panel
                for mut p in &mut players {
                    if p.idx == seat {
                        p.pos += Vec2::new(3.0, -2.0 - seat as f32);
                    }
                }
                acts.write(Act(Action::Say(seat)));
                if let Some(mut sim) = sim {
                    if let Some((x, y)) = ja_core::Game::new(MissionConfig { map: sim.0.state().map, cadet: false, coop: true, live: false, seed: 1 }, &ja_core::Weather::archive()).find_free_spot(ja_core::ModuleKind::Solar) {
                        let _ = sim.apply(seat, Command::Build { kind: ja_core::ModuleKind::Solar, x: x + 3 + 3 * seat as i32, y: y - 3 });
                    }
                }
                *step = 3;
                *t = 0.0;
            }
            3 if *t > 3.0 => {
                if let Some(sim) = &sim {
                    let st = sim.0.state();
                    info!("NET-TEST guest {seat}: crew {}, solar panels {}, me at {:?}, chat {}", st.players.len(), st.count(ja_core::ModuleKind::Solar), st.players.get(seat as usize).map(|p| p.pos), net.chat.len());
                }
                shot(&mut commands, "net_guest");
                *step = 4;
                *t = 0.0;
            }
            4 if *t > 8.0 => {
                net.stop();
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ja_core::{Game, MapId, ModuleKind, Weather};

    fn wait() {
        std::thread::sleep(std::time::Duration::from_millis(15));
    }

    fn local(h: &HostLink) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), h.port)
    }

    /// Runs the handshake: every guest gets a seat (or a "full").
    fn shake(h: &mut HostLink, gs: &mut [&mut GuestLink]) {
        for _ in 0..30 {
            for g in gs.iter_mut() {
                g.send(1.0, (0.0, 0.0));
            }
            wait();
            h.poll(0.1);
            h.send(1.0, None, 0, None, None);
            wait();
            for g in gs.iter_mut() {
                g.poll(0.1);
            }
            if gs.iter().all(|g| g.heard || g.full) {
                break;
            }
        }
    }

    /// A guest's command travels to the host, runs there, and the new state comes back.
    #[test]
    fn guest_command_reaches_host_and_state_comes_back() {
        let mut h = HostLink::bind(0).expect("bind");
        let mut g = GuestLink::connect(local(&h), 11).unwrap();
        let cfg = MissionConfig { map: MapId::Moon, cadet: false, coop: true, live: false, seed: 5 };
        let mut game = Game::new(cfg, &Weather::archive());
        let mut mirror = Game::new(cfg, &Weather::archive());
        shake(&mut h, &mut [&mut g]);
        assert!(h.connected() == 1 && g.heard && g.seat == 1, "handshake");
        let (x, y) = game.find_free_spot(ModuleKind::Solar).unwrap();
        g.queue(Msg::Cmd(Command::Build { kind: ModuleKind::Solar, x, y }));
        let before = game.state().count(ModuleKind::Solar);
        for _ in 0..20 {
            g.send(1.0, (4.0, 5.0));
            wait();
            h.poll(0.1);
            for (seat, pos, msgs) in h.take_input() {
                if let Some((px, py)) = pos {
                    game.set_player_pos(seat, px, py);
                }
                for m in msgs {
                    if let Msg::Cmd(c) = m {
                        game.apply(seat, c).unwrap();
                    }
                }
            }
            game.tick(0.1);
            h.send(1.0, Some(cfg), 2, Some(game.state()), None);
            wait();
            g.poll(0.1);
            if let Some(s) = g.state.take() {
                mirror.mirror(s);
            }
            if mirror.state().count(ModuleKind::Solar) > before {
                break;
            }
        }
        assert_eq!(game.state().count(ModuleKind::Solar), before + 1, "applied exactly once on the host");
        assert_eq!(mirror.state().count(ModuleKind::Solar), before + 1, "the guest sees it");
        assert_eq!(mirror.state().players[1].pos, (4.0, 5.0));
        for _ in 0..5 {
            g.send(1.0, (4.0, 5.0));
            wait();
            h.poll(0.1);
            assert!(h.take_input().iter().all(|x| x.2.is_empty()), "re-sent packets don't build twice");
        }
    }

    /// Three friends get seats 1–3, a fourth is told "full"; a dropped guest gets its seat back.
    #[test]
    fn seats_room_and_reconnect() {
        let mut h = HostLink::bind(0).expect("bind");
        assert!((1000..=9999).contains(&h.code));
        let addr = local(&h);
        let (mut a, mut b, mut c, mut d) = (GuestLink::connect(addr, 1).unwrap(), GuestLink::connect(addr, 2).unwrap(), GuestLink::connect(addr, 3).unwrap(), GuestLink::connect(addr, 4).unwrap());
        shake(&mut h, &mut [&mut a, &mut b, &mut c]);
        let mut seats = vec![a.seat, b.seat, c.seat];
        seats.sort();
        assert_eq!(seats, vec![1, 2, 3]);
        assert_eq!(h.connected(), 3);
        shake(&mut h, &mut [&mut d]);
        assert!(d.full && d.lost(), "no fifth seat");
        // the mission starts; b's Wi‑Fi drops for a while
        h.room = Room::Active;
        let b_seat = b.seat;
        for _ in 0..60 {
            a.send(0.1, (1.0, 1.0));
            c.send(0.1, (1.0, 1.0));
            wait();
            h.poll(0.1);
        }
        assert!(h.missing() && h.connected() == 2, "b is missing, the seat is kept");
        let mut e = GuestLink::connect(addr, 99).unwrap();
        shake(&mut h, &mut [&mut e]);
        assert!(e.full, "a stranger can't take the seat mid-mission");
        let mut b2 = GuestLink::connect(addr, 2).unwrap();
        shake(&mut h, &mut [&mut b2]);
        assert!(b2.heard && b2.seat == b_seat && !h.missing(), "b is back on its seat");
    }

    /// A phrase goes guest → host → everyone, once, and the host shows it too.
    #[test]
    fn phrases_are_relayed_once() {
        let mut h = HostLink::bind(0).expect("bind");
        let addr = local(&h);
        let (mut a, mut b) = (GuestLink::connect(addr, 1).unwrap(), GuestLink::connect(addr, 2).unwrap());
        shake(&mut h, &mut [&mut a, &mut b]);
        a.queue(Msg::Say(1));
        a.queue(Msg::Say(2)); // too soon after the first: dropped
        for _ in 0..10 {
            a.send(1.0, (0.0, 0.0));
            b.send(1.0, (0.0, 0.0));
            wait();
            h.poll(0.1);
            h.send(1.0, None, 0, None, None);
            wait();
            a.poll(0.1);
            b.poll(0.1);
        }
        let host_saw = h.take_said();
        assert_eq!(host_saw.len(), 1);
        assert_eq!((host_saw[0].seat, host_saw[0].phrase), (a.seat, 1));
        assert_eq!(b.said.len(), 1, "shown once despite re-sent packets");
        assert_eq!(a.said.len(), 1);
    }

    #[test]
    fn packets_fit_in_one_datagram() {
        let cfg = MissionConfig { map: MapId::Europa, cadet: false, coop: true, live: false, seed: 9 };
        let mut game = Game::with_mode(cfg, ja_core::Mode::Sandbox, &Weather::archive());
        for _ in 0..4000 {
            game.tick(0.5);
        }
        let d = Down { proto: PROTO, cfg: Some(cfg), crew: 4, state: Some(game.state().clone()), ..Default::default() };
        let p = Packet::Down(Box::new(d));
        assert!(encode(&p).len() < 65_000, "{} bytes", encode(&p).len());
        let c = ja_core::content::load(&ja_core::content::default_dir()).expect("content");
        let mut camp = Campaign::new(ja_core::ExpeditionConfig::new(4, ja_core::Level::Advanced), &c);
        for _ in 0..2000 {
            camp.tick(0.25);
        }
        let d = Down { proto: PROTO, ship: Some(camp.state().clone()), ..Default::default() };
        assert!(encode(&Packet::Down(Box::new(d))).len() < 65_000);
    }
}
