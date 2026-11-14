/* ============================================================
   Junior Astronaut Mission Trainer — game data
   All real-world numbers are attributed in JA.SOURCES and shown
   in-game. Game values are scaled, but ratios follow real data.
   ============================================================ */
window.JA = window.JA || {};

/* Bilingual helper: tt('рус', 'eng') -> {ru, en} */
JA.tt = (ru, en) => ({ ru, en });
const tt = JA.tt;

/* ---------- Real reference values (with source ids) ---------- */
JA.REAL = {
  marsDose: { v: 0.64, unit: 'mSv/day', src: 'rad' },          // Curiosity RAD, surface
  cruiseDose: { v: 1.84, unit: 'mSv/day', src: 'rad' },        // Curiosity RAD, cruise to Mars
  moonDose: { v: 1.369, unit: 'mSv/day', src: 'change4' },     // Chang'e-4 LND, lunar surface
  earthDose: { v: 0.008, unit: 'mSv/day', src: 'nrc' },        // ~3 mSv/yr natural background
  careerLimit: { v: 600, unit: 'mSv', src: 'nasastd' },         // NASA career limit
  marsSol: '24 ч 39 мин',
};

/* ---------- Maps / outposts (unlock by lifetime points) ---------- */
/* gen(sol, run) -> power blocks produced that sol (before effects)   */
JA.MAPS = [
  {
    id: 'earth', icon: '🏜️', need: 0, tier: 1, sols: 5, color: '#f2b33d',
    name: tt('Аналоговая база «Пустыня»', 'Desert Analog Base'),
    short: tt('Тренировка на Земле', 'Training on Earth'),
    desc: tt('Тренировочная база в пустыне на Земле — как настоящие аналоговые миссии NASA. Здесь атмосфера и магнитное поле защищают от радиации.',
             'A training base in an Earth desert, like real NASA analog missions. The atmosphere and magnetic field protect you from radiation.'),
    gen: () => 8, battery: 4,
    dose: 0.06, doseReal: tt('≈0,008 мЗв/сутки (природный фон Земли)', '≈0.008 mSv/day (Earth natural background)'), doseSrc: 'nrc',
    delayMs: 0, delayReal: tt('нет задержки', 'no delay'),
    gravity: tt('100% — как дома', '100% — like home'),
    events: ['failure', 'emotion', 'harvest', 'discovery', 'supply', 'flareEarth'],
    fact: tt('NASA проводит «аналоговые» миссии на Земле: экипажи месяцами живут в изолированных модулях, чтобы потренироваться перед Луной и Марсом.',
             'NASA runs “analog” missions on Earth: crews live for months in isolated habitats to practice for the Moon and Mars.'),
  },
  {
    id: 'moon', icon: '🌙', need: 60, tier: 2, sols: 8, color: '#b9c3d9',
    name: tt('Луна: Южный полюс', 'Moon: South Pole'),
    short: tt('Регион программы Artemis', 'Artemis region'),
    desc: tt('Здесь хотят высадиться астронавты Artemis. В тёмных кратерах есть водяной лёд, а длинная лунная ночь отключает солнечные панели.',
             'Artemis astronauts plan to land here. Dark craters hide water ice, and the long lunar night shuts off solar panels.'),
    gen: (sol) => (sol === 4 || sol === 5 ? 3 : 9), battery: 5,
    nightSols: [4, 5],
    dose: 10, doseReal: tt('1,369 мЗв/сутки на поверхности (Chang’e-4, 2019)', '1.369 mSv/day on the surface (Chang’e-4, 2019)'), doseSrc: 'change4',
    delayMs: 1300, delayReal: tt('≈1,3 секунды в одну сторону', '≈1.3 seconds one way'),
    gravity: tt('≈16% от земной', '≈16% of Earth'),
    events: ['flare', 'failure', 'emotion', 'harvest', 'ice', 'leak', 'supply', 'discovery'],
    fact: tt('Лунные сутки длятся около 29,5 земных суток: примерно 2 недели света и 2 недели темноты.',
             'A lunar day lasts about 29.5 Earth days: roughly 2 weeks of light and 2 weeks of darkness.'),
  },
  {
    id: 'mars', icon: '🔴', need: 180, tier: 3, sols: 10, color: '#e0613a',
    name: tt('Марс: кратер Езеро', 'Mars: Jezero Crater'),
    short: tt('Место работы Perseverance', 'Where Perseverance works'),
    desc: tt('Древнее озеро, где ровер Perseverance ищет следы жизни. Пылевые бури закрывают Солнце, а связь с Землёй идёт минутами.',
             'An ancient lake where the Perseverance rover searches for signs of life. Dust storms block the Sun and messages take minutes.'),
    gen: () => 6, battery: 5,
    dose: 4.7, doseReal: tt('0,64 мЗв/сутки на поверхности (Curiosity RAD)', '0.64 mSv/day on the surface (Curiosity RAD)'), doseSrc: 'rad',
    delayMs: 6000, delayReal: tt('4–24 минуты в одну сторону (в игре ускорено до 6 с)', '4–24 minutes one way (sped up to 6 s in game)'),
    gravity: tt('≈38% от земной', '≈38% of Earth'),
    events: ['flare', 'dust', 'failure', 'emotion', 'harvest', 'leak', 'discovery', 'conjunction'],
    fact: tt('Сутки на Марсе называются «сол» и длятся 24 часа 39 минут.',
             'A day on Mars is called a “sol” and lasts 24 hours 39 minutes.'),
  },
  {
    id: 'gateway', icon: '🛰️', need: 400, tier: 4, sols: 10, color: '#7fb2ff',
    name: tt('Окололунная станция', 'Lunar Orbit Station'),
    short: tt('Глубокий космос', 'Deep space'),
    desc: tt('Станция на орбите Луны, далеко за магнитным полем Земли. Солнце светит всегда, но защищает только твой щит.',
             'A station orbiting the Moon, far beyond Earth’s magnetic field. The Sun always shines, but only your shield protects you.'),
    gen: () => 9, battery: 5,
    dose: 13.4, doseReal: tt('≈1,84 мЗв/сутки в межпланетном пространстве (Curiosity RAD в полёте)', '≈1.84 mSv/day in deep space (Curiosity RAD in cruise)'), doseSrc: 'rad',
    delayMs: 1300, delayReal: tt('≈1,3 секунды', '≈1.3 seconds'),
    gravity: tt('невесомость (микрогравитация)', 'weightless (microgravity)'),
    events: ['flare', 'flare', 'failure', 'emotion', 'leak', 'supply', 'discovery'],
    fact: tt('Микрогравитация — это когда всё «парит». Без тренировок кости и мышцы астронавтов слабеют, поэтому на МКС занимаются спортом около 2 часов в день.',
             'Microgravity means everything floats. Without exercise, bones and muscles weaken, so ISS astronauts exercise about 2 hours a day.'),
  },
  {
    id: 'europa', icon: '🧊', need: 700, tier: 5, sols: 12, color: '#9fe3ff',
    name: tt('Европа (фантазийная миссия)', 'Europa (imagination mission)'),
    short: tt('Ледяная луна Юпитера', 'Jupiter’s icy moon'),
    desc: tt('Фантазийная миссия будущего! Солнце здесь в 27 раз слабее, поэтому работает ядерный генератор (РИТЭГ). Радиация Юпитера огромная — без щита никак.',
             'A future imagination mission! Sunlight is 27× weaker, so a nuclear generator (RTG) provides power. Jupiter’s radiation is huge — you must shield.'),
    gen: () => 6, battery: 6,
    dose: 20, doseReal: tt('в тысячи раз выше земной (в игре ограничено)', 'thousands of times Earth’s (capped in game)'), doseSrc: 'clipper',
    delayMs: 10000, delayReal: tt('≈35–52 минуты (в игре 10 с)', '≈35–52 minutes (10 s in game)'),
    gravity: tt('≈13% от земной', '≈13% of Earth'),
    events: ['flare', 'failure', 'emotion', 'harvest', 'leak', 'discovery', 'ice'],
    fact: tt('Под ледяной коркой Европы, вероятно, скрыт океан. Зонд NASA Europa Clipper стартовал в 2024 году, чтобы его изучить.',
             'Europa probably hides an ocean under its ice. NASA’s Europa Clipper launched in 2024 to study it.'),
  },
];

