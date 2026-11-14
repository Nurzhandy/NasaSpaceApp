//! Static game data. All real-world numbers are attributed in `SOURCES`.

use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MapId {
    Earth,
    Moon,
    Mars,
    Orbit,
    Europa,
}

pub struct MapDef {
    pub id: MapId,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    /// lifetime points needed to unlock
    pub need: u32,
    pub tier: u32,
    pub sols: u32,
    /// power blocks generated per sol (daylight)
    pub gen: u32,
    pub battery_start: u32,
    /// base dose % per sol without shield (scaled from real mSv/day)
    pub dose: f32,
    pub dose_real_ru: &'static str,
    pub dose_real_en: &'static str,
    /// one-way comms delay in seconds as used in game
    pub delay_s: f32,
    pub delay_real_ru: &'static str,
    pub delay_real_en: &'static str,
    pub gravity: f32,
    /// sols with no sunlight (lunar night)
    pub night_sols: &'static [u32],
    pub sky: Color,
    pub ground: Color,
    pub ground2: Color,
    pub fog: f32,
    pub events: &'static [EventKind],
    pub fact_ru: &'static str,
    pub fact_en: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EventKind {
    Flare,
    Dust,
    Failure,
    Leak,
    Harvest,
    Discovery,
    Supply,
    Emotion,
    Conjunction,
}

pub const MAPS: &[MapDef] = &[
    MapDef {
        id: MapId::Earth, icon: "desert", name_ru: "База «Пустыня»", name_en: "Desert Base",
        need: 0, tier: 1, sols: 4, gen: 8, battery_start: 4,
        dose: 0.06, dose_real_ru: "0,008 мЗв/сут — природный фон", dose_real_en: "0.008 mSv/day — natural background",
        delay_s: 0.0, delay_real_ru: "без задержки", delay_real_en: "no delay", gravity: 1.0,
        night_sols: &[],
        sky: Color::srgb(0.45, 0.70, 0.98), ground: Color::srgb(0.80, 0.62, 0.38), ground2: Color::srgb(0.70, 0.50, 0.28), fog: 260.0,
        events: &[EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Discovery, EventKind::Supply],
        fact_ru: "NASA тренирует экипажи в пустынях Земли — это «аналоговые» миссии.",
        fact_en: "NASA trains crews in Earth deserts — these are “analog” missions.",
    },
    MapDef {
        id: MapId::Moon, icon: "moon", name_ru: "Луна: Южный полюс", name_en: "Moon: South Pole",
        need: 60, tier: 2, sols: 6, gen: 9, battery_start: 5,
        dose: 10.0, dose_real_ru: "1,369 мЗв/сут (Chang’e‑4)", dose_real_en: "1.369 mSv/day (Chang’e‑4)",
        delay_s: 1.3, delay_real_ru: "1,3 с", delay_real_en: "1.3 s", gravity: 0.16,
        night_sols: &[4],
        sky: Color::srgb(0.01, 0.01, 0.03), ground: Color::srgb(0.58, 0.60, 0.66), ground2: Color::srgb(0.40, 0.42, 0.48), fog: 320.0,
        events: &[EventKind::Flare, EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Leak, EventKind::Supply, EventKind::Discovery],
        fact_ru: "Лунные сутки длятся 29,5 земных: две недели света и две недели тьмы.",
        fact_en: "A lunar day lasts 29.5 Earth days: two weeks of light and two of darkness.",
    },
    MapDef {
        id: MapId::Mars, icon: "mars", name_ru: "Марс: кратер Езеро", name_en: "Mars: Jezero Crater",
        need: 180, tier: 3, sols: 7, gen: 6, battery_start: 5,
        dose: 4.7, dose_real_ru: "0,64 мЗв/сут (Curiosity RAD)", dose_real_en: "0.64 mSv/day (Curiosity RAD)",
        delay_s: 6.0, delay_real_ru: "4–24 мин (в игре 6 с)", delay_real_en: "4–24 min (6 s in game)", gravity: 0.38,
        night_sols: &[],
        sky: Color::srgb(0.86, 0.62, 0.45), ground: Color::srgb(0.72, 0.34, 0.18), ground2: Color::srgb(0.55, 0.25, 0.12), fog: 180.0,
        events: &[EventKind::Flare, EventKind::Dust, EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Leak, EventKind::Discovery, EventKind::Conjunction],
        fact_ru: "Сутки на Марсе — «сол» — длятся 24 часа 39 минут.",
        fact_en: "A Mars day — a “sol” — lasts 24 hours 39 minutes.",
    },
    MapDef {
        id: MapId::Orbit, icon: "satellite", name_ru: "Окололунная станция", name_en: "Lunar Orbit Station",
        need: 400, tier: 4, sols: 7, gen: 9, battery_start: 5,
        dose: 13.4, dose_real_ru: "1,84 мЗв/сут в глубоком космосе (RAD)", dose_real_en: "1.84 mSv/day in deep space (RAD)",
        delay_s: 1.3, delay_real_ru: "1,3 с", delay_real_en: "1.3 s", gravity: 0.0,
        night_sols: &[],
        sky: Color::srgb(0.0, 0.0, 0.02), ground: Color::srgb(0.75, 0.78, 0.85), ground2: Color::srgb(0.55, 0.58, 0.66), fog: 400.0,
        events: &[EventKind::Flare, EventKind::Flare, EventKind::Failure, EventKind::Emotion, EventKind::Leak, EventKind::Supply, EventKind::Discovery],
        fact_ru: "На МКС астронавты занимаются спортом 2 часа в день — иначе мышцы слабеют.",
        fact_en: "ISS astronauts exercise 2 hours a day — otherwise muscles weaken.",
    },
    MapDef {
        id: MapId::Europa, icon: "ice", name_ru: "Европа (миссия будущего)", name_en: "Europa (future mission)",
        need: 700, tier: 5, sols: 8, gen: 6, battery_start: 6,
        dose: 20.0, dose_real_ru: "в тысячи раз выше земной", dose_real_en: "thousands of times Earth’s",
        delay_s: 10.0, delay_real_ru: "35–52 мин (в игре 10 с)", delay_real_en: "35–52 min (10 s in game)", gravity: 0.13,
        night_sols: &[],
        sky: Color::srgb(0.03, 0.05, 0.14), ground: Color::srgb(0.70, 0.86, 0.95), ground2: Color::srgb(0.50, 0.70, 0.85), fog: 220.0,
        events: &[EventKind::Flare, EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Leak, EventKind::Discovery],
        fact_ru: "Под льдом Европы, вероятно, океан. Europa Clipper летит туда с 2024 года.",
        fact_en: "Europa probably hides an ocean under its ice. Europa Clipper has been flying there since 2024.",
    },
];

pub fn map(id: MapId) -> &'static MapDef {
    MAPS.iter().find(|m| m.id == id).expect("map")
}

