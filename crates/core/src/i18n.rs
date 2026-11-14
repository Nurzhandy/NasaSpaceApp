//! Every sentence a child sees, in Russian and English. Part B shows these and never writes its own.
//! The game is icon-first: each message carries an icon name from `assets/icons`.

use crate::data::*;
use crate::game::{Cause, Forecast, GameEvent, Refusal};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

impl Lang {
    pub fn pick<'a>(self, ru: &'a str, en: &'a str) -> &'a str {
        match self {
            Lang::Ru => ru,
            Lang::En => en,
        }
    }
}

/// How a message should look (colour / sound).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Info,
    Good,
    Warn,
    Danger,
    Badge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    pub icon: &'static str,
    pub ru: String,
    pub en: String,
    pub tone: Tone,
}

impl Msg {
    pub fn new(tone: Tone, icon: &'static str, ru: impl Into<String>, en: impl Into<String>) -> Self {
        Self { icon, ru: ru.into(), en: en.into(), tone }
    }
    pub fn text(&self, lang: Lang) -> &str {
        lang.pick(&self.ru, &self.en)
    }
}

/// Child-facing message for an event (`None` = nothing to show, e.g. a silent bookkeeping event).
pub fn event_msg(e: &GameEvent) -> Option<Msg> {
    use GameEvent::*;
    use Tone::*;
    Some(match e {
        NewSol(n) => Msg::new(Info, "hourglass", format!("Сол {n}"), format!("Sol {n}")),
        Night => Msg::new(Warn, "night", "Лунная ночь: солнечные панели не работают. Береги батарею!", "Lunar night: solar panels don't work. Save the battery!"),
        ForecastSent { arrives_in } if *arrives_in > 0.5 => Msg::new(Info, "radio", format!("ЦУП отправил прогноз. Сигнал летит {arrives_in:.1} с"), format!("Mission Control sent a forecast. The signal takes {arrives_in:.1} s")),
        ForecastSent { .. } => return None,
        ForecastArrived(f) => forecast_msg(*f),
        Flare { label_ru, label_en, date, live } => {
            let (sr, se) = if *live { ("LIVE", "LIVE") } else { ("архив", "archive") };
            Msg::new(Danger, "radiation", format!("ВСПЫШКА! Беги в купол. NASA DONKI: {label_ru} · {date} · {sr}"), format!("FLARE! Run into the dome. NASA DONKI: {label_en} · {date} · {se}"))
        }
        FlareEnded => Msg::new(Good, "check", "Вспышка закончилась — можно выходить", "The flare is over — you can go outside"),
        DustStorm => Msg::new(Warn, "tornado", "Пылевая буря! Панели в пыли — подойди и держи F, чтобы почистить", "Dust storm! The panels are dusty — walk up and hold F to clean them"),
        DustGone => Msg::new(Good, "sun", "Буря закончилась, панели снова чистые", "The storm is over, the panels are clean again"),
        ModuleBroken(_) => Msg::new(Warn, "wrench", "Генератор кислорода сломался — подойди и держи F", "The oxygen generator broke — walk up and hold F"),
        Leak(_) => Msg::new(Warn, "comet", "Микрометеорит! Утечка воздуха в куполе — подойди и держи F", "Micrometeorite! An air leak in the dome — walk up and hold F"),
        HarvestReady => Msg::new(Good, "plant", "Урожай готов в оранжерее", "The greenhouse harvest is ready"),
        Discovery(_) => Msg::new(Info, "gem", "Робот БИП нашёл новые находки вокруг базы", "Robot BEEP found new things around the base"),
        CargoLanding { .. } => Msg::new(Info, "box", "Грузовой корабль садится у купола", "A cargo ship is landing by the dome"),
        CrewNeedsSupport(m) => Msg::new(Info, "speech", format!("{} нужна поддержка", MOODS[*m].who_ru), format!("{} needs support", MOODS[*m].who_en)),
        SignalLost => Msg::new(Warn, "antenna_bars", "Солнце между нами и Землёй: связи не будет 2 сола", "The Sun is between us and Earth: no signal for 2 sols"),
        Built { kind, .. } => {
            let d = module_def(*kind);
            Msg::new(Good, d.icon, format!("Построено: {}", d.name_ru), format!("Built: {}", d.name_en))
        }
        Demolished { kind, refund } => {
            let d = module_def(*kind);
            Msg::new(Info, "hammer", format!("{} разобран(а): +{refund} материалов", d.name_ru), format!("{} taken apart: +{refund} materials", d.name_en))
        }
        Repaired(_) => Msg::new(Good, "check", "Починено! Ремонт взял немного батареи", "Fixed! The repair used a little battery"),
        Cleaned(_) => Msg::new(Good, "broom", "Панель чистая — энергии больше", "Panel clean — more power"),
        Collected { kind, .. } => match kind {
            SiteKind::Sample => Msg::new(Good, "gem", "Образец взят. Отнеси его в лабораторию", "Sample picked up. Take it to the lab"),
            SiteKind::Ore => Msg::new(Good, "rock", "+3 материала для стройки", "+3 building materials"),
            SiteKind::Wreck => Msg::new(Good, "rocket", "Детали старого модуля: +6 материалов", "Old lander parts: +6 materials"),
            SiteKind::Ice => Msg::new(Good, "ice", "Лёд растоплен: +10 кислорода", "Ice melted: +10 oxygen"),
        },
        SamplesDelivered { count, science } => Msg::new(Good, "science", format!("Образцы в лаборатории ({count}): +{science:.1} науки"), format!("Samples in the lab ({count}): +{science:.1} science")),
        Sheltered { .. } => Msg::new(Info, "house", "В убежище. Здесь радиации почти нет", "In the shelter. Almost no radiation here"),
        LeftShelter { .. } => return None,
        Harvested { eat: true } => Msg::new(Good, "plate", "Вкусный ужин! +12 еды, +10 настроения", "Tasty dinner! +12 food, +10 mood"),
        Harvested { eat: false } => Msg::new(Good, "science", "Растения изучены: +2 науки", "Plants studied: +2 science"),
        CargoUnloaded { all: true } => Msg::new(Good, "box", "Груз разгружен: еда, кислород и материалы", "Cargo unloaded: food, oxygen and materials"),
        CargoUnloaded { all: false } => Msg::new(Good, "bread", "Взяли еду: +8", "Took food: +8"),
        FeelingRight(f) => Msg::new(Good, FEELINGS[*f].icon, format!("Да, похоже на {}. Как поддержать?", FEELINGS[*f].name_ru), format!("Yes, it looks like {}. How can you help?", FEELINGS[*f].name_en)),
        FeelingTryAgain(_) => Msg::new(Info, "hint", "Может быть. А что ещё? Посмотри на подсказки в истории", "Maybe. What else could it be? Look at the clues in the story"),
        Supported(_) => Msg::new(Good, "heart", "Экипажу стало легче · настроение +20", "The crew feels better · mood +20"),
        LowOxygen => Msg::new(Danger, "oxygen", "Мало кислорода! Дай больше энергии генератору кислорода", "Low oxygen! Give more power to the oxygen generator"),
        BatteryEmpty => Msg::new(Danger, "battery", "Батарея пуста — модули работают слабее. Убавь энергию или построй панели", "Battery empty — modules run weaker. Use less power or build panels"),
        MissionEnded(r) => {
            if r.won {
                Msg::new(Good, "trophy", "Миссия выполнена!", "Mission complete!")
            } else {
                Msg::new(Warn, "rocket", "Экипаж эвакуирован — все целы", "Crew evacuated — everyone is safe")
            }
        }
    })
}

