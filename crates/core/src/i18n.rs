//! All child-facing texts of the rules (RU/EN). PART A (A8) completes this; part B uses it.

use crate::{GameEvent, ModuleKind, Refusal, SiteKind};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

impl Lang {
    pub fn t<'a>(self, ru: &'a str, en: &'a str) -> &'a str {
        match self {
            Lang::Ru => ru,
            Lang::En => en,
        }
    }
}

fn module_name(k: ModuleKind, l: Lang) -> &'static str {
    let s = crate::catalog::module_spec(k);
    l.t(s.name_ru, s.name_en)
}

/// Why an action is not possible — short and kind.
pub fn refusal(r: &Refusal, l: Lang) -> String {
    match r {
        Refusal::NotEnoughPower => l.t("Не хватает энергии ⚡", "Not enough power ⚡").into(),
        Refusal::NotEnoughMaterials => l.t("Нужно больше материалов — ищи обломки", "Need more materials — look for debris").into(),
        Refusal::CellBusy => l.t("Здесь уже что-то стоит", "Something is already here").into(),
        Refusal::TooFar => l.t("Подойди ближе", "Come closer").into(),
        Refusal::OutsideBase => l.t("Строить можно только рядом с базой", "You can build only near the base").into(),
        Refusal::NeedsModule(k) => format!("{} {}", l.t("Сначала построй:", "Build this first:"), module_name(*k, l)),
        Refusal::LabNeedsPower => l.t("Лаборатории нужна энергия", "The lab needs power").into(),
        Refusal::CannotDemolish => l.t("Жилой модуль сносить нельзя", "The habitat can’t be removed").into(),
        Refusal::NothingToDo => l.t("Здесь пока нечего делать", "Nothing to do here yet").into(),
        Refusal::MaxPower => l.t("Больше энергии сюда не нужно", "It can’t take more power").into(),
    }
}

/// Icon + text for an event alert, or None if the event needs no alert.
pub fn event(e: &GameEvent, l: Lang) -> Option<(&'static str, String)> {
    Some(match e {
        GameEvent::Flare { label_ru, label_en, date, live } => (
            "sun",
            format!("{} {} ({}) · {}", l.t("ВСПЫШКА! Беги в жилой модуль.", "FLARE! Run to the habitat."), l.t(label_ru, label_en), date, if *live { "LIVE" } else { l.t("архив", "archive") }),
        ),
        GameEvent::FlareEnded => ("check", l.t("Вспышка закончилась", "The flare is over").into()),
        GameEvent::NewSol(n) => ("hourglass", format!("{} {n}", l.t("Новый сол", "New sol"))),
        GameEvent::ModuleBroken(_) => ("wrench", l.t("Модуль сломался — подойди и держи F", "A module broke — go and hold F").into()),
        GameEvent::Leak(_) => ("comet", l.t("Микрометеорит! Утечка воздуха", "Micrometeorite! Air leak").into()),
        GameEvent::DustStorm => ("tornado", l.t("Пылевая буря! Почисти панели", "Dust storm! Clean the panels").into()),
        GameEvent::HarvestReady => ("plant", l.t("Урожай готов в оранжерее", "Harvest ready in the greenhouse").into()),
        GameEvent::CargoLanding => ("box", l.t("Грузовой корабль садится", "A cargo ship is landing").into()),
        GameEvent::CrewNeedsSupport(_) => ("speech", l.t("Члену экипажа нужна поддержка", "A crewmate needs support").into()),
        GameEvent::Built(_) => ("hammer", l.t("Построено!", "Built!").into()),
        GameEvent::Demolished => ("box", l.t("Модуль разобран", "Module removed").into()),
        GameEvent::Repaired(_) => ("check", l.t("Готово, всё работает", "Fixed, all working").into()),
        GameEvent::Picked(SiteKind::Sample) => ("gem", l.t("Образец в рюкзаке — неси в лабораторию", "Sample in your pack — take it to the lab").into()),
        GameEvent::Picked(SiteKind::Ice) => ("ice", l.t("Лёд → кислород +10%", "Ice → oxygen +10%").into()),
        GameEvent::Picked(SiteKind::Wreck) => ("wrench", l.t("Материалы +4", "Materials +4").into()),
        GameEvent::SamplesDelivered { science } => ("science", format!("{} +{:.0}", l.t("Наука", "Science"), science)),
        GameEvent::FeelingChecked { .. } => return None,
        GameEvent::Supported => ("heart", l.t("Экипажу стало лучше", "The crew feels better").into()),
        GameEvent::BadgeEarned(_) => ("medal", l.t("Новая нашивка!", "New patch!").into()),
        GameEvent::ForecastArrived(_) | GameEvent::MissionEnded(_) => return None,
    })
}

/// Explanation on the debrief screen.
pub fn cause(c: crate::Cause, l: Lang) -> &'static str {
    match c {
        crate::Cause::Oxygen => l.t("Кислород закончился. Экипаж тратит 12% за сол — генератору нужно больше энергии.", "Oxygen ran out. The crew uses 12% per sol — the generator needs more power."),
        crate::Cause::Food => l.t("Еда закончилась. Оранжерее нужно около 2 блоков энергии.", "Food ran out. The greenhouse needs about 2 power blocks."),
        crate::Cause::Dose => l.t("Доза дошла до предела. Щит и убежище защищают от вспышек. У NASA тоже есть предел: 600 мЗв за карьеру.", "The dose hit the limit. The shield and shelter protect from flares. NASA has a limit too: 600 mSv per career."),
    }
}