/// Archived real DONKI events (used when the live API is unreachable; always labeled as archive).
pub struct Donki {
    pub kind: &'static str,
    pub date: &'static str,
    pub label_ru: &'static str,
    pub label_en: &'static str,
    pub power: f32,
}

pub const DONKI_ARCHIVE: &[Donki] = &[
    Donki { kind: "FLR", date: "2024-10-03", label_ru: "Вспышка X9.0 — сильнейшая в 25-м цикле", label_en: "X9.0 flare — strongest of Cycle 25", power: 1.0 },
    Donki { kind: "FLR", date: "2024-05-14", label_ru: "Вспышка X8.7", label_en: "X8.7 solar flare", power: 0.95 },
    Donki { kind: "GST", date: "2024-05-10", label_ru: "Буря G5 (Kp 9): сияния видели на юге", label_en: "G5 storm (Kp 9): auroras far south", power: 0.9 },
    Donki { kind: "GST", date: "2024-10-10", label_ru: "Сильная буря G4", label_en: "Severe G4 storm", power: 0.8 },
    Donki { kind: "SEP", date: "2017-09-10", label_ru: "Вспышка X8.2: Curiosity зафиксировал рост радиации на Марсе", label_en: "X8.2 flare: Curiosity saw radiation rise on Mars", power: 0.9 },
    Donki { kind: "SEP", date: "2021-10-28", label_ru: "Вспышка X1.0 с потоком частиц", label_en: "X1.0 flare with particle event", power: 0.6 },
    Donki { kind: "FLR", date: "2003-10-28", label_ru: "«Хэллоуинские бури»: вспышка X17", label_en: "“Halloween storms”: X17 flare", power: 1.0 },
];

pub struct Skin {
    pub id: &'static str,
    pub price: u32,
    pub color: Color,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
}