pub fn forecast_msg(f: Forecast) -> Msg {
    use Tone::*;
    match f {
        Forecast::Sure(EventKind::Flare) => Msg::new(Warn, "sun", "Завтра вспышка! Готовь щит и убежище", "Flare tomorrow! Get the shield and shelter ready"),
        Forecast::Maybe(EventKind::Flare) => Msg::new(Warn, "sun", "Возможно, завтра вспышка (~70%)", "Maybe a flare tomorrow (~70%)"),
        Forecast::Sure(EventKind::Dust) => Msg::new(Warn, "tornado", "Завтра пылевая буря: энергии будет мало", "Dust storm tomorrow: little power"),
        Forecast::Sure(EventKind::Failure) => Msg::new(Warn, "wrench", "Завтра генератор может сломаться", "The generator may fail tomorrow"),
        Forecast::Sure(EventKind::Leak) => Msg::new(Warn, "comet", "Завтра поток микрометеоритов", "Micrometeorite shower tomorrow"),
        Forecast::Night => Msg::new(Warn, "night", "Завтра лунная ночь: зарядите батарею", "Lunar night tomorrow: charge the battery"),
        Forecast::NoSignal => Msg::new(Warn, "antenna_bars", "…связи нет: Солнце глушит сигнал", "…no signal: the Sun blocks the radio"),
        Forecast::NoAntenna => Msg::new(Warn, "satellite", "Антенна без энергии — прогноз не пришёл", "The antenna has no power — no forecast"),
        _ => Msg::new(Info, "check", "Завтра всё спокойно", "Tomorrow looks calm"),
    }
}