/* ---------- Real space weather (archived DONKI events) ----------
   Used when the live DONKI API is unreachable, and always labeled
   as archive in the UI. */
JA.DONKI_ARCHIVE = [
  { type: 'FLR', date: '2024-10-03', label: tt('Вспышка класса X9.0 — самая сильная в 25-м солнечном цикле', 'X9.0 flare — strongest of Solar Cycle 25'), power: 1.0 },
  { type: 'FLR', date: '2024-05-14', label: tt('Вспышка класса X8.7', 'X8.7 solar flare'), power: 0.95 },
  { type: 'GST', date: '2024-05-10', label: tt('Магнитная буря G5 (Kp 9) — полярные сияния видели даже на юге', 'G5 geomagnetic storm (Kp 9) — auroras seen far south'), power: 0.9 },
  { type: 'GST', date: '2024-10-10', label: tt('Сильная магнитная буря G4', 'Severe G4 geomagnetic storm'), power: 0.8 },
  { type: 'SEP', date: '2017-09-10', label: tt('Вспышка X8.2 и поток частиц: ровер Curiosity на Марсе зафиксировал рост радиации', 'X8.2 flare and particle storm: Curiosity measured a radiation jump on Mars'), power: 0.9 },
  { type: 'SEP', date: '2021-10-28', label: tt('Вспышка X1.0 с потоком энергичных частиц', 'X1.0 flare with an energetic particle event'), power: 0.6 },
  { type: 'FLR', date: '2003-10-28', label: tt('«Хэллоуинские бури»: вспышка X17', '“Halloween storms”: X17 flare'), power: 1.0 },
];

