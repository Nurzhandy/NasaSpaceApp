//! Part C — co-op over the local network (two laptops on the same Wi‑Fi), plain UDP, no extra crates.
//!
//! The **host** runs the rules (`ja_core::Game`) exactly like single-screen co-op; its keyboard
//! drives astronaut 0. The **guest** drives astronaut 1: it sends its position and numbered
//! `Command`s (re-sent until the host acknowledges them), and shows the host's `State`, which the
//! host broadcasts ~15 times per second together with the `GameEvent`s. The guest's `Game` is a
//! mirror (`Game::mirror`): it never simulates, so the two computers can't drift apart.
//!
//! Finding a game: the host announces itself every second on UDP port `BEACON_PORT`; the guest
//! lists the announcements. Typing the host's IP works too (some school Wi‑Fi blocks broadcasts).

use crate::art::Art;
use crate::kit::{self, Act, Action, C_GOOD, C_MUTED, C_TEXT, C_WARN};
use crate::player::Player;
use crate::world::{Happened, Refused};
use crate::{AppState, Prof, Set, Setup, Sim};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use ja_core::{Command, GameEvent, Lang, MissionConfig, Refusal, State, Status};
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

pub const PORT: u16 = 7777;
pub const BEACON_PORT: u16 = 7778;
const PROTO: u32 = 1;
const SEND_EVERY: f32 = 1.0 / 15.0;
/// no packets for this long = the other computer is gone
const TIMEOUT: f32 = 6.0;
const MAX_PACKET: usize = 65_000;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Packet {
    /// host → everyone on the LAN, once a second
    Beacon { proto: u32 },
    /// guest → host until the first answer
    Hello { proto: u32 },
    /// guest → host: where astronaut 1 stands + commands not yet acknowledged
    Up { pos: (f32, f32), cmds: Vec<(u32, Command)> },
    /// host → guest: the mission (if any), the last applied guest command, the state and news
    Down { proto: u32, cfg: Option<MissionConfig>, ack: u32, state: Option<State>, events: Vec<GameEvent>, refusals: Vec<Refusal> },
    /// polite goodbye
    Bye,
}