pub const SUITS: &[Skin] = &[
    Skin { id: "white", price: 0, color: Color::srgb(0.95, 0.96, 0.98), icon: "shirt", name_ru: "Белый", name_en: "White" },
    Skin { id: "orange", price: 40, color: Color::srgb(1.0, 0.52, 0.2), icon: "shirt", name_ru: "Оранжевый", name_en: "Orange" },
    Skin { id: "blue", price: 60, color: Color::srgb(0.3, 0.55, 1.0), icon: "shirt", name_ru: "Синий", name_en: "Blue" },
    Skin { id: "purple", price: 90, color: Color::srgb(0.6, 0.4, 1.0), icon: "shirt", name_ru: "Фиолетовый", name_en: "Purple" },
    Skin { id: "mint", price: 90, color: Color::srgb(0.25, 0.85, 0.7), icon: "shirt", name_ru: "Мятный", name_en: "Mint" },
    Skin { id: "gold", price: 200, color: Color::srgb(1.0, 0.82, 0.25), icon: "shirt", name_ru: "Золотой", name_en: "Gold" },
    Skin { id: "pink", price: 120, color: Color::srgb(1.0, 0.45, 0.75), icon: "shirt", name_ru: "Розовый", name_en: "Pink" },
];
pub const VISORS: &[Skin] = &[
    Skin { id: "blue", price: 0, color: Color::srgb(0.45, 0.8, 1.0), icon: "goggles", name_ru: "Голубой", name_en: "Blue" },
    Skin { id: "gold", price: 50, color: Color::srgb(1.0, 0.78, 0.25), icon: "goggles", name_ru: "Золотой (как у NASA)", name_en: "Gold (NASA style)" },
    Skin { id: "pink", price: 50, color: Color::srgb(1.0, 0.55, 0.85), icon: "goggles", name_ru: "Розовый", name_en: "Pink" },
    Skin { id: "green", price: 70, color: Color::srgb(0.4, 1.0, 0.6), icon: "goggles", name_ru: "Зелёный", name_en: "Green" },
    Skin { id: "dark", price: 90, color: Color::srgb(0.15, 0.15, 0.25), icon: "goggles", name_ru: "Тёмный", name_en: "Dark" },
];
pub const HATS: &[Skin] = &[
    Skin { id: "none", price: 0, color: Color::WHITE, icon: "cross", name_ru: "Без украшения", name_en: "None" },
    Skin { id: "antenna", price: 30, color: Color::srgb(0.9, 0.9, 0.9), icon: "radio", name_ru: "Антенна", name_en: "Antenna" },
    Skin { id: "star", price: 40, color: Color::srgb(1.0, 0.85, 0.2), icon: "star", name_ru: "Звезда", name_en: "Star" },
    Skin { id: "cat", price: 80, color: Color::srgb(1.0, 0.6, 0.7), icon: "cat", name_ru: "Кошачьи ушки", name_en: "Cat ears" },
    Skin { id: "crown", price: 150, color: Color::srgb(1.0, 0.8, 0.1), icon: "crown", name_ru: "Корона", name_en: "Crown" },
    Skin { id: "alien", price: 250, color: Color::srgb(0.4, 1.0, 0.4), icon: "alien", name_ru: "Друг‑пришелец", name_en: "Alien buddy" },
];

pub struct Badge {
    pub id: &'static str,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
}
pub const BADGES: &[Badge] = &[
    Badge { id: "first", icon: "rocket", name_ru: "Первая миссия", name_en: "First mission" },
    Badge { id: "shield", icon: "shield", name_ru: "Успел в убежище", name_en: "Made it to shelter" },
    Badge { id: "storm", icon: "tornado", name_ru: "Пережил бурю", name_en: "Storm survivor" },
    Badge { id: "night", icon: "night", name_ru: "Лунная ночь", name_en: "Lunar night" },
    Badge { id: "scientist", icon: "science", name_ru: "Учёный", name_en: "Scientist" },
    Badge { id: "helper", icon: "heart", name_ru: "Поддержка экипажа", name_en: "Crew support" },
    Badge { id: "beacon", icon: "beacon", name_ru: "Знаю, как успокоиться", name_en: "I know how to calm down" },
    Badge { id: "comeback", icon: "retry", name_ru: "Попробовал снова", name_en: "Tried again" },
    Badge { id: "moon", icon: "moon", name_ru: "Лунный житель", name_en: "Moon dweller" },
    Badge { id: "mars", icon: "mars", name_ru: "Марсианин", name_en: "Martian" },
    Badge { id: "coop", icon: "coop", name_ru: "Командная работа", name_en: "Teamwork" },
    Badge { id: "collector", icon: "gem", name_ru: "Коллекционер", name_en: "Collector" },
    Badge { id: "live", icon: "sun", name_ru: "Живая погода", name_en: "Live weather" },
    Badge { id: "europa", icon: "ice", name_ru: "Покоритель Европы", name_en: "Europa explorer" },
];

pub struct Feeling {
    pub id: &'static str,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
}
pub const FEELINGS: &[Feeling] = &[
    Feeling { id: "sad", icon: "f_sad", name_ru: "грусть", name_en: "sad" },
    Feeling { id: "worried", icon: "f_worried", name_ru: "тревога", name_en: "worried" },
    Feeling { id: "angry", icon: "f_angry", name_ru: "злость", name_en: "angry" },
    Feeling { id: "happy", icon: "f_happy", name_ru: "радость", name_en: "happy" },
    Feeling { id: "tired", icon: "f_tired", name_ru: "усталость", name_en: "tired" },
    Feeling { id: "lonely", icon: "f_lonely", name_ru: "скучает", name_en: "misses home" },
];