/* ---------- Crew ---------- */
JA.CREW = [
  { id: 'aya', name: tt('Ая', 'Aya'), role: tt('биолог', 'biologist'), face: '👩🏽‍🚀' },
  { id: 'timur', name: tt('Тимур', 'Timur'), role: tt('инженер', 'engineer'), face: '👨🏻‍🚀' },
  { id: 'beep', name: tt('БИП', 'BEEP'), role: tt('робот-помощник', 'helper robot'), face: '🤖' },
];

/* ---------- Skins ---------- */
JA.SKINS = {
  suit: [
    { id: 'white', price: 0, color: '#f4f6fb', name: tt('Классический белый', 'Classic white') },
    { id: 'orange', price: 40, color: '#ff8a3d', name: tt('Оранжевый спасатель', 'Rescue orange') },
    { id: 'blue', price: 60, color: '#4f8cff', name: tt('Небесно-синий', 'Sky blue') },
    { id: 'purple', price: 90, color: '#9b6bff', name: tt('Космо-фиолетовый', 'Cosmo purple') },
    { id: 'mint', price: 90, color: '#3ddbb0', name: tt('Мятная туманность', 'Mint nebula') },
    { id: 'gold', price: 200, color: '#ffd24a', name: tt('Золотой командир', 'Golden commander') },
    { id: 'rainbow', price: 320, color: 'rainbow', name: tt('Радужная комета', 'Rainbow comet') },
  ],
  visor: [
    { id: 'blue', price: 0, color: '#7ad0ff', name: tt('Голубое стекло', 'Blue glass') },
    { id: 'gold', price: 50, color: '#ffc94a', name: tt('Золотое (как у NASA)', 'Gold (NASA style)') },
    { id: 'pink', price: 50, color: '#ff8fd1', name: tt('Розовый рассвет', 'Pink dawn') },
    { id: 'green', price: 70, color: '#6dff9a', name: tt('Зелёное сияние', 'Aurora green') },
  ],
  extra: [
    { id: 'none', price: 0, icon: '', name: tt('Без украшений', 'No accessory') },
    { id: 'antenna', price: 30, icon: '📡', name: tt('Антенна', 'Antenna') },
    { id: 'star', price: 40, icon: '⭐', name: tt('Звезда', 'Star') },
    { id: 'cat', price: 80, icon: '🐱', name: tt('Кошачьи ушки', 'Cat ears') },
    { id: 'crown', price: 150, icon: '👑', name: tt('Корона', 'Crown') },
    { id: 'alien', price: 250, icon: '👽', name: tt('Друг-пришелец', 'Alien buddy') },
  ],
  rover: [
    { id: 'red', price: 0, color: '#e0613a', name: tt('Марсианский', 'Martian') },
    { id: 'silver', price: 40, color: '#cfd6e6', name: tt('Серебристый', 'Silver') },
    { id: 'lime', price: 60, color: '#a6ff4d', name: tt('Лаймовый', 'Lime') },
    { id: 'cosmic', price: 140, color: '#5b3cff', name: tt('Космический', 'Cosmic') },
  ],
};

