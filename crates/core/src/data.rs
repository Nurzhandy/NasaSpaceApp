//! Static game data. Every real-world number is attributed in `SOURCES`.
//! Colors are plain sRGB triples so the crate stays engine-free.

pub type Rgb = [f32; 3];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum MapId {
    Earth,
    Moon,
    Mars,
    Orbit,
    Europa,
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

pub struct MapDef {
    pub id: MapId,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    /// lifetime points needed to unlock
    pub need: u32,
    pub tier: u32,
    pub sols: u32,
    /// power blocks per sol from one clean solar panel in daylight
    pub panel_yield: f32,
    pub start_panels: u8,
    pub start_rtg: u8,
    pub start_battery: f32,
    pub start_materials: u32,
    /// base dose % per sol without shield (real mSv/day × 7.3, see README)
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
    /// water ice can be found here
    pub ice: bool,
    pub sky: Rgb,
    pub ground: Rgb,
    pub ground2: Rgb,
    pub events: &'static [EventKind],
    pub fact_ru: &'static str,
    pub fact_en: &'static str,
}

pub const MAPS: &[MapDef] = &[
    MapDef {
        id: MapId::Earth, icon: "desert", name_ru: "База «Пустыня»", name_en: "Desert Base",
        need: 0, tier: 1, sols: 4, panel_yield: 2.0, start_panels: 4, start_rtg: 0, start_battery: 4.0, start_materials: 12,
        dose: 0.06, dose_real_ru: "0,008 мЗв/сут — природный фон", dose_real_en: "0.008 mSv/day — natural background",
        delay_s: 0.0, delay_real_ru: "без задержки", delay_real_en: "no delay", gravity: 1.0,
        night_sols: &[], ice: false,
        sky: [0.45, 0.70, 0.98], ground: [0.80, 0.62, 0.38], ground2: [0.70, 0.50, 0.28],
        events: &[EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Discovery, EventKind::Supply],
        fact_ru: "NASA тренирует экипажи в пустынях Земли — это «аналоговые» миссии.",
        fact_en: "NASA trains crews in Earth deserts — these are “analog” missions.",
    },
    MapDef {
        id: MapId::Moon, icon: "moon", name_ru: "Луна: Южный полюс", name_en: "Moon: South Pole",
        need: 60, tier: 2, sols: 6, panel_yield: 2.25, start_panels: 4, start_rtg: 0, start_battery: 5.0, start_materials: 12,
        dose: 10.0, dose_real_ru: "1,369 мЗв/сут (Chang’e‑4)", dose_real_en: "1.369 mSv/day (Chang’e‑4)",
        delay_s: 1.3, delay_real_ru: "1,3 с", delay_real_en: "1.3 s", gravity: 0.16,
        night_sols: &[4], ice: true,
        sky: [0.01, 0.01, 0.03], ground: [0.58, 0.60, 0.66], ground2: [0.40, 0.42, 0.48],
        events: &[EventKind::Flare, EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Leak, EventKind::Supply, EventKind::Discovery],
        fact_ru: "Лунные сутки длятся 29,5 земных: две недели света и две недели тьмы.",
        fact_en: "A lunar day lasts 29.5 Earth days: two weeks of light and two of darkness.",
    },
    MapDef {
        id: MapId::Mars, icon: "mars", name_ru: "Марс: кратер Езеро", name_en: "Mars: Jezero Crater",
        need: 180, tier: 3, sols: 7, panel_yield: 1.5, start_panels: 4, start_rtg: 0, start_battery: 5.0, start_materials: 14,
        dose: 4.7, dose_real_ru: "0,64 мЗв/сут (Curiosity RAD)", dose_real_en: "0.64 mSv/day (Curiosity RAD)",
        delay_s: 6.0, delay_real_ru: "4–24 мин (в игре 6 с)", delay_real_en: "4–24 min (6 s in game)", gravity: 0.38,
        night_sols: &[], ice: false,
        sky: [0.86, 0.62, 0.45], ground: [0.72, 0.34, 0.18], ground2: [0.55, 0.25, 0.12],
        events: &[EventKind::Flare, EventKind::Dust, EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Leak, EventKind::Discovery, EventKind::Conjunction],
        fact_ru: "Сутки на Марсе — «сол» — длятся 24 часа 39 минут.",
        fact_en: "A Mars day — a “sol” — lasts 24 hours 39 minutes.",
    },
    MapDef {
        id: MapId::Orbit, icon: "satellite", name_ru: "Окололунная станция", name_en: "Lunar Orbit Station",
        need: 400, tier: 4, sols: 7, panel_yield: 2.25, start_panels: 4, start_rtg: 0, start_battery: 5.0, start_materials: 14,
        dose: 13.4, dose_real_ru: "1,84 мЗв/сут в глубоком космосе (RAD)", dose_real_en: "1.84 mSv/day in deep space (RAD)",
        delay_s: 1.3, delay_real_ru: "1,3 с", delay_real_en: "1.3 s", gravity: 0.0,
        night_sols: &[], ice: false,
        sky: [0.0, 0.0, 0.02], ground: [0.75, 0.78, 0.85], ground2: [0.55, 0.58, 0.66],
        events: &[EventKind::Flare, EventKind::Flare, EventKind::Failure, EventKind::Emotion, EventKind::Leak, EventKind::Supply, EventKind::Discovery],
        fact_ru: "На МКС астронавты занимаются спортом 2 часа в день — иначе мышцы слабеют.",
        fact_en: "ISS astronauts exercise 2 hours a day — otherwise muscles weaken.",
    },
    MapDef {
        id: MapId::Europa, icon: "ice", name_ru: "Европа (миссия будущего)", name_en: "Europa (future mission)",
        // the Sun is 27× dimmer at Jupiter: panels barely help, the base runs on RTGs like Curiosity
        need: 700, tier: 5, sols: 8, panel_yield: 0.75, start_panels: 2, start_rtg: 2, start_battery: 6.0, start_materials: 16,
        // real surface dose (~5.4 Sv/day) is far beyond any shield: the game base hides under the ice
        dose: 13.0, dose_real_ru: "≈5 Зв/сут на поверхности — поэтому база спрятана подо льдом", dose_real_en: "≈5 Sv/day on the surface — so the base hides under the ice",
        delay_s: 10.0, delay_real_ru: "35–52 мин (в игре 10 с)", delay_real_en: "35–52 min (10 s in game)", gravity: 0.13,
        night_sols: &[], ice: true,
        sky: [0.03, 0.05, 0.14], ground: [0.70, 0.86, 0.95], ground2: [0.50, 0.70, 0.85],
        events: &[EventKind::Flare, EventKind::Failure, EventKind::Emotion, EventKind::Harvest, EventKind::Leak, EventKind::Discovery],
        fact_ru: "Под льдом Европы, вероятно, океан. Europa Clipper летит туда с 2024 года.",
        fact_en: "Europa probably hides an ocean under its ice. Europa Clipper has been flying there since 2024.",
    },
];

pub fn map(id: MapId) -> &'static MapDef {
    MAPS.iter().find(|m| m.id == id).expect("every MapId is in MAPS")
}