fn encode(p: &Packet) -> Vec<u8> {
    let mut b = serde_json::to_vec(p).unwrap_or_default();
    if b.len() > MAX_PACKET {
        // a very long sandbox mission: drop the per-sol history rather than the packet
        if let Packet::Down { proto, cfg, ack, state: Some(s), events, refusals } = p {
            let mut s = s.clone();
            s.history.clear();
            b = serde_json::to_vec(&Packet::Down { proto: *proto, cfg: *cfg, ack: *ack, state: Some(s), events: events.clone(), refusals: refusals.clone() }).unwrap_or_default();
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

// ---------------------------------------------------------------- host

pub struct HostLink {
    sock: UdpSocket,
    beacon: Option<UdpSocket>,
    beacon_t: f32,
    /// the UDP port actually bound (tests bind port 0)
    #[allow(dead_code)]
    pub port: u16,
    pub guest: Option<SocketAddr>,
    silence: f32,
    applied: u32,
    pos: Option<(f32, f32)>,
    new_cmds: Vec<Command>,
    send_t: f32,
    pub out_events: Vec<GameEvent>,
    pub out_refusals: Vec<Refusal>,
}

impl HostLink {
    pub fn bind(port: u16) -> std::io::Result<Self> {
        let sock = UdpSocket::bind(("0.0.0.0", port))?;
        sock.set_nonblocking(true)?;
        let port = sock.local_addr()?.port();
        let beacon = UdpSocket::bind("0.0.0.0:0").ok().filter(|b| b.set_broadcast(true).is_ok() && b.set_nonblocking(true).is_ok());
        Ok(Self { sock, beacon, beacon_t: 0.0, port, guest: None, silence: 0.0, applied: 0, pos: None, new_cmds: vec![], send_t: 0.0, out_events: vec![], out_refusals: vec![] })
    }
    pub fn connected(&self) -> bool {
        self.guest.is_some()
    }
    /// Receive what the guest sent; tell the LAN we exist.
    pub fn poll(&mut self, dt: f32) {
        self.silence += dt;
        if self.guest.is_some() && self.silence > TIMEOUT {
            self.guest = None;
        }
        for (p, from) in drain(&self.sock) {
            match p {
                Packet::Hello { proto } if proto == PROTO => {
                    if self.guest.is_none() || self.guest == Some(from) {
                        if self.guest.is_none() {
                            // a new guest starts counting its commands from 1
                            self.applied = 0;
                        }
                        self.guest = Some(from);
                        self.silence = 0.0;
                        self.send_t = SEND_EVERY; // answer right away
                    }
                }
                Packet::Up { pos, cmds } if self.guest == Some(from) => {
                    self.silence = 0.0;
                    self.pos = Some(pos);
                    let mut cmds = cmds;
                    cmds.sort_by_key(|c| c.0);
                    for (seq, c) in cmds {
                        if seq == self.applied + 1 {
                            self.applied = seq;
                            self.new_cmds.push(c);
                        }
                    }
                }
                Packet::Bye if self.guest == Some(from) => self.guest = None,
                _ => {}
            }
        }
        self.beacon_t -= dt;
        if self.beacon_t <= 0.0 {
            self.beacon_t = 1.0;
            if let Some(b) = &self.beacon {
                let _ = b.send_to(&encode(&Packet::Beacon { proto: PROTO }), (Ipv4Addr::BROADCAST, BEACON_PORT));
            }
        }
    }
    /// The guest's latest position and the commands to run for astronaut 1, in order.
    pub fn take_input(&mut self) -> (Option<(f32, f32)>, Vec<Command>) {
        (self.pos.take(), std::mem::take(&mut self.new_cmds))
    }
    /// Send the current mission state to the guest (rate-limited).
    pub fn send(&mut self, dt: f32, cfg: Option<MissionConfig>, state: Option<&State>) {
        self.send_t += dt;
        let Some(to) = self.guest else {
            self.out_events.clear();
            self.out_refusals.clear();
            return;
        };
        if self.send_t < SEND_EVERY {
            return;
        }
        self.send_t = 0.0;
        let p = Packet::Down { proto: PROTO, cfg, ack: self.applied, state: state.cloned(), events: std::mem::take(&mut self.out_events), refusals: std::mem::take(&mut self.out_refusals) };
        let _ = self.sock.send_to(&encode(&p), to);
    }
    pub fn bye(&self) {
        if let Some(to) = self.guest {
            let _ = self.sock.send_to(&encode(&Packet::Bye), to);
        }
    }
}

// ---------------------------------------------------------------- guest

pub struct GuestLink {
    sock: UdpSocket,
    pub host: SocketAddr,
    next: u32,
    unacked: Vec<(u32, Command)>,
    silence: f32,
    pub heard: bool,
    send_t: f32,
    pub cfg: Option<MissionConfig>,
    pub state: Option<State>,
    pub events: Vec<GameEvent>,
    pub refusals: Vec<Refusal>,
}

impl GuestLink {
    pub fn connect(host: SocketAddr) -> std::io::Result<Self> {
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.set_nonblocking(true)?;
        Ok(Self { sock, host, next: 1, unacked: vec![], silence: 0.0, heard: false, send_t: 1.0, cfg: None, state: None, events: vec![], refusals: vec![] })
    }
    /// The host stopped answering (or never answered).
    pub fn lost(&self) -> bool {
        self.silence > if self.heard { TIMEOUT } else { 8.0 }
    }
    pub fn queue(&mut self, cmd: Command) {
        self.unacked.push((self.next, cmd));
        self.next += 1;
    }
    pub fn poll(&mut self, dt: f32) {
        self.silence += dt;
        for (p, from) in drain(&self.sock) {
            if from.ip() != self.host.ip() {
                continue;
            }
            if let Packet::Down { proto, cfg, ack, state, events, refusals } = p {
                if proto != PROTO {
                    continue;
                }
                self.heard = true;
                self.silence = 0.0;
                self.unacked.retain(|(s, _)| *s > ack);
                if cfg.map(|c| c.seed) != self.cfg.map(|c| c.seed) {
                    // a new mission: its commands start from 1 on the host side too
                    self.next = ack + 1 + self.unacked.len() as u32;
                }
                self.cfg = cfg;
                if state.is_some() {
                    self.state = state;
                }
                self.events.extend(events);
                self.refusals.extend(refusals);
            } else if let Packet::Bye = p {
                self.silence = TIMEOUT + 1.0;
                self.heard = true;
            }
        }
    }
    /// Tell the host where astronaut 1 is and what it wants to do (rate-limited, commands re-sent until acknowledged).
    pub fn send(&mut self, dt: f32, pos: (f32, f32)) {
        self.send_t += dt;
        if self.send_t < SEND_EVERY && self.unacked.is_empty() {
            return;
        }
        if self.send_t < SEND_EVERY / 2.0 {
            return;
        }
        self.send_t = 0.0;
        let p = if self.heard { Packet::Up { pos, cmds: self.unacked.clone() } } else { Packet::Hello { proto: PROTO } };
        let _ = self.sock.send_to(&encode(&p), self.host);
    }
    pub fn bye(&self) {
        let _ = self.sock.send_to(&encode(&Packet::Bye), self.host);
    }
}

/// Listens for hosts announcing themselves on the LAN.
pub struct Finder {
    sock: Option<UdpSocket>,
    pub found: Vec<(IpAddr, f32)>,
}

impl Finder {
    pub fn new() -> Self {
        let sock = UdpSocket::bind(("0.0.0.0", BEACON_PORT)).ok().filter(|s| s.set_nonblocking(true).is_ok());
        Self { sock, found: vec![] }
    }
    /// Returns true when the list changed.
    pub fn poll(&mut self, dt: f32) -> bool {
        let before: Vec<IpAddr> = self.found.iter().map(|f| f.0).collect();
        for f in &mut self.found {
            f.1 += dt;
        }
        self.found.retain(|f| f.1 < 4.0);
        if let Some(s) = &self.sock {
            for (p, from) in drain(s) {
                if p == (Packet::Beacon { proto: PROTO }) {
                    match self.found.iter_mut().find(|f| f.0 == from.ip()) {
                        Some(f) => f.1 = 0.0,
                        None => self.found.push((from.ip(), 0.0)),
                    }
                }
            }
        }
        before != self.found.iter().map(|f| f.0).collect::<Vec<_>>()
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

#[derive(Resource, Default)]
pub struct Net {
    pub role: Role,
    /// seed of the host mission the guest is playing (or played last)
    joined: Option<u64>,
    ended: bool,
}

impl Net {
    pub fn active(&self) -> bool {
        !matches!(self.role, Role::Off)
    }
    /// The astronaut controlled by the other computer.
    pub fn remote_idx(&self) -> Option<u8> {
        match self.role {
            Role::Off => None,
            Role::Host(_) => Some(1),
            Role::Guest(_) => Some(0),
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
    }
}

/// Lobby page state: found hosts, the typed IP, the last message.
#[derive(Resource, Default)]
pub struct Lobby {
    finder: Option<Finder>,
    pub hosts: Vec<IpAddr>,
    pub ip_text: String,
    pub note: Option<(&'static str, &'static str)>,
    /// bump to rebuild the lobby page
    pub rev: u32,
}

/// The host's mission the guest should start (read by `main::start_mission`).
#[derive(Resource)]
pub struct NetStart(pub MissionConfig);

#[derive(Component)]
struct Badge;
#[derive(Component)]
struct BadgeText;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Net>()
            .init_resource::<Lobby>()
            .add_systems(Startup, spawn_badge)
            .add_systems(Update, (lobby_actions, type_ip.run_if(in_state(AppState::Lobby)), find_hosts.run_if(in_state(AppState::Lobby))).after(Set::Input))
            .add_systems(Update, (host_in, guest_in).in_set(Set::Logic).before(crate::world::TickRules))
            .add_systems(Update, (host_out, guest_out, badge).in_set(Set::View))
            .add_systems(Update, autotest.run_if(|| std::env::var_os("JA_NET_TEST").is_some()))
            .add_systems(Last, goodbye);
    }
}

fn lobby_actions(mut acts: EventReader<Act>, mut net: ResMut<Net>, mut lobby: ResMut<Lobby>, mut setup: ResMut<Setup>, mut next: ResMut<NextState<AppState>>) {
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
                        next.set(AppState::Maps);
                    }
                    Err(_) => lobby.note = Some(("Не получилось открыть порт 7777. Может, игра уже запущена?", "Couldn't open port 7777. Is the game already running?")),
                }
                lobby.rev += 1;
            }
            Action::NetJoin(i) => {
                if let Some(ip) = lobby.hosts.get(i as usize).copied() {
                    join(&mut net, &mut lobby, SocketAddr::new(ip, PORT));
                }
            }
            Action::NetJoinTyped => match lobby.ip_text.trim().parse::<IpAddr>() {
                Ok(ip) => join(&mut net, &mut lobby, SocketAddr::new(ip, PORT)),
                Err(_) => {
                    lobby.note = Some(("Это не похоже на IP-адрес. Пример: 192.168.1.5", "That doesn't look like an IP address. Example: 192.168.1.5"));
                    lobby.rev += 1;
                }
            },
            Action::NetStop => {
                net.stop();
                lobby.note = None;
                lobby.rev += 1;
                setup.refresh += 1;
            }
            _ => {}
        }
    }
}