/* ---------- Badges (achievements) ---------- */
JA.BADGES = [
  { id: 'first', icon: '🚀', name: tt('Первая миссия', 'First mission'), desc: tt('Заверши любую миссию на базе', 'Finish any outpost mission') },
  { id: 'storm', icon: '🌪️', name: tt('Пережил бурю', 'Storm survivor'), desc: tt('Заверши миссию на Марсе с пылевой бурей', 'Finish a Mars mission with a dust storm') },
  { id: 'shield', icon: '🛡️', name: tt('Щит вовремя', 'Shield in time'), desc: tt('Встреть вспышку с включённым щитом', 'Meet a solar flare with shields up') },
  { id: 'night', icon: '🌑', name: tt('Пережил лунную ночь', 'Lunar night'), desc: tt('Проведи базу через лунную ночь', 'Get your base through the lunar night') },
  { id: 'comms', icon: '📡', name: tt('Связист', 'Comms expert'), desc: tt('Подтверди сообщение партнёра в мультиплеере', 'Confirm a partner’s message in multiplayer') },
  { id: 'beacon', icon: '🌟', name: tt('Знаю, как успокоиться', 'I know how to calm down'), desc: tt('Попробуй Космический маяк', 'Try the Space Beacon') },
  { id: 'helper', icon: '💛', name: tt('Поддержка экипажа', 'Crew support'), desc: tt('Помоги члену экипажа с его чувствами', 'Help a crewmate with their feelings') },
  { id: 'scientist', icon: '🔬', name: tt('Учёный', 'Scientist'), desc: tt('Выполни научную цель миссии', 'Reach a mission science goal') },
  { id: 'comeback', icon: '🔁', name: tt('Попробую снова', 'Try again'), desc: tt('Вернись на базу после неудачной миссии', 'Return after an unsuccessful mission') },
  { id: 'trainer', icon: '🎓', name: tt('Курсант', 'Cadet'), desc: tt('Пройди 3 разные тренировки', 'Complete 3 different trainings') },
  { id: 'moon', icon: '🌙', name: tt('Лунный житель', 'Moon dweller'), desc: tt('Заверши миссию на Луне', 'Finish a Moon mission') },
  { id: 'mars', icon: '🔴', name: tt('Марсианин', 'Martian'), desc: tt('Заверши миссию на Марсе', 'Finish a Mars mission') },
  { id: 'europa', icon: '🧊', name: tt('Покоритель Европы', 'Europa explorer'), desc: tt('Заверши фантазийную миссию на Европе', 'Finish the Europa mission') },
  { id: 'live', icon: '☀️', name: tt('Живая погода', 'Live weather'), desc: tt('Сыграй миссию с реальной космической погодой', 'Play a mission with real space weather') },
];

