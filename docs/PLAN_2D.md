# План: 2D инди-игра «Юный астронавт: Аванпост» (десктоп-приложение)

NASA Space Apps 2026 · челлендж **Junior Astronaut Mission Trainer** · дети 7–12 лет.
Двое разработчиков работают параллельно: **Часть A** — сокомандник с GPT, **Часть B** — сокомандник с Claude.

---

## 1. Что делаем и почему (выводы ресёрча)

| Вывод | Источник | Что это значит для игры |
|---|---|---|
| Самый популярный жанр у детей — **строить и создавать** (54%) | Ofcom | Ядро игры — ребёнок сам строит базу из модулей |
| Детям 8–12 нравятся игры про **создание, исследование и общение** | YPulse | Строить + исследовать планету + играть вдвоём |
| 62–73% детей хотят **играть вместе с родителями** | ESA (GamesBeat) | Кооператив вдвоём на одном экране |
| Знания должны быть **встроены в сам игровой процесс**: такая игра учит лучше, а в свободное время в неё играют в 7 раз дольше | Habgood & Ainsworth | Щит строишь, потому что пришла настоящая вспышка NASA, а не ради викторины |
| Свобода исследовать → больше мотивации; ограничения нужно объяснять | arXiv 1709.09931, Harvard GSE | Открытый мир с понятными замками («нужна антенна, чтобы…») |
| Среди победителей Space Apps 2025 — **пиксельная игра на данных NASA** и проект про космическую погоду для детей | Space Apps 2025 | Пиксель-арт + космическая погода — проверенный формат |

**Концепция:** уютная 2D-игра в пиксель-арте, вид сверху. Астронавт ходит по поверхности Луны или Марса, **строит базу на сетке** (жилой модуль, солнечные панели, оранжерея, генератор кислорода, щит, лаборатория, антенна), **исследует** окрестности с ровером и собирает образцы. Энергии не хватает на всё, и каждое решение чего-то стоит. Вспышки на Солнце — **настоящие события NASA DONKI**: живые, если есть интернет, и архивные, если его нет; на экране всегда честная подпись LIVE или АРХИВ. Экипаж никогда не погибает: при неудаче происходит эвакуация и разбор миссии.

**Формат:** десктоп-приложение (Windows / macOS / Linux), запуск без интернета, один `.exe` / `.app` + папка `assets`.

### Что переносим из текущей 3D-версии
- Симуляция ресурсов (кислород, еда, доза, наука, настроение, батарея) — `src/sim.rs`
- Данные карт, реальные дозы (RAD, Chang’e‑4), архив DONKI, скины, нашивки, источники — `src/data.rs`
- Загрузка DONKI в фоне + архив — `src/nasa.rs`; профиль игрока — `src/save.rs`
- Психология: план «если — то», маяк-дыхание, поддержка экипажа, разбор миссии
- Баланс-тест Монте-Карло

---

## 2. Технологии

- **Rust + Bevy 0.16** (2D: `Sprite`, атласы, `bevy_ecs_tilemap` по желанию). Почему: вся логика у нас уже на Rust, Bevy собирает нативное приложение под все ОС.
- **Структура — Cargo workspace из двух крейтов:**

```
NasaSpaceApp/
├─ Cargo.toml              # [workspace] members = ["crates/core", "crates/game"]
├─ crates/
│  ├─ core/                # ЧАСТЬ A — чистый Rust, БЕЗ Bevy: правила игры
│  │  └─ src/ lib.rs, sim.rs, build.rs, events.rs, explore.rs, data.rs, nasa.rs, save.rs, i18n.rs
│  └─ game/                # ЧАСТЬ B — Bevy-приложение: всё, что видно и слышно
│     └─ src/ main.rs, render/, player.rs, camera.rs, ui/, menu/, audio.rs, input.rs
├─ assets/                 # ЧАСТЬ B: спрайты, тайлы, шрифты, звуки
└─ docs/                   # общие документы (этот план, контракт, скриншоты)
```