pub struct Mood {
    pub who: &'static str,
    pub text_ru: &'static str,
    pub text_en: &'static str,
    pub ok: &'static [&'static str],
    /// (icon, ru, en)
    pub support: &'static [(&'static str, &'static str, &'static str)],
}
pub const MOODS: &[Mood] = &[
    Mood { who: "Ая", text_ru: "Ая долго смотрит на маленькую Землю в иллюминаторе и молчит.", text_en: "Aya stares at the tiny Earth through the window and says nothing.",
        ok: &["lonely", "sad"], support: &[("s_call", "Позвонить семье", "Call family"), ("s_talk", "Спросить, о чём думает", "Ask what’s on her mind"), ("s_photo", "Посмотреть фото из дома", "Look at photos from home")] },
    Mood { who: "Тимур", text_ru: "Тимур третий раз чинит фильтр — не выходит. Он бросает ключ.", text_en: "Timur fails to fix the filter a third time and throws the wrench.",
        ok: &["angry", "tired", "worried"], support: &[("beacon", "Пауза и подышать вместе", "Pause and breathe together"), ("hint", "Найти причину по шагам", "Find the cause step by step"), ("radio", "Спросить совет у ЦУП", "Ask Mission Control")] },
    Mood { who: "Ая", text_ru: "Салат в оранжерее наконец вырос! Ая показывает его всем.", text_en: "The lettuce finally grew! Aya shows it to everyone.",
        ok: &["happy"], support: &[("thumbs", "Порадоваться вместе", "Celebrate together"), ("plate", "Праздничный ужин", "Celebration dinner"), ("s_photo", "Сфотографировать для Земли", "Photo for Earth")] },
    Mood { who: "БИП", text_ru: "Робот БИП завтра впервые едет в тёмный кратер. Лампочки мигают всё быстрее.", text_en: "Robot BEEP drives into a dark crater tomorrow. Its lights blink faster and faster.",
        ok: &["worried"], support: &[("clipboard", "Составить план «если — то»", "Make an if–then plan"), ("check", "Проверить чек‑лист", "Check the checklist"), ("s_talk", "Сказать: волноваться — нормально", "Say: it’s OK to be nervous")] },
    Mood { who: "Тимур", text_ru: "Тимур работал весь сол и зевает за ужином.", text_en: "Timur worked the whole sol and yawns at dinner.",
        ok: &["tired"], support: &[("night", "Напомнить про сон", "Remind about sleep"), ("strong", "Взять часть задач", "Take some tasks"), ("s_music", "Включить спокойную музыку", "Play calm music")] },
];

pub struct Source {
    pub title: &'static str,
    pub url: &'static str,
}
pub const SOURCES: &[Source] = &[
    Source { title: "NASA DONKI — Space Weather Database", url: "https://ccmc.gsfc.nasa.gov/tools/DONKI/" },
    Source { title: "Curiosity RAD: 0.64 mSv/day on Mars, 1.84 in cruise (Hassler 2014)", url: "https://science.nasa.gov/resource/curiosity-rad-measurements/" },
    Source { title: "Chang’e‑4 LND: 1.369 mSv/day on the Moon (Zhang 2020)", url: "https://www.science.org/doi/10.1126/sciadv.aaz1334" },
    Source { title: "NASA OSDR — plants & crew health in space", url: "https://osdr.nasa.gov/" },
    Source { title: "NASA Veggie — lettuce eaten on ISS in 2015", url: "https://www.nasa.gov/missions/station/veggie/" },
    Source { title: "MOXIE — 122 g of oxygen from Mars air", url: "https://www.jpl.nasa.gov/missions/mars-2020-perseverance-rover/" },
    Source { title: "NASA — Space Communications (4–24 min to Mars)", url: "https://www.nasa.gov/missions/tech-demonstration/space-communications-7-things-you-need-to-know/" },
    Source { title: "NASA HRP — 5 Hazards of Human Spaceflight", url: "https://www.nasa.gov/hrp/hazards/" },
    Source { title: "NASA-STD-3001 — 600 mSv career limit", url: "https://www.nasa.gov/reference/nasa-std-3001-technical-documents/" },
    Source { title: "Obradović 2021 — deep breaths reduce children’s arousal", url: "https://pubmed.ncbi.nlm.nih.gov/34813098/" },
    Source { title: "Pandey 2018 — self‑regulation interventions (JAMA Pediatrics)", url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC6059379/" },
    Source { title: "Harvard CDC — executive function activities 7–12", url: "https://developingchild.harvard.edu/" },
    Source { title: "Icons: OpenMoji (CC BY-SA 4.0)", url: "https://openmoji.org/" },
];