/* ---------- Quiz: true or myth ---------- */
JA.QUIZ = [
  { q: tt('Сутки на Марсе немного длиннее земных.', 'A day on Mars is a little longer than on Earth.'), a: true,
    why: tt('Да! Марсианские сутки — сол — длятся 24 часа 39 минут.', 'True! A Mars day — a sol — lasts 24 h 39 min.') },
  { q: tt('Сообщение с Марса доходит до Земли мгновенно.', 'A message from Mars reaches Earth instantly.'), a: false,
    why: tt('Миф. Радиосигналу нужно от 4 до 24 минут в одну сторону.', 'Myth. A radio signal needs 4 to 24 minutes one way.') },
  { q: tt('На Луне есть водяной лёд.', 'There is water ice on the Moon.'), a: true,
    why: tt('Да! Он прячется в вечно тёмных кратерах у полюсов. Из него можно получать воду и кислород.', 'True! It hides in permanently dark craters near the poles and could give water and oxygen.') },
  { q: tt('Растения не могут расти в космосе.', 'Plants can’t grow in space.'), a: false,
    why: tt('Миф. В 2015 году астронавты на МКС съели салат, выращенный в установке Veggie.', 'Myth. In 2015, ISS astronauts ate lettuce grown in the Veggie system.') },
  { q: tt('Атмосфера и магнитное поле Земли защищают нас от большей части космической радиации.', 'Earth’s atmosphere and magnetic field protect us from most space radiation.'), a: true,
    why: tt('Да! Поэтому на Луне и Марсе астронавтам нужен дополнительный щит.', 'True! That’s why astronauts on the Moon and Mars need extra shielding.') },
  { q: tt('На Марсе можно дышать без скафандра.', 'You can breathe on Mars without a spacesuit.'), a: false,
    why: tt('Миф. Воздух Марса очень тонкий и почти весь состоит из углекислого газа.', 'Myth. Mars air is very thin and almost all carbon dioxide.') },
  { q: tt('Прибор MOXIE на ровере Perseverance сделал кислород из воздуха Марса.', 'The MOXIE device on Perseverance made oxygen from Mars air.'), a: true,
    why: tt('Да! За 16 запусков MOXIE получил 122 грамма кислорода.', 'True! Over 16 runs MOXIE made 122 grams of oxygen.') },
  { q: tt('Пылевая буря на Марсе может закрыть всю планету.', 'A Mars dust storm can cover the whole planet.'), a: true,
    why: tt('Да. В 2018 году глобальная буря закрыла Солнце, и ровер Opportunity остался без энергии.', 'True. In 2018 a global storm blocked the Sun and the Opportunity rover ran out of power.') },
  { q: tt('Гравитация на Луне такая же, как на Земле.', 'Gravity on the Moon is the same as on Earth.'), a: false,
    why: tt('Миф. На Луне ты весишь примерно в 6 раз меньше.', 'Myth. On the Moon you weigh about 6 times less.') },
  { q: tt('Лунная ночь длится одну земную ночь.', 'A lunar night lasts one Earth night.'), a: false,
    why: tt('Миф. Лунная ночь длится около 14 земных суток!', 'Myth. A lunar night lasts about 14 Earth days!') },
  { q: tt('Астронавты на МКС занимаются спортом около 2 часов в день.', 'ISS astronauts exercise about 2 hours a day.'), a: true,
    why: tt('Да! В невесомости мышцы и кости слабеют без тренировок.', 'True! In microgravity muscles and bones weaken without exercise.') },
  { q: tt('Солнечные вспышки опасны для астронавтов далеко от Земли.', 'Solar flares are dangerous for astronauts far from Earth.'), a: true,
    why: tt('Да. Потоки частиц от Солнца — одна из главных причин строить радиационные убежища.', 'True. Solar particle storms are a main reason to build radiation shelters.') },
  { q: tt('Детям 6–12 лет обычно рекомендуют спать 9–12 часов.', 'Kids aged 6–12 are usually advised to sleep 9–12 hours.'), a: true,
    why: tt('Да, так советуют врачи (CDC). Сон помогает думать и учиться — и астронавтам тоже!', 'True, says the CDC. Sleep helps you think and learn — astronauts too!') },
];

