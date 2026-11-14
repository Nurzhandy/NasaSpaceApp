/* ============================================================
   Multiplayer: "Mission Control & Crew" (co-op)
   - Mission Control (ЦУП) sees the forecast, real DONKI data and the
     manual, but can't touch the controls.
   - Crew controls power, but has no forecast.
   - They talk ONLY with preset phrase chips (child-safe), and every
     message arrives with the map's communication delay.
   Modes: online (2 devices, WebRTC via PeerJS) or one shared device.
   ============================================================ */
(function () {
  const t = (...a) => JA.t(...a);
  const L = (o) => JA.L(o);
  const tt = JA.tt;
  const esc = JA.esc;
  const app = () => document.getElementById('app');
  const bind = (...a) => JA.bind(...a);
  const PEER_PREFIX = 'ja-junior-astro-2026-';
  const PEERJS_URL = 'https://unpkg.com/peerjs@1.5.4/dist/peerjs.min.js';

  const CHIPS = {
    mcc: [
      ['flare', tt('⚠️ Завтра возможна вспышка!', '⚠️ A flare may hit tomorrow!')],
      ['dust', tt('🌪️ Идёт пылевая буря', '🌪️ A dust storm is coming')],
      ['night', tt('🌑 Скоро ночь — копите энергию', '🌑 Night is coming — save power')],
      ['shield', tt('🛡️ Поднимите щит (2–3 блока)', '🛡️ Raise the shield (2–3 blocks)')],
      ['shelter', tt('🏠 Спрячьтесь в убежище', '🏠 Go to the shelter')],
      ['o2', tt('🫁 Больше энергии на кислород', '🫁 More power to oxygen')],
      ['food', tt('🌱 Больше энергии оранжерее', '🌱 More power to the greenhouse')],
      ['save', tt('🔋 Сохраните энергию в батарее', '🔋 Save power in the battery')],
      ['sci', tt('🔬 Можно заняться наукой', '🔬 You can do science')],
      ['calm', tt('✅ Завтра всё спокойно', '✅ Tomorrow looks calm')],
      ['good', tt('👍 Отличная работа, экипаж!', '👍 Great job, crew!')],
      ['beacon', tt('🌟 Сделайте паузу, подышите', '🌟 Take a pause, breathe')],
    ],
    crew: [
      ['ok', tt('✅ Принято!', '✅ Copy that!')],
      ['repeat', tt('🔁 Повторяю ваш совет', '🔁 Repeating your advice')],
      ['again', tt('❓ Повторите, пожалуйста', '❓ Say again, please')],
      ['help', tt('🆘 Нужен совет!', '🆘 We need advice!')],
      ['long', tt('⏳ Сколько это продлится?', '⏳ How long will it last?')],
      ['done', tt('👍 Сделано', '👍 Done')],
    ],
  };
  const chipText = (from, id) => { const c = CHIPS[from].find((x) => x[0] === id); return c ? L(c[1]) : '…'; };

  let room = null;

  /* ============================================================
     LOBBY
     ============================================================ */
  JA.screens.multi = function () {
    const maps = JA.MAPS.filter(JA.unlocked);
    app().innerHTML = `${JA.UI.back()}
      <h1>👥 ${t('Мультиплеер: ЦУП и экипаж', 'Multiplayer: Mission Control & Crew')}</h1>
      <div class="roles">
        <div class="card role"><span class="big-ico">📡</span><h3>${t('Центр управления (ЦУП)', 'Mission Control')}</h3>
          <p>${t('Видит прогноз космической погоды, реальные данные NASA DONKI и справочник. Не может нажимать кнопки базы — только советовать.', 'Sees the space weather forecast, real NASA DONKI data and the manual. Can’t press base controls — only advise.')}</p></div>
        <div class="card role"><span class="big-ico">🧑‍🚀</span><h3>${t('Экипаж', 'Crew')}</h3>
          <p>${t('Управляет энергией базы и решает, что делать с событиями. Не видит прогноз — нужно слушать ЦУП.', 'Controls base power and decides on events. Can’t see the forecast — must listen to Mission Control.')}</p></div>
      </div>
      <p class="muted">${t('Общение — только готовыми фразами, и они приходят с задержкой связи, как в настоящем космосе. Тренируйте «скажи — повтори — подтверди»!', 'You talk only with ready-made phrases, and they arrive with a comms delay like in real space. Practise “say – repeat – confirm”!')}</p>
      <div class="card">
        <label class="field"><span>🪐 ${t('Карта', 'Map')}</span>
          <select id="m-map">${maps.map((m) => `<option value="${m.id}">${m.icon} ${L(m.name)} — ${t('задержка', 'delay')} ${L(m.delayReal)}</option>`).join('')}</select></label>
        <div class="mode-row">
          <button class="mode-card multi" id="m-local"><span class="mode-ico">📱</span><b>${t('На одном устройстве', 'One shared device')}</b><small>${t('Передавайте планшет друг другу. Отлично для класса!', 'Pass the tablet back and forth. Great for classrooms!')}</small></button>
          <button class="mode-card solo" id="m-host"><span class="mode-ico">🌐</span><b>${t('Онлайн: создать комнату', 'Online: create room')}</b><small>${t('Друг подключится по коду', 'A friend joins with a code')}</small></button>
        </div>
        <div class="join"><label class="field"><span>🔑 ${t('Есть код комнаты?', 'Have a room code?')}</span>
          <div class="row"><input id="m-code" maxlength="4" placeholder="ABCD" autocomplete="off" style="text-transform:uppercase"><button class="btn primary" id="m-join">${t('Подключиться', 'Join')}</button></div></label></div>
        <p class="tiny">${t('Онлайн-режим соединяет два браузера напрямую (WebRTC). Мы не собираем имена и не храним переписку.', 'Online mode connects two browsers directly (WebRTC). We don’t collect names or store chats.')}</p>
      </div>`;
    JA.UI.bindCommon();
    JA.$('#m-local').onclick = () => startLocal(JA.$('#m-map').value);
    JA.$('#m-host').onclick = () => host(JA.$('#m-map').value);
    JA.$('#m-join').onclick = () => { const c = JA.$('#m-code').value.trim().toUpperCase(); if (/^[A-Z]{4}$/.test(c)) join(c); else JA.toast(t('Код — 4 буквы', 'The code is 4 letters')); };
  };

  /* ============================================================
     ROOM (shared state; host or local device is authoritative)
     ============================================================ */
  function newRoom(mode, mapId, myRole) {
    const m = JA.Sim.mapById(mapId);
    if (!m || !JA.unlocked(m)) mapId = 'earth';
    const s = JA.Sim.create(mapId, JA.profile.age, { live: JA.weather.live && mapId !== 'earth' });
    JA.Sim.startSol(s);
    return {
      mode, myRole, conn: null, peer: null,
      st: { s, phase: 'brief', report: null, confirms: 0, round: 1 },
      alloc: { ls: 0, green: 0, shield: 0, sci: 0 },
      inbox: [],          // messages I can read: {from, id, at}
      outbox: [],         // local mode: queued for the other role
      sent: [],           // my sent messages (for status)
      lastHeard: null,
      localTurn: 'mcc',   // local mode: whose turn on the device
      handoff: true,
    };
  }

  const isAuthority = () => room.mode === 'local' || room.mode === 'host';
  const delayMs = () => Math.min(JA.Sim.mapById(room.st.s.mapId).delayMs, 10000);
  const viewRole = () => (room.mode === 'local' ? room.localTurn : room.myRole);

  /* Apply an action (authority only) */
  function apply(act) {
    const st = room.st, s = st.s;
    if (act.a === 'toPlan' && st.phase === 'brief') st.phase = 'plan';
    else if (act.a === 'feel' && s.today) {
      const mood = JA.MOODS[s.today.data.mood];
      s.feelingPicked = act.id;
      const f = JA.FEELINGS.find((x) => x.id === act.id);
      s.feelingNote = mood.ok.includes(act.id) ? t(`Да, похоже на ${L(f.name)}. Как поддержать?`, `Yes, looks like ${L(f.name)}. How can you help?`)
        : t(`Может быть! Подсказки говорят и о: ${mood.ok.map((id) => L(JA.FEELINGS.find((x) => x.id === id).name)).join(', ')}.`, `Maybe! Clues also suggest: ${mood.ok.map((id) => L(JA.FEELINGS.find((x) => x.id === id).name)).join(', ')}.`);
    }
    else if (act.a === 'choose' && st.phase === 'plan') JA.Sim.choose(s, act.i);
    else if (act.a === 'run' && st.phase === 'plan') {
      const a = act.alloc;
      if (a.ls + a.green + a.shield + a.sci > JA.Sim.available(s)) return;
      st.report = JA.Sim.resolve(s, a);
      st.phase = s.status === 'playing' ? 'report' : 'end';
    }
    else if (act.a === 'next' && st.phase === 'report') { st.phase = 'brief'; st.report = null; }
    else if (act.a === 'confirm') st.confirms++;
    broadcast();
  }

  function dispatch(act) {
    if (isAuthority()) { apply(act); afterState(); }
    else send({ t: 'act', act });
  }

  function send(msg) { if (room.conn && room.conn.open) room.conn.send(msg); }
  function broadcast() { if (room.mode === 'host') send({ t: 'state', st: room.st }); }

  function afterState() {
    const st = room.st;
    if (room.mode === 'local') {
      // decide who holds the device
      const want = st.phase === 'brief' ? 'mcc' : st.phase === 'plan' ? 'crew' : room.localTurn;
      if (want !== room.localTurn) { room.localTurn = want; room.handoff = true; }
    }
    if (st.phase === 'plan' && room.lastPhase !== 'plan') room.alloc = { ls: 0, green: 0, shield: 0, sci: 0 };
    if (st.phase === 'end' && room.lastPhase !== 'end') finishMulti();
    room.lastPhase = st.phase;
    draw();
  }

  /* ---------- chips with comms delay ---------- */
  function sendChip(id) {
    const from = viewRole();
    let text = null;
    if (from === 'crew' && id === 'repeat') {
      if (!room.lastHeard) return JA.toast(t('Пока нечего повторять', 'Nothing to repeat yet'));
      text = chipText('mcc', room.lastHeard);
    }
    if (from === 'crew' && (id === 'ok' || id === 'repeat') && room.lastHeard && !room.confirmedThisSol) {
      room.confirmedThisSol = true; dispatch({ a: 'confirm' });
      JA.toast(t('📡 Сообщение подтверждено — так работает связь экипажей!', '📡 Message confirmed — that’s how crews talk!'));
    }
    const msg = { from, id, text };
    room.sent.push({ ...msg, at: Date.now() });
    JA.sfx('radio');
    if (room.mode === 'local') room.outbox.push(msg);
    else send({ t: 'chip', msg: { from, id } });
    draw();
  }

  function receiveChip(msg) {
    const at = Date.now() + delayMs();
    room.inbox.push({ ...msg, at });
    draw();
    setTimeout(() => { if (room) { JA.sfx('radio'); if (msg.from === 'mcc') room.lastHeard = msg.id; draw(); } }, delayMs() + 50);
  }

  /* ============================================================
     LOCAL (one device)
     ============================================================ */
  function startLocal(mapId) {
    room = newRoom('local', mapId, 'both');
    JA.cleanup = () => { room = null; };
    afterState();
  }

  /* ============================================================
     ONLINE (PeerJS / WebRTC)
     ============================================================ */
  function loadPeer() {
    return new Promise((res, rej) => {
      if (window.Peer) return res();
      const sc = document.createElement('script');
      sc.src = PEERJS_URL; sc.onload = res; sc.onerror = () => rej(new Error('peerjs'));
      document.head.appendChild(sc);
    });
  }
  const genCode = () => Array.from({ length: 4 }, () => 'ABCDEFGHJKLMNPQRSTUVWXYZ'[Math.floor(Math.random() * 24)]).join('');

  function waiting(html) {
    app().innerHTML = `${JA.UI.back('multi')}<section class="card center">${html}</section>`;
    JA.UI.bindCommon();
  }

  async function host(mapId, tries = 0) {
    waiting(`<p>${t('Создаём комнату…', 'Creating a room…')}</p>`);
    try { await loadPeer(); } catch (e) { return offline(); }
    const code = genCode();
    room = newRoom('host', mapId, 'mcc');
    const r = room;                       // handlers only act on THIS room (ignore stale connections)
    const live = () => room === r;
    const peer = new window.Peer(PEER_PREFIX + code, { debug: 0 });
    room.peer = peer;
    JA.cleanup = () => { if (live()) room = null; try { peer.destroy(); } catch (e) { /* ignore */ } };
    peer.on('open', () => {
      if (!live()) return;
      waiting(`<span class="big-ico">🔑</span><h2>${t('Код комнаты', 'Room code')}</h2><p class="code">${code}</p>
        <p>${t('Скажи код другу. Он выберет «Подключиться» на своём устройстве.', 'Tell your friend the code. They choose “Join” on their device.')}</p>
        <div class="field"><span>${t('Твоя роль', 'Your role')}</span><div class="seg">
          <button class="seg-btn on" data-r="mcc">📡 ${t('ЦУП', 'Mission Control')}</button><button class="seg-btn" data-r="crew">🧑‍🚀 ${t('Экипаж', 'Crew')}</button></div></div>
        <p class="muted"><span class="spinner"></span> ${t('Ждём второго игрока…', 'Waiting for player 2…')}</p>`);
      bind('[data-r]', 'click', (e, el) => { if (!live() || room.roleLocked) return; room.myRole = el.dataset.r; JA.$$('[data-r]').forEach((b) => b.classList.toggle('on', b === el)); });
    });
    peer.on('connection', (conn) => {
      if (!live() || room.conn) { conn.close(); return; }
      room.conn = conn;
      conn.on('open', () => {
        if (!live()) return;
        room.roleLocked = true;
        send({ t: 'hello', role: room.myRole === 'mcc' ? 'crew' : 'mcc', lang: JA.profile.lang });
        broadcast(); afterState();
      });
      conn.on('data', (m) => { if (live()) onData(m); });
      conn.on('close', () => { if (live()) lost(); });
    });
    peer.on('error', (err) => {
      if (!live()) return;
      if (err.type === 'unavailable-id' && tries < 3) { room = null; peer.destroy(); return host(mapId, tries + 1); }
      offline(err.type);
    });
  }

  async function join(code) {
    waiting(`<p><span class="spinner"></span> ${t('Подключаемся к комнате', 'Joining room')} ${esc(code)}…</p>`);
    try { await loadPeer(); } catch (e) { return offline(); }
    room = { mode: 'guest', myRole: 'crew', st: null, alloc: { ls: 0, green: 0, shield: 0, sci: 0 }, inbox: [], sent: [], outbox: [], lastHeard: null };
    const r = room;
    const live = () => room === r;
    const peer = new window.Peer({ debug: 0 });
    room.peer = peer;
    JA.cleanup = () => { if (live()) room = null; try { peer.destroy(); } catch (e) { /* ignore */ } };
    peer.on('open', () => {
      if (!live()) return;
      const conn = peer.connect(PEER_PREFIX + code, { reliable: true });
      room.conn = conn;
      conn.on('data', (m) => { if (live()) onData(m); });
      conn.on('close', () => { if (live()) lost(); });
      setTimeout(() => { if (live() && !room.st) offline('timeout'); }, 12000);
    });
    peer.on('error', (err) => { if (live()) offline(err.type); });
  }

  function onData(msg) {
    if (!room || !msg || typeof msg !== 'object') return;
    if (msg.t === 'hello' && room.mode === 'guest') { room.myRole = msg.role === 'mcc' ? 'mcc' : 'crew'; }
    else if (msg.t === 'state' && room.mode === 'guest') {
      const prevSol = room.st && room.st.s.sol;
      room.st = msg.st;
      if (prevSol !== room.st.s.sol) room.confirmedThisSol = false;
      afterState();
    }
    else if (msg.t === 'act' && room.mode === 'host') { apply(msg.act); afterState(); }
    else if (msg.t === 'chip' && msg.msg && CHIPS[msg.msg.from] && CHIPS[msg.msg.from].some((c) => c[0] === msg.msg.id)) {
      // only accept known chip ids; free text is never sent or rendered
      let text = null;
      if (msg.msg.from === 'crew' && msg.msg.id === 'repeat') {
        const mine = room.sent.filter((m) => m.from === 'mcc');
        if (mine.length) text = chipText('mcc', mine[mine.length - 1].id);
      }
      receiveChip({ from: msg.msg.from, id: msg.msg.id, text });
    }
  }

  function lost() {
    if (!room) return;
    if (room.st && room.st.phase === 'end') return;
    JA.modal(`<h2>📴 ${t('Связь потеряна', 'Connection lost')}</h2><p>${t('Второй игрок отключился. Так бывает и в настоящем космосе! Попробуйте снова.', 'The other player disconnected. It happens in real space too! Try again.')}</p>
      <button class="btn primary" data-close>OK</button>`, { onClose: () => JA.go('multi') });
  }

  function offline(type) {
    if (room && room.peer) { try { room.peer.destroy(); } catch (e) { /* ignore */ } }
    room = null;
    waiting(`<span class="big-ico">🛰️</span><h2>${type === 'peer-unavailable' ? t('Комната не найдена', 'Room not found') : t('Онлайн-связь недоступна', 'Online link unavailable')}</h2>
      <p>${type === 'peer-unavailable' ? t('Проверь код и попробуй ещё раз.', 'Check the code and try again.') : t('Похоже, нет интернета или сеть блокирует соединение. Играйте вдвоём на одном устройстве — это тоже мультиплеер!', 'Looks like there’s no internet or the network blocks the link. Play together on one device — that’s multiplayer too!')}</p>
      <button class="btn primary" data-go="multi">← ${t('К мультиплееру', 'Back to multiplayer')}</button>`);
  }

  /* ============================================================
     DRAW
     ============================================================ */
  function draw() {
    if (!room || !room.st) return;
    const st = room.st, s = st.s;
    const role = viewRole();
    const map = JA.Sim.mapById(s.mapId);

    // local mode: hand-off screen between players
    if (room.mode === 'local' && room.handoff && st.phase !== 'end' && st.phase !== 'report') {
      const who = role === 'mcc' ? `📡 ${t('ЦУП', 'Mission Control')}` : `🧑‍🚀 ${t('Экипаж', 'Crew')}`;
      const pending = room.outbox.length;
      app().innerHTML = `<section class="card center handoff">
        <span class="big-ico">📱➡️</span>
        <h1>${t('Передай устройство', 'Pass the device to')}: ${who}</h1>
        <p>${t('Сол', 'Sol')} ${s.sol}/${s.sols} · ${map.icon} ${L(map.name)}</p>
        ${pending && delayMs() ? `<p class="sending">📡 ${t(`Сигнал (${pending} сообщ.) летит ${L(map.delayReal)}…`, `Signal (${pending} msg) travelling ${L(map.delayReal)}…`)}</p>` : ''}
        <button class="btn big primary" id="ready">${t('Я готов(а)', 'I’m ready')} ▶</button></section>`;
      JA.$('#ready').onclick = () => {
        JA.sfx('click');
        room.handoff = false;
        const msgs = room.outbox.splice(0);
        msgs.forEach((m) => receiveChip(m));
        if (role === 'crew') room.confirmedThisSol = false;
        draw();
      };
      return;
    }

    if (st.phase === 'end') return drawEnd();

    const now = Date.now();
    const incoming = room.inbox.filter((m) => m.from !== role);
    const arrived = incoming.filter((m) => m.at <= now).slice(-6);
    const inFlight = incoming.filter((m) => m.at > now).length;
    const chips = CHIPS[role].map(([id, lab]) => `<button class="chip" data-chip="${id}">${L(lab)}</button>`).join('');
    const msgPanel = `<div class="comms">
      <div class="comms-head"><b>📻 ${t('Связь', 'Comms')}</b><small>${t('задержка', 'delay')}: ${L(map.delayReal)}</small></div>
      <ul class="msgs">${arrived.map((m) => `<li class="${m.from}">${m.from === 'mcc' ? '📡' : '🧑‍🚀'} ${esc(chipText(m.from, m.id))}${m.text ? ` — «${esc(m.text)}»` : ''}</li>`).join('') || `<li class="muted">${t('Пока тихо…', 'Quiet so far…')}</li>`}
        ${inFlight ? `<li class="sending">📡 ${t('Сигнал в пути…', 'Signal on its way…')}</li>` : ''}</ul>
      <div class="chips">${chips}</div></div>`;

    const header = `<div class="game-top">${JA.UI.back('multi', t('Выйти', 'Exit'))}
      <h2>${role === 'mcc' ? '📡 ' + t('Ты — ЦУП', 'You are Mission Control') : '🧑‍🚀 ' + t('Ты — экипаж', 'You are the Crew')} · ${map.icon}</h2>
      <span class="pill">${room.mode === 'local' ? '📱 ' + t('одно устройство', 'one device') : '🌐 online'} · 🤝 ${st.confirms}</span></div>`;

    let body = '';
    if (st.phase === 'report') {
      body = `<div class="card">${JA.UI.reportHtml(st.report)}
        <button class="btn primary big" id="next">${t('Следующий сол', 'Next sol')} ▶</button></div>${msgPanel}`;
    } else if (role === 'mcc') {
      const fc = JA.UI.forecastText(s);
      const w = JA.weather;
      body = `<div class="game-grid">
        <div class="col-left">
          <div class="card radar">
            <h3>🛰️ ${t('Радар космической погоды', 'Space weather radar')}</h3>
            <p class="forecast">${fc ? '⚠️ ' + esc(fc) : (s.effects.some((e) => e.id === 'silence' && e.until >= s.sol) ? t('…помехи от Солнца, прогноза нет', '…solar interference, no forecast') : '✅ ' + t('Завтра всё спокойно.', 'Tomorrow looks calm.'))}</p>
            <p class="tiny">${w.live ? '<span class="live-dot">LIVE · NASA DONKI</span>' : `<span class="archive-dot">${t('АРХИВ · NASA DONKI', 'ARCHIVE · NASA DONKI')}</span>`}</p>
            <ul class="ev-list">${w.events.slice(0, 3).map((e) => `<li><span class="tag ${e.type}">${e.type}</span> ${esc(L(e.label))} <small>${e.date}</small></li>`).join('')}</ul>
          </div>
          <div class="card manual"><h3>📘 ${t('Справочник ЦУП', 'Mission Control manual')}</h3><ul>
            <li>🫁 ${t('Экипаж тратит 12% кислорода за сол. 1 блок = +6%.', 'Crew uses 12% oxygen per sol. 1 block = +6%.')}</li>
            ${s.age === 'cadet' ? '' : `<li>🥔 ${t('Еда: −10% за сол. 1 блок оранжереи = +5%.', 'Food: −10% per sol. 1 greenhouse block = +5%.')}</li>`}
            <li>🛡️ ${t('1 блок щита = −30% дозы (максимум 3).', '1 shield block = −30% dose (max 3).')}</li>
            <li>☢️ ${t('Радиация здесь', 'Radiation here')}: ${L(map.doseReal)}</li>
            <li>📡 ${t('Твои сообщения идут', 'Your messages take')} ${L(map.delayReal)}</li></ul></div>
        </div>
        <div class="col-right">
          ${JA.UI.hud(s)}
          ${s.today ? JA.UI.eventCard(s, false) : ''}
          ${st.phase === 'plan' ? `<p class="note">🧑‍🚀 ${t('Экипаж распределяет энергию… Ты можешь продолжать отправлять советы.', 'The crew is splitting power… You can keep sending advice.')}</p>` : ''}
          ${msgPanel}
          ${st.phase === 'brief' ? `<button class="btn primary big" id="toPlan">${t('Сеанс связи окончен → ход экипажа', 'End of comms → crew’s turn')} ▶</button>` : ''}
        </div></div>`;
    } else {
      body = `${JA.UI.hud(s)}
        <div class="game-grid">
          <div class="col-left">${JA.UI.scene(s, room.alloc)}${msgPanel}</div>
          <div class="col-right">
            ${st.phase === 'brief' ? `<div class="card center"><p><span class="spinner"></span> ${t('ЦУП изучает прогноз и готовит советы…', 'Mission Control is studying the forecast…')}</p></div>`
              : `${JA.UI.eventCard(s, true)}${JA.UI.alloc(s, room.alloc)}
              <div class="actions"><button class="btn primary big" id="run" ${s.today && s.choice == null ? 'disabled' : ''}>${t('Выполнить сол', 'Run the sol')} ▶</button></div>`}
          </div></div>`;
    }
    app().innerHTML = header + body;
    JA.UI.bindCommon();
    bind('[data-chip]', 'click', (e, el) => sendChip(el.dataset.chip));
    bind('[data-inc]', 'click', (e, el) => { room.alloc[el.dataset.inc]++; draw(); });
    bind('[data-dec]', 'click', (e, el) => { room.alloc[el.dataset.dec] = Math.max(0, room.alloc[el.dataset.dec] - 1); draw(); });
    bind('[data-choice]', 'click', (e, el) => dispatch({ a: 'choose', i: +el.dataset.choice }));
    bind('[data-feel]', 'click', (e, el) => dispatch({ a: 'feel', id: el.dataset.feel }));
    const tp = JA.$('#toPlan'); if (tp) tp.onclick = () => dispatch({ a: 'toPlan' });
    const rn = JA.$('#run'); if (rn) rn.onclick = () => { room.confirmedThisSol = false; dispatch({ a: 'run', alloc: { ...room.alloc } }); };
    const nx = JA.$('#next'); if (nx) nx.onclick = () => dispatch({ a: 'next' });
  }

  /* ---------- end of co-op mission ---------- */
  function finishMulti() {
    const s = room.st.s;
    const won = s.status === 'won';
    let pts = JA.Sim.pointsFor(s);
    if (room.st.confirms >= 2) { pts += 10; JA.award('comms'); }
    room.pts = pts;
    const p = JA.profile;
    p.log.unshift({ date: new Date().toISOString().slice(0, 10), map: s.mapId, won, stars: JA.Sim.starsFor(s), sol: s.sol, sols: s.sols, science: s.science, dose: Math.round(s.dose), mode: 'multi' });
    p.log = p.log.slice(0, 30);
    JA.save();
    JA.addPoints(pts, t('за командную миссию', 'for the team mission'));
    JA.award('first');
  }

  function drawEnd() {
    const s = room.st.s, won = s.status === 'won', stars = JA.Sim.starsFor(s);
    app().innerHTML = `<section class="card debrief ${won ? 'won' : 'lost'}">
      <div class="big-ico">${won ? '🏆' : '🛟'}</div>
      <h1>${won ? t('Командная миссия выполнена!', 'Team mission complete!') : t('Экипаж безопасно вернулся домой', 'The crew returned home safely')}</h1>
      <p class="stars-big">${'★'.repeat(stars)}${'☆'.repeat(3 - stars)}</p>
      <p>🤝 ${t('Подтверждённых сообщений', 'Confirmed messages')}: <b>${room.st.confirms}</b> ${room.st.confirms >= 2 ? '— ' + t('бонус +10 за связь!', '+10 comms bonus!') : ''}</p>
      <p class="pts">+${room.pts || 0} ⭐</p>
      ${won ? '' : `<div class="why"><p>${esc(JA.Sim.failReason(s))}</p></div>`}
      ${JA.UI.chart(s)}
      <div class="reflect"><h3>💭 ${t('Обсудите вместе', 'Talk it over')}</h3>
        <ul class="tips"><li>${t('Какое сообщение помогло больше всего?', 'Which message helped most?')}</li>
        <li>${t('Было ли что-то непонятно? Как можно было уточнить?', 'Was anything unclear? How could you clarify?')}</li>
        <li>${t('Каково было ждать ответа из-за задержки?', 'How did it feel to wait because of the delay?')}</li></ul></div>
      <div class="row center">
        ${room.mode === 'local' ? `<button class="btn primary big" id="swap">🔄 ${t('Поменяться ролями', 'Swap roles')}</button>` : ''}
        <button class="btn ghost" data-go="multi">👥 ${t('В лобби', 'To lobby')}</button></div>
    </section>`;
    JA.UI.bindCommon();
    const sw = JA.$('#swap'); if (sw) sw.onclick = () => startLocal(s.mapId);
  }

})();
