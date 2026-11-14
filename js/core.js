/* ============================================================
   Core: language, profile (local save), sound, speech, helpers
   ============================================================ */
(function () {
  const KEY = 'ja-profile-v1';

  const DEFAULT = {
    name: '',
    lang: 'ru',
    age: 'pilot',            // 'cadet' (7–9) | 'pilot' (10–12)
    total: 0,                // lifetime points (unlock maps) — never decreases
    stars: 0,                // spendable balance (buy skins)
    owned: { suit: ['white'], visor: ['blue'], extra: ['none'], rover: ['red'] },
    wear: { suit: 'white', visor: 'blue', extra: 'none', rover: 'red' },
    badges: [],
    trainings: {},           // id -> times completed
    log: [],                 // mission log (local only)
    best: {},                // mapId -> best stars
    lastFailed: false,
    settings: { sound: true, voice: false, motion: true, big: false },
  };

  function load() {
    try {
      const raw = localStorage.getItem(KEY);
      if (!raw) return structuredClone(DEFAULT);
      const p = JSON.parse(raw);
      return Object.assign(structuredClone(DEFAULT), p, {
        owned: Object.assign(structuredClone(DEFAULT.owned), p.owned || {}),
        wear: Object.assign(structuredClone(DEFAULT.wear), p.wear || {}),
        settings: Object.assign(structuredClone(DEFAULT.settings), p.settings || {}),
      });
    } catch (e) { return structuredClone(DEFAULT); }
  }

  JA.profile = load();
  JA.save = function () {
    try { localStorage.setItem(KEY, JSON.stringify(JA.profile)); } catch (e) { /* private mode: progress lives in memory */ }
  };
  JA.resetProfile = function () {
    JA.profile = structuredClone(DEFAULT);
    JA.save();
  };

  /* ---------- language ---------- */
  JA.t = (ru, en) => (JA.profile.lang === 'en' ? en : ru);
  JA.L = (o) => (o == null ? '' : typeof o === 'string' ? o : (o[JA.profile.lang] || o.ru));

  /* ---------- points & badges ---------- */
  JA.addPoints = function (n, reason) {
    n = Math.max(0, Math.round(n));
    if (!n) return 0;
    JA.profile.total += n;
    JA.profile.stars += n;
    JA.save();
    JA.toast(`+${n} ⭐ ${reason || ''}`);
    JA.sfx('coin');
    return n;
  };
  JA.award = function (id) {
    if (JA.profile.badges.includes(id)) return false;
    const b = JA.BADGES.find((x) => x.id === id);
    if (!b) return false;
    JA.profile.badges.push(id);
    JA.save();
    setTimeout(() => { JA.toast(`${b.icon} ${JA.t('Новая нашивка', 'New patch')}: ${JA.L(b.name)}`, 3200); JA.sfx('badge'); }, 600);
    return true;
  };
  JA.trainingDone = function (id) {
    JA.profile.trainings[id] = (JA.profile.trainings[id] || 0) + 1;
    JA.save();
    if (Object.keys(JA.profile.trainings).filter((k) => k !== 'beacon').length >= 3) JA.award('trainer');
  };
  JA.unlocked = (map) => JA.profile.total >= map.need;

  /* ---------- toast ---------- */
  JA.toast = function (msg, ms = 2200) {
    let box = document.getElementById('toasts');
    if (!box) { box = document.createElement('div'); box.id = 'toasts'; box.setAttribute('aria-live', 'polite'); document.body.appendChild(box); }
    const el = document.createElement('div');
    el.className = 'toast';
    el.textContent = msg;
    box.appendChild(el);
    setTimeout(() => { el.classList.add('out'); setTimeout(() => el.remove(), 400); }, ms);
  };

  /* ---------- sound (WebAudio synth, no files) ---------- */
  let ctx = null;
  function tone(freq, dur, type = 'sine', vol = 0.08, delay = 0) {
    if (!JA.profile.settings.sound) return;
    try {
      ctx = ctx || new (window.AudioContext || window.webkitAudioContext)();
      const o = ctx.createOscillator(), g = ctx.createGain();
      o.type = type; o.frequency.value = freq;
      g.gain.setValueAtTime(vol, ctx.currentTime + delay);
      g.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + delay + dur);
      o.connect(g); g.connect(ctx.destination);
      o.start(ctx.currentTime + delay); o.stop(ctx.currentTime + delay + dur + 0.05);
    } catch (e) { /* audio not available */ }
  }
  JA.sfx = function (name) {
    const s = {
      click: () => tone(660, 0.06, 'square', 0.03),
      coin: () => { tone(880, 0.1, 'square', 0.04); tone(1320, 0.15, 'square', 0.04, 0.08); },
      badge: () => [523, 659, 784, 1047].forEach((f, i) => tone(f, 0.18, 'triangle', 0.06, i * 0.1)),
      good: () => { tone(523, 0.12, 'triangle'); tone(784, 0.2, 'triangle', 0.08, 0.1); },
      bad: () => { tone(300, 0.2, 'sawtooth', 0.04); tone(220, 0.3, 'sawtooth', 0.04, 0.15); },
      alarm: () => [0, 0.25, 0.5].forEach((d) => tone(880, 0.15, 'square', 0.04, d)),
      radio: () => { tone(1200, 0.05, 'square', 0.03); tone(900, 0.05, 'square', 0.03, 0.07); },
      whoosh: () => tone(180, 0.5, 'sine', 0.06),
    }[name];
    if (s) s();
  };

  /* ---------- speech (read aloud for young readers) ---------- */
  JA.speak = function (text, force) {
    if (!('speechSynthesis' in window)) return;
    if (!force && !JA.profile.settings.voice) return;
    try {
      speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(String(text).replace(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/gu, ''));
      u.lang = JA.profile.lang === 'en' ? 'en-US' : 'ru-RU';
      u.rate = 0.95;
      speechSynthesis.speak(u);
    } catch (e) { /* ignore */ }
  };

  /* ---------- helpers ---------- */
  JA.esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  JA.rand = (a, b) => a + Math.floor(Math.random() * (b - a + 1));
  JA.pick = (arr) => arr[Math.floor(Math.random() * arr.length)];
  JA.shuffle = (arr) => { const a = arr.slice(); for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a; };
  JA.clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  JA.$ = (s, r = document) => r.querySelector(s);
  JA.$$ = (s, r = document) => Array.from(r.querySelectorAll(s));

  /* ---------- astronaut avatar (SVG, skinnable) ---------- */
  JA.avatar = function (wear = JA.profile.wear, size = 120) {
    const suit = JA.SKINS.suit.find((s) => s.id === wear.suit) || JA.SKINS.suit[0];
    const visor = JA.SKINS.visor.find((s) => s.id === wear.visor) || JA.SKINS.visor[0];
    const extra = JA.SKINS.extra.find((s) => s.id === wear.extra) || JA.SKINS.extra[0];
    const id = 'g' + Math.random().toString(36).slice(2, 7);
    const fill = suit.color === 'rainbow' ? `url(#${id})` : suit.color;
    return `<svg class="avatar" width="${size}" height="${size}" viewBox="0 0 120 120" role="img" aria-label="${JA.t('Твой астронавт', 'Your astronaut')}">
      <defs><linearGradient id="${id}" x1="0" x2="1" y1="0" y2="1">
        <stop offset="0" stop-color="#ff5e7e"/><stop offset=".33" stop-color="#ffd24a"/><stop offset=".66" stop-color="#4dd9a6"/><stop offset="1" stop-color="#6b8cff"/></linearGradient></defs>
      <rect x="30" y="64" width="60" height="44" rx="18" fill="${fill}" stroke="#1b2140" stroke-width="3"/>
      <rect x="47" y="76" width="26" height="14" rx="4" fill="#1b2140" opacity=".25"/>
      <circle cx="54" cy="83" r="3" fill="#ff5e7e"/><circle cx="64" cy="83" r="3" fill="#4dd9a6"/>
      <circle cx="60" cy="44" r="30" fill="${fill}" stroke="#1b2140" stroke-width="3"/>
      <rect x="38" y="30" width="44" height="30" rx="14" fill="${visor.color}" stroke="#1b2140" stroke-width="3"/>
      <path d="M46 36 q6 -4 12 -2" stroke="#fff" stroke-width="4" stroke-linecap="round" fill="none" opacity=".8"/>
      ${extra.icon ? `<text x="88" y="26" font-size="26" text-anchor="middle">${extra.icon}</text>` : ''}
    </svg>`;
  };
})();