/* ---------- Mood remote: situations (multiple valid answers) ---------- */
JA.FEELINGS = [
  { id: 'sad', icon: '😢', name: tt('грусть', 'sad') },
  { id: 'worried', icon: '😟', name: tt('тревога', 'worried') },
  { id: 'angry', icon: '😠', name: tt('злость', 'angry') },
  { id: 'happy', icon: '😄', name: tt('радость', 'happy') },
  { id: 'proud', icon: '😊', name: tt('гордость', 'proud') },
  { id: 'tired', icon: '🥱', name: tt('усталость', 'tired') },
  { id: 'lonely', icon: '🥺', name: tt('скучает', 'misses home') },
];
JA.MOODS = [
  { who: 'aya', text: tt('Ая смотрит в иллюминатор на маленькую голубую Землю и долго молчит.', 'Aya looks at the tiny blue Earth through the window and is quiet for a long time.'),
    ok: ['lonely', 'sad'], support: [
      tt('Предложить записать видео-привет семье', 'Suggest recording a video hello for her family'),
      tt('Спросить: «Хочешь рассказать, о чём думаешь?»', 'Ask: “Want to tell me what you’re thinking about?”'),
      tt('Вместе посмотреть фото из дома', 'Look at photos from home together') ] },
  { who: 'timur', text: tt('Тимур третий раз пытается починить фильтр, но он снова не работает. Тимур бросает гаечный ключ.', 'Timur tries to fix the filter for the third time, but it still fails. He throws down the wrench.'),
    ok: ['angry', 'tired', 'worried'], support: [
      tt('Предложить паузу и подышать вместе', 'Suggest a pause and breathing together'),
      tt('Помочь найти причину поломки по шагам', 'Help find the cause step by step'),
      tt('Сказать: «Давай попросим совет у Центра управления»', 'Say: “Let’s ask Mission Control for advice”') ] },
  { who: 'aya', text: tt('Салат в оранжерее наконец вырос! Ая показывает его всем.', 'The greenhouse lettuce finally grew! Aya shows it to everyone.'),
    ok: ['happy', 'proud'], support: [
      tt('Порадоваться вместе и сказать, что получилось', 'Celebrate together and say what worked'),
      tt('Устроить праздничный ужин экипажа', 'Have a crew celebration dinner'),
      tt('Сфотографировать урожай для Земли', 'Take a photo of the harvest for Earth') ] },
  { who: 'beep', text: tt('Робот БИП завтра впервые поедет в тёмный кратер. Его лампочки мигают всё быстрее.', 'BEEP the robot drives into a dark crater for the first time tomorrow. Its lights blink faster and faster.'),
    ok: ['worried'], support: [
      tt('Вместе составить план «если…, то…»', 'Make an “if…, then…” plan together'),
      tt('Проверить фонари и чек-лист', 'Check the lights and the checklist'),
      tt('Сказать: «Волноваться — нормально. Я буду на связи»', 'Say: “It’s OK to feel nervous. I’ll be on the radio”') ] },
  { who: 'timur', text: tt('Тимур работал весь сол и зевает за ужином.', 'Timur worked the whole sol and is yawning at dinner.'),
    ok: ['tired'], support: [
      tt('Напомнить про сон по расписанию', 'Remind him about sleep on schedule'),
      tt('Взять часть его задач завтра', 'Take some of his tasks tomorrow'),
      tt('Включить спокойную музыку', 'Play some calm music') ] },
];

