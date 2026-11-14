/* ============================================================
   Outpost simulation — pure & serializable (used by single
   player and multiplayer). Every choice has a cost.

   Each sol:  startSol()  ->  choose() (if event)  ->  resolve(alloc)
   Power blocks go to: ls (life support → O₂), green (greenhouse → food),
   shield (radiation), sci (science). Unused blocks charge the battery.
   ============================================================ */
(function () {
  const tt = JA.tt;
  const O2_USE = 12, O2_PER = 6;       // % per sol
  const FOOD_USE = 10, FOOD_PER = 5;
  const SHIELD_PER = 0.3, SHIELD_MAX = 3;
  const BATTERY_MAX = 10;

  const mapById = (id) => JA.MAPS.find((m) => m.id === id);

  /* ---------- event library ---------- */
  const EVENTS = {
    flare: {
      icon: '☀️', alarm: true,
      title: tt('Солнечная вспышка!', 'Solar flare!'),
      text: (s, d) => tt(
        `Солнце выбросило поток заряженных частиц. Это реальное событие из базы NASA DONKI: «${d.label.ru}» (${d.date})${d.live ? ', недавнее — живые данные' : ', архив'}. Щит и убежище снижают дозу.`,
        `The Sun blasted out charged particles. This is a real event from NASA DONKI: “${d.label.en}” (${d.date})${d.live ? ', recent — live data' : ', archive'}. Shields and the shelter cut the dose.`),
      choices: [
        { label: tt('🏠 Спрятаться в убежище', '🏠 Hide in the shelter'), cost: tt('Наука сегодня = 0', 'No science today'), good: tt('Доза от вспышки −85%', 'Flare dose −85%'),
          apply: (s, m, d) => { m.sciMult = 0; m.spike = d.spike * 0.15; } },
        { label: tt('🔬 Продолжить работу', '🔬 Keep working'), cost: tt('Вся доза вспышки (щит помогает)', 'Full flare dose (shields help)'), good: tt('Наука как обычно', 'Science as usual'),
          apply: (s, m, d) => { m.spike = d.spike; } },
      ],
    },
    flareEarth: {
      icon: '☀️',
      title: tt('Вспышка на Солнце', 'Solar flare'),
      text: (s, d) => tt(
        `Реальное событие DONKI: «${d.label.ru}» (${d.date}). На Земле атмосфера и магнитное поле защищают нас, поэтому доза не растёт. Но радиосвязь ненадолго пропала!`,
        `Real DONKI event: “${d.label.en}” (${d.date}). On Earth the atmosphere and magnetic field protect us, so the dose doesn’t rise. But the radio dropped out for a while!`),
      choices: [
        { label: tt('📝 Записать наблюдение', '📝 Record the observation'), cost: tt('−1 блок энергии', '−1 power block'), good: tt('+2 науки', '+2 science'),
          apply: (s, m) => { m.genDelta -= 1; m.sciAdd += 2; } },
        { label: tt('🔧 Проверить антенну', '🔧 Check the antenna'), cost: tt('Наука −50% сегодня', 'Science −50% today'), good: tt('+10 настроения: связь стабильна', '+10 mood: comms stable'),
          apply: (s, m) => { m.sciMult *= 0.5; m.morale += 10; } },
      ],
    },
    dust: {
      icon: '🌪️', alarm: true,
      title: tt('Пылевая буря!', 'Dust storm!'),
      text: () => tt(
        'Пыль закрыла Солнце на 3 сола — солнечные панели дают намного меньше энергии. В 2018 году такая буря оставила ровер Opportunity без энергии.',
        'Dust blocks the Sun for 3 sols — solar panels make much less power. In 2018 a storm like this left the Opportunity rover without power.'),
      choices: [
        { label: tt('🧹 Выйти и чистить панели', '🧹 Go out and clean panels'), cost: tt('+3% дозы (выход наружу)', '+3% dose (going outside)'), good: tt('Энергии в бурю больше', 'More power during the storm'),
          apply: (s, m) => { m.doseAdd += 3; s.effects.push({ id: 'storm', until: s.sol + 2, genMult: 0.7 }); s.flags.storm = true; } },
        { label: tt('🔋 Режим экономии', '🔋 Power-saving mode'), cost: tt('Наука −50% на 3 сола', 'Science −50% for 3 sols'), good: tt('Без выхода наружу', 'No going outside'),
          apply: (s) => { s.effects.push({ id: 'storm', until: s.sol + 2, genMult: 0.45, sciMult: 0.5 }); s.flags.storm = true; } },
      ],
    },
    failure: {
      icon: '🔧', alarm: true,
      title: tt('Поломка генератора кислорода', 'Oxygen generator failure'),
      text: () => tt('Генератор кислорода барахлит. Можно чинить сразу или работать вполсилы.', 'The oxygen generator is glitching. Fix it now or run at half power.'),
      choices: [
        { label: tt('🔧 Чинить сейчас', '🔧 Fix it now'), cost: tt('−2 блока энергии сегодня', '−2 power blocks today'), good: tt('Всё работает', 'Everything works'),
          apply: (s, m) => { m.genDelta -= 2; } },
        { label: tt('⏳ Чинить потом', '⏳ Fix later'), cost: tt('Кислород вполсилы 3 сола', 'Half oxygen for 3 sols'), good: tt('Энергия сохранена', 'Power saved'),
          apply: (s) => { s.effects.push({ id: 'broken', until: s.sol + 2, lsMult: 0.5 }); } },
      ],
    },
    leak: {
      icon: '☄️', alarm: true,
      title: tt('Удар микрометеорита', 'Micrometeorite hit'),
      text: () => tt('Крошечный камешек пробил обшивку — медленно уходит воздух.', 'A tiny space pebble punched the hull — air is slowly leaking.'),
      choices: [
        { label: tt('🧑‍🚀 Заделать снаружи', '🧑‍🚀 Patch from outside'), cost: tt('+3% дозы и −1 блок', '+3% dose and −1 block'), good: tt('Утечка устранена', 'Leak fixed'),
          apply: (s, m) => { m.doseAdd += 3; m.genDelta -= 1; } },
        { label: tt('🩹 Временная заплатка', '🩹 Temporary patch'), cost: tt('−4% кислорода за сол, 3 сола', '−4% oxygen per sol for 3 sols'), good: tt('Без выхода наружу', 'No going outside'),
          apply: (s) => { s.effects.push({ id: 'leak', until: s.sol + 2, o2Add: -4 }); } },
      ],
    },
    harvest: {
      icon: '🥬',
      title: tt('Урожай в оранжерее!', 'Greenhouse harvest!'),
      text: () => tt('Салат вырос! Как в эксперименте NASA Veggie: в 2015 году астронавты на МКС впервые съели салат, выращенный в космосе. А ещё растения делают кислород.',
                     'The lettuce grew! Like NASA’s Veggie experiment: in 2015 ISS astronauts first ate lettuce grown in space. Plants also make oxygen.'),
      choices: [
        { label: tt('🥗 Съесть сейчас', '🥗 Eat it now'), cost: tt('Запасы семян −', 'Uses seed stock'), good: tt('+12% еды, +10 настроения', '+12% food, +10 mood'),
          apply: (s, m) => { m.foodAdd += 12; m.morale += 10; m.o2Add += 3; } },
        { label: tt('🧪 Отдать на исследование', '🧪 Send for research'), cost: tt('Еды меньше', 'Less food'), good: tt('+2 науки', '+2 science'),
          apply: (s, m) => { m.foodAdd += 4; m.sciAdd += 2; m.o2Add += 3; } },
      ],
    },
    ice: {
      icon: '🧊',
      title: tt('Найден водяной лёд!', 'Water ice found!'),
      text: () => tt('В тёмном кратере нашёлся лёд. Если растопить его и разложить воду током (электролиз), получится кислород!', 'There’s ice in a dark crater. Melt it and split the water with electricity (electrolysis) to get oxygen!'),
      choices: [
        { label: tt('⛏️ Добыть и сделать O₂', '⛏️ Mine it, make O₂'), cost: tt('−2 блока энергии', '−2 power blocks'), good: tt('+18% кислорода', '+18% oxygen'),
          apply: (s, m) => { m.genDelta -= 2; m.o2Add += 18; } },
        { label: tt('🗺️ Нанести на карту', '🗺️ Put it on the map'), cost: tt('Кислород не получен', 'No oxygen now'), good: tt('+2 науки', '+2 science'),
          apply: (s, m) => { m.sciAdd += 2; } },
      ],
    },
    discovery: {
      icon: '🪨',
      title: tt('Необычный камень!', 'A strange rock!'),
      text: () => tt('БИП нашёл камень со странными слоями. Возможно, здесь когда-то была вода.', 'BEEP found a rock with odd layers. Maybe water was here long ago.'),
      choices: [
        { label: tt('🔬 Изучить подробно', '🔬 Study it closely'), cost: tt('−2 блока энергии', '−2 power blocks'), good: tt('+3 науки', '+3 science'),
          apply: (s, m) => { m.genDelta -= 2; m.sciAdd += 3; } },
        { label: tt('📸 Только сфотографировать', '📸 Just take a photo'), cost: tt('Меньше открытий', 'Fewer discoveries'), good: tt('+1 наука, без затрат', '+1 science, free'),
          apply: (s, m) => { m.sciAdd += 1; } },
      ],
    },
    supply: {
      icon: '📦',
      title: tt('Прибыл грузовой корабль', 'A cargo ship arrived'),
      text: () => tt('С Земли прилетели еда и запчасти. Разгрузка требует энергии для манипулятора.', 'Food and spare parts arrived from Earth. Unloading needs power for the robot arm.'),
      choices: [
        { label: tt('🦾 Разгрузить всё', '🦾 Unload everything'), cost: tt('−2 блока энергии', '−2 power blocks'), good: tt('+20% еды, +8% O₂', '+20% food, +8% O₂'),
          apply: (s, m) => { m.genDelta -= 2; m.foodAdd += 20; m.o2Add += 8; } },
        { label: tt('🤲 Взять самое нужное', '🤲 Take only essentials'), cost: tt('Меньше припасов', 'Fewer supplies'), good: tt('+8% еды, без затрат', '+8% food, free'),
          apply: (s, m) => { m.foodAdd += 8; } },
      ],
    },
    conjunction: {
      icon: '🌞',
      title: tt('Солнечное соединение', 'Solar conjunction'),
      text: () => tt('Солнце оказалось между Землёй и Марсом и глушит радиосигнал. Это бывает примерно раз в 2 года. Два сола Центр управления не сможет предупреждать о событиях.',
                     'The Sun is between Earth and Mars and blocks radio. This happens about every 2 years. For two sols, Mission Control can’t warn you about events.'),
      choices: [
        { label: tt('📋 Работать по плану «если — то»', '📋 Follow our “if–then” plan'), cost: tt('Прогнозов нет 2 сола', 'No forecasts for 2 sols'), good: tt('+10 настроения: у нас есть план', '+10 mood: we have a plan'),
          apply: (s, m) => { m.morale += 10; s.effects.push({ id: 'silence', until: s.sol + 1 }); } },
        { label: tt('🔋 Копить энергию на всякий случай', '🔋 Save power just in case'), cost: tt('Наука −50% сегодня', 'Science −50% today'), good: tt('+2 блока в батарею', '+2 blocks to battery'),
          apply: (s, m) => { m.sciMult *= 0.5; s.battery = Math.min(BATTERY_MAX, s.battery + 2); s.effects.push({ id: 'silence', until: s.sol + 1 }); } },
      ],
    },
    emotion: {
      icon: '💬', emotion: true,
      title: tt('Экипажу нужна поддержка', 'A crewmate needs support'),
      text: (s, d) => JA.MOODS[d.mood].text,
      choices: [ // labels filled from the mood's support list
        { cost: tt('−1 блок (связь с Землёй)', '−1 block (call Earth)'), apply: (s, m) => { m.genDelta -= 1; m.morale += 20; } },
        { cost: tt('Наука −50% сегодня', 'Science −50% today'), apply: (s, m) => { m.sciMult *= 0.5; m.morale += 20; } },
        { cost: tt('−3% кислорода (дольше не спали)', '−3% oxygen (stayed up longer)'), apply: (s, m) => { m.o2Add -= 3; m.morale += 15; } },
      ],
    },
  };
  JA.EVENTS = EVENTS;

  /* ---------- create a new run ---------- */
  function create(mapId, age, opts = {}) {
    const map = mapById(mapId);
    const cadet = age === 'cadet';
    const s = {
      mapId, age, sol: 1, sols: map.sols,
      battery: map.battery, o2: 65, food: cadet ? 100 : 65, dose: 0, science: 0, morale: 70,
      goal: Math.round(map.sols * (cadet ? 0.8 : 1.2)),
      schedule: {}, effects: [], status: 'playing', today: null, forecast: null,
      flags: {}, history: [], reports: [], live: !!opts.live, failCause: null, choicesMade: 0,
    };
    // build a random event schedule (replayability)
    const pool = map.events.slice();
    let last = null, usedOnce = {};
    for (let sol = 2; sol <= map.sols; sol++) {
      if (map.nightSols && map.nightSols.includes(sol)) continue;
      if (Math.random() < 0.6) {
        let id, tries = 0;
        do { id = JA.pick(pool); tries++; } while ((id === last || usedOnce[id]) && tries < 20);
        if (id === 'dust' || id === 'conjunction') usedOnce[id] = true;
        if (id === 'dust' && sol > map.sols - 2) continue;
        s.schedule[sol] = { id, data: eventData(s, id) };
        last = id;
      }
    }
    // live space weather: if real risk is high, guarantee a flare
    if (opts.live && map.id !== 'earth') {
      const risk = JA.spaceRisk && JA.spaceRisk();
      const sol = JA.rand(2, Math.min(4, map.sols));
      if (!s.schedule[sol] || risk && risk.level >= 2) s.schedule[sol] = { id: 'flare', data: eventData(s, 'flare', true) };
    }
    // forecasting is uncertain (pilot mode): occasional false alarms / surprises
    if (!cadet) {
      for (const k of Object.keys(s.schedule)) {
        const e = s.schedule[k];
        if (e.id === 'flare' && Math.random() < 0.2) e.surprise = true;      // no warning
      }
      const free = [];
      for (let sol = 2; sol <= map.sols; sol++) if (!s.schedule[sol]) free.push(sol);
      if (free.length && Math.random() < 0.5 && map.id !== 'earth') s.schedule[JA.pick(free)] = { id: null, falseAlarm: 'flare' };
    }
    return s;
  }

  function eventData(s, id, preferLive) {
    const map = mapById(s.mapId);
    if (id === 'flare' || id === 'flareEarth') {
      const r = JA.realFlare ? JA.realFlare() : { event: JA.DONKI_ARCHIVE[0], live: false };
      const ev = preferLive && JA.weather && JA.weather.live && JA.weather.events.length ? { event: JA.weather.events[0], live: true } : r;
      const factor = { moon: 1, mars: 0.8, gateway: 1.25, europa: 1.1 }[map.id] || 0;
      return { label: ev.event.label, date: ev.event.date, live: ev.live, type: ev.event.type,
        spike: Math.round((12 + 18 * (ev.event.power || 0.5)) * factor) };
    }
    if (id === 'emotion') return { mood: Math.floor(Math.random() * JA.MOODS.length) };
    return {};
  }

  /* ---------- per-sol helpers ---------- */
  function activeEffects(s) { return s.effects.filter((e) => e.until >= s.sol); }

  function baseGen(s) {
    const map = mapById(s.mapId);
    let g = map.gen(s.sol);
    activeEffects(s).forEach((e) => { if (e.genMult) g = Math.round(g * e.genMult); });
    if (s.age === 'cadet') g = Math.max(2, g - 1);   // cadets don't run the greenhouse
    return g;
  }

  function startSol(s) {
    s.mods = { genDelta: 0, lsMult: 1, sciMult: 1, sciAdd: 0, doseAdd: 0, o2Add: 0, foodAdd: 0, morale: 0, spike: 0 };
    s.choice = null;
    const ev = s.schedule[s.sol];
    s.today = ev && ev.id ? { id: ev.id, data: ev.data } : null;
    if (s.today && s.mapId === 'earth' && s.today.id === 'flare') s.today.id = 'flareEarth';
    // forecast for tomorrow from Mission Control
    const silent = activeEffects(s).some((e) => e.id === 'silence');
    const next = s.schedule[s.sol + 1];
    const map = mapById(s.mapId);
    s.forecast = null;
    if (!silent && s.sol < s.sols) {
      if (map.nightSols && map.nightSols.includes(s.sol + 1) && !map.nightSols.includes(s.sol)) s.forecast = { id: 'night', sure: true };
      else if (next && next.falseAlarm) s.forecast = { id: next.falseAlarm, sure: false };
      else if (next && next.id && !next.surprise && EVENTS[next.id] && EVENTS[next.id].alarm) s.forecast = { id: next.id, sure: next.id !== 'flare' };
    }
    s.gen = baseGen(s);
    return s;
  }

  function choose(s, idx) {
    if (!s.today || s.choice != null) return s;
    const def = EVENTS[s.today.id];
    const c = def.choices[idx];
    if (!c) return s;
    c.apply(s, s.mods, s.today.data);
    s.choice = idx;
    s.choicesMade++;
    if (s.today.id === 'emotion') s.flags.helped = true;
    return s;
  }

  function available(s) {
    return Math.max(0, s.gen + s.mods.genDelta) + s.battery;
  }

  /* ---------- resolve the sol ---------- */
  function resolve(s, alloc) {
    const map = mapById(s.mapId);
    const cadet = s.age === 'cadet';
    const a = { ls: alloc.ls | 0, green: cadet ? 0 : alloc.green | 0, shield: alloc.shield | 0, sci: alloc.sci | 0 };
    const used = a.ls + a.green + a.shield + a.sci;
    const gen = Math.max(0, s.gen + s.mods.genDelta);
    if (used > gen + s.battery) throw new Error('over budget');
    const fromBattery = Math.max(0, used - gen);
    const toBattery = Math.max(0, gen - used);
    const prev = { o2: s.o2, food: s.food, dose: s.dose, science: s.science, battery: s.battery, morale: s.morale };

    const eff = activeEffects(s);
    let lsMult = s.mods.lsMult, sciMult = s.mods.sciMult, o2Effect = 0;
    eff.forEach((e) => { if (e.lsMult) lsMult *= e.lsMult; if (e.sciMult) sciMult *= e.sciMult; if (e.o2Add) o2Effect += e.o2Add; });

    s.battery = JA.clamp(s.battery - fromBattery + toBattery, 0, BATTERY_MAX);
    const o2Made = Math.round(a.ls * O2_PER * lsMult);
    s.o2 = JA.clamp(s.o2 + o2Made - O2_USE + s.mods.o2Add + o2Effect, 0, 100);
    const foodMade = a.green * FOOD_PER;
    if (!cadet) s.food = JA.clamp(s.food + foodMade - FOOD_USE + s.mods.foodAdd, 0, 100);
    const shieldRed = Math.min(SHIELD_MAX, a.shield) * SHIELD_PER;
    const doseBase = map.dose * (1 - shieldRed);
    const doseSpike = s.mods.spike * (1 - shieldRed);
    s.dose = JA.clamp(s.dose + doseBase + doseSpike + s.mods.doseAdd, 0, 100);
    s.morale = JA.clamp(s.morale - 4 + s.mods.morale, 0, 100);
    let sciGain = a.sci * sciMult + s.mods.sciAdd;
    let teamBonus = 0;
    if (s.morale >= 80 && a.sci > 0 && sciMult > 0) teamBonus = 1;
    if (s.morale < 30) sciGain *= 0.5;
    sciGain = Math.round((sciGain + teamBonus) * 10) / 10;
    s.science = Math.round((s.science + sciGain) * 10) / 10;

    if (s.today && (s.today.id === 'flare') && a.shield >= 2) s.flags.shieldInTime = true;
    if (map.nightSols && s.sol === map.nightSols[map.nightSols.length - 1]) s.flags.night = true;

    // explanation lines (cause → effect)
    const t = JA.t;
    const lines = [];
    lines.push({ k: 'o2', icon: '🫁', text: t(
      `Кислород: +${o2Made} от ${a.ls} бл. жизнеобеспечения${lsMult < 1 ? ' (генератор сломан)' : ''}, −${O2_USE} дыхание экипажа${o2Effect ? `, ${o2Effect} утечка` : ''}${s.mods.o2Add ? `, ${s.mods.o2Add > 0 ? '+' : ''}${s.mods.o2Add} событие` : ''}`,
      `Oxygen: +${o2Made} from ${a.ls} life-support blocks${lsMult < 1 ? ' (generator broken)' : ''}, −${O2_USE} crew breathing${o2Effect ? `, ${o2Effect} leak` : ''}${s.mods.o2Add ? `, ${s.mods.o2Add > 0 ? '+' : ''}${s.mods.o2Add} event` : ''}`), d: s.o2 - prev.o2 });
    if (!cadet) lines.push({ k: 'food', icon: '🥔', text: t(
      `Еда: +${foodMade} из оранжереи (${a.green} бл.), −${FOOD_USE} обед экипажа${s.mods.foodAdd ? `, +${s.mods.foodAdd} событие` : ''}`,
      `Food: +${foodMade} from greenhouse (${a.green} blocks), −${FOOD_USE} crew meals${s.mods.foodAdd ? `, +${s.mods.foodAdd} event` : ''}`), d: s.food - prev.food });
    lines.push({ k: 'dose', icon: '☢️', text: t(
      `Радиация: +${(doseBase + doseSpike + s.mods.doseAdd).toFixed(1)}% — щит снизил дозу на ${Math.round(shieldRed * 100)}%${doseSpike ? `, вспышка +${doseSpike.toFixed(1)}` : ''}${s.mods.doseAdd ? `, выход наружу +${s.mods.doseAdd}` : ''}`,
      `Radiation: +${(doseBase + doseSpike + s.mods.doseAdd).toFixed(1)}% — shield cut the dose by ${Math.round(shieldRed * 100)}%${doseSpike ? `, flare +${doseSpike.toFixed(1)}` : ''}${s.mods.doseAdd ? `, going outside +${s.mods.doseAdd}` : ''}`), d: s.dose - prev.dose, bad: true });
    lines.push({ k: 'sci', icon: '🔬', text: t(
      `Наука: +${sciGain}${teamBonus ? ' (команда в ударе: +1)' : ''}${s.morale < 30 ? ' (экипаж устал: −50%)' : ''}`,
      `Science: +${sciGain}${teamBonus ? ' (great teamwork: +1)' : ''}${s.morale < 30 ? ' (crew tired: −50%)' : ''}`), d: sciGain });
    lines.push({ k: 'bat', icon: '🔋', text: t(
      `Батарея: ${toBattery ? `+${toBattery} лишних блоков сохранено` : fromBattery ? `−${fromBattery} блоков взято из батареи` : 'без изменений'}`,
      `Battery: ${toBattery ? `+${toBattery} spare blocks stored` : fromBattery ? `−${fromBattery} blocks used from battery` : 'no change'}`), d: s.battery - prev.battery });

    const report = { sol: s.sol, alloc: a, lines, event: s.today ? s.today.id : null };
    s.reports.push(report);
    s.history.push({ sol: s.sol, o2: s.o2, food: s.food, dose: s.dose, battery: s.battery, science: s.science, alloc: a });

    // end conditions — no crew harm: they safely evacuate
    if (s.o2 <= 0) { s.status = 'lost'; s.failCause = 'o2'; }
    else if (!cadet && s.food <= 0) { s.status = 'lost'; s.failCause = 'food'; }
    else if (s.dose >= 100) { s.status = 'lost'; s.failCause = 'dose'; }
    else if (s.sol >= s.sols) { s.status = 'won'; }
    else { s.sol++; startSol(s); }
    return report;
  }

  function starsFor(s) {
    if (s.status !== 'won') return 0;
    let st = 1;
    if (s.science >= s.goal) st++;
    if (s.dose < 50) st++;
    return st;
  }

  function pointsFor(s) {
    const map = mapById(s.mapId);
    const per = [0, 20, 30, 40, 50, 60][map.tier];
    if (s.status === 'won') return per * starsFor(s) + (s.live ? 10 : 0);
    return 10 + Math.min(20, (s.sol - 1) * 3);    // learning from failure still counts
  }

  /* Simple "why" for a failed mission, based on history */
  function failReason(s) {
    const t = JA.t;
    const h = s.history;
    const last = h[h.length - 1] || {};
    if (s.failCause === 'o2') {
      const lowLs = h.filter((x) => x.alloc.ls < 2).length;
      return t(`Кислород закончился на соле ${s.sol}. В ${lowLs} из ${h.length} солов на жизнеобеспечение шло меньше 2 блоков — а экипаж тратит ${O2_USE}% за сол, и 1 блок даёт только ${O2_PER}%.`,
               `Oxygen ran out on sol ${s.sol}. In ${lowLs} of ${h.length} sols life support got fewer than 2 blocks — the crew uses ${O2_USE}% per sol and 1 block makes only ${O2_PER}%.`);
    }
    if (s.failCause === 'food') return t(`Еда закончилась на соле ${s.sol}. Оранжерее нужно хотя бы 2 блока, чтобы кормить экипаж.`, `Food ran out on sol ${s.sol}. The greenhouse needs about 2 blocks to feed the crew.`);
    const avgShield = h.reduce((a, x) => a + x.alloc.shield, 0) / Math.max(1, h.length);
    return t(`Доза радиации достигла безопасного предела на соле ${s.sol}. В среднем на щит шло ${avgShield.toFixed(1)} блока. Настоящие миссии тоже следят за пределом дозы (у NASA — 600 мЗв за всю карьеру).`,
             `The radiation dose reached the safe limit on sol ${s.sol}. On average the shield got ${avgShield.toFixed(1)} blocks. Real missions track a dose limit too (NASA: 600 mSv per career).`);
  }

  JA.Sim = { create, startSol, choose, resolve, available, starsFor, pointsFor, failReason, mapById, BATTERY_MAX, O2_USE, O2_PER, FOOD_USE, FOOD_PER, SHIELD_PER, SHIELD_MAX };
})();