// ---------------------------------------------------------------- base modules

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModuleKind {
    /// the dome: crew quarters and radiation shelter; exactly one, cannot be built or removed
    Habitat,
    Solar,
    /// radioisotope generator (like Curiosity's MMRTG): steady power day and night
    Rtg,
    Battery,
    OxygenGen,
    Greenhouse,
    Shield,
    Lab,
    Antenna,
}

pub struct ModuleDef {
    pub kind: ModuleKind,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    /// footprint in grid cells
    pub w: i32,
    pub h: i32,
    /// materials to build
    pub cost: u32,
    /// power blocks it can take (0 = not a consumer)
    pub max_power: u8,
    /// maps where it can be built (empty = everywhere)
    pub only: &'static [MapId],
    pub desc_ru: &'static str,
    pub desc_en: &'static str,
}

pub const MODULES: &[ModuleDef] = &[
    ModuleDef { kind: ModuleKind::Habitat, icon: "house", name_ru: "Жилой купол", name_en: "Habitat dome", w: 3, h: 3, cost: 0, max_power: 0, only: &[],
        desc_ru: "Здесь живёт экипаж. Во время вспышки это убежище: внутри радиации почти нет.", desc_en: "The crew lives here. During a flare it is the shelter: almost no radiation inside." },
    ModuleDef { kind: ModuleKind::Solar, icon: "sun", name_ru: "Солнечная панель", name_en: "Solar panel", w: 1, h: 1, cost: 3, max_power: 0, only: &[],
        desc_ru: "Даёт энергию, пока светит Солнце. Пыль и ночь её выключают.", desc_en: "Makes power while the Sun shines. Dust and night switch it off." },
    ModuleDef { kind: ModuleKind::Rtg, icon: "radiation", name_ru: "РИТЭГ", name_en: "RTG", w: 1, h: 1, cost: 10, max_power: 0, only: &[MapId::Mars, MapId::Orbit, MapId::Europa],
        desc_ru: "Ядерная батарейка, как у марсохода Curiosity: даёт энергию днём и ночью.", desc_en: "A nuclear battery like the Curiosity rover's: power day and night." },
    ModuleDef { kind: ModuleKind::Battery, icon: "battery", name_ru: "Батарея", name_en: "Battery", w: 1, h: 1, cost: 4, max_power: 0, only: &[],
        desc_ru: "Запасает лишнюю энергию на ночь и бури (+5).", desc_en: "Stores spare power for nights and storms (+5)." },
    ModuleDef { kind: ModuleKind::OxygenGen, icon: "oxygen", name_ru: "Генератор кислорода", name_en: "Oxygen generator", w: 1, h: 1, cost: 5, max_power: 3, only: &[],
        desc_ru: "1 блок энергии = +6 кислорода за сол. Экипаж дышит 12.", desc_en: "1 power block = +6 oxygen per sol. The crew breathes 12." },
    ModuleDef { kind: ModuleKind::Greenhouse, icon: "plant", name_ru: "Оранжерея", name_en: "Greenhouse", w: 3, h: 1, cost: 5, max_power: 3, only: &[],
        desc_ru: "1 блок = +5 еды за сол. Экипаж ест 10. Иногда даёт урожай.", desc_en: "1 block = +5 food per sol. The crew eats 10. Sometimes gives a harvest." },
    ModuleDef { kind: ModuleKind::Shield, icon: "shield", name_ru: "Щит от радиации", name_en: "Radiation shield", w: 1, h: 1, cost: 6, max_power: 3, only: &[],
        desc_ru: "1 блок = −30% радиации на базе (вместе не больше −90%).", desc_en: "1 block = −30% radiation at the base (−90% at most)." },
    ModuleDef { kind: ModuleKind::Lab, icon: "science", name_ru: "Лаборатория", name_en: "Lab", w: 2, h: 2, cost: 6, max_power: 3, only: &[],
        desc_ru: "1 блок = +1 науки за сол. Принимает образцы, только когда включена.", desc_en: "1 block = +1 science per sol. Accepts samples only when powered." },
    ModuleDef { kind: ModuleKind::Antenna, icon: "satellite", name_ru: "Антенна", name_en: "Antenna", w: 1, h: 1, cost: 3, max_power: 1, only: &[],
        desc_ru: "Связь с Землёй: без энергии ЦУП не пришлёт прогноз.", desc_en: "Link to Earth: without power Mission Control can't send forecasts." },
];