pub fn refusal_msg(r: Refusal) -> Msg {
    use Refusal::*;
    let w = |icon, ru: &str, en: &str| Msg::new(Tone::Warn, icon, ru, en);
    match r {
        MissionOver => w("flag", "Миссия уже закончилась", "The mission is over"),
        NotEnoughPower => w("power", "Не хватает энергии. Убавь другой модуль или построй панели", "Not enough power. Turn another module down or build panels"),
        NotEnoughMaterials => w("rock", "Не хватает материалов. Собери руду вокруг базы", "Not enough materials. Collect ore around the base"),
        CellBusy => w("cross", "Здесь занято", "This spot is taken"),
        OutsideBase => w("cross", "Слишком далеко от купола", "Too far from the dome"),
        NotAvailableHere => w("cross", "На этой базе это построить нельзя", "This can't be built at this base"),
        OnlyOne => w("house", "Купол может быть только один", "There can be only one dome"),
        CannotDemolish => w("house", "Купол разбирать нельзя — там живёт экипаж", "The dome can't be removed — the crew lives there"),
        NoModuleThere => w("cross", "Здесь ничего нет", "Nothing here"),
        NoPowerSlot => w("power", "Этот модуль не берёт энергию, он её даёт или хранит", "This module doesn't use power, it makes or stores it"),
        TooFar => w("walk", "Подойди поближе", "Walk closer"),
        InsideShelter => w("exit", "Сначала выйди из купола", "Leave the dome first"),
        NothingToRepair => w("check", "Здесь всё в порядке", "Everything is fine here"),
        NothingToClean => w("check", "Панель уже чистая", "The panel is already clean"),
        HandsFull => w("gem", "Руки заняты: отнеси образцы в лабораторию", "Hands full: take the samples to the lab"),
        NothingToDeliver => w("gem", "Сначала найди образцы", "Find some samples first"),
        NeedsModule(k) => {
            let d = module_def(k);
            w(d.icon, &format!("Сначала построй: {}", d.name_ru), &format!("Build this first: {}", d.name_en))
        }
        LabNeedsPower => w("power", "Лаборатории нужна энергия", "The lab needs power"),
        NotReady => w("hourglass", "Пока не готово", "Not ready yet"),
        AlreadyInside => w("house", "Ты уже в куполе", "You're already inside"),
        NotInside => w("house", "Ты снаружи", "You're outside"),
        NoMood => w("smile", "Всё хорошо, помощь не нужна", "All good, no help needed"),
        NameFeelingFirst => w("speech", "Сначала угадай, что он(а) чувствует", "First guess how they feel"),
        UnknownId => w("cross", "Не найдено", "Not found"),
    }
}

/// Why the crew had to evacuate, with an if–then tip (for the debrief).
pub fn cause_msg(c: Cause) -> Msg {
    match c {
        Cause::Oxygen => Msg::new(Tone::Warn, "oxygen", "Кончился кислород. Если — то: давай больше энергии генератору и сразу чини поломки.", "Oxygen ran out. If–then: give the generator more power and fix breakdowns right away."),
        Cause::Food => Msg::new(Tone::Warn, "food", "Кончилась еда. Оранжерея и грузовой корабль помогают продержаться.", "Food ran out. The greenhouse and the cargo ship help you last."),
        Cause::Dose => Msg::new(Tone::Warn, "radiation", "Слишком большая доза радиации. Построй щит и прячься в купол во время вспышки.", "The radiation dose got too high. Build a shield and hide in the dome during a flare."),
    }
}

