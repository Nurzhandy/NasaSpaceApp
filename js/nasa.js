/* ============================================================
   NASA DONKI live space weather (with honest archive fallback)
   API: https://api.nasa.gov  (CORS enabled)
   Replace DEMO_KEY with a free key from api.nasa.gov for more requests.
   ============================================================ */
(function () {
  // A free personal key (https://api.nasa.gov) allows 1,000 requests/hour; DEMO_KEY only ~30/hour per IP.
  const DEFAULT_KEY = 'DEMO_KEY';
  const KEY_STORE = 'ja-nasa-key';
  JA.nasaKey = function (set) {
    try {
      if (set !== undefined) { if (set) localStorage.setItem(KEY_STORE, set); else localStorage.removeItem(KEY_STORE); localStorage.removeItem(CACHE); }
      return localStorage.getItem(KEY_STORE) || DEFAULT_KEY;
    } catch (e) { return DEFAULT_KEY; }
  };
  const BASE = 'https://api.nasa.gov/DONKI/';
  const CACHE = 'ja-donki-v1';
  const TTL = 6 * 60 * 60 * 1000; // 6 hours (be gentle with the API rate limit)

  const fmt = (d) => d.toISOString().slice(0, 10);

  JA.weather = { status: 'loading', live: false, events: [], counts: {}, kpMax: null, fetchedAt: null };

  async function get(type, start, end) {
    const url = `${BASE}${type}?startDate=${start}&endDate=${end}&api_key=${encodeURIComponent(JA.nasaKey())}`;
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), 9000);
    try {
      const r = await fetch(url, { signal: ctrl.signal });
      if (r.status === 404) return [];          // DONKI answers 404 when a window has no events
      if (!r.ok) throw new Error('HTTP ' + r.status);
      const txt = await r.text();
      return txt ? JSON.parse(txt) : [];
    } finally { clearTimeout(timer); }
  }

  function flarePower(cls) {
    const k = (cls || 'C1').charAt(0).toUpperCase();
    const n = parseFloat((cls || '').slice(1)) || 1;
    if (k === 'X') return Math.min(1, 0.6 + n / 25);
    if (k === 'M') return 0.3 + n / 40;
    return 0.15;
  }

  function normalize(flr, gst, sep, cme) {
    const ev = [];
    let kpMax = null;
    flr.forEach((f) => ev.push({
      type: 'FLR', date: (f.peakTime || f.beginTime || '').slice(0, 10), power: flarePower(f.classType),
      label: JA.tt(`Солнечная вспышка класса ${f.classType}`, `Class ${f.classType} solar flare`), link: f.link,
    }));
    gst.forEach((g) => {
      const kp = Math.max(...(g.allKpIndex || []).map((k) => k.kpIndex || 0), 0);
      kpMax = Math.max(kpMax || 0, kp);
      ev.push({ type: 'GST', date: (g.startTime || '').slice(0, 10), power: JA.clamp((kp - 4) / 5, 0.2, 1),
        label: JA.tt(`Магнитная буря, индекс Kp ${kp}`, `Geomagnetic storm, Kp index ${kp}`), link: g.link });
    });
    sep.forEach((s) => ev.push({ type: 'SEP', date: (s.eventTime || '').slice(0, 10), power: 0.8,
      label: JA.tt('Поток солнечных энергичных частиц (SEP)', 'Solar energetic particle event (SEP)'), link: s.link }));
    const fast = cme.filter((c) => (c.cmeAnalyses || []).some((a) => (a.speed || 0) > 700));
    fast.forEach((c) => {
      const sp = Math.max(...c.cmeAnalyses.map((a) => a.speed || 0));
      ev.push({ type: 'CME', date: (c.startTime || '').slice(0, 10), power: JA.clamp(sp / 2500, 0.3, 0.9),
        label: JA.tt(`Выброс солнечной плазмы, ${Math.round(sp)} км/с`, `Coronal mass ejection, ${Math.round(sp)} km/s`), link: c.link });
    });
    ev.sort((a, b) => (a.date < b.date ? 1 : -1));
    return {
      events: ev, kpMax,
      counts: { FLR: flr.length, GST: gst.length, SEP: sep.length, CME: cme.length },
    };
  }

  JA.loadWeather = async function () {
    let cached = null;
    try {
      cached = JSON.parse(localStorage.getItem(CACHE) || 'null');
      if (cached && Date.now() - cached.fetchedAt < TTL) {
        Object.assign(JA.weather, cached, { status: 'ok', live: true });
        document.dispatchEvent(new CustomEvent('ja:weather'));
        return JA.weather;
      }
    } catch (e) { /* ignore */ }

    const end = new Date();
    const start = new Date(Date.now() - 30 * 864e5);
    try {
      const [flr, gst, sep, cme] = await Promise.all(
        ['FLR', 'GST', 'SEP', 'CME'].map((t) => get(t, fmt(start), fmt(end)).catch((e) => { if (t === 'FLR' || t === 'GST') throw e; return []; }))
      );
      const n = normalize(flr, gst, sep, cme);
      Object.assign(JA.weather, n, { status: 'ok', live: true, fetchedAt: Date.now(), range: [fmt(start), fmt(end)] });
      try { localStorage.setItem(CACHE, JSON.stringify({ events: n.events, counts: n.counts, kpMax: n.kpMax, fetchedAt: JA.weather.fetchedAt, range: JA.weather.range })); } catch (e) { /* ignore */ }
    } catch (e) {
      if (cached && cached.events) {
        // real DONKI data fetched earlier — still real, shown with its fetch date (never as "now")
        Object.assign(JA.weather, cached, { status: 'ok', live: true, stale: true });
      } else {
        Object.assign(JA.weather, { status: 'archive', live: false, events: JA.DONKI_ARCHIVE.slice(), counts: {}, kpMax: null });
      }
    }
    document.dispatchEvent(new CustomEvent('ja:weather'));
    return JA.weather;
  };

  /* Pick a real event for an in-game flare. Prefer strong live events,
     otherwise use the archive. Returns {event, live}. */
  JA.realFlare = function () {
    const live = JA.weather.live ? JA.weather.events.filter((e) => e.power >= 0.3) : [];
    if (live.length && Math.random() < 0.7) return { event: JA.pick(live.slice(0, 6)), live: true };
    return { event: JA.pick(JA.DONKI_ARCHIVE), live: false };
  };

  /* A simple, kid-readable risk level from the last 30 days of DONKI */
  JA.spaceRisk = function () {
    const w = JA.weather;
    if (!w.live) return null;
    const strong = w.events.filter((e) => e.power >= 0.6).length;
    const recent = w.events.filter((e) => Date.now() - new Date(e.date).getTime() < 7 * 864e5).length;
    const score = strong * 2 + recent + (w.kpMax >= 6 ? 2 : 0);
    if (score >= 6) return { level: 3, label: JA.tt('высокая', 'high'), color: '#ff6b6b' };
    if (score >= 2) return { level: 2, label: JA.tt('средняя', 'medium'), color: '#ffc94a' };
    return { level: 1, label: JA.tt('спокойная', 'calm'), color: '#4dd9a6' };
  };
})();