pub fn module_def(kind: ModuleKind) -> &'static ModuleDef {
    MODULES.iter().find(|m| m.kind == kind).expect("every ModuleKind is in MODULES")
}

// ---------------------------------------------------------------- exploration sites

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SiteKind {
    /// a rock sample: carry it to the lab for science
    Sample,
    /// metal-rich rocks: building materials
    Ore,
    /// an old lander: lots of materials
    Wreck,
    /// water ice: turned into oxygen
    Ice,
}

pub struct SiteDef {
    pub kind: SiteKind,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    pub fact_ru: &'static str,
    pub fact_en: &'static str,
}

pub const SITES: &[SiteDef] = &[
    SiteDef { kind: SiteKind::Sample, icon: "gem", name_ru: "Образец породы", name_en: "Rock sample",
        fact_ru: "Perseverance собирает образцы Марса в пробирки, чтобы их однажды привезли на Землю.", fact_en: "Perseverance seals Mars samples in tubes so they can one day come to Earth." },
    SiteDef { kind: SiteKind::Ore, icon: "rock", name_ru: "Рудные камни", name_en: "Ore rocks",
        fact_ru: "Строить из того, что есть на месте, — это ISRU, важная идея NASA для баз.", fact_en: "Building with what you find on site is ISRU, a key NASA idea for bases." },
    SiteDef { kind: SiteKind::Wreck, icon: "rocket", name_ru: "Старый посадочный модуль", name_en: "Old lander",
        fact_ru: "На Луне остались спускаемые ступени «Аполлонов» — с 1969–1972 годов.", fact_en: "Apollo descent stages are still on the Moon, from 1969–1972." },
    SiteDef { kind: SiteKind::Ice, icon: "ice", name_ru: "Водяной лёд", name_en: "Water ice",
        fact_ru: "В 2009 году LCROSS нашёл водяной лёд в тёмном кратере у южного полюса Луны.", fact_en: "In 2009 LCROSS found water ice in a dark crater near the Moon's south pole." },
];