/// (icon, ru, en) — first-mission tutorial pages.
pub const TUTORIAL: &[(&str, &str, &str)] = &[
    ("walk", "Ходи клавишами WASD или стиком. Подойди к модулю — над ним появится подсказка.", "Walk with WASD or the stick. Walk up to a module and a hint appears."),
    ("power", "Энергии мало. Открой пульт и раздели её между кислородом, едой, щитом и наукой.", "Power is short. Open the console and share it between oxygen, food, shield and science."),
    ("hammer", "Режим стройки: выбери модуль и поставь его на свободную клетку. Модули стоят материалов.", "Build mode: pick a module and place it on a free cell. Modules cost materials."),
    ("rock", "Материалы и образцы лежат вокруг базы. Собери их и отнеси образцы в лабораторию.", "Materials and samples lie around the base. Collect them and take samples to the lab."),
    ("sun", "Вспышка на Солнце? Беги в купол и нажми F. Внутри радиации почти нет.", "Solar flare? Run into the dome and press F. There is almost no radiation inside."),
];

/// (icon, ru, en) — the if–then plan shown in every briefing.
pub const IF_THEN: &[(&str, &str, &str)] = &[
    ("sun", "Если вспышка — то бегу в купол и жду внутри.", "If there is a flare — then I run into the dome and wait."),
    ("wrench", "Если поломка — то подхожу и держу F.", "If something breaks — then I walk up and hold F."),
    ("oxygen", "Если кислород падает — то даю больше энергии генератору.", "If oxygen drops — then I give the generator more power."),
    ("battery", "Если скоро ночь — то заряжаю батарею заранее.", "If night is coming — then I charge the battery early."),
];

/// Debrief reflection questions — no right answers, never scored.
pub const REFLECT_HELPED: &[(&str, &str, &str)] = &[
    ("shield", "Щит", "Shield"),
    ("house", "Убежище", "Shelter"),
    ("clipboard", "План «если — то»", "If–then plan"),
    ("heart", "Команда", "Teamwork"),
    ("beacon", "Пауза", "Pausing"),
    ("hammer", "Стройка", "Building"),
];
pub const REFLECT_WHERE: &[(&str, &str, &str)] = &[
    ("book", "В школе", "At school"),
    ("home", "Дома", "At home"),
    ("run", "В спорте", "In sports"),
    ("coop", "С друзьями", "With friends"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::*;
    use crate::weather::Weather;

    #[test]
    fn every_refusal_and_cause_has_both_languages() {
        let all = [
            Refusal::MissionOver, Refusal::NotEnoughPower, Refusal::NotEnoughMaterials, Refusal::CellBusy, Refusal::OutsideBase,
            Refusal::NotAvailableHere, Refusal::OnlyOne, Refusal::CannotDemolish, Refusal::NoModuleThere, Refusal::NoPowerSlot,
            Refusal::TooFar, Refusal::InsideShelter, Refusal::NothingToRepair, Refusal::NothingToClean, Refusal::HandsFull,
            Refusal::NothingToDeliver, Refusal::NeedsModule(ModuleKind::Lab), Refusal::LabNeedsPower, Refusal::NotReady,
            Refusal::AlreadyInside, Refusal::NotInside, Refusal::NoMood, Refusal::NameFeelingFirst, Refusal::UnknownId,
        ];
        for r in all {
            let m = refusal_msg(r);
            assert!(!m.ru.is_empty() && !m.en.is_empty() && m.ru != m.en, "{r:?}");
        }
        for c in [Cause::Oxygen, Cause::Food, Cause::Dose] {
            assert!(!cause_msg(c).en.is_empty());
        }
    }

    #[test]
    fn a_whole_mission_produces_readable_messages() {
        let mut g = Game::new(MissionConfig { map: MapId::Mars, cadet: false, coop: false, live: false, seed: 3 }, &Weather::archive());
        let mut shown = 0;
        while g.state().status == Status::Playing {
            for e in g.tick(0.5) {
                if let Some(m) = event_msg(&e) {
                    assert!(!m.text(Lang::Ru).is_empty() && !m.text(Lang::En).is_empty());
                    shown += 1;
                }
            }
        }
        assert!(shown >= 7, "at least one message per sol");
    }
}