fn join(net: &mut Net, lobby: &mut Lobby, addr: SocketAddr) {
    net.stop();
    match GuestLink::connect(addr) {
        Ok(g) => {
            net.role = Role::Guest(g);
            lobby.note = None;
        }
        Err(_) => lobby.note = Some(("Не получилось подключиться.", "Couldn't connect.")),
    }
    lobby.rev += 1;
}

/// Digits and dots typed on the lobby page go into the IP field.
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
        lobby.hosts = finder.found.iter().map(|f| f.0).collect();
        lobby.rev += 1;
    }
}

// ---- host

fn host_in(time: Res<Time>, mut net: ResMut<Net>, sim: Option<ResMut<Sim>>, mut ok: EventWriter<Happened>, mut setup: ResMut<Setup>) {
    let Role::Host(h) = &mut net.role else { return };
    let was = h.connected();
    h.poll(time.delta_secs());
    if was != h.connected() {
        setup.refresh += 1;
    }
    let Some(mut sim) = sim else {
        h.take_input();
        return;
    };
    let (pos, cmds) = h.take_input();
    if let Some((x, y)) = pos {
        sim.set_player_pos(1, x, y);
    }
    for c in cmds {
        match sim.apply(1, c) {
            Ok(evs) => {
                for e in evs {
                    ok.write(Happened(e));
                }
            }
            // the guest sees why; the host's screen stays calm
            Err(r) => h.out_refusals.push(r),
        }
    }
}