pub fn site_def(kind: SiteKind) -> &'static SiteDef {
    SITES.iter().find(|s| s.kind == kind).expect("every SiteKind is in SITES")
}

// ---------------------------------------------------------------- DONKI archive

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

// ---------------------------------------------------------------- skins

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkinSlot {
    Suit,
    Visor,
    Hat,
}

pub struct Skin {
    pub id: &'static str,
    pub price: u32,
    pub color: Rgb,
    pub icon: &'static str,
    pub name_ru: &'static str,
    pub name_en: &'static str,
}

pub const SUITS: &[Skin] = &[
    Skin { id: "white", price: 0, color: [0.95, 0.96, 0.98], icon: "shirt", name_ru: "Белый", name_en: "White" },
    Skin { id: "orange", price: 40, color: [1.0, 0.52, 0.2], icon: "shirt", name_ru: "Оранжевый", name_en: "Orange" },
    Skin { id: "blue", price: 60, color: [0.3, 0.55, 1.0], icon: "shirt", name_ru: "Синий", name_en: "Blue" },
    Skin { id: "purple", price: 90, color: [0.6, 0.4, 1.0], icon: "shirt", name_ru: "Фиолетовый", name_en: "Purple" },
    Skin { id: "mint", price: 90, color: [0.25, 0.85, 0.7], icon: "shirt", name_ru: "Мятный", name_en: "Mint" },
    Skin { id: "pink", price: 120, color: [1.0, 0.45, 0.75], icon: "shirt", name_ru: "Розовый", name_en: "Pink" },
    Skin { id: "gold", price: 200, color: [1.0, 0.82, 0.25], icon: "shirt", name_ru: "Золотой", name_en: "Gold" },
];
pub const VISORS: &[Skin] = &[
    Skin { id: "blue", price: 0, color: [0.45, 0.8, 1.0], icon: "goggles", name_ru: "Голубой", name_en: "Blue" },
    Skin { id: "gold", price: 50, color: [1.0, 0.78, 0.25], icon: "goggles", name_ru: "Золотой (как у NASA)", name_en: "Gold (NASA style)" },
    Skin { id: "pink", price: 50, color: [1.0, 0.55, 0.85], icon: "goggles", name_ru: "Розовый", name_en: "Pink" },
    Skin { id: "green", price: 70, color: [0.4, 1.0, 0.6], icon: "goggles", name_ru: "Зелёный", name_en: "Green" },
    Skin { id: "dark", price: 90, color: [0.15, 0.15, 0.25], icon: "goggles", name_ru: "Тёмный", name_en: "Dark" },
];
pub const HATS: &[Skin] = &[
    Skin { id: "none", price: 0, color: [1.0, 1.0, 1.0], icon: "cross", name_ru: "Без украшения", name_en: "None" },
    Skin { id: "antenna", price: 30, color: [0.9, 0.9, 0.9], icon: "radio", name_ru: "Антенна", name_en: "Antenna" },
    Skin { id: "star", price: 40, color: [1.0, 0.85, 0.2], icon: "star", name_ru: "Звезда", name_en: "Star" },
    Skin { id: "cat", price: 80, color: [1.0, 0.6, 0.7], icon: "cat", name_ru: "Кошачьи ушки", name_en: "Cat ears" },
    Skin { id: "crown", price: 150, color: [1.0, 0.8, 0.1], icon: "crown", name_ru: "Корона", name_en: "Crown" },
    Skin { id: "alien", price: 250, color: [0.4, 1.0, 0.4], icon: "alien", name_ru: "Друг‑пришелец", name_en: "Alien buddy" },
];

pub fn skins(slot: SkinSlot) -> &'static [Skin] {
    match slot {
        SkinSlot::Suit => SUITS,
        SkinSlot::Visor => VISORS,
        SkinSlot::Hat => HATS,
    }
}