- **Пиксель-арт:** тайлы 16×16 или 32×32, масштаб ×3. Источники: свои спрайты в Aseprite/LibreSprite или паки [Kenney](https://kenney.nl/) (CC0). Иконки OpenMoji уже есть в `assets/icons`. **У каждого ассета проверяем лицензию** и записываем её в `assets/CREDITS.md`.
- **Сборка:** нужны Rust и (на Windows) Visual Studio Build Tools «Разработка классических приложений на C++».

---

## 3. Контракт между частями — ГОТОВ (`crates/core`, пакет `ja-core`)

> Пакет называется `ja-core`, а не `core`: имя `core` занято стандартной библиотекой Rust.
> Заглушка `mock` не понадобилась: настоящая логика уже готова и покрыта тестами, часть B подключает её сразу.

Подключение в `crates/game/Cargo.toml`: `ja-core = { path = "../core" }`, и добавить `"crates/game"` в `members` корневого `Cargo.toml`.

```rust
use ja_core::{Game, MissionConfig, MapId, Command, GameEvent, Refusal, ModuleKind, Weather, Profile, i18n, progress, weather};

// старт: погода NASA в фоне (один раз при запуске приложения)
let rx = weather::spawn_fetch(weather::api_key(&profile.nasa_key)); // rx.try_recv() каждый кадр → Weather (LIVE или АРХИВ)
let mut game = Game::new(MissionConfig { map: MapId::Moon, cadet: profile.cadet, coop: false, live: true, seed: rng_seed }, &weather);

// каждый кадр
game.set_player_pos(0, x_cells, y_cells);       // где стоит астронавт (в клетках сетки)
for e in game.tick(dt) { /* звук, анимация, i18n::event_msg(&e) */ }

// действие игрока
match game.apply(0, Command::Build { kind: ModuleKind::Shield, x, y }) {
    Ok(events) => { /* показать i18n::event_msg */ }
    Err(r) => { /* показать i18n::refusal_msg(r) */ }
}
game.can_build(kind, x, y)      // призрак стройки: Ok — зелёный, Err(причина) — красный
game.find_free_spot(kind)       // подсказка «куда поставить»
game.set_paused(true)           // меню, маяк-дыхание, окна в одиночной игре
let s = game.state();           // всё для отрисовки: modules, sites, players, o2, food, dose, battery, flare, mc, forecast…

// конец миссии: GameEvent::MissionEnded(result)
let rewards = profile.finish(&result); progress::save(&profile);
```

**Команды** (`Command`): `Build{kind,x,y}`, `Demolish{x,y}`, `SetPower{module,blocks}`, `Repair{module}` (после «держи F 3 с»), `Clean{module}` (пыльная панель), `Collect{site}` (образец, руда, обломки, лёд), `DeliverSamples`, `EnterShelter`, `ExitShelter`, `ChooseHarvest(eat)`, `ChooseCargo(all)`, `NameFeeling(i)`, `Support(i)`.

**Правила для части B:**
- Координаты — клетки сетки (`f32` для игроков и находок, `i32` для модулей). Купол занимает клетки −1..1, дверь — `game::DOOR`, площадка груза — `game::CARGO_PAD`. Строить можно в квадрате ±`BUILD_R` (11) клеток, исследовать — в радиусе `WORLD_R` (36).
- «Подойди и нажми» проверяет ядро: игрок должен быть не дальше `NEAR` (2,5 клетки). Иначе придёт `Refusal::TooFar`.
- Все тексты для ребёнка: `i18n::event_msg`, `refusal_msg`, `forecast_msg`, `cause_msg`, `TUTORIAL`, `IF_THEN`, `REFLECT_HELPED`, `REFLECT_WHERE`, а также имена и описания в `data` (`MAPS`, `MODULES`, `SITES`, `SUITS` / `VISORS` / `HATS`, `BADGES`, `MOODS`, `SOURCES`). Сам ничего не пиши.
- Цвета карт и скинов — `[f32; 3]` sRGB: `Color::srgb(c[0], c[1], c[2])`.
- Нашивку «маяк» выдаёт B: `profile.award("beacon")` после двух спокойных вдохов.
- Сборка с живыми данными NASA (фича `live`, по умолчанию) требует VS Build Tools на Windows. Без них: `--no-default-features`, тогда всегда будет архив.
- Изменения API ядра — только по договорённости.

---

## 4. Часть A — «Мозг игры» (`crates/core`) — ВЫПОЛНЕНО

| # | Задача | Статус |
|---|---|---|
| A0 | Контракт (раздел 3) | ✅ без заглушки: сразу настоящий API |
| A1 | Перенос симуляции, данных, DONKI, профиля без Bevy | ✅ `game.rs`, `data.rs`, `weather.rs`, `progress.rs` |
| A2 | Строительство: сетка, 9 типов модулей, стоимость, снос (½ материалов назад) | ✅ |
| A3 | Энергия по модулям: панели (день/ночь/пыль), РИТЭГ, батареи +5, потребители по блокам | ✅ |
| A4 | Исследование: образцы → лаборатория (только запитанная), руда +3, обломки +6, лёд +10 O₂ | ✅ |
| A5 | События: вспышки DONKI LIVE/АРХИВ, буря, поломка, утечка, урожай, груз, эмоции, затмение связи, прогноз ЦУП с задержкой (без энергии антенны прогноза нет) | ✅ |
| A6 | Прогресс: очки → карты, звёзды → скины, 15 нашивок (+ «Строитель базы»), сохранение `~/.junior_astronaut_outpost.json` (`JA_PROFILE` — свой путь) | ✅ |
| A7 | Баланс Монте-Карло через публичный API | ✅ таблица ниже |
| A8 | Тексты RU/EN для всех событий, отказов, прогнозов, причин, обучения, плана «если — то», рефлексии | ✅ |

**Тесты:** `cargo test -p ja-core` — 21 тест. **Баланс:** `cargo test -p ja-core --release balance -- --ignored --nocapture`.

| База | Аккуратный: побед | доза | Небрежный: побед | доза |
|---|---|---|---|---|
| Земля (обучение) | 100% | 0% | 100% | 0% |
| Луна | 100% | 58% | 68% | 93% |
| Марс | 100% | 37% | 100% | 61% |
| Окололунная станция | 100% | 55% | 0% | 100% |
| Европа | 100% | 63% | 0% | 100% |

«Аккуратный» строит щит, слушает прогноз и прячется от вспышек; «небрежный» — нет. Небрежный проигрывает только из-за радиации. Значит, щит и убежище действительно решают исход. Третью звезду (доза < 50%) аккуратный игрок на Луне, Орбите и Европе получает не всегда: это задача для тех, кто играет лучше.

---

## 5. Часть B — «Тело игры» (`crates/game` + `assets`) — ВЫПОЛНЕНО

Всё, что видно, слышно и нажимается. Берёт данные только из `Game::state()`, событий и `i18n` ядра `ja-core`.

| # | Задача | Статус |
|---|---|---|
| B0 | Workspace, крейт `game` (Bevy 0.16), подключён настоящий `ja-core` | ✅ `crates/game` в `members` |
| B1 | Арт: свой пиксель-арт 16×16 ×3 (астронавт 4 направления × 4 кадра + визор/шляпы как слои, тайлы 5 карт, 9 модулей, находки, ровер, груз, иконка) | ✅ `crates/game/tools/make_sprites.py`, `assets/CREDITS.md` |
| B2 | Мир: тайлы, камни/кратеры, граница `WORLD_R`, камера с плавным следованием, день/ночь, пыльная буря, рамка вспышки, свечение щита | ✅ `world.rs`, `camera.rs` |
| B3 | Игрок: ходьба, коллизии, «подойди и нажми F», «держи F» → `Repair` / `Clean`, подсказка над головой | ✅ `player.rs` |
| B4 | Стройка: панель модулей (только разрешённые на карте), призрак по `can_build` (зелёный/красный + `refusal_msg`), снос, мышь/клавиатура/геймпад | ✅ `build.rs` |
| B5 | HUD и окна: ресурсы, сол-часы, баннер вспышки LIVE/АРХИВ, радио ЦУП с задержкой (`mc.arrive_at`), пульт энергии, урожай, груз, поддержка экипажа, маяк (нашивка после 2 вдохов), пауза, обучение `TUTORIAL` | ✅ `hud.rs`, `windows.rs` |
| B6 | Меню: главное, выбор базы с замками, брифинг с `IF_THEN`, магазин скинов, нашивки, «Данные NASA и источники», разбор с графиком и `REFLECT_*` (без очков) | ✅ `menu.rs` |
| B7 | Кооператив на одном экране: общая камера, P2 — стрелки + правый Shift / второй геймпад | ✅ |
| B8 | Звуки событий, музыка (свой чиптюн), RU/EN, звук/музыка/возраст | ✅ `audio.rs`, `tools/make_music.py` |
| B9 | Релиз: `bash crates/game/tools/package.sh` → `dist/JuniorAstronaut.app` + zip (на Linux/Windows — запустить скрипт там); скриншоты `docs/screenshots_2d/` (тур: `JA_SHOTS=папка cargo run -p game`) | ✅ macOS; Windows/Linux — собрать на своей ОС |

**Запуск:** `cargo run -p game --release` (из корня; живые данные NASA — `NASA_API_KEY=… cargo run -p game --release`).

**Управление:** WASD — ходить · F — действие (держать — ремонт/чистка) · B — стройка (клик/F строить, ПКМ/X разобрать, Q/R выбор) · Tab — пульт энергии · Esc — пауза. Игрок 2: стрелки, правый Shift — действие, правый Ctrl — стройка. Геймпад: стик, A, Y — стройка, X — снос, Select — энергия, Start — пауза.

**Заметка для части A:** дверь купола `DOOR` стоит на севере (y = 2.5), а на пиксель-арте дверь нарисована снизу (вид сверху 3/4). Если удобно — перенести `DOOR` на (0.5, −1.5), чтобы астронавт выходил из нарисованной двери.

---

## 6. Порядок работы и сроки

| Этап | Часть A (GPT) | Часть B (Claude) | Встреча |
|---|---|---|---|
| День 1 | ✅ A0, A1 | ✅ B0, B1 | Утверждаем контракт |
| День 2 | ✅ A2, A3 | ✅ B1, B2, B3 | Демо: ходим по базе на `mock` |
| День 3 | ✅ A4, A5 | ✅ B4, B5 | B уже на настоящем `ja-core` |
| День 4 | ✅ A6, A7, A8 | ✅ B6, B7 | Полный круг игры |
| День 5 | Правки баланса по игре | ✅ B8, B9 | Плейтест на ребёнке 7–12, запись видео |

---

## 7. Правила Git, чтобы не мешать друг другу

- Ветки: `part-a/<задача>` и `part-b/<задача>`. В `main` — только через Pull Request.
- **A трогает только `crates/core/`, B — только `crates/game/` и `assets/`.** Общие файлы (`Cargo.toml` в корне, `docs/`, контракт в `lib.rs`) меняем отдельным маленьким PR и предупреждаем друг друга.
- Перед PR: `cargo fmt`, `cargo clippy`, `cargo test`.
- Каждый день: `git pull`, короткий созвон на 10 минут — что сделал, что дальше, что мешает.
- Никаких ключей API в коде: ключ NASA — только через переменную окружения `NASA_API_KEY`.

---

## 8. Что должно быть в финальной версии (для судей)

- **Impact:** понятно ребёнку 7–12 без чтения инструкций: иконки, крупные кнопки, RU/EN, два режима возраста, игра вдвоём.
- **Creativity:** строишь свою базу, а настоящие вспышки NASA проверяют, правильно ли ты её построил.
- **Validity:** реальные дозы (Curiosity RAD, Chang’e‑4), события DONKI с подписью LIVE/АРХИВ, баланс-тест, страница источников в игре.
- **Relevance:** жизнеобеспечение, защита от радиации, энергия, еда, задержка связи, открытые данные NASA.
- **Presentation:** красивый пиксель-арт, видео 30–60 секунд, скриншоты, README.

**Риски и что делаем:**
- *Не успеваем арт.* Сначала цветные квадраты, арт — после того, как игра работает.
- *Контракт меняется слишком часто.* Изменения — только на встрече.
- *Живые данные NASA недоступны на демо.* Архив всегда работает и честно подписан.

---

## 9. Источники ресёрча

**Что любят дети 7–12**
- Ofcom — Growing up gaming (research summary): https://www.ofcom.org.uk/siteassets/resources/documents/online-safety/research-statistics-and-data/protecting-children/grow-up-gaming-summary/growing-up-gaming-research-summary.pdf
- YPulse — Tweens’ Top 10 Favorite Video and Mobile Games (2026): https://www.ypulse.com/article/2026/07/06/tweens-top-10-favorite-video-and-mobile-games/
- YPulse — Tweens: Gaming (опрос 1 500 детей 8–12, 2026): https://www.ypulse.com/report/2026/06/30/tweens-gaming/
- YPulse — The Top Kinds of Video Games Tweens Like to Play: https://www.ypulse.com/?p=186364
- GamesBeat — ESA: kids prefer playing games with their parents: https://gamesbeat.com/kids-love-games-but-a-new-esa-survey-finds-they-prefer-playing-them-with-their-parents/
- Game Developer — What kind of games are popular with both children and parents: https://gamedeveloper.com/business/what-kind-of-games-are-popular-with-both-children-and-parents-
- GWI — What Gen Alpha’s habits tell us about the future of gaming: https://gwi.com/blog/gen-alphas-habits-future-of-gaming

**Как дети учатся в играх**
- Habgood & Ainsworth — Motivating children to learn effectively: intrinsic integration in educational games: https://nottingham-repository.worktribe.com/output/23577510/motivating-children-to-learn-effectively-exploring-the-value-of-intrinsic-integration-in-educational-games
- Educational game design: game elements for promoting engagement (arXiv 1709.09931): https://arxiv.org/pdf/1709.09931
- Harvard Graduate School of Education — Find fun: https://www.gse.harvard.edu/ideas/usable-knowledge/21/06/find-fun
- LEGO Foundation — Learning through digital play (full report): https://cms.learningthroughplay.com/media/ny5ndvdq/learning-through-digital-play_full-report.pdf

**Что побеждает на Space Apps**
- Space Apps 2025 — Meet the Global Winners: https://www.spaceappschallenge.org/2025/meet-the-2025-global-winners/
- Space Apps 2022 — Global Winners: https://www.spaceappschallenge.org/blog/2022-global-winners/
- itch.io — игры с тегом spaceappchallenge: https://itch.io/games/tag-spaceappchallenge

**Научные данные в игре** (уже используются в 3D-версии)
- NASA DONKI: https://ccmc.gsfc.nasa.gov/tools/DONKI/
- Curiosity RAD (Hassler 2014): https://science.nasa.gov/resource/curiosity-rad-measurements/
- Chang’e‑4 LND (Zhang 2020): https://www.science.org/doi/10.1126/sciadv.aaz1334
- NASA HRP — 5 Hazards of Human Spaceflight: https://www.nasa.gov/hrp/hazards/
- Obradović 2021 — дыхание и возбуждение у детей: https://pubmed.ncbi.nlm.nih.gov/34813098/
- Полный список — в `src/data.rs` (`SOURCES`) и в README.
