/* ============================================================
   Boot: starfield background, live data, first screen
   ============================================================ */
(function () {
  // Starfield (respects reduced motion)
  const canvas = document.getElementById('stars');
  const ctx = canvas.getContext('2d');
  let stars = [];
  const still = () => matchMedia('(prefers-reduced-motion: reduce)').matches || !JA.profile.settings.motion;
  function resize() {
    canvas.width = innerWidth; canvas.height = innerHeight;
    stars = Array.from({ length: Math.min(220, Math.floor(innerWidth * innerHeight / 7000)) }, () => ({
      x: Math.random() * canvas.width, y: Math.random() * canvas.height, r: Math.random() * 1.6 + 0.2, s: Math.random() * 0.25 + 0.03, tw: Math.random() * 6,
    }));
  }
  function draw(time) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (const st of stars) {
      if (!still()) { st.y += st.s; if (st.y > canvas.height) st.y = 0; }
      ctx.globalAlpha = 0.5 + 0.5 * Math.sin(time / 900 + st.tw);
      ctx.fillStyle = '#fff';
      ctx.beginPath(); ctx.arc(st.x, st.y, st.r, 0, Math.PI * 2); ctx.fill();
    }
    requestAnimationFrame(draw);
  }
  addEventListener('resize', resize);
  resize(); requestAnimationFrame(draw);

  // Floating beacon button — always available (pause is never punished)
  const fab = document.getElementById('fab-beacon');
  fab.addEventListener('click', () => JA.beacon());

  document.documentElement.lang = JA.profile.lang;
  JA.go('home');
  JA.loadWeather();

  // keyboard: Esc closes the top modal
  addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { const m = document.querySelectorAll('.modal-wrap'); const last = m[m.length - 1]; if (last) { const c = last.querySelector('[data-close]'); if (c) c.click(); } }
  });
})();