// ---------------------------------------------------------------- patches, feelings, crew

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
    Badge { id: "builder", icon: "hammer", name_ru: "Строитель базы", name_en: "Base builder" },
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
    pub who_ru: &'static str,
    pub who_en: &'static str,
    pub text_ru: &'static str,
    pub text_en: &'static str,
    /// feeling ids that fit the story
    pub ok: &'static [&'static str],
    /// (icon, ru, en) — every option is a good way to help
    pub support: &'static [(&'static str, &'static str, &'static str)],
}

pub const MOODS: &[Mood] = &[
    Mood { who_ru: "Ая", who_en: "Aya", text_ru: "Ая долго смотрит на маленькую Землю в иллюминаторе и молчит.", text_en: "Aya stares at the tiny Earth through the window and says nothing.",
        ok: &["lonely", "sad"], support: &[("s_call", "Позвонить семье", "Call family"), ("s_talk", "Спросить, о чём думает", "Ask what’s on her mind"), ("s_photo", "Посмотреть фото из дома", "Look at photos from home")] },
    Mood { who_ru: "Тимур", who_en: "Timur", text_ru: "Тимур третий раз чинит фильтр — не выходит. Он бросает ключ.", text_en: "Timur fails to fix the filter a third time and throws the wrench.",
        ok: &["angry", "tired", "worried"], support: &[("beacon", "Пауза и подышать вместе", "Pause and breathe together"), ("hint", "Найти причину по шагам", "Find the cause step by step"), ("radio", "Спросить совет у ЦУП", "Ask Mission Control")] },
    Mood { who_ru: "Ая", who_en: "Aya", text_ru: "Салат в оранжерее наконец вырос! Ая показывает его всем.", text_en: "The lettuce finally grew! Aya shows it to everyone.",
        ok: &["happy"], support: &[("thumbs", "Порадоваться вместе", "Celebrate together"), ("plate", "Праздничный ужин", "Celebration dinner"), ("s_photo", "Сфотографировать для Земли", "Photo for Earth")] },
    Mood { who_ru: "БИП", who_en: "BEEP", text_ru: "Робот БИП завтра впервые едет в тёмный кратер. Лампочки мигают всё быстрее.", text_en: "Robot BEEP drives into a dark crater tomorrow. Its lights blink faster and faster.",
        ok: &["worried"], support: &[("clipboard", "Составить план «если — то»", "Make an if–then plan"), ("check", "Проверить чек‑лист", "Check the checklist"), ("s_talk", "Сказать: волноваться — нормально", "Say: it’s OK to be nervous")] },
    Mood { who_ru: "Тимур", who_en: "Timur", text_ru: "Тимур работал весь сол и зевает за ужином.", text_en: "Timur worked the whole sol and yawns at dinner.",
        ok: &["tired"], support: &[("night", "Напомнить про сон", "Remind about sleep"), ("strong", "Взять часть задач", "Take some tasks"), ("s_music", "Включить спокойную музыку", "Play calm music")] },
];

// ---------------------------------------------------------------- sources

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
    Source { title: "MOXIE — oxygen from Mars air", url: "https://www.jpl.nasa.gov/missions/mars-2020-perseverance-rover/" },
    Source { title: "NASA — Space Communications (4–24 min to Mars)", url: "https://www.nasa.gov/missions/tech-demonstration/space-communications-7-things-you-need-to-know/" },
    Source { title: "NASA — Radioisotope Power Systems (MMRTG)", url: "https://science.nasa.gov/planetary-science/programs/radioisotope-power-systems/" },
    Source { title: "NASA LCROSS — water ice at the lunar south pole (2009)", url: "https://www.nasa.gov/mission/lcross/" },
    Source { title: "NASA HRP — 5 Hazards of Human Spaceflight", url: "https://www.nasa.gov/hrp/hazards/" },
    Source { title: "NASA-STD-3001 — 600 mSv career limit", url: "https://www.nasa.gov/reference/nasa-std-3001-technical-documents/" },
    Source { title: "Obradović 2021 — deep breaths reduce children’s arousal", url: "https://pubmed.ncbi.nlm.nih.gov/34813098/" },
    Source { title: "Habgood & Ainsworth 2011 — intrinsic integration in educational games", url: "https://nottingham-repository.worktribe.com/output/23577510/motivating-children-to-learn-effectively-exploring-the-value-of-intrinsic-integration-in-educational-games" },
    Source { title: "Icons: OpenMoji (CC BY-SA 4.0)", url: "https://openmoji.org/" },
];