fn host_out(time: Res<Time>, mut net: ResMut<Net>, sim: Option<Res<Sim>>, mut ev: EventReader<Happened>, state: Res<bevy::state::state::State<AppState>>) {
    let Role::Host(h) = &mut net.role else {
        ev.clear();
        return;
    };
    h.out_events.extend(ev.read().map(|Happened(e)| e.clone()));
    let playing = *state.get() == AppState::Playing;
    match sim.filter(|_| playing) {
        Some(sim) => {
            let st = sim.0.state();
            let cfg = MissionConfig { map: st.map, cadet: st.cadet, coop: true, live: st.live, seed: sim.1.seed() };
            h.send(time.delta_secs(), Some(cfg), Some(st));
        }
        None => h.send(time.delta_secs(), None, None),
    }
}

// ---- guest

#[allow(clippy::too_many_arguments)]
fn guest_in(
    mut commands: Commands,
    time: Res<Time>,
    mut net: ResMut<Net>,
    sim: Option<ResMut<Sim>>,
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
    let playing = *state.get() == AppState::Playing;
    if g.lost() {
        let heard = g.heard;
        net.stop();
        lobby.note = Some(if heard { ("Связь с хостом потеряна.", "Lost the connection to the host.") } else { ("Хост не отвечает. Проверь IP и что оба компьютера в одной Wi‑Fi.", "The host doesn't answer. Check the IP and that both computers are on the same Wi‑Fi.") });
        lobby.rev += 1;
        // leaving `Playing` drops the mission (save.rs), never remove it mid-frame
        next.set(AppState::Lobby);
        return;
    }
    // the host started a mission we are not in yet → join it
    let host_playing = g.state.as_ref().is_some_and(|s| s.status == Status::Playing);
    if let (Some(cfg), true) = (g.cfg, host_playing) {
        if !playing && net.joined != Some(cfg.seed) {
            net.joined = Some(cfg.seed);
            net.ended = false;
            setup.map = cfg.map;
            setup.coop = true;
            commands.insert_resource(NetStart(cfg));
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
        g.queue(c);
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

fn guest_out(time: Res<Time>, mut net: ResMut<Net>, players: Query<&Player>) {
    let Role::Guest(g) = &mut net.role else { return };
    let pos = players.iter().find(|p| p.idx == 1).map(|p| (p.pos.x, p.pos.y)).unwrap_or(ja_core::game::DOOR);
    g.send(time.delta_secs(), pos);
}

fn goodbye(mut exit: EventReader<AppExit>, mut net: ResMut<Net>) {
    if exit.read().count() > 0 {
        net.stop();
    }
}

// ---- status badge (bottom left, every screen)

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
    match &net.role {
        Role::Off => (String::new(), C_TEXT),
        Role::Host(h) if h.connected() => (format!("{} {}", l.pick("Сеть: второй игрок подключён ·", "Network: player 2 connected ·"), local_ip()), C_GOOD),
        Role::Host(_) => (format!("{} {}", l.pick("Сеть: ждём второго игрока · твой IP", "Network: waiting for player 2 · your IP"), local_ip()), C_WARN),
        Role::Guest(g) if g.heard => (format!("{} {}", l.pick("Сеть: подключено к", "Network: connected to"), g.host.ip()), C_GOOD),
        Role::Guest(g) => (format!("{} {}…", l.pick("Сеть: подключаемся к", "Network: connecting to"), g.host.ip()), C_WARN),
    }
}

/// The lobby page (built by `menu::build_page`).
pub fn lobby_page(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, net: &Net, lobby: &Lobby) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(26.0)), row_gap: Val::Px(14.0), ..default() }).with_children(|c| {
        kit::text(c, l.pick("Игра по сети: два компьютера", "Network game: two computers"), &art.head, 32.0, C_TEXT);
        kit::wrap_text(c, l.pick("Оба компьютера должны быть в одной Wi‑Fi. Один создаёт игру и выбирает базу, второй подключается и играет вторым астронавтом. Если Windows спросит про сеть — нажми «Разрешить».", "Both computers must be on the same Wi‑Fi. One creates the game and picks the base, the other joins as the second astronaut. If Windows asks about the network, press “Allow”."), &art.body, 18.0, C_MUTED, 900.0);
        if let Role::Guest(g) = &net.role {
            let (s, col) = status_line(net, l);
            kit::text(c, s, &art.head, 22.0, col);
            if g.heard {
                kit::text(c, l.pick("Ждём, когда хост запустит миссию…", "Waiting for the host to launch a mission…"), &art.body, 18.0, C_TEXT);
            }
            kit::button(c, art, Action::NetStop, 1, "cross", l.pick("Отключиться", "Disconnect"), false);
            return;
        }
        if let Role::Host(_) = &net.role {
            let (s, col) = status_line(net, l);
            kit::text(c, s, &art.head, 22.0, col);
            c.spawn(kit::row(10.0)).with_children(|r| {
                kit::button(r, art, Action::Go(AppState::Maps), 1, "map", l.pick("Выбрать базу", "Pick a base"), true);
                kit::button(r, art, Action::NetStop, 2, "cross", l.pick("Закрыть игру", "Close the game"), false);
            });
            return;
        }
        kit::tile(c, art, Action::NetHost, 1, "satellite", l.pick("Создать игру", "Create a game"), 260.0, Color::srgb(0.9, 0.42, 0.22));
        kit::text(c, l.pick("Или подключись к игре друга:", "Or join a friend's game:"), &art.head, 22.0, C_TEXT);
        if lobby.hosts.is_empty() {
            kit::text(c, l.pick("Ищем игры в этой сети…", "Looking for games on this network…"), &art.body, 17.0, C_MUTED);
        }
        c.spawn(kit::row(10.0)).with_children(|r| {
            for (i, ip) in lobby.hosts.iter().enumerate().take(4) {
                kit::button(r, art, Action::NetJoin(i as u8), 2 + i as u16, "coop", &format!("{} {ip}", l.pick("Игра на", "Game at")), true);
            }
        });
        c.spawn(kit::row(10.0)).with_children(|r| {
            kit::text(r, l.pick("IP хоста:", "Host IP:"), &art.body, 18.0, C_TEXT);
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

// ---- end-to-end check on one computer: JA_NET_TEST=host / JA_NET_TEST=guest

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
    *t += time.delta_secs();
    let shots = std::env::var_os("JA_SHOTS").map(std::path::PathBuf::from);
    let shot = |commands: &mut Commands, name: &str| {
        if let Some(d) = &shots {
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(d.join(format!("{name}.png"))));
        }
    };
    let playing = *state.get() == AppState::Playing;
    if role == "host" {
        match *step {
            0 if *t > 1.0 => {
                acts.write(Act(Action::NetHost));
                *step = 1;
            }
            1 if matches!(&net.role, Role::Host(h) if h.connected()) => {
                setup.map = ja_core::MapId::Earth;
                next.set(AppState::Playing);
                *step = 2;
                *t = 0.0;
            }
            2 if playing && *t > 8.0 => {
                if let Some(sim) = &sim {
                    let st = sim.0.state();
                    info!("NET-TEST host: guest at {:?}, solar panels {}, modules {}", st.players[1].pos, st.count(ja_core::ModuleKind::Solar), st.modules.len());
                }
                shot(&mut commands, "net_host");
                *step = 3;
                *t = 0.0;
            }
            3 if *t > 2.0 => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    } else {
        // no host answered: the lobby must explain it; take a picture and stop
        if *step == 1 && *state.get() == AppState::Lobby && *t > 12.0 {
            shot(&mut commands, "net_lobby");
            *step = 9;
            *t = 0.0;
        }
        if *step == 9 && *t > 1.5 {
            exit.write(AppExit::Success);
        }
        match *step {
            0 if *t > 2.0 => {
                lobby.ip_text = "127.0.0.1".into();
                acts.write(Act(Action::NetJoinTyped));
                *step = 1;
            }
            1 if playing && sim.is_some() && *t > 2.0 => {
                *step = 2;
                *t = 0.0;
            }
            2 if *t > 2.0 => {
                // walk astronaut 1 a few cells and ask the host to build a solar panel
                for mut p in &mut players {
                    if p.idx == 1 {
                        p.pos += Vec2::new(3.0, -2.0);
                    }
                }
                if let Some(mut sim) = sim {
                    let before = sim.0.state().count(ja_core::ModuleKind::Solar);
                    info!("NET-TEST guest: mirror={} solar before {before}", sim.0.is_mirror());
                    if let Some((x, y)) = ja_core::Game::new(MissionConfig { map: sim.0.state().map, cadet: false, coop: true, live: false, seed: 1 }, &ja_core::Weather::archive()).find_free_spot(ja_core::ModuleKind::Solar) {
                        let _ = sim.apply(1, Command::Build { kind: ja_core::ModuleKind::Solar, x: x + 6, y: y - 3 });
                    }
                }
                *step = 3;
                *t = 0.0;
            }
            3 if *t > 3.0 => {
                if let Some(sim) = &sim {
                    let st = sim.0.state();
                    info!("NET-TEST guest: solar panels now {}, me at {:?}, host at {:?}, sol {} t {:.3}", st.count(ja_core::ModuleKind::Solar), st.players[1].pos, st.players[0].pos, st.sol, st.t);
                }
                shot(&mut commands, "net_guest");
                *step = 4;
                *t = 0.0;
            }
            4 if *t > 6.0 => {
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

    fn pump(host: &mut HostLink, guest: &mut GuestLink) {
        std::thread::sleep(std::time::Duration::from_millis(15));
        host.poll(0.1);
        guest.poll(0.1);
    }

    /// A guest's command travels to the host, runs there, and the new state comes back.
    #[test]
    fn guest_command_reaches_host_and_state_comes_back() {
        let mut h = HostLink::bind(0).expect("bind");
        let mut g = GuestLink::connect(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), h.port)).unwrap();
        let cfg = MissionConfig { map: MapId::Moon, cadet: false, coop: true, live: false, seed: 5 };
        let mut game = Game::new(cfg, &Weather::archive());
        let mut mirror = Game::new(cfg, &Weather::archive());
        // handshake
        for _ in 0..20 {
            g.send(1.0, (0.0, 0.0));
            pump(&mut h, &mut g);
            h.send(1.0, Some(cfg), Some(game.state()));
            pump(&mut h, &mut g);
            if g.heard {
                break;
            }
        }
        assert!(h.connected() && g.heard, "handshake");
        // the guest builds; the command is sent until acknowledged
        let (x, y) = game.find_free_spot(ModuleKind::Solar).unwrap();
        g.queue(Command::Build { kind: ModuleKind::Solar, x, y });
        let before = game.state().count(ModuleKind::Solar);
        for _ in 0..20 {
            g.send(1.0, (4.0, 5.0));
            pump(&mut h, &mut g);
            let (pos, cmds) = h.take_input();
            if let Some((px, py)) = pos {
                game.set_player_pos(1, px, py);
            }
            for c in cmds {
                game.apply(1, c).unwrap();
            }
            game.tick(0.1);
            h.send(1.0, Some(cfg), Some(game.state()));
            pump(&mut h, &mut g);
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
        // re-sent packets don't build twice
        for _ in 0..5 {
            g.send(1.0, (4.0, 5.0));
            pump(&mut h, &mut g);
            assert!(h.take_input().1.is_empty());
        }
    }

    #[test]
    fn packets_fit_in_one_datagram() {
        let cfg = MissionConfig { map: MapId::Europa, cadet: false, coop: true, live: false, seed: 9 };
        let mut game = Game::with_mode(cfg, ja_core::Mode::Sandbox, &Weather::archive());
        for _ in 0..4000 {
            game.tick(0.5);
        }
        let p = Packet::Down { proto: PROTO, cfg: Some(cfg), ack: 0, state: Some(game.state().clone()), events: vec![], refusals: vec![] };
        assert!(encode(&p).len() < 65_000, "{} bytes", encode(&p).len());
    }
}