/* ---------- Preflight checklist (correct order) ---------- */
JA.CHECKLISTS = [
  { title: tt('Запуск исследовательского зонда', 'Launching a science probe'), steps: [
    tt('Проверить заряд батареи', 'Check the battery charge'),
    tt('Включить связь и проверить сигнал', 'Turn on radio and check the signal'),
    tt('Загрузить маршрут', 'Upload the route'),
    tt('Проверить, что путь свободен', 'Check the path is clear'),
    tt('Нажать «Старт»', 'Press “Start”') ] },
  { title: tt('Выход на поверхность', 'Going outside (spacewalk)'), steps: [
    tt('Надеть скафандр', 'Put on the spacesuit'),
    tt('Проверить запас кислорода', 'Check the oxygen supply'),
    tt('Проверить радио с напарником', 'Radio check with your partner'),
    tt('Закрыть внутренний люк шлюза', 'Close the inner airlock hatch'),
    tt('Открыть внешний люк', 'Open the outer hatch') ] },
  { title: tt('Запуск оранжереи', 'Starting the greenhouse'), steps: [
    tt('Проверить воду в баке', 'Check the water tank'),
    tt('Посадить семена в подушки', 'Plant seeds in grow pillows'),
    tt('Включить светодиодные лампы', 'Turn on the LED lights'),
    tt('Настроить температуру', 'Set the temperature'),
    tt('Записать в журнал дату посадки', 'Log the planting date') ] },
];

