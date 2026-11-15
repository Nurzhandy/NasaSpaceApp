//! «NASA data & sources» for adults, teachers and judges.

use super::*;

pub(super) fn info(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, sky: &Sky) {
    p.spawn(Node { flex_grow: 1.0, padding: UiRect::all(Val::Px(22.0)), ..default() }).with_children(|o| {
        o.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), max_width: Val::Px(1000.0), ..default() }).with_children(|c| {
            c.spawn(kit::row(10.0)).with_children(|r| {
                kit::icon(r, art, "teacher", 38.0);
                kit::text(r, l.pick("Для взрослых, учителей и судей", "For adults, teachers & judges"), &art.head, 24.0, C_TEXT);
            });
            for (ic, ru, en) in [
                ("hammer", "Ребёнок сам строит базу: панели, батареи, кислород, оранжерея, щит, лаборатория. Энергии всегда меньше, чем нужно — каждое решение чего-то стоит.", "The child builds the base: panels, batteries, oxygen, greenhouse, shield, lab. There’s never enough power — every decision costs something."),
                ("radiation", "Дозы пропорциональны реальным измерениям: Луна 1,369 мЗв/сут (Chang’e‑4), Марс 0,64 (Curiosity RAD), глубокий космос 1,84.", "Doses follow real measurements: Moon 1.369 mSv/day (Chang’e‑4), Mars 0.64 (Curiosity RAD), deep space 1.84."),
                ("sun", "Вспышки — реальные события NASA DONKI. Без интернета используется архив, и на экране честно написано «АРХИВ».", "Flares are real NASA DONKI events. Offline, the archive is used and the screen honestly says “ARCHIVE”."),
                ("radio", "Сообщения ЦУП приходят с задержкой: Луна 1,3 с (реально), Марс 4–24 мин (в игре ускорено и подписано).", "Mission Control messages arrive late: Moon 1.3 s (real), Mars 4–24 min (sped up and labeled)."),
                ("heart", "Экипаж не погибает: эвакуация и разбор причины. Подсказки бесплатны, дыхательный маяк добровольный, рейтинга нет.", "No crew deaths: evacuation and a debrief. Hints are free, the breathing beacon is voluntary, no ranking."),
                ("key", "Живые данные: запустите игру с NASA_API_KEY=… (бесплатный ключ на api.nasa.gov).", "Live data: run the game with NASA_API_KEY=… (free key at api.nasa.gov)."),
            ] {
                c.spawn(kit::row(10.0)).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                    kit::icon(r, art, ic, 26.0);
                    kit::wrap_text(r, l.pick(ru, en), &art.body, 16.0, C_TEXT, 900.0);
                });
            }
            kit::text(c, if sky.weather.live { format!("DONKI LIVE · {} · {}", sky.weather.range, sky.weather.events.len()) } else { l.pick("DONKI: архив (нет сети или ключа)", "DONKI: archive (offline or no key)").to_string() }, &art.head, 15.0, if sky.weather.live { C_GOOD } else { C_WARN });
            kit::text(c, l.pick("Источники:", "Sources:"), &art.head, 17.0, C_TEXT);
            for s in SOURCES {
                kit::wrap_text(c, format!("• {} — {}", s.title, s.url), &art.body, 12.0, C_MUTED, 940.0);
            }
        });
    });
}
