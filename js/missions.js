/* ============================================================
   Training center — short skill drills (from the research doc):
   checklist, rover re-route, robot debug, comms, mood remote,
   crew day, true-or-myth quiz, space beacon.
   ============================================================ */
(function () {
  const t = (...a) => JA.t(...a);
  const L = (o) => JA.L(o);
  const tt = JA.tt;
  const esc = JA.esc;
  const app = () => document.getElementById('app');
  const bind = (...a) => JA.bind(...a);

  JA.TRAININGS = [
    { id: 'checklist', icon: '✅', points: 20, name: tt('Проверка перед стартом', 'Pre-launch check'), skill: tt('Внимание и порядок действий', 'Focus and step order') },
    { id: 'rover', icon: '🛞', points: 30, name: tt('Марсоход меняет маршрут', 'Rover re-route'), skill: tt('Гибкость: новый план', 'Flexibility: a new plan') },
    { id: 'robot', icon: '🤖', points: 30, name: tt('Робот ошибся', 'Robot bug'), skill: tt('Найти причину ошибки', 'Find the cause of a mistake') },
    { id: 'comms', icon: '📻', points: 25, name: tt('Связь с экипажем', 'Crew comms'), skill: tt('Уточнить и подтвердить', 'Clarify and confirm') },
    { id: 'mood', icon: '🎛️', points: 20, name: tt('Пульт настроения', 'Mood remote'), skill: tt('Чувства и поддержка', 'Feelings and support') },
    { id: 'day', icon: '🗓️', points: 20, name: tt('День экипажа', 'Crew day'), skill: tt('Режим: сон, спорт, работа', 'Routine: sleep, sport, work') },
    { id: 'quiz', icon: '❓', points: 18, name: tt('Правда или миф', 'True or myth'), skill: tt('Факты NASA', 'NASA facts') },
    { id: 'beacon', icon: '🌟', points: 0, name: tt('Космический маяк', 'Space beacon'), skill: tt('Спокойное дыхание', 'Calm breathing') },
  ];

  function shell(tr, body) {
    app().innerHTML = `${JA.UI.back('solo', t('Тренировки', 'Training'))}
      <section class="card train">
        <div class="brief-head"><span class="big-ico">${tr.icon}</span><div><h1>${L(tr.name)}</h1><p class="muted">${L(tr.skill)}</p></div></div>
        <div id="tbody">${body}</div>
      </section>`;
    JA.UI.bindCommon();
  }

  function complete(tr, msg, pts = tr.points) {
    JA.trainingDone(tr.id);
    const body = JA.$('#tbody');
    body.insertAdjacentHTML('beforeend', `<div class="done-box"><h2>🎉 ${t('Тренировка пройдена!', 'Training complete!')}</h2><p>${esc(msg)}</p>
      <div class="row center"><button class="btn primary" id="again">🔁 ${t('Ещё раз (новое задание)', 'Again (new task)')}</button><button class="btn ghost" data-go="solo" data-arg="train">🎓 ${t('Другие тренировки', 'Other training')}</button></div></div>`);
    JA.UI.bindCommon(body);
    JA.$('#again').onclick = () => JA.go('train', tr.id);
    JA.addPoints(pts, L(tr.name));
    JA.speak(msg);
    JA.$('.done-box').scrollIntoView({ behavior: 'smooth', block: 'center' });
  }

  JA.screens.train = function (id) {
    const tr = JA.TRAININGS.find((x) => x.id === id);
    ({ checklist, rover, robot, comms, mood, day, quiz, beacon })[id](tr);
  };

  /* ---------- 1. Pre-launch checklist ---------- */
  function checklist(tr) {
    const list = JA.pick(JA.CHECKLISTS);
    const order = list.steps.map((s, i) => i);
    const shuffled = JA.shuffle(order);
    let next = 0, slips = 0;
    shell(tr, `
      <p>${t('Расставь шаги в правильном порядке: нажимай их по очереди. Сначала подумай — «Остановись, посмотри, выбери».', 'Put the steps in the right order: tap them one by one. Think first — “Stop, look, choose”.')} ${JA.UI.say(L(list.title))}</p>
      <h3>📋 ${L(list.title)}</h3>
      <ol class="check-done" id="done"></ol>
      <div class="check-pool">${shuffled.map((i) => `<button class="choice" data-step="${i}">${L(list.steps[i])}</button>`).join('')}</div>
      <p id="note" class="note" hidden></p>`);
    bind('[data-step]', 'click', (e, el) => {
      const i = +el.dataset.step;
      const note = JA.$('#note');
      if (i === next) {
        el.remove();
        JA.$('#done').insertAdjacentHTML('beforeend', `<li>✔ ${L(list.steps[i])}</li>`);
        next++; note.hidden = true; JA.sfx('good');
        if (next === order.length) complete(tr, slips === 0
          ? t('Все шаги по порядку с первого раза! Ты проверял каждое действие.', 'All steps in order on the first try! You checked every action.')
          : t(`Готово! Ты ${slips} раз(а) остановился и исправил порядок — именно так работают чек-листы.`, `Done! You stopped and fixed the order ${slips} time(s) — that’s exactly how checklists work.`));
      } else {
        slips++; JA.sfx('bad');
        note.hidden = false;
        note.textContent = t(`Подумай: что нужно сделать до шага «${L(list.steps[i])}»?`, `Think: what must happen before “${L(list.steps[i])}”?`);
      }
    });
  }

  /* ---------- 2. Rover re-route (grid path planning) ---------- */
  function rover(tr) {
    const W = 7, H = 6;
    const start = [0, JA.rand(0, H - 1)], goal = [W - 1, JA.rand(0, H - 1)];
    let rocks = new Set();
    const key = (x, y) => x + ',' + y;
    // random rocks that keep a path possible
    do {
      rocks = new Set();
      for (let i = 0; i < 9; i++) { const x = JA.rand(1, W - 2), y = JA.rand(0, H - 1); rocks.add(key(x, y)); }
    } while (!bfs(start, goal, rocks));
    let path = [start], phase = 1, hints = 0;
    const roverColor = (JA.SKINS.rover.find((r) => r.id === JA.profile.wear.rover) || JA.SKINS.rover[0]).color;

    function bfs(a, b, blocked) {
      const q = [[a]], seen = new Set([key(...a)]);
      while (q.length) {
        const p = q.shift(); const [x, y] = p[p.length - 1];
        if (x === b[0] && y === b[1]) return p;
        for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
          const nx = x + dx, ny = y + dy, k = key(nx, ny);
          if (nx < 0 || ny < 0 || nx >= W || ny >= H || blocked.has(k) || seen.has(k)) continue;
          seen.add(k); q.push(p.concat([[nx, ny]]));
        }
      }
      return null;
    }

    function draw(msg) {
      const onPath = new Set(path.map((p) => key(...p)));
      const last = path[path.length - 1];
      let cells = '';
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const k = key(x, y);
        const isS = x === start[0] && y === start[1], isG = x === goal[0] && y === goal[1];
        const adj = Math.abs(x - last[0]) + Math.abs(y - last[1]) === 1;
        cells += `<button class="cell ${rocks.has(k) ? 'rock' : ''} ${onPath.has(k) ? 'path' : ''} ${adj && !rocks.has(k) ? 'adj' : ''}" data-x="${x}" data-y="${y}" aria-label="${x},${y}">
          ${isS ? `<span class="rv" style="--rv:${roverColor}">🚙</span>` : isG ? '🧪' : rocks.has(k) ? '🪨' : ''}</button>`;
      }
      JA.$('#tbody').innerHTML = `
        <p>${phase === 1
          ? t('Проложи путь марсохода к образцу 🧪: нажимай на соседние клетки. Камни 🪨 объезжай.', 'Plan the rover’s path to the sample 🧪: tap neighbouring cells. Drive around rocks 🪨.')
          : t('⚠️ Оползень! На пути появились новые камни. Марсоход вернулся к последней безопасной точке. Построй новый маршрут.', '⚠️ Landslide! New rocks blocked the path. The rover backed up to the last safe spot. Build a new route.')} ${JA.UI.say(t('Проложи путь марсохода к образцу', 'Plan the rover path to the sample'))}</p>
        <div class="grid" style="--w:${W}">${cells}</div>
        ${msg ? `<p class="note">${msg}</p>` : ''}
        <div class="row"><button class="btn ghost" id="undo">↩ ${t('Шаг назад', 'Undo')}</button>
          <button class="btn ghost" id="hint">💡 ${t('Подсказка', 'Hint')}</button>
          <button class="btn primary" id="check">${t('Проверить маршрут', 'Check route')} ▶</button></div>`;
      JA.UI.bindCommon(JA.$('#tbody'));
      bind('.cell', 'click', (e, el) => {
        const x = +el.dataset.x, y = +el.dataset.y, k = key(x, y);
        const l = path[path.length - 1];
        if (rocks.has(k) || Math.abs(x - l[0]) + Math.abs(y - l[1]) !== 1) return;
        const idx = path.findIndex((p) => key(...p) === k);
        if (idx >= 0) path = path.slice(0, idx + 1); else path.push([x, y]);
        draw();
      });
      JA.$('#undo').onclick = () => { if (path.length > 1) path.pop(); draw(); };
      JA.$('#hint').onclick = () => {
        hints++;
        const p = bfs(path[path.length - 1], goal, rocks);
        if (p && p[1]) { path.push(p[1]); draw(t('Подсказка показала следующий шаг. Пользоваться подсказками — нормально!', 'The hint showed the next step. Using hints is fine!')); }
      };
      JA.$('#check').onclick = check;
    }

    function check() {
      const l = path[path.length - 1];
      if (l[0] !== goal[0] || l[1] !== goal[1]) return draw(t('Маршрут пока не доходит до образца 🧪. Продолжи путь.', 'The route doesn’t reach the sample 🧪 yet. Keep going.'));
      if (phase === 1) {
        // add a landslide on the path (but keep a solution possible)
        const mid = path.slice(2, -1);
        for (let tries = 0; tries < 40; tries++) {
          const r = JA.pick(mid.length ? mid : path.slice(1, -1));
          const test = new Set(rocks); test.add(key(...r));
          const n = JA.pick([[0, 1], [0, -1]]); const extra = key(r[0] + n[0], r[1] + n[1]);
          if (!(r[0] + n[0] === goal[0] && r[1] + n[1] === goal[1])) test.add(extra);
          if (bfs(start, goal, test)) {
            rocks = test;
            const cut = path.findIndex((p) => rocks.has(key(...p)));
            path = path.slice(0, Math.max(1, cut));
            phase = 2; JA.sfx('alarm');
            return draw();
          }
        }
      }
      JA.sfx('good');
      complete(tr, t('Ты заметил препятствие, изменил план и проверил новый путь. Так и работают операторы марсоходов!', 'You noticed the obstacle, changed the plan and checked the new path. That’s how rover drivers work!'));
    }
    shell(tr, '');
    draw();
  }

  /* ---------- 3. Robot made a mistake (debug a program) ---------- */
  function robot(tr) {
    const N = 5;
    const DIRS = { U: [0, -1, '⬆️'], R: [1, 0, '➡️'], D: [0, 1, '⬇️'], L: [-1, 0, '⬅️'] };
    let level = 1, prog, start, goal, fixes = 0;

    function makeLevel() {
      start = [0, N - 1];
      const len = level === 1 ? 5 : 7;
      // random walk to make a correct program
      let p, pos, correct;
      for (;;) {
        pos = start.slice(); correct = []; const seen = new Set([pos.join()]);
        let ok = true;
        for (let i = 0; i < len; i++) {
          const opts = Object.keys(DIRS).filter((d) => { const nx = pos[0] + DIRS[d][0], ny = pos[1] + DIRS[d][1]; return nx >= 0 && ny >= 0 && nx < N && ny < N && !seen.has(nx + ',' + ny); });
          if (!opts.length) { ok = false; break; }
          const d = JA.pick(opts); correct.push(d); pos = [pos[0] + DIRS[d][0], pos[1] + DIRS[d][1]]; seen.add(pos.join());
        }
        if (ok) break;
      }
      goal = pos;
      prog = correct.slice();
      const bug = JA.rand(1, prog.length - 1);
      const wrong = Object.keys(DIRS).filter((d) => d !== prog[bug]);
      prog[bug] = JA.pick(wrong);
      prog.bug = bug;
    }

    function run() {
      let pos = start.slice(); const trail = [pos.slice()];
      for (const c of prog) { pos = [JA.clamp(pos[0] + DIRS[c][0], 0, N - 1), JA.clamp(pos[1] + DIRS[c][1], 0, N - 1)]; trail.push(pos.slice()); }
      return trail;
    }

    function draw(msg, showTrail) {
      const trail = showTrail ? run() : [];
      const tk = new Set(trail.map((p) => p.join()));
      const end = trail[trail.length - 1];
      let cells = '';
      for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
        const isS = x === start[0] && y === start[1], isG = x === goal[0] && y === goal[1];
        const isEnd = end && end[0] === x && end[1] === y;
        cells += `<div class="cell ${tk.has(x + ',' + y) ? 'path' : ''}">${isEnd ? '🤖' : isS ? '🏁' : isG ? '🔋' : ''}${isEnd && isG ? '' : ''}</div>`;
      }
      JA.$('#tbody').innerHTML = `
        <p>${t(`Уровень ${level}/2. Робот БИП должен доехать до батареи 🔋, но в программе одна ошибка. Запусти, посмотри, где он свернул не туда, и нажми на неправильную команду, чтобы её поменять.`, `Level ${level}/2. BEEP must reach the battery 🔋, but one command is wrong. Run it, see where it went off track, and tap the wrong command to change it.`)}</p>
        <div class="robot-wrap"><div class="grid small" style="--w:${N}">${cells}</div>
        <div class="prog">${prog.map((c, i) => `<button class="cmd" data-cmd="${i}" aria-label="${c}">${i + 1}. ${DIRS[c][2]}</button>`).join('')}</div></div>
        ${msg ? `<p class="note">${msg}</p>` : ''}
        <div class="row"><button class="btn ghost" id="hint">💡 ${t('Подсказка', 'Hint')}</button><button class="btn primary" id="go">▶ ${t('Запустить робота', 'Run robot')}</button></div>`;
      bind('[data-cmd]', 'click', (e, el) => {
        const i = +el.dataset.cmd; const order = ['U', 'R', 'D', 'L'];
        prog[i] = order[(order.indexOf(prog[i]) + 1) % 4]; fixes++; draw(null, false);
      });
      JA.$('#hint').onclick = () => draw(t(`Сравни путь робота с картой шаг за шагом. Проверь команду №${prog.bug + 1}.`, `Compare the robot’s path with the map step by step. Check command #${prog.bug + 1}.`), true);
      JA.$('#go').onclick = () => {
        const tr2 = run(); const e = tr2[tr2.length - 1];
        if (e[0] === goal[0] && e[1] === goal[1]) {
          JA.sfx('good');
          if (level === 1) { level = 2; makeLevel(); draw(t('Получилось! Теперь новая, более длинная программа — без подсказки сначала.', 'It works! Now a new, longer program — try without a hint first.')); }
          else { draw(null, true); complete(tr, t('Ты запускал программу, искал, где робот свернул не туда, и исправлял причину. Это и есть отладка!', 'You ran the program, found where the robot went wrong and fixed the cause. That’s debugging!')); }
        } else { JA.sfx('bad'); draw(t('Робот не доехал. Посмотри на след: где он впервые ушёл с правильного пути?', 'The robot didn’t make it. Look at the trail: where did it first go off track?'), true); }
      };
    }
    makeLevel();
    shell(tr, '');
    draw();
  }

  /* ---------- 4. Crew comms (solo, scripted partner) ---------- */
  function comms(tr) {
    const SHAPES = [['●', tt('круг', 'circle')], ['■', tt('квадрат', 'square')], ['▲', tt('треугольник', 'triangle')]];
    const COLORS = [['#4f8cff', tt('синий', 'blue'), tt('синяя', 'blue')], ['#ff5e7e', tt('красный', 'red'), tt('красная', 'red')], ['#ffd24a', tt('жёлтый', 'yellow'), tt('жёлтая', 'yellow')]];
    let round = 1, clarifications = 0, misses = 0;
    let target, built, step, clarified;

    function newRound() {
      target = Array.from({ length: 4 }, () => [JA.rand(0, 2), JA.rand(0, 2)]);
      built = []; step = 0; clarified = false;
    }

    function partnerSays() {
      const [s, c] = target[step];
      // Round 1: messages are vague on purpose (only color) → kid should ask to clarify
      if (round === 1 && !clarified) return t(`БИП: «Деталь ${step + 1}: ${L(COLORS[c][2])}.»`, `BEEP: “Part ${step + 1}: the ${L(COLORS[c][1])} one.”`);
      return t(`БИП: «Деталь ${step + 1}: ${L(COLORS[c][1])} ${L(SHAPES[s][1])}.»`, `BEEP: “Part ${step + 1}: ${L(COLORS[c][1])} ${L(SHAPES[s][1])}.”`);
    }

    function draw(msg) {
      const parts = [];
      for (let s = 0; s < 3; s++) for (let c = 0; c < 3; c++) parts.push(`<button class="part" data-p="${s},${c}" style="color:${COLORS[c][0]}" aria-label="${L(COLORS[c][1])} ${L(SHAPES[s][1])}">${SHAPES[s][0]}</button>`);
      JA.$('#tbody').innerHTML = `
        <p>${round === 1
          ? t('Робот БИП видит чертёж антенны, а ты — детали. Собери антенну по его сообщениям. Если сообщение непонятное — уточни!', 'BEEP sees the antenna plan, you see the parts. Build it from BEEP’s messages. If a message is unclear — ask!')
          : t('Раунд 2: используй протокол «скажи — повтори — подтверди». Сначала повтори сообщение, потом ставь деталь.', 'Round 2: use “say – repeat – confirm”. Repeat the message first, then place the part.')}</p>
        <div class="radio-msg">📻 <span id="msg">${esc(partnerSays())}</span></div>
        <div class="row">
          <button class="chip" id="ask">❓ ${t('Уточни, пожалуйста: какая форма?', 'Please clarify: which shape?')}</button>
          ${round === 2 ? `<button class="chip" id="rep">🔁 ${t('Повторяю, как понял', 'Repeating what I heard')}</button>` : ''}
        </div>
        <div class="antenna">${Array.from({ length: 4 }, (_, i) => built[i] ? `<span style="color:${COLORS[built[i][1]][0]}">${SHAPES[built[i][0]][0]}</span>` : `<span class="slot">${i + 1}</span>`).join('')}</div>
        <div class="parts">${parts.join('')}</div>
        ${msg ? `<p class="note">${esc(msg)}</p>` : ''}`;
      JA.$('#ask').onclick = () => { JA.sfx('radio'); clarifications++; clarified = true; draw(t('Ты уточнил — и сообщение стало понятным. Отлично!', 'You asked — and the message became clear. Great!')); };
      const rep = JA.$('#rep');
      if (rep) rep.onclick = () => { JA.sfx('radio'); clarified = true; const [s, c] = target[step]; draw(t(`Ты: «Понял: ${L(COLORS[c][1])} ${L(SHAPES[s][1])}». БИП: «Подтверждаю!»`, `You: “Got it: ${L(COLORS[c][1])} ${L(SHAPES[s][1])}.” BEEP: “Confirmed!”`)); };
      bind('[data-p]', 'click', (e, el) => {
        const [s, c] = el.dataset.p.split(',').map(Number);
        const [ts, tc] = target[step];
        if (s === ts && c === tc) {
          built.push([s, c]); step++; clarified = round === 2 ? false : clarified; JA.sfx('good');
          if (round === 1) clarified = false;
          if (step === 4) {
            if (round === 1) { round = 2; newRound(); draw(t('Антенна собрана! Теперь новый чертёж.', 'Antenna built! Now a new plan.')); }
            else complete(tr, t(`Готово! Ты уточнял ${clarifications} раз(а). Уточнять и подтверждать — главный навык связи настоящих экипажей.`, `Done! You asked to clarify ${clarifications} time(s). Clarifying and confirming is a key real crew skill.`));
            return;
          }
          draw();
        } else {
          misses++; JA.sfx('bad');
          draw(t('Не та деталь. Ничего страшного — может, стоит уточнить сообщение?', 'Not that part. No problem — maybe ask to clarify the message?'));
        }
      });
    }
    newRound();
    shell(tr, '');
    draw();
  }

  /* ---------- 5. Mood remote ---------- */
  function mood(tr) {
    const items = JA.shuffle(JA.MOODS).slice(0, 3);
    let i = 0;
    function draw() {
      const m = items[i];
      const who = JA.CREW.find((c) => c.id === m.who);
      JA.$('#tbody').innerHTML = `
        <p class="muted">${i + 1} / ${items.length}</p>
        <div class="story"><span class="big-ico">${who.face}</span><p>${esc(L(m.text))} ${JA.UI.say(L(m.text))}</p></div>
        <p><b>${t(`Что, возможно, чувствует ${L(who.name)}?`, `What might ${L(who.name)} be feeling?`)}</b> ${t('(правильных ответов может быть несколько)', '(there can be several right answers)')}</p>
        <div class="feel-row">${JA.FEELINGS.map((f) => `<button class="chip" data-f="${f.id}">${f.icon} ${L(f.name)}</button>`).join('')}</div>
        <div id="step2"></div>`;
      bind('[data-f]', 'click', (e, el) => {
        const ok = m.ok.includes(el.dataset.f);
        JA.$$('[data-f]').forEach((b) => { b.disabled = true; b.classList.toggle('on', b === el); });
        JA.$('#step2').innerHTML = `<p class="note">${ok
          ? t('Да, похоже на то. Ты связал чувство с тем, что случилось.', 'Yes, it seems so. You linked the feeling to what happened.')
          : t(`Возможно! А ещё подсказки в истории говорят о: ${m.ok.map((id) => L(JA.FEELINGS.find((x) => x.id === id).name)).join(', ')}.`, `Maybe! Clues in the story also point to: ${m.ok.map((id) => L(JA.FEELINGS.find((x) => x.id === id).name)).join(', ')}.`)}</p>
          <p><b>${t('Как можно поддержать?', 'How could you help?')}</b></p>
          <div class="choices">${m.support.map((s, k) => `<button class="choice" data-s="${k}">${L(s)}</button>`).join('')}</div>`;
        bind('[data-s]', 'click', () => {
          JA.sfx('good');
          i++;
          if (i < items.length) draw();
          else { JA.award('helper'); complete(tr, t('Ты замечал чувства экипажа и выбирал способ помочь. Поддержка — важная часть любой миссии.', 'You noticed the crew’s feelings and chose ways to help. Support is a key part of every mission.')); }
        });
      });
    }
    shell(tr, '');
    draw();
  }

  /* ---------- 6. Crew day (routine planning) ---------- */
  function day(tr) {
    const acts = [
      { k: 'sleep', icon: '😴', name: tt('Сон', 'Sleep'), v: 7 },
      { k: 'study', icon: '🔬', name: tt('Учёба и исследования', 'Study & research'), v: 9 },
      { k: 'sport', icon: '🏃', name: tt('Спорт', 'Exercise'), v: 0 },
      { k: 'meals', icon: '🍲', name: tt('Еда', 'Meals'), v: 2 },
      { k: 'free', icon: '🎨', name: tt('Отдых и игры', 'Free time & play'), v: 6 },
    ];
    function total() { return acts.reduce((a, x) => a + x.v, 0); }
    function feedback() {
      const f = [];
      const g = (k) => acts.find((a) => a.k === k).v;
      f.push(g('sleep') >= 9 && g('sleep') <= 12 ? ['ok', t('😴 Сон 9–12 часов — как советуют врачи детям 6–12 лет (CDC).', '😴 9–12 hours of sleep — what doctors advise for kids 6–12 (CDC).')] : ['no', t('😴 Детям 6–12 лет советуют спать 9–12 часов (CDC).', '😴 Kids 6–12 are advised to sleep 9–12 hours (CDC).')]);
      f.push(g('sport') >= 1 ? ['ok', t('🏃 Есть спорт! Астронавты на МКС тренируются около 2 часов в день, чтобы мышцы и кости оставались сильными.', '🏃 Exercise included! ISS astronauts train about 2 hours a day to keep muscles and bones strong.')] : ['no', t('🏃 Добавь спорт: в космосе без него мышцы и кости слабеют.', '🏃 Add exercise: in space, muscles and bones weaken without it.')]);
      f.push(g('meals') >= 2 ? ['ok', t('🍲 Время на еду есть.', '🍲 There’s time for meals.')] : ['no', t('🍲 Нужно хотя бы 2 часа на еду в течение дня.', '🍲 Plan at least 2 hours for meals.')]);
      f.push(g('free') >= 1 ? ['ok', t('🎨 Отдых помогает экипажу не уставать.', '🎨 Free time keeps the crew from burning out.')] : ['no', t('🎨 Без отдыха экипаж быстро устаёт.', '🎨 Without rest the crew gets tired fast.')]);
      f.push(g('study') >= 3 ? ['ok', t('🔬 На учёбу и науку время есть.', '🔬 There’s time for study and science.')] : ['no', t('🔬 Миссии нужна и работа — хотя бы 3 часа.', '🔬 The mission needs work too — at least 3 hours.')]);
      return f;
    }
    function draw() {
      const tot = total();
      const fb = feedback();
      JA.$('#tbody').innerHTML = `
        <p>${t('Спланируй сутки юного астронавта — ровно 24 часа. Сон, спорт, еда, учёба и отдых: всем нужно время!', 'Plan a junior astronaut’s day — exactly 24 hours. Sleep, exercise, meals, study and play all need time!')}</p>
        <div class="day-bar">${acts.map((a) => `<i style="flex:${a.v}" title="${L(a.name)}">${a.v ? a.icon : ''}</i>`).join('')}</div>
        <p class="center"><b class="${tot === 24 ? 'ok-text' : 'warn-text'}">${tot} / 24 ${t('ч', 'h')}</b></p>
        ${acts.map((a) => `<div class="alloc-row"><span class="a-ico">${a.icon}</span><div class="a-name"><b>${L(a.name)}</b></div>
          <div class="a-ctrl"><button class="step" data-d="${a.k}" ${a.v <= 0 ? 'disabled' : ''}>−</button><b class="hours">${a.v} ${t('ч', 'h')}</b><button class="step" data-i="${a.k}" ${tot >= 24 ? 'disabled' : ''}>+</button></div></div>`).join('')}
        <ul class="fb">${fb.map(([c, x]) => `<li class="${c}">${c === 'ok' ? '✔' : '•'} ${esc(x)}</li>`).join('')}</ul>
        <button class="btn primary" id="ok" ${tot !== 24 ? 'disabled' : ''}>${t('Утвердить расписание', 'Approve schedule')}</button>`;
      bind('[data-i]', 'click', (e, el) => { acts.find((a) => a.k === el.dataset.i).v++; draw(); });
      bind('[data-d]', 'click', (e, el) => { acts.find((a) => a.k === el.dataset.d).v--; draw(); });
      JA.$('#ok').onclick = () => {
        const good = feedback().filter((x) => x[0] === 'ok').length;
        if (good < 5) { JA.toast(t('Почти! Посмотри на пункты с точкой • ниже.', 'Almost! Check the items marked • below.')); JA.sfx('bad'); return; }
        complete(tr, t('Сбалансированный день! Отдых и сон — такая же часть миссии, как работа.', 'A balanced day! Rest and sleep are as much part of the mission as work.'));
      };
    }
    shell(tr, '');
    draw();
  }

  /* ---------- 7. True or myth ---------- */
  function quiz(tr) {
    const qs = JA.shuffle(JA.QUIZ).slice(0, 6);
    let i = 0, right = 0;
    function draw() {
      const q = qs[i];
      JA.$('#tbody').innerHTML = `
        <p class="muted">${i + 1} / ${qs.length}</p>
        <div class="story"><span class="big-ico">❓</span><p class="q">${esc(L(q.q))} ${JA.UI.say(L(q.q))}</p></div>
        <div class="row center"><button class="btn big primary" data-a="1">✅ ${t('Правда', 'True')}</button><button class="btn big" data-a="0">🙅 ${t('Миф', 'Myth')}</button></div>
        <div id="why"></div>`;
      bind('[data-a]', 'click', (e, el) => {
        const ok = (el.dataset.a === '1') === q.a;
        if (ok) right++;
        JA.sfx(ok ? 'good' : 'bad');
        JA.$$('[data-a]').forEach((b) => (b.disabled = true));
        JA.$('#why').innerHTML = `<p class="note">${ok ? '🎉 ' + t('Верно!', 'Correct!') : '🤔 ' + t('Не совсем.', 'Not quite.')} ${esc(L(q.why))}</p>
          <button class="btn primary" id="next">${i < qs.length - 1 ? t('Дальше', 'Next') : t('Итог', 'Finish')} ▶</button>`;
        JA.speak(L(q.why));
        JA.$('#next').onclick = () => { i++; if (i < qs.length) draw(); else complete(tr, t(`Правильных ответов: ${right} из ${qs.length}. Каждый факт — из материалов NASA.`, `Correct answers: ${right} of ${qs.length}. Every fact comes from NASA material.`), right * 3); };
      });
    }
    shell(tr, '');
    draw();
  }

  /* ---------- 8. Beacon ---------- */
  function beacon(tr) {
    shell(tr, `<p>${t('Космический маяк — способ немного успокоиться перед трудным делом. Он доступен в каждой миссии по кнопке 🌟.', 'The Space Beacon helps you calm down before something hard. It’s in every mission under the 🌟 button.')}</p>
      <button class="btn big primary" id="open">🌟 ${t('Открыть маяк', 'Open the beacon')}</button>`);
    JA.$('#open').onclick = () => JA.beacon();
  }
})();