/* ---------- Sources (shown on the "For adults" page) ---------- */
JA.SOURCES = [
  { id: 'donki', title: 'NASA DONKI — Space Weather Database Of Notifications, Knowledge, Information', url: 'https://ccmc.gsfc.nasa.gov/tools/DONKI/',
    use: tt('Живые и архивные вспышки, магнитные бури и выбросы массы — события радиации в игре.', 'Live and archived flares, storms and CMEs — radiation events in the game.') },
  { id: 'rad', title: 'Hassler D. M. et al. (2014), Mars’ Surface Radiation Environment Measured with the Mars Science Laboratory’s Curiosity Rover, Science 343', url: 'https://science.nasa.gov/resource/curiosity-rad-measurements/',
    use: tt('0,64 мЗв/сутки на Марсе и 1,84 мЗв/сутки в перелёте — пропорции доз на картах.', '0.64 mSv/day on Mars and 1.84 mSv/day in cruise — map dose ratios.') },
  { id: 'change4', title: 'Zhang S. et al. (2020), First measurements of the radiation dose on the lunar surface, Science Advances 6', url: 'https://www.science.org/doi/10.1126/sciadv.aaz1334',
    use: tt('1,369 мЗв/сутки на Луне — доза на лунной карте.', '1.369 mSv/day on the Moon — lunar map dose.') },
  { id: 'osdr', title: 'NASA Open Science Data Repository (OSDR / GeneLab) — растения и здоровье в космосе', url: 'https://osdr.nasa.gov/',
    use: tt('Опыты с растениями (Veggie, APEX) и здоровьем экипажа — события оранжереи и факты о теле в космосе.', 'Plant (Veggie, APEX) and crew-health studies — greenhouse events and body facts.') },
  { id: 'veggie', title: 'NASA — Veggie: growing plants on the ISS', url: 'https://www.nasa.gov/missions/station/veggie/',
    use: tt('Первый салат, съеденный на МКС в 2015 году.', 'First lettuce eaten on the ISS in 2015.') },
  { id: 'moxie', title: 'NASA JPL — MOXIE oxygen experiment on Perseverance', url: 'https://www.jpl.nasa.gov/missions/mars-2020-perseverance-rover/',
    use: tt('122 г кислорода из воздуха Марса — механика жизнеобеспечения.', '122 g of oxygen from Mars air — life-support mechanic.') },
  { id: 'comms', title: 'NASA — Space Communications: 7 Things You Need to Know', url: 'https://www.nasa.gov/missions/tech-demonstration/space-communications-7-things-you-need-to-know/',
    use: tt('Задержка связи с Марсом 4–24 минуты — механика задержки сообщений.', 'Mars delay of 4–24 minutes — message delay mechanic.') },
  { id: 'hazards', title: 'NASA Human Research Program — 5 Hazards of Human Spaceflight', url: 'https://www.nasa.gov/hrp/hazards/',
    use: tt('Радиация, изоляция, удалённость, гравитация, замкнутая среда — основа миссий.', 'Radiation, isolation, distance, gravity, hostile environment — mission basis.') },
  { id: 'behav', title: 'NASA — Human Factors and Behavioral Performance', url: 'https://www.nasa.gov/reference/about-human-factors-and-behavioral-performance/',
    use: tt('Командная работа, общение, сон и благополучие экипажа.', 'Teamwork, communication, sleep and crew well-being.') },
  { id: 'nasastd', title: 'NASA-STD-3001 — career radiation limit of 600 mSv', url: 'https://www.nasa.gov/reference/nasa-std-3001-technical-documents/',
    use: tt('Почему у дозы радиации есть предел.', 'Why radiation dose has a limit.') },
  { id: 'nrc', title: 'U.S. NRC — Natural background radiation (~3 mSv/yr)', url: 'https://www.nrc.gov/about-nrc/radiation/around-us/doses-daily-lives.html',
    use: tt('Сравнение с дозой на Земле.', 'Comparison with dose on Earth.') },
  { id: 'clipper', title: 'NASA — Europa Clipper mission', url: 'https://science.nasa.gov/mission/europa-clipper/',
    use: tt('Океан под льдом и сильная радиация Юпитера.', 'Ocean under ice and Jupiter’s strong radiation.') },
  { id: 'breath', title: 'Obradović J. et al. (2021), Taking a few deep breaths reduces children’s physiological arousal, Developmental Psychobiology', url: 'https://pubmed.ncbi.nlm.nih.gov/34813098/',
    use: tt('Основа добровольного «Космического маяка».', 'Basis for the voluntary “Space Beacon”.') },
  { id: 'selfreg', title: 'Pandey A. et al. (2018), Self-regulation interventions in children, JAMA Pediatrics', url: 'https://pmc.ncbi.nlm.nih.gov/articles/PMC6059379/',
    use: tt('Пауза перед действием, план «если — то», проверка.', 'Pause before acting, if–then plans, checking.') },
  { id: 'sel', title: 'Cipriano C. et al. (2023), SEL meta-analysis, Child Development', url: 'https://pubmed.ncbi.nlm.nih.gov/37448158/',
    use: tt('Истории про эмоции и поддержку экипажа.', 'Stories about emotions and crew support.') },
  { id: 'harvard', title: 'Harvard Center on the Developing Child — Executive function activities for 7–12 year-olds', url: 'https://developingchild.harvard.edu/wp-content/uploads/2024/10/Executive-Function-Activities-for-7-to-12-year-olds.pdf',
    use: tt('Возрастные режимы «Кадет 7–9» и «Пилот 10–12».', 'Age modes “Cadet 7–9” and “Pilot 10–12”.') },
  { id: 'cdc', title: 'CDC — About Sleep', url: 'https://www.cdc.gov/sleep/about/index.html',
    use: tt('Рекомендации по сну для детей в миссии «День экипажа».', 'Kids’ sleep guidance in “Crew Day”.') },
  { id: 'missionx', title: 'Mission X: Train Like an Astronaut', url: 'https://trainlikeanastronaut.org/',
    use: tt('Существующий образовательный аналог NASA.', 'Existing NASA education analog.') },
];
