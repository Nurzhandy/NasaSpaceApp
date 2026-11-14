/* ============================================================
   UI: router, shared components, home, single-player outpost
   ============================================================ */
(function () {
  const t = (...a) => JA.t(...a);
  const L = (o) => JA.L(o);
  const esc = JA.esc;
  const app = () => document.getElementById('app');

  JA.UI = {};
  JA.screens = {};
  let current = 'home', currentArg = null;

  /* ---------- router ---------- */
  JA.go = function (name, arg) {
    if (JA.cleanup) { try { JA.cleanup(); } catch (e) { /* ignore */ } JA.cleanup = null; }
    if ('speechSynthesis' in window) speechSynthesis.cancel();
    current = name; currentArg = arg;
    const fn = JA.screens[name] || JA.screens.home;
    fn(arg);
    window.scrollTo({ top: 0, behavior: 'instant' in window ? 'instant' : 'auto' });
    JA.UI.topbar();
  };
  JA.rerender = () => JA.go(current, currentArg);

  function bind(sel, ev, fn, root = document) { JA.$$(sel, root).forEach((el) => el.addEventListener(ev, (e) => { JA.sfx('click'); fn(e, el); })); }
  JA.bind = bind;

  /* ---------- top bar ---------- */
  JA.UI.topbar = function () {
    const p = JA.profile;
    const bar = document.getElementById('topbar');
    bar.innerHTML = `
      <button class="brand" data-go="home" aria-label="${t('На главную', 'Home')}"><span class="logo">🚀</span><span class="brand-text">${t('Юный астронавт', 'Junior Astronaut')}</span></button>
      <div class="top-right">
        <span class="pill" title="${t('Всего очков: открывают новые карты', 'Total points: unlock maps')}">🏅 <b>${p.total}</b></span>
        <span class="pill gold" title="${t('Звёзды: тратятся в магазине', 'Stars: spend in the shop')}">⭐ <b>${p.stars}</b></span>
        <button class="icon-btn" id="tb-lang" aria-label="Language">${p.lang === 'ru' ? 'EN' : 'RU'}</button>
        <button class="icon-btn" id="tb-sound" aria-label="${t('Звук', 'Sound')}">${p.settings.sound ? '🔊' : '🔇'}</button>
        <button class="icon-btn" id="tb-voice" aria-label="${t('Озвучка', 'Read aloud')}" title="${t('Озвучка текста', 'Read text aloud')}" aria-pressed="${p.settings.voice}">${p.settings.voice ? '🗣️' : '💬'}</button>
      </div>`;
    bind('[data-go]', 'click', (e, el) => JA.go(el.dataset.go), bar);
    bar.querySelector('#tb-lang').onclick = () => { p.lang = p.lang === 'ru' ? 'en' : 'ru'; document.documentElement.lang = p.lang; JA.save(); JA.rerender(); };
    bar.querySelector('#tb-sound').onclick = () => { p.settings.sound = !p.settings.sound; JA.save(); JA.UI.topbar(); };
    bar.querySelector('#tb-voice').onclick = () => { p.settings.voice = !p.settings.voice; JA.save(); JA.UI.topbar(); if (p.settings.voice) JA.speak(t('Озвучка включена', 'Read aloud is on')); };
  };

  JA.UI.back = (to = 'home', label) => `<button class="back" data-go="${to}">← ${label || t('Назад', 'Back')}</button>`;
  JA.UI.say = (text) => `<button class="say" data-say="${esc(text)}" aria-label="${t('Прочитать вслух', 'Read aloud')}">🔊</button>`;
  JA.UI.bindCommon = function (root = app()) {
    bind('[data-go]', 'click', (e, el) => JA.go(el.dataset.go, el.dataset.arg), root);
    bind('[data-say]', 'click', (e, el) => JA.speak(el.dataset.say, true), root);
  };

  /* ---------- modal ---------- */
  JA.modal = function (html, opts = {}) {
    const wrap = document.createElement('div');
    wrap.className = 'modal-wrap';
    wrap.innerHTML = `<div class="modal ${opts.cls || ''}" role="dialog" aria-modal="true">${html}</div>`;
    document.body.appendChild(wrap);
    const close = () => { wrap.remove(); if (opts.onClose) opts.onClose(); };
    wrap.addEventListener('click', (e) => { if (e.target === wrap && opts.dismiss !== false) close(); });
    JA.$$('[data-close]', wrap).forEach((b) => b.addEventListener('click', () => { JA.sfx('click'); close(); }));
    JA.$$('[data-say]', wrap).forEach((b) => b.addEventListener('click', () => JA.speak(b.dataset.say, true)));
    const f = wrap.querySelector('button, [tabindex]'); if (f) f.focus();
    return { el: wrap, close };
  };

  /* ============================================================
     WELCOME (first run)
     ============================================================ */
  JA.screens.welcome = function () {
    const p = JA.profile;
    app().innerHTML = `
      <section class="hero card">
        <div class="hero-av">${JA.avatar(p.wear, 140)}</div>
        <h1>${t('Добро пожаловать в отряд юных астронавтов!', 'Welcome to the Junior Astronaut Corps!')}</h1>
        <p class="lead">${t('Ты будешь управлять базой на Луне и Марсе, делить энергию между кислородом, едой, щитом и наукой — как настоящие планировщики миссий NASA.', 'You’ll run a base on the Moon and Mars and split power between oxygen, food, shields and science — just like real NASA mission planners.')}</p>
        <label class="field"><span>${t('Твой позывной (не настоящее имя)', 'Your call sign (not your real name)')}</span>
          <input id="w-name" maxlength="16" placeholder="${t('Например: Комета', 'e.g. Comet')}" value="${esc(p.name)}"></label>
        <div class="field"><span>${t('Выбери режим', 'Choose a mode')}</span>
          <div class="seg">
            <button class="seg-btn ${p.age === 'cadet' ? 'on' : ''}" data-age="cadet">🐣 ${t('Кадет', 'Cadet')} <small>7–9</small><em>${t('3 ресурса, больше подсказок', '3 resources, more hints')}</em></button>
            <button class="seg-btn ${p.age === 'pilot' ? 'on' : ''}" data-age="pilot">🦅 ${t('Пилот', 'Pilot')} <small>10–12</small><em>${t('4 ресурса, неточные прогнозы', '4 resources, uncertain forecasts')}</em></button>
          </div></div>
        <button class="btn big primary" id="w-go">${t('Начать подготовку', 'Start training')} 🚀</button>
        <p class="tiny">${t('Без регистрации. Прогресс хранится только в этом браузере.', 'No sign-up. Progress is saved only in this browser.')}</p>
      </section>`;
    bind('[data-age]', 'click', (e, el) => { p.age = el.dataset.age; JA.$$('[data-age]').forEach((b) => b.classList.toggle('on', b === el)); });
    JA.$('#w-go').onclick = () => {
      p.name = (JA.$('#w-name').value || '').trim().slice(0, 16) || t('Комета', 'Comet');
      if (p.age === 'cadet') p.settings.voice = true;
      JA.save(); JA.sfx('whoosh'); JA.go('home');
    };
  };

  /* ============================================================
     HOME
     ============================================================ */
  JA.screens.home = function () {
    const p = JA.profile;
    if (!p.name) return JA.screens.welcome();
    const nextMap = JA.MAPS.find((m) => !JA.unlocked(m));
    app().innerHTML = `
      <section class="home-grid">
        <div class="card hello">
          <div class="hello-av" data-go="shop" role="button" tabindex="0" title="${t('Сменить скин', 'Change skin')}">${JA.avatar(p.wear, 120)}</div>
          <div>
            <h1>${t('Привет', 'Hi')}, ${esc(p.name)}!</h1>
            <p>${p.age === 'cadet' ? '🐣 ' + t('Кадет 7–9', 'Cadet 7–9') : '🦅 ' + t('Пилот 10–12', 'Pilot 10–12')} · <button class="link" id="h-age">${t('сменить', 'change')}</button></p>
            ${nextMap ? `<div class="progress-line"><span>${t('До карты', 'Next map')} ${nextMap.icon} ${L(nextMap.name)}</span>
              <div class="bar"><i style="width:${Math.min(100, (p.total / nextMap.need) * 100)}%"></i></div><small>${p.total} / ${nextMap.need} 🏅</small></div>`
              : `<p>🌌 ${t('Все карты открыты!', 'All maps unlocked!')}</p>`}
          </div>
        </div>
        <div class="mode-row">
          <button class="mode-card solo" data-go="solo">
            <span class="mode-ico">🧑‍🚀</span><b>${t('Одиночная игра', 'Single player')}</b>
            <small>${t('Базы на Луне и Марсе + тренировки', 'Moon & Mars bases + training')}</small></button>
          <button class="mode-card multi" data-go="multi">
            <span class="mode-ico">👥</span><b>${t('Мультиплеер', 'Multiplayer')}</b>
            <small>${t('ЦУП и экипаж: работайте вместе', 'Mission Control & crew: team up')}</small></button>
        </div>
        <div class="weather card" id="weather"></div>
        <div class="tile-row">
          <button class="tile" data-go="shop">🛍️<b>${t('Магазин скинов', 'Skin shop')}</b></button>
          <button class="tile" data-go="badges">🎖️<b>${t('Нашивки', 'Patches')}</b><small>${p.badges.length}/${JA.BADGES.length}</small></button>
          <button class="tile" data-go="log">📓<b>${t('Бортжурнал', 'Mission log')}</b></button>
          <button class="tile" data-go="adults">🧑‍🏫<b>${t('Для взрослых и судей', 'For adults & judges')}</b></button>
        </div>
      </section>`;
    JA.UI.bindCommon();
    JA.$('#h-age').onclick = () => { p.age = p.age === 'cadet' ? 'pilot' : 'cadet'; JA.save(); JA.rerender(); };
    JA.UI.weatherWidget(JA.$('#weather'));
    const onW = () => { const el = JA.$('#weather'); if (el) JA.UI.weatherWidget(el); };
    document.addEventListener('ja:weather', onW);
    JA.cleanup = () => document.removeEventListener('ja:weather', onW);
  };

  /* ---------- live space weather widget ---------- */
  JA.UI.weatherWidget = function (el) {
    const w = JA.weather;
    if (w.status === 'loading') { el.innerHTML = `<h2>☀️ ${t('Космическая погода', 'Space weather')}</h2><p class="muted">${t('Связываемся с NASA DONKI…', 'Contacting NASA DONKI…')}</p>`; return; }
    const risk = JA.spaceRisk();
    const list = w.events.slice(0, 4).map((e) => `<li><span class="tag ${e.type}">${e.type}</span> ${esc(L(e.label))} <small>${e.date}</small></li>`).join('');
    el.innerHTML = `
      <div class="weather-head">
        <h2>☀️ ${t('Космическая погода', 'Space weather')}</h2>
        ${w.live && !w.stale ? `<span class="live-dot">LIVE · NASA DONKI</span>` : w.live ? `<span class="archive-dot">${t('NASA DONKI · сохранённые данные', 'NASA DONKI · saved data')}</span>` : `<span class="archive-dot">${t('АРХИВ · NASA DONKI', 'ARCHIVE · NASA DONKI')}</span>`}
      </div>
      ${w.live && w.fetchedAt ? `<p class="tiny">${t('Данные получены', 'Data fetched')}: ${new Date(w.fetchedAt).toLocaleString()}${w.range ? ` · ${t('период', 'period')} ${w.range[0]} — ${w.range[1]}` : ''}</p>` : ''}
      ${w.live ? `<p>${t('Последние 30 дней', 'Last 30 days')}: ☀️ ${t('вспышек', 'flares')} <b>${w.counts.FLR}</b> · 💥 ${t('выбросов', 'CMEs')} <b>${w.counts.CME}</b> · 🧲 ${t('бурь', 'storms')} <b>${w.counts.GST}</b>${w.kpMax ? ` (Kp ${t('макс.', 'max')} ${w.kpMax})` : ''}</p>
        <p class="risk" style="--c:${risk.color}">${t('Радиационная обстановка для твоей базы', 'Radiation outlook for your base')}: <b>${L(risk.label)}</b></p>`
        : `<p class="muted">${t('Живые данные сейчас недоступны — показываем реальные события из архива DONKI.', 'Live data is unavailable right now — showing real events from the DONKI archive.')}</p>`}
      <ul class="ev-list">${list}</ul>
      <p class="tiny">${t('Эти события используются в игре: вспышка в миссии — это настоящая вспышка с Солнца.', 'These events are used in the game: a flare in your mission is a real flare from the Sun.')}</p>`;
  };

  /* ============================================================
     SINGLE PLAYER HUB
     ============================================================ */
  JA.screens.solo = function (tab) {
    tab = tab || 'maps';
    const p = JA.profile;
    const maps = JA.MAPS.map((m) => {
      const open = JA.unlocked(m);
      const best = p.best[m.id] || 0;
      return `<button class="map-card ${open ? '' : 'locked'}" ${open ? `data-map="${m.id}"` : 'disabled'} style="--mc:${m.color}">
        <span class="map-ico">${open ? m.icon : '🔒'}</span>
        <b>${L(m.name)}</b><small>${L(m.short)} · ${m.sols} ${t('солов', 'sols')}</small>
        ${open ? `<span class="stars">${'★'.repeat(best)}${'☆'.repeat(3 - best)}</span>`
          : `<span class="need">${t('Нужно', 'Need')} ${m.need} 🏅 <i class="bar"><i style="width:${Math.min(100, p.total / m.need * 100)}%"></i></i></span>`}
      </button>`;
    }).join('');
    const trainings = JA.TRAININGS.map((tr) => `
      <button class="train-card" data-train="${tr.id}">
        <span class="map-ico">${tr.icon}</span><b>${L(tr.name)}</b><small>${L(tr.skill)}</small>
        <span class="reward">${tr.points ? `+${tr.points} ⭐` : t('без очков — просто для тебя', 'no points — just for you')}</span>
        ${p.trainings[tr.id] ? `<span class="done">✓ ${p.trainings[tr.id]}</span>` : ''}
      </button>`).join('');
    app().innerHTML = `
      ${JA.UI.back()}
      <h1>🧑‍🚀 ${t('Одиночная игра', 'Single player')}</h1>
      <div class="tabs" role="tablist">
        <button class="tab ${tab === 'maps' ? 'on' : ''}" data-tab="maps">🪐 ${t('Базы', 'Outposts')}</button>
        <button class="tab ${tab === 'train' ? 'on' : ''}" data-tab="train">🎓 ${t('Тренировочный центр', 'Training center')}</button>
      </div>
      ${tab === 'maps'
        ? `<p class="muted">${t('Набирай очки в миссиях и тренировках — новые карты откроются сами. Звёзды можно тратить в магазине, а открытые карты останутся открытыми.', 'Earn points in missions and training — new maps unlock automatically. Spend stars in the shop; unlocked maps stay unlocked.')}</p><div class="map-grid">${maps}</div>`
        : `<p class="muted">${t('Короткие тренировки навыков экипажа: внимание, план, общение, чувства. Каждая даёт очки.', 'Short crew skill drills: focus, planning, communication, feelings. Each gives points.')}</p><div class="map-grid">${trainings}</div>`}`;
    JA.UI.bindCommon();
    bind('[data-tab]', 'click', (e, el) => JA.go('solo', el.dataset.tab));
    bind('[data-map]', 'click', (e, el) => JA.go('briefing', el.dataset.map));
    bind('[data-train]', 'click', (e, el) => JA.go('train', el.dataset.train));
  };

  /* ============================================================
     MAP BRIEFING
     ============================================================ */
  const PLANS = [
    tt2('Если станет трудно — открою подсказку 💡', 'If it gets hard — I’ll open a hint 💡'),
    tt2('Если волнуюсь — сделаю паузу на Маяке 🌟', 'If I feel nervous — I’ll pause at the Beacon 🌟'),
    tt2('Если что-то сломалось — сначала найду причину 🔍', 'If something breaks — I’ll find the cause first 🔍'),
  ];
  function tt2(ru, en) { return { ru, en }; }
  JA.PLANS = PLANS;

  JA.screens.briefing = function (mapId) {
    const m = JA.Sim.mapById(mapId);
    const p = JA.profile;
    let plan = 0, live = JA.weather.live;
    const cadet = p.age === 'cadet';
    app().innerHTML = `
      ${JA.UI.back('solo')}
      <section class="card briefing" style="--mc:${m.color}">
        <div class="brief-head"><span class="big-ico">${m.icon}</span><div><h1>${L(m.name)}</h1><p>${L(m.desc)} ${JA.UI.say(L(m.desc))}</p></div></div>
        <div class="facts">
          <div><span>☢️ ${t('Радиация (реальные данные)', 'Radiation (real data)')}</span><b>${L(m.doseReal)}</b></div>
          <div><span>📡 ${t('Задержка связи', 'Comms delay')}</span><b>${L(m.delayReal)}</b></div>
          <div><span>🪶 ${t('Гравитация', 'Gravity')}</span><b>${L(m.gravity)}</b></div>
          <div><span>📅 ${t('Длина миссии', 'Mission length')}</span><b>${m.sols} ${t('солов', 'sols')}</b></div>
        </div>
        <p class="fact">💡 ${L(m.fact)}</p>
        <div class="goal">
          <h3>🎯 ${t('Задача', 'Goal')}</h3>
          <ul>
            <li>${t(`Продержаться ${m.sols} солов: кислород${cadet ? '' : ' и еда'} не должны закончиться, а доза радиации — дойти до 100%.`, `Last ${m.sols} sols: don’t run out of oxygen${cadet ? '' : ' or food'}, and keep radiation dose under 100%.`)}</li>
            <li>${t('⭐ за выживание · ⭐ за научную цель · ⭐ если доза меньше 50%', '⭐ for surviving · ⭐ for the science goal · ⭐ if dose stays under 50%')}</li>
            <li>${t('Сол — это одни сутки на базе. Каждый сол ты делишь энергию.', 'A sol is one day at the base. Each sol you split the power.')}</li>
          </ul>
        </div>
        <div class="field"><span>📋 ${t('Твой план «если — то» (так делают и настоящие экипажи)', 'Your “if–then” plan (real crews do this too)')}</span>
          <div class="plan-list">${PLANS.map((pl, i) => `<button class="chip ${i === 0 ? 'on' : ''}" data-plan="${i}">${L(pl)}</button>`).join('')}</div></div>
        ${m.id !== 'earth' ? `<label class="toggle"><input type="checkbox" id="b-live" ${live ? 'checked' : ''} ${JA.weather.live ? '' : 'disabled'}>
          <span>☀️ ${t('Миссия по реальной космической погоде (NASA DONKI)', 'Mission with real space weather (NASA DONKI)')} ${JA.weather.live ? '' : t('— живые данные недоступны', '— live data unavailable')}</span></label>` : ''}
        <button class="btn big primary" id="b-go">${t('Начать миссию', 'Launch mission')} 🚀</button>
      </section>`;
    JA.UI.bindCommon();
    bind('[data-plan]', 'click', (e, el) => { plan = +el.dataset.plan; JA.$$('[data-plan]').forEach((b) => b.classList.toggle('on', b === el)); });
    const lv = JA.$('#b-live'); if (lv) lv.onchange = () => { live = lv.checked; };
    JA.$('#b-go').onclick = () => JA.go('outpost', { mapId, plan, live: m.id !== 'earth' && live });
  };

  /* ============================================================
     SHARED OUTPOST COMPONENTS (also used by multiplayer)
     ============================================================ */
  JA.UI.meter = function (icon, label, val, opts = {}) {
    const bad = !opts.neutral && (opts.inverse ? val >= 70 : val <= 25);
    const warn = !opts.neutral && (opts.inverse ? val >= 45 : val <= 45);
    return `<div class="meter ${bad ? 'bad' : warn ? 'warn' : ''}" title="${esc(opts.title || '')}">
      <span class="m-ico" aria-hidden="true">${icon}</span>
      <div class="m-body"><span class="m-label">${label} <b>${opts.text || Math.round(val) + '%'}</b>${bad ? ` <i class="m-flag">${opts.inverse ? t('много!', 'high!') : t('мало!', 'low!')}</i>` : ''}</span>
      <div class="m-bar"><i style="width:${JA.clamp(val, 0, 100)}%"></i></div></div></div>`;
  };

  JA.UI.hud = function (s) {
    const cadet = s.age === 'cadet';
    const moodFace = s.morale >= 70 ? '😄' : s.morale >= 40 ? '🙂' : s.morale >= 25 ? '😕' : '😣';
    return `<div class="hud">
      <div class="sol-badge"><small>${t('Сол', 'Sol')}</small><b>${s.sol}</b><small>/ ${s.sols}</small></div>
      ${JA.UI.meter('🫁', t('Кислород', 'Oxygen'), s.o2, { title: t('Экипаж тратит 12% за сол', 'Crew uses 12% per sol') })}
      ${cadet ? '' : JA.UI.meter('🥔', t('Еда', 'Food'), s.food, { title: t('Экипаж съедает 10% за сол', 'Crew eats 10% per sol') })}
      ${JA.UI.meter('☢️', t('Доза', 'Dose'), s.dose, { inverse: true, title: t('Предел безопасной дозы миссии — 100%', 'Mission safe dose limit is 100%') })}
      ${JA.UI.meter('🔋', t('Батарея', 'Battery'), s.battery / JA.Sim.BATTERY_MAX * 100, { text: `${s.battery}/${JA.Sim.BATTERY_MAX}` })}
      ${JA.UI.meter('🔬', t('Наука', 'Science'), Math.min(100, s.science / s.goal * 100), { text: `${s.science}/${s.goal}`, neutral: true })}
      <div class="crew" title="${t('Настроение экипажа', 'Crew mood')}">${JA.CREW.map((c) => `<span>${c.face}</span>`).join('')}<b>${moodFace}</b></div>
    </div>`;
  };

  /* Outpost scene — reacts to state (night, storm, shield, greenhouse) */
  JA.UI.scene = function (s, alloc = {}) {
    const m = JA.Sim.mapById(s.mapId);
    const night = m.nightSols && m.nightSols.includes(s.sol);
    const storm = (s.effects || []).some((e) => e.id === 'storm' && e.until >= s.sol);
    const rover = (JA.SKINS.rover.find((r) => r.id === JA.profile.wear.rover) || JA.SKINS.rover[0]).color;
    const shield = alloc.shield || 0;
    const ground = { earth: '#c99a5b', moon: '#8e94a3', mars: '#b5532f', gateway: '#1a2246', europa: '#bfe6f5' }[m.id];
    const sky = { earth: '#5aa3e8', moon: '#05060f', mars: '#d9a27a', gateway: '#05060f', europa: '#0b1030' }[m.id];
    const isStation = m.id === 'gateway';
    return `<div class="scene ${night ? 'night' : ''} ${storm ? 'storm' : ''}" style="--sky:${sky};--ground:${ground}" aria-hidden="true">
      <div class="sun ${night ? 'off' : ''}"></div>
      ${m.id === 'europa' ? '<div class="jupiter"></div>' : ''}
      ${m.id === 'moon' || m.id === 'gateway' ? '<div class="earth-dot"></div>' : ''}
      ${isStation ? `<div class="station"><i class="panel"></i><i class="core"></i><i class="panel"></i></div><div class="moon-big"></div>`
        : `<div class="ground"></div>
      <div class="habitat">
        <div class="shield-ring" style="opacity:${Math.min(1, shield / 3)}"></div>
        <div class="dome"></div>
        <div class="greenhouse ${alloc.green ? 'on' : ''}"></div>
        <div class="panels ${night || storm ? 'dim' : ''}"><i></i><i></i><i></i></div>
        <div class="rover" style="--rv:${rover}"></div>
        <div class="mini-av">${JA.avatar(JA.profile.wear, 44)}</div>
      </div>`}
      ${storm ? '<div class="dust"></div>' : ''}
      ${night ? `<div class="scene-label">🌑 ${t('Лунная ночь: солнечные панели почти не работают', 'Lunar night: solar panels barely work')}</div>` : ''}
      ${storm ? `<div class="scene-label">🌪️ ${t('Пылевая буря', 'Dust storm')}</div>` : ''}
    </div>`;
  };

  const ROWS = [
    { k: 'ls', icon: '🫁', name: tt2('Жизнеобеспечение', 'Life support'), eff: tt2('1 блок = +6% кислорода', '1 block = +6% oxygen') },
    { k: 'green', icon: '🌱', name: tt2('Оранжерея', 'Greenhouse'), eff: tt2('1 блок = +5% еды', '1 block = +5% food'), pilot: true },
    { k: 'shield', icon: '🛡️', name: tt2('Радиационный щит', 'Radiation shield'), eff: tt2('1 блок = −30% дозы (макс. 3)', '1 block = −30% dose (max 3)') },
    { k: 'sci', icon: '🔬', name: tt2('Наука', 'Science'), eff: tt2('1 блок = +1 к научной цели', '1 block = +1 science') },
  ];
  JA.UI.ROWS = ROWS;

  /* Power allocation panel */
  JA.UI.alloc = function (s, alloc, readonly) {
    const cadet = s.age === 'cadet';
    const gen = Math.max(0, s.gen + s.mods.genDelta);
    const total = JA.Sim.available(s);
    const used = ROWS.reduce((a, r) => a + (alloc[r.k] || 0), 0);
    const left = total - used;
    const fromBat = Math.max(0, used - gen);
    return `<div class="alloc ${readonly ? 'ro' : ''}">
      <div class="alloc-head">
        <div><b>⚡ ${t('Энергия на этот сол', 'Power this sol')}</b>
          <small>${t('Солнце/генератор', 'Generated')}: ${gen}${s.mods.genDelta ? ` (${s.mods.genDelta} ${t('событие', 'event')})` : ''} + ${t('батарея', 'battery')}: ${s.battery}</small></div>
        <div class="left ${left < 0 ? 'neg' : ''}">${t('Свободно', 'Free')}: <b>${left}</b></div>
      </div>
      <div class="power-strip" aria-hidden="true">${Array.from({ length: total }, (_, i) => `<i class="${i < used ? 'used' : ''} ${i >= gen ? 'bat' : ''}"></i>`).join('')}</div>
      ${fromBat ? `<p class="tiny warn-text">🔋 ${t(`Ты берёшь ${fromBat} бл. из батареи`, `You’re using ${fromBat} blocks from the battery`)}</p>` : left > 0 ? `<p class="tiny">🔋 ${t(`${Math.min(left, gen - Math.min(gen, used))} свободных бл. зарядят батарею`, `${Math.min(left, gen - Math.min(gen, used))} free blocks will charge the battery`)}</p>` : ''}
      ${ROWS.filter((r) => !(r.pilot && cadet)).map((r) => `
        <div class="alloc-row">
          <span class="a-ico">${r.icon}</span>
          <div class="a-name"><b>${L(r.name)}</b><small>${L(r.eff)}</small></div>
          <div class="a-ctrl">
            ${readonly ? '' : `<button class="step" data-dec="${r.k}" aria-label="−" ${!(alloc[r.k] > 0) ? 'disabled' : ''}>−</button>`}
            <span class="blocks">${Array.from({ length: Math.max(alloc[r.k] || 0, 0) }, () => '<i></i>').join('') || '<em>0</em>'}</span>
            ${readonly ? '' : `<button class="step" data-inc="${r.k}" aria-label="+" ${left <= 0 || (r.k === 'shield' && alloc.shield >= JA.Sim.SHIELD_MAX) ? 'disabled' : ''}>+</button>`}
          </div>
        </div>`).join('')}
    </div>`;
  };

  /* Event card with choices */
  JA.UI.eventCard = function (s, canChoose) {
    if (!s.today) return '';
    const def = JA.EVENTS[s.today.id];
    const d = s.today.data;
    let choices = def.choices;
    let text = L(def.text(s, d));
    let extra = '';
    if (def.emotion) {
      const mood = JA.MOODS[d.mood];
      const who = JA.CREW.find((c) => c.id === mood.who);
      extra = `<p class="who">${who.face} <b>${L(who.name)}</b>, ${L(who.role)}</p>`;
      choices = def.choices.map((c, i) => Object.assign({}, c, { label: mood.support[i], good: tt2('Экипажу станет лучше', 'The crew will feel better') }));
    }
    const chosen = s.choice != null;
    return `<div class="event-card ${def.alarm ? 'alarm' : ''}">
      <div class="ev-title"><span class="ev-ico">${def.icon}</span><h3>${L(def.title)}</h3>${JA.UI.say(L(def.title) + '. ' + text)}</div>
      ${extra}<p>${esc(text)}</p>
      ${def.emotion && !chosen && s.feelingPicked == null ? `<p><b>${t('Как ты думаешь, что чувствует член экипажа?', 'What do you think your crewmate feels?')}</b></p>
        <div class="feel-row">${JA.FEELINGS.map((f) => `<button class="chip" data-feel="${f.id}" ${canChoose ? '' : 'disabled'}>${f.icon} ${L(f.name)}</button>`).join('')}</div>`
      : `${def.emotion && s.feelingNote ? `<p class="note">${esc(s.feelingNote)}</p>` : ''}
      <div class="choices">${choices.map((c, i) => `
        <button class="choice ${s.choice === i ? 'picked' : ''}" data-choice="${i}" ${!canChoose || chosen ? 'disabled' : ''}>
          <b>${L(c.label)}</b>
          ${c.good ? `<span class="plus">✔ ${L(c.good)}</span>` : ''}
          <span class="minus">✖ ${L(c.cost)}</span>
        </button>`).join('')}</div>
      ${!chosen ? `<p class="tiny">${t('Бесплатного выбора нет — у каждого варианта есть цена. Как у настоящих планировщиков миссий.', 'No choice is free — every option has a cost, just like for real mission planners.')}</p>` : ''}`}
    </div>`;
  };

  JA.UI.forecastText = function (s) {
    if (!s.forecast) return null;
    const f = s.forecast;
    const names = {
      flare: tt2('солнечная вспышка', 'a solar flare'), dust: tt2('пылевая буря', 'a dust storm'), night: tt2('лунная ночь — 2 сола без солнца', 'the lunar night — 2 sols without sun'),
      failure: tt2('сбой генератора кислорода — датчики показывают износ', 'oxygen generator trouble — sensors show wear'), leak: tt2('поток микрометеоритов', 'a micrometeorite shower'),
    };
    const what = L(names[f.id] || tt2('событие', 'an event'));
    if (s.age === 'cadet' || f.sure) return t(`Завтра ожидается ${what}. Подготовься!`, `Tomorrow expect ${what}. Get ready!`);
    return t(`Возможно, завтра ${what} (вероятность ~70%). Прогнозы космической погоды не всегда точны — решай, стоит ли готовиться.`,
             `Tomorrow there may be ${what} (~70% chance). Space weather forecasts aren’t always right — decide whether to prepare.`);
  };

  /* Context hint (free, never penalized) */
  JA.UI.hint = function (s, alloc) {
    const cadet = s.age === 'cadet';
    const tips = [];
    if ((alloc.ls || 0) < 2) tips.push(t('🫁 Экипаж тратит 12% кислорода за сол, а 1 блок даёт 6%. Значит, нужно 2 блока, чтобы кислород не падал.', '🫁 The crew uses 12% oxygen per sol and 1 block makes 6%. So you need 2 blocks to keep oxygen steady.'));
    if (!cadet && (alloc.green || 0) < 2 && s.food < 60) tips.push(t('🥔 Экипаж съедает 10% еды за сол, 1 блок оранжереи даёт 5%. 2 блока — чтобы еда не убывала.', '🥔 The crew eats 10% food per sol; 1 greenhouse block makes 5%. 2 blocks keep food steady.'));
    const m = JA.Sim.mapById(s.mapId);
    if (m.dose >= 4 && (alloc.shield || 0) < 2) tips.push(t(`🛡️ Здесь радиация выше, чем на Земле (${L(m.doseReal)}). Каждый блок щита убирает 30% дозы.`, `🛡️ Radiation here is higher than on Earth (${L(m.doseReal)}). Each shield block removes 30% of the dose.`));
    if (s.forecast) tips.push(t('📡 ЦУП предупредил о событии — подумай, не оставить ли энергию в батарее или включить щит заранее.', '📡 Mission Control warned you — think about saving battery or raising shields early.'));
    if (s.today && JA.EVENTS[s.today.id].alarm && s.choice == null) tips.push(t('⚠️ Сравни цену каждого варианта события: что важнее сейчас — безопасность или наука?', '⚠️ Compare the cost of each option: what matters more now — safety or science?'));
    if (!tips.length) tips.push(t('🔬 Основные системы в порядке. Лишние блоки можно отдать науке или сохранить в батарее на трудный день.', '🔬 Core systems look fine. Spare blocks can go to science or into the battery for a hard day.'));
    return tips;
  };

  /* Report after a sol */
  JA.UI.reportHtml = function (rep) {
    return `<h2>📋 ${t('Итоги сола', 'Sol report')} ${rep.sol}</h2>
      <ul class="report">${rep.lines.map((l) => {
        const good = l.bad ? l.d <= 0.5 : l.d >= 0;
        return `<li class="${good ? 'up' : 'down'}"><span>${esc(l.icon)}</span><span>${esc(l.text)}</span><b>${l.d > 0 ? '+' : ''}${Math.round(l.d * 10) / 10}</b></li>`;
      }).join('')}</ul>`;
  };

  /* Tiny line chart of the mission (SVG) */
  JA.UI.chart = function (s) {
    const h = s.history;
    if (h.length < 2) return '';
    const W = 320, H = 120, n = h.length;
    const x = (i) => 10 + (i / (n - 1)) * (W - 20);
    const y = (v) => H - 10 - (v / 100) * (H - 20);
    const line = (k, c) => `<polyline fill="none" stroke="${c}" stroke-width="3" stroke-linejoin="round" points="${h.map((p, i) => `${x(i)},${y(p[k])}`).join(' ')}"/>`;
    return `<figure class="chart"><svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${t('График ресурсов по солам', 'Resources by sol')}">
      <line x1="10" x2="${W - 10}" y1="${y(0)}" y2="${y(0)}" stroke="currentColor" opacity=".2"/>
      ${line('o2', '#4fc3ff')}${s.age === 'cadet' ? '' : line('food', '#ffb347')}${line('dose', '#ff5e7e')}
    </svg><figcaption><span style="color:#4fc3ff">● ${t('кислород', 'oxygen')}</span> ${s.age === 'cadet' ? '' : `<span style="color:#ffb347">● ${t('еда', 'food')}</span>`} <span style="color:#ff5e7e">● ${t('доза', 'dose')}</span></figcaption></figure>`;
  };

  /* ============================================================
     SINGLE PLAYER OUTPOST GAME
     ============================================================ */
  JA.screens.outpost = function (cfg) {
    const p = JA.profile;
    if (p.lastFailed) JA.award('comeback');
    const s = JA.Sim.create(cfg.mapId, p.age, { live: cfg.live });
    s.plan = cfg.plan;
    JA.Sim.startSol(s);
    let alloc = defaultAlloc(s);
    let forecastShown = false;
    const map = JA.Sim.mapById(cfg.mapId);

    function defaultAlloc(st) {
      return { ls: 0, green: 0, shield: 0, sci: 0 };
    }

    function draw() {
      const msg = JA.UI.forecastText(s);
      app().innerHTML = `
        <div class="game-top">${JA.UI.back('solo', t('Выйти', 'Exit'))}<h2>${map.icon} ${L(map.name)}</h2>
          <button class="btn ghost small" id="g-beacon">🌟 ${t('Маяк', 'Beacon')}</button></div>
        ${JA.UI.hud(s)}
        <div class="game-grid">
          <div class="col-left">
            ${JA.UI.scene(s, alloc)}
            <div class="mcc" id="mcc">
              <span class="mcc-ico">📡</span>
              <div><b>${t('Центр управления (ЦУП)', 'Mission Control')}</b>
                <p id="mcc-text">${msg ? (forecastShown ? esc(msg) : `<span class="sending">${t('Сигнал идёт с Земли…', 'Signal travelling from Earth…')} ${map.delayMs ? `(${L(map.delayReal)})` : ''}</span>`) : (s.effects.some((e) => e.id === 'silence' && e.until >= s.sol) ? t('…тишина: связи нет из-за Солнца.', '…silence: the Sun blocks the radio.') : t('Прогноз на завтра: всё спокойно.', 'Tomorrow’s forecast: all calm.'))}</p></div>
            </div>
            <p class="plan-reminder">📋 ${L(PLANS[s.plan || 0])}</p>
          </div>
          <div class="col-right">
            ${JA.UI.eventCard(s, true)}
            ${JA.UI.alloc(s, alloc)}
            <div class="actions">
              <button class="btn ghost" id="g-hint">💡 ${t('Подсказка', 'Hint')}</button>
              <button class="btn primary big" id="g-run" ${s.today && s.choice == null ? 'disabled' : ''}>${t('Выполнить сол', 'Run the sol')} ▶</button>
            </div>
            ${s.today && s.choice == null ? `<p class="tiny center">${t('Сначала реши, что делать с событием.', 'First decide what to do about the event.')}</p>` : ''}
          </div>
        </div>`;
      JA.UI.bindCommon();
      bindGame();
      if (msg && !forecastShown) {
        const ms = Math.min(map.delayMs, 6000) || 300;
        const timer = setTimeout(() => { forecastShown = true; JA.sfx('radio'); const el = JA.$('#mcc-text'); if (el) { el.textContent = msg; JA.speak(msg); } }, ms);
        JA.cleanup = () => clearTimeout(timer);
      }
    }

    function bindGame() {
      bind('[data-inc]', 'click', (e, el) => { alloc[el.dataset.inc]++; draw(); });
      bind('[data-dec]', 'click', (e, el) => { alloc[el.dataset.dec] = Math.max(0, alloc[el.dataset.dec] - 1); draw(); });
      bind('[data-feel]', 'click', (e, el) => {
        const mood = JA.MOODS[s.today.data.mood];
        const f = JA.FEELINGS.find((x) => x.id === el.dataset.feel);
        const ok = mood.ok.includes(f.id);
        s.feelingPicked = f.id;
        s.feelingNote = ok
          ? t(`Да, похоже на ${L(f.name)}. Чувства бывают разными — и это нормально. Как можно поддержать?`, `Yes, that looks like ${L(f.name)}. Feelings can differ — and that’s OK. How can you help?`)
          : t(`Может быть! Но подсказки в истории говорят скорее о: ${mood.ok.map((id) => L(JA.FEELINGS.find((x) => x.id === id).name)).join(', ')}. Люди могут чувствовать по-разному. Как можно поддержать?`,
              `Maybe! But clues in the story point more to: ${mood.ok.map((id) => L(JA.FEELINGS.find((x) => x.id === id).name)).join(', ')}. People can feel different things. How can you help?`);
        draw();
      });
      bind('[data-choice]', 'click', (e, el) => {
        JA.Sim.choose(s, +el.dataset.choice);
        if (s.today.id === 'emotion') JA.award('helper');
        draw();
      });
      JA.$('#g-hint').onclick = () => {
        const tips = JA.UI.hint(s, alloc);
        JA.modal(`<h2>💡 ${t('Подсказка', 'Hint')}</h2><ul class="tips">${tips.map((x) => `<li>${esc(x)}</li>`).join('')}</ul>
          <p class="tiny">${t('Подсказки бесплатные. Просить помощь — это навык настоящего астронавта.', 'Hints are free. Asking for help is a real astronaut skill.')}</p>
          <button class="btn primary" data-close>${t('Понятно', 'Got it')}</button>`);
        JA.speak(tips.join(' '));
      };
      JA.$('#g-beacon').onclick = () => JA.beacon();
      JA.$('#g-run').onclick = run;
    }

    function run() {
      const used = alloc.ls + alloc.green + alloc.shield + alloc.sci;
      if (used > JA.Sim.available(s)) return;
      const warn = [];
      if (alloc.ls < 2 && s.o2 - JA.Sim.O2_USE + alloc.ls * JA.Sim.O2_PER < 30) warn.push(t('🫁 Кислорода станет мало.', '🫁 Oxygen will get low.'));
      if (warn.length && !run.confirmed) {
        // "Pause before action" — a gentle check, not a block
        JA.modal(`<h2>✋ ${t('Пауза перед действием', 'Pause before acting')}</h2>
          <p>${t('Остановись, посмотри, выбери:', 'Stop, look, choose:')}</p><ul class="tips">${warn.map((w) => `<li>${w}</li>`).join('')}</ul>
          <div class="row"><button class="btn ghost" data-close>${t('Изменить план', 'Change plan')}</button>
          <button class="btn primary" id="m-go">${t('Всё равно выполнить', 'Run anyway')}</button></div>`);
        JA.$('#m-go').onclick = () => { JA.$('.modal-wrap').remove(); run.confirmed = true; run(); };
        return;
      }
      run.confirmed = false;
      const wasToday = s.today ? s.today.id : null;
      const rep = JA.Sim.resolve(s, alloc);
      JA.sfx(rep.lines.some((l) => (l.bad ? l.d > 10 : l.d < -5)) ? 'bad' : 'good');
      if (wasToday === 'flare' && alloc.shield >= 2) JA.award('shield');
      const m = JA.modal(`${JA.UI.reportHtml(rep)}
        ${feedbackFor(rep, wasToday)}
        <button class="btn primary big" data-close>${s.status === 'playing' ? t('Следующий сол', 'Next sol') + ' ▶' : t('Итоги миссии', 'Mission results')}</button>`,
        { dismiss: false, onClose: () => { if (s.status === 'playing') { alloc = defaultAlloc(s); forecastShown = false; draw(); } else finish(); } });
      JA.speak(rep.lines.map((l) => l.text).join('. '));
    }

    function feedbackFor(rep, ev) {
      // Feedback describes actions, not personality
      const out = [];
      const a = rep.alloc;
      if (ev === 'flare' && a.shield >= 2) out.push(t('Ты включил щит во время вспышки — доза выросла меньше.', 'You raised the shield during the flare — the dose rose less.'));
      if (a.ls >= 2) out.push(t('Ты дал жизнеобеспечению достаточно энергии — экипажу хватает воздуха.', 'You gave life support enough power — the crew has air.'));
      if (s.battery >= 4 && s.forecast) out.push(t('Ты держишь энергию в батарее перед событием — хороший запас.', 'You kept power in the battery before an event — a good reserve.'));
      if (!out.length) return '';
      return `<p class="feedback">👍 ${esc(out[0])}</p>`;
    }

    function finish() {
      const won = s.status === 'won';
      const stars = JA.Sim.starsFor(s);
      const pts = JA.Sim.pointsFor(s);
      p.lastFailed = !won;
      p.best[s.mapId] = Math.max(p.best[s.mapId] || 0, stars);
      p.log.unshift({ date: new Date().toISOString().slice(0, 10), map: s.mapId, won, stars, sol: s.sol, sols: s.sols, science: s.science, dose: Math.round(s.dose), mode: 'solo' });
      p.log = p.log.slice(0, 30);
      JA.save();
      const before = JA.MAPS.filter(JA.unlocked).length;
      JA.addPoints(pts, t('за миссию', 'for the mission'));
      const after = JA.MAPS.filter(JA.unlocked).length;
      JA.award('first');
      if (won) {
        if (s.science >= s.goal) JA.award('scientist');
        if (s.mapId === 'moon') JA.award('moon');
        if (s.mapId === 'mars') JA.award('mars');
        if (s.mapId === 'europa') JA.award('europa');
        if (s.flags.storm) JA.award('storm');
        if (s.flags.night) JA.award('night');
      }
      if (s.live) JA.award('live');
      JA.go('debrief', { s, won, stars, pts, newMap: after > before ? JA.MAPS.filter(JA.unlocked)[after - 1] : null });
    }

    draw();
    if (map.id === 'earth' && !p.log.length) setTimeout(() => tutorial(), 300);

    function tutorial() {
      const steps = [
        t('Это твоя база! Сверху — ресурсы: кислород, еда, доза радиации, батарея и наука.', 'This is your base! At the top are your resources: oxygen, food, radiation dose, battery and science.'),
        t('Каждый сол у тебя есть блоки энергии ⚡. Нажимай «+», чтобы отдать их системам.', 'Each sol you get power blocks ⚡. Press “+” to give them to systems.'),
        t('Энергии всегда не хватает на всё — придётся выбирать. Так работают настоящие миссии!', 'There’s never enough power for everything — you must choose. That’s how real missions work!'),
        t('Если трудно — жми 💡 «Подсказка». А 🌟 «Маяк» поможет спокойно подышать.', 'If it’s hard, press 💡 Hint. The 🌟 Beacon helps you breathe calmly.'),
      ];
      let i = 0;
      const show = () => {
        const m = JA.modal(`<h2>🧑‍🚀 ${t('Инструктор', 'Instructor')} ${i + 1}/${steps.length}</h2><p>${steps[i]} ${JA.UI.say(steps[i])}</p>
          <button class="btn primary" data-close>${i < steps.length - 1 ? t('Дальше', 'Next') : t('Поехали!', 'Let’s go!')}</button>`,
          { onClose: () => { i++; if (i < steps.length) show(); } });
        JA.speak(steps[i]);
      };
      show();
    }
  };

  /* ============================================================
     DEBRIEF (reflection + thought check)
     ============================================================ */
  JA.screens.debrief = function ({ s, won, stars, pts, newMap }) {
    const map = JA.Sim.mapById(s.mapId);
    const helped = [
      tt2('Прогноз ЦУП', 'Mission Control forecast'), tt2('Запас в батарее', 'Battery reserve'), tt2('Щит вовремя', 'Shield in time'),
      tt2('Подсказка', 'A hint'), tt2('Пауза на Маяке', 'Beacon pause'), tt2('Мой план «если — то»', 'My if–then plan'),
    ];
    const life = [
      tt2('Делить карманные деньги', 'Splitting pocket money'), tt2('Планировать время: уроки и игры', 'Planning time: homework and play'),
      tt2('Готовиться заранее к контрольной', 'Preparing early for a test'), tt2('Договариваться с друзьями', 'Agreeing with friends'),
    ];
    let reflected = false;
    app().innerHTML = `
      <section class="card debrief ${won ? 'won' : 'lost'}">
        <div class="big-ico">${won ? '🏆' : '🛟'}</div>
        <h1>${won ? t('Миссия выполнена!', 'Mission complete!') : t('Экипаж безопасно вернулся домой', 'The crew returned home safely')}</h1>
        <p class="stars-big">${'★'.repeat(stars)}${'☆'.repeat(3 - stars)}</p>
        <p>${map.icon} ${L(map.name)} · ${t('солов', 'sols')}: ${s.sol}/${s.sols} · 🔬 ${s.science}/${s.goal} · ☢️ ${Math.round(s.dose)}%</p>
        <p class="pts">+${pts} ⭐ ${t('и', 'and')} +${pts} 🏅</p>
        ${newMap ? `<p class="unlock">🔓 ${t('Открыта новая карта', 'New map unlocked')}: ${newMap.icon} <b>${L(newMap.name)}</b>!</p>` : ''}
        ${won ? '' : `<div class="why"><h3>🔍 ${t('Разберёмся, что случилось', 'Let’s figure out what happened')}</h3><p>${esc(JA.Sim.failReason(s))}</p></div>
          <div class="thought" id="thought">
            <h3>🧠 ${t('Проверка мысли', 'Thought check')}</h3>
            <p>${t('Тимур говорит: «Миссия не удалась — значит, я ничего не умею». Какая мысль точнее?', 'Timur says: “The mission failed — so I’m no good at anything.” Which thought is more accurate?')}</p>
            <div class="choices">
              <button class="choice" data-th="1">${t('«Я знаю, что пошло не так, и могу попробовать по-другому»', '“I know what went wrong and can try another way”')}</button>
              <button class="choice" data-th="0">${t('«У меня никогда не получится»', '“I’ll never get it”')}</button>
            </div><p id="th-note" class="note" hidden></p>
          </div>`}
        ${JA.UI.chart(s)}
        <div class="reflect">
          <h3>💭 ${t('Что тебе помогло?', 'What helped you?')}</h3>
          <div class="plan-list">${helped.map((h, i) => `<button class="chip" data-help="${i}">${L(h)}</button>`).join('')}</div>
          ${s.age === 'pilot' ? `<h3>🌍 ${t('Где на Земле тоже нужно делить ограниченные ресурсы?', 'Where on Earth do you also split limited resources?')}</h3>
          <div class="plan-list">${life.map((h, i) => `<button class="chip" data-life="${i}">${L(h)}</button>`).join('')}</div>` : ''}
        </div>
        <p class="fact">🛰️ ${t('В реальности', 'In real life')}: ${L(map.fact)}</p>
        <div class="row center">
          <button class="btn primary big" data-go="briefing" data-arg="${s.mapId}">🔁 ${t('Сыграть ещё раз', 'Play again')}</button>
          <button class="btn ghost" data-go="solo">🪐 ${t('К картам', 'To maps')}</button>
        </div>
      </section>`;
    JA.UI.bindCommon();
    bind('[data-help],[data-life]', 'click', (e, el) => {
      el.classList.toggle('on');
      if (!reflected) { reflected = true; JA.addPoints(5, t('за разбор миссии', 'for reflecting')); }
    });
    bind('[data-th]', 'click', (e, el) => {
      const n = JA.$('#th-note'); n.hidden = false;
      n.textContent = el.dataset.th === '1'
        ? t('Точно! Ошибка — это данные. Теперь ты знаешь, что изменить. Так учатся и настоящие экипажи.', 'Exactly! A mistake is data. Now you know what to change. Real crews learn this way too.')
        : t('Это очень тяжёлая мысль, и она не совсем правдива: одна миссия — это не «никогда». Что известно точно? Что именно пошло не так — и это можно изменить.', 'That’s a heavy thought, and not quite true: one mission isn’t “never”. What do we know for sure? What exactly went wrong — and that can change.');
      JA.$$('[data-th]').forEach((b) => b.classList.toggle('picked', b === el));
    });
    JA.speak(won ? t('Миссия выполнена!', 'Mission complete!') : t('Экипаж безопасно вернулся. Разберёмся, что случилось.', 'The crew returned safely. Let’s see what happened.'));
  };

  /* ============================================================
     SPACE BEACON (voluntary breathing pause; never scored)
     ============================================================ */
  JA.beacon = function () {
    const reduce = !JA.profile.settings.motion || matchMedia('(prefers-reduced-motion: reduce)').matches;
    const m = JA.modal(`
      <h2>🌟 ${t('Космический маяк', 'Space Beacon')}</h2>
      <p>${t('Можно подышать вместе с маяком несколько раз. Это по желанию — остановись в любой момент.', 'You can breathe with the beacon a few times. It’s optional — stop any time.')}</p>
      <div class="beacon ${reduce ? 'still' : ''}"><div class="beacon-orb" id="orb"></div><p id="b-say">${t('Нажми «Начать»', 'Press “Start”')}</p></div>
      <div class="row center">
        <button class="btn primary" id="b-start">${t('Начать', 'Start')}</button>
        <button class="btn ghost" data-close>${t('Вернуться к миссии', 'Back to mission')}</button>
      </div>
      <div id="b-after" hidden><p>${t('Как ты сейчас? (можно не отвечать)', 'How do you feel now? (optional)')}</p>
        <div class="plan-list"><button class="chip" data-close>😌 ${t('спокойнее', 'calmer')}</button><button class="chip" data-close>🙂 ${t('так же', 'the same')}</button><button class="chip" data-close>😕 ${t('не помогло — попробую другое', 'didn’t help — I’ll try something else')}</button></div></div>
      <p class="tiny">${t('Если неприятно — просто остановись и дыши как обычно. Маяк не измеряет твоё волнение и не ставит баллы.', 'If it feels uncomfortable, just stop and breathe normally. The beacon doesn’t measure your feelings or give scores.')}</p>`,
    { onClose: () => clearInterval(timer) });
    let timer = null;
    JA.$('#b-start', m.el).onclick = () => {
      JA.award('beacon'); JA.trainingDone('beacon');
      let cycle = 0, phase = 0;
      const orb = JA.$('#orb', m.el), say = JA.$('#b-say', m.el);
      const step = () => {
        if (!document.body.contains(orb)) return clearInterval(timer);
        if (phase === 0) { orb.className = 'beacon-orb in'; say.textContent = t('Вдох… 1, 2, 3, 4', 'Breathe in… 1, 2, 3, 4'); }
        else { orb.className = 'beacon-orb out'; say.textContent = t('Выдох… 1, 2, 3, 4, 5', 'Breathe out… 1, 2, 3, 4, 5'); cycle++; }
        JA.speak(say.textContent.split('…')[0]);
        phase = 1 - phase;
        if (cycle >= 4 && phase === 0) { clearInterval(timer); say.textContent = t('Молодец, что попробовал!', 'Nice job trying it!'); JA.$('#b-after', m.el).hidden = false; }
      };
      step(); clearInterval(timer); timer = setInterval(step, 4500);
      JA.$('#b-start', m.el).disabled = true;
    };
  };

  /* ============================================================
     SHOP
     ============================================================ */
  JA.screens.shop = function (cat) {
    cat = cat || 'suit';
    const p = JA.profile;
    const cats = [['suit', '🧑‍🚀', tt2('Скафандры', 'Suits')], ['visor', '🥽', tt2('Визоры', 'Visors')], ['extra', '✨', tt2('Украшения', 'Accessories')], ['rover', '🚙', tt2('Роверы', 'Rovers')]];
    const items = JA.SKINS[cat].map((it) => {
      const own = p.owned[cat].includes(it.id), wearing = p.wear[cat] === it.id;
      const sw = it.icon ? `<span class="sw-ico">${it.icon || '—'}</span>` : `<span class="swatch" style="background:${it.color === 'rainbow' ? 'linear-gradient(135deg,#ff5e7e,#ffd24a,#4dd9a6,#6b8cff)' : it.color}"></span>`;
      return `<div class="shop-item ${wearing ? 'wearing' : ''}">
        ${it.id === 'none' ? '<span class="sw-ico">🚫</span>' : sw}<b>${L(it.name)}</b>
        ${wearing ? `<span class="tag ok">${t('Надето', 'Wearing')}</span>`
          : own ? `<button class="btn small" data-wear="${it.id}">${t('Надеть', 'Wear')}</button>`
          : `<button class="btn small primary" data-buy="${it.id}" ${p.stars < it.price ? 'disabled' : ''}>⭐ ${it.price}</button>`}
      </div>`;
    }).join('');
    app().innerHTML = `
      ${JA.UI.back()}
      <h1>🛍️ ${t('Магазин скинов', 'Skin shop')}</h1>
      <div class="shop">
        <div class="card preview">${JA.avatar(p.wear, 180)}<p>⭐ ${t('Баланс', 'Balance')}: <b>${p.stars}</b></p>
          <p class="tiny">${t('Покупки не закрывают карты: карты открываются по сумме всех заработанных очков 🏅.', 'Buying never relocks maps: maps unlock by total points ever earned 🏅.')}</p></div>
        <div>
          <div class="tabs">${cats.map(([id, ic, nm]) => `<button class="tab ${cat === id ? 'on' : ''}" data-cat="${id}">${ic} ${L(nm)}</button>`).join('')}</div>
          <div class="shop-grid">${items}</div>
        </div>
      </div>`;
    JA.UI.bindCommon();
    bind('[data-cat]', 'click', (e, el) => JA.go('shop', el.dataset.cat));
    bind('[data-wear]', 'click', (e, el) => { p.wear[cat] = el.dataset.wear; JA.save(); JA.go('shop', cat); });
    bind('[data-buy]', 'click', (e, el) => {
      const it = JA.SKINS[cat].find((x) => x.id === el.dataset.buy);
      if (p.stars < it.price) return;
      p.stars -= it.price; p.owned[cat].push(it.id); p.wear[cat] = it.id; JA.save();
      JA.sfx('badge'); JA.toast(`✨ ${L(it.name)}!`); JA.go('shop', cat);
    });
  };

  /* ============================================================
     BADGES, LOG
     ============================================================ */
  JA.screens.badges = function () {
    const p = JA.profile;
    app().innerHTML = `${JA.UI.back()}<h1>🎖️ ${t('Нашивки миссий', 'Mission patches')}</h1>
      <div class="badge-grid">${JA.BADGES.map((b) => {
        const has = p.badges.includes(b.id);
        return `<div class="badge ${has ? 'has' : ''}"><span>${has ? b.icon : '❔'}</span><b>${L(b.name)}</b><small>${L(b.desc)}</small></div>`;
      }).join('')}</div>`;
    JA.UI.bindCommon();
  };

  JA.screens.log = function () {
    const p = JA.profile;
    app().innerHTML = `${JA.UI.back()}<h1>📓 ${t('Бортжурнал', 'Mission log')}</h1>
      <p class="muted">${t('Здесь только твои миссии. Мы не делаем общий рейтинг — каждый астронавт учится в своём темпе.', 'Only your own missions are here. There’s no public ranking — every astronaut learns at their own pace.')}</p>
      ${p.log.length ? `<ul class="log">${p.log.map((l) => {
        const m = JA.Sim.mapById(l.map);
        return `<li><span>${m.icon}</span><div><b>${L(m.name)}</b> ${l.mode === 'multi' ? '👥' : ''}<small>${l.date} · ${t('сол', 'sol')} ${l.sol}/${l.sols} · 🔬 ${l.science} · ☢️ ${l.dose}%</small></div>
          <span>${l.won ? '★'.repeat(l.stars) + '☆'.repeat(3 - l.stars) : '🛟'}</span></li>`;
      }).join('')}</ul>` : `<p>${t('Пока пусто. Начни первую миссию!', 'Empty so far. Start your first mission!')}</p>`}
      <button class="btn ghost small" id="reset">🗑️ ${t('Сбросить прогресс', 'Reset progress')}</button>`;
    JA.UI.bindCommon();
    JA.$('#reset').onclick = () => {
      const m = JA.modal(`<h2>${t('Сбросить весь прогресс?', 'Reset all progress?')}</h2><p>${t('Очки, скины и нашивки удалятся с этого устройства.', 'Points, skins and patches will be removed from this device.')}</p>
        <div class="row"><button class="btn ghost" data-close>${t('Отмена', 'Cancel')}</button><button class="btn primary" id="r-ok">${t('Сбросить', 'Reset')}</button></div>`);
      JA.$('#r-ok', m.el).onclick = () => { JA.resetProfile(); m.close(); JA.go('home'); };
    };
  };

  /* ============================================================
     FOR ADULTS & JUDGES
     ============================================================ */
  JA.screens.adults = function () {
    const w = JA.weather;
    app().innerHTML = `${JA.UI.back()}
      <h1>🧑‍🏫 ${t('Для взрослых, учителей и судей', 'For adults, teachers & judges')}</h1>
      <section class="card prose">
        <h2>🎯 ${t('Что изучает ребёнок', 'What the child learns')}</h2>
        <ul>
          <li><b>${t('Компромиссы миссии', 'Mission trade-offs')}:</b> ${t('энергии всегда меньше, чем нужно. Каждый блок уходит либо в кислород, либо в еду, щит или науку — бесплатных решений нет.', 'there’s never enough power. Each block goes to oxygen, food, shielding or science — no choice is free.')}</li>
          <li><b>${t('Радиация', 'Radiation')}:</b> ${t('дозы на картах пропорциональны реальным измерениям (Луна 1,369 мЗв/сут, Марс 0,64, глубокий космос 1,84). Вспышки — реальные события NASA DONKI.', 'map doses are proportional to real measurements (Moon 1.369 mSv/day, Mars 0.64, deep space 1.84). Flares are real NASA DONKI events.')}</li>
          <li><b>${t('Задержка связи', 'Comms delay')}:</b> ${t('сообщения ЦУП приходят с задержкой (Луна — реальные 1,3 с; Марс — 4–24 мин, в игре ускорено и это подписано).', 'Mission Control messages arrive late (Moon — real 1.3 s; Mars — 4–24 min, sped up in game and labeled).')}</li>
          <li><b>${t('Навыки экипажа', 'Crew skills')}:</b> ${t('пауза перед действием, план «если — то», распознавание чувств, поддержка, проверка мысли, протокол «скажи — повтори — подтверди».', 'pause before acting, if–then plans, recognising feelings, support, thought checks, “say–repeat–confirm” protocol.')}</li>
        </ul>
        <h2>🧪 ${t('Как устроена симуляция', 'How the simulation works')}</h2>
        <table class="tbl"><tr><th>${t('Система', 'System')}</th><th>${t('Правило', 'Rule')}</th><th>${t('Реальная основа', 'Real basis')}</th></tr>
          <tr><td>🫁 ${t('Кислород', 'Oxygen')}</td><td>−12%/${t('сол', 'sol')}, +6% ${t('за блок', 'per block')}</td><td>${t('Жизнеобеспечение требует постоянной энергии; MOXIE получил O₂ из атмосферы Марса', 'Life support needs constant power; MOXIE made O₂ from Mars air')}</td></tr>
          <tr><td>🥔 ${t('Еда', 'Food')}</td><td>−10%/${t('сол', 'sol')}, +5% ${t('за блок', 'per block')}</td><td>${t('Veggie/APEX на МКС; данные о растениях в NASA OSDR', 'Veggie/APEX on ISS; plant data in NASA OSDR')}</td></tr>
          <tr><td>🛡️ ${t('Щит', 'Shield')}</td><td>−30% ${t('дозы за блок, макс. 90%', 'dose per block, max 90%')}</td><td>${t('Экранирование и убежища от солнечных частиц', 'Shielding and storm shelters against solar particles')}</td></tr>
          <tr><td>☢️ ${t('Доза', 'Dose')}</td><td>${t('пропорции по картам = реальные измерения × 7,3 (игровой масштаб)', 'map ratios = real measurements × 7.3 (game scale)')}</td><td>Curiosity RAD, Chang’e-4 LND, NASA-STD-3001</td></tr>
          <tr><td>⚡ ${t('Энергия', 'Power')}</td><td>${t('лунная ночь, пылевые бури, РИТЭГ на Европе', 'lunar night, dust storms, RTG on Europa')}</td><td>${t('Opportunity (буря 2018), лунные сутки 29,5 сут', 'Opportunity (2018 storm), lunar day 29.5 days')}</td></tr>
        </table>
        <p>${t('Числа в игре упрощены, чтобы ребёнок мог считать в уме, но соотношения и причинно-следственные связи взяты из реальных данных. Статичные данные никогда не выдаются за живые: на экране всегда есть метка LIVE или АРХИВ.', 'Game numbers are simplified so kids can do the maths in their heads, but ratios and cause–effect come from real data. Static data is never shown as live: the screen always says LIVE or ARCHIVE.')}</p>
        <h2>📡 ${t('Данные NASA сейчас', 'NASA data right now')}</h2>
        <p>DONKI: ${w.live ? `<span class="live-dot">LIVE</span> ${t('получено', 'fetched')} ${new Date(w.fetchedAt).toLocaleString()} · ${w.events.length} ${t('событий за 30 дней', 'events in 30 days')}` : `<span class="archive-dot">${t('АРХИВ', 'ARCHIVE')}</span> ${t('используется встроенный набор реальных событий', 'using the built-in set of real events')}`}</p>
        <div class="field"><span>🔑 ${t('Ключ NASA API (необязательно)', 'NASA API key (optional)')}</span>
          <div class="row"><input id="k-key" placeholder="DEMO_KEY" value="${esc(JA.nasaKey() === 'DEMO_KEY' ? '' : JA.nasaKey())}" autocomplete="off" style="max-width:360px">
          <button class="btn small" id="k-save">${t('Сохранить и обновить', 'Save & refresh')}</button></div>
          <small>${t('DEMO_KEY даёт около 30 запросов в час. Бесплатный ключ с api.nasa.gov — 1000 в час. Ключ хранится только в этом браузере.', 'DEMO_KEY allows ~30 requests per hour. A free key from api.nasa.gov allows 1,000/hour. The key stays in this browser.')}</small></div>
        <h2>💛 ${t('Принципы психологической безопасности', 'Psychological safety principles')}</h2>
        <ul>
          <li>${t('Нет гибели экипажа: при неудаче экипаж безопасно эвакуируется, а игра объясняет причину.', 'No crew deaths: on failure the crew safely evacuates and the game explains why.')}</li>
          <li>${t('Подсказки бесплатны и не снижают награду. Пауза и выход доступны всегда.', 'Hints are free and never reduce rewards. Pause and exit are always available.')}</li>
          <li>${t('Дыхательный «Маяк» добровольный, без баллов и без «измерения стресса».', 'The breathing Beacon is voluntary, unscored and never “measures stress”.')}</li>
          <li>${t('Обратная связь описывает действия («ты включил щит»), а не личность.', 'Feedback describes actions (“you raised the shield”), not personality.')}</li>
          <li>${t('Нет публичного рейтинга. Мультиплеер — кооперативный, чат только из готовых фраз (безопасно для детей).', 'No public ranking. Multiplayer is cooperative, chat uses preset phrases only (child-safe).')}</li>
          <li>${t('Нет регистрации и персональных данных: позывной и прогресс хранятся только в браузере.', 'No sign-up or personal data: call sign and progress stay in the browser.')}</li>
        </ul>
        <p class="tiny">${t('Тренажёр — образовательная игра. Он не оценивает готовность к реальному полёту и не заменяет психологическую помощь.', 'This trainer is an educational game. It doesn’t assess readiness for real spaceflight and doesn’t replace psychological support.')}</p>
        <h2>📚 ${t('Источники данных', 'Data sources')}</h2>
        <ol class="sources">${JA.SOURCES.map((s) => `<li><a href="${s.url}" target="_blank" rel="noopener">${esc(s.title)}</a><br><small>${esc(L(s.use))}</small></li>`).join('')}</ol>
      </section>`;
    JA.UI.bindCommon();
    JA.$('#k-save').onclick = async () => {
      JA.nasaKey(JA.$('#k-key').value.trim());
      JA.weather.status = 'loading';
      await JA.loadWeather();
      JA.toast(JA.weather.live && !JA.weather.stale ? t('✅ Живые данные NASA получены', '✅ Live NASA data received') : t('Не удалось получить живые данные', 'Could not get live data'));
      JA.rerender();
    };
  };
})();
