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

## 3. Контракт между частями (делаем ВМЕСТЕ в первый день)

Обе части общаются только через публичный API крейта `core`. Часть B не меняет правила игры, часть A ничего не рисует. Контракт лежит в `crates/core/src/lib.rs`; любое его изменение — отдельный маленький PR, который смотрят оба.

```rust
// crates/core/src/lib.rs — набросок, уточнить в первый день
pub struct MissionConfig { pub map: MapId, pub cadet: bool, pub coop: bool, pub live: bool, pub seed: u64 }

pub struct Game { /* всё состояние миссии */ }

impl Game {
    pub fn new(cfg: MissionConfig, weather: &Weather) -> Game;
    /// Продвинуть время на dt секунд; вернуть, что случилось (для звуков, анимаций, тревог).
    pub fn tick(&mut self, dt: f32) -> Vec<GameEvent>;
    /// Действие игрока. Ошибка объясняет, почему нельзя (показываем ребёнку).
    pub fn apply(&mut self, player: u8, cmd: Command) -> Result<Vec<GameEvent>, Refusal>;
    /// Только чтение: всё, что нужно нарисовать.
    pub fn state(&self) -> &State;
}

pub enum Command {
    Build { kind: ModuleKind, x: i32, y: i32 },
    Demolish { x: i32, y: i32 },
    SetPower { module: ModuleId, blocks: u8 },
    Repair { module: ModuleId },          // игра B вызывает после «держи F 3 секунды»
    PickSample { site: SiteId },
    DeliverSamples,
    EnterShelter, ExitShelter,
    ChooseHarvest(bool), ChooseCargo(bool),
    NameFeeling(FeelingId), Support(usize),
}

pub enum GameEvent {
    Flare { label_ru: String, label_en: String, date: String, live: bool },
    FlareEnded, NewSol(u32), ModuleBroken(ModuleId), Leak(ModuleId), DustStorm,
    HarvestReady, CargoLanding, CrewNeedsSupport(MoodId), ForecastArrived(Forecast),
    BadgeEarned(&'static str), MissionEnded(MissionResult),
}

pub enum Refusal { NotEnoughPower, NotEnoughMaterials, CellBusy, TooFar, NeedsModule(ModuleKind), /* … */ }
```

Правила контракта:
- Координаты — клетки сетки (`i32`), время — секунды (`f32`), цвета в `core` не храним.
- Все тексты, которые видит ребёнок, лежат в `core::i18n` (RU/EN), чтобы часть B не придумывала формулировки.
- Пока часть A не готова, часть B работает с **заглушкой** `core::mock::Game` с тем же API.

---

## 4. Часть A — сокомандник с GPT: «Мозг игры» (`crates/core`)

Только чистый Rust, без Bevy, всё покрыто тестами (`cargo test -p core`).

| # | Задача | Готово, когда |
|---|---|---|
| A0 | Вместе с B написать контракт (раздел 3) и заглушку `mock` | `cargo build` проходит у обоих |
| A1 | Перенести `sim.rs`, `data.rs`, `nasa.rs`, `save.rs` из 3D-версии в `core`, убрать зависимость от Bevy (`Color` → `[f32; 3]` или убрать совсем) | Старые формулы и баланс-тест работают в `core` |
| A2 | **Строительство:** сетка базы, типы модулей, стоимость (энергия + материалы), соседство (панели рядом с батареей, щит покрывает радиус), снос | Тесты: нельзя строить на занятой клетке, нельзя без материалов |
| A3 | **Энергия по модулям:** каждый модуль потребляет блоки, панели дают по солнцу (день/ночь, пыль), батарея копит | Тест: ночью на Луне без батареи кислород падает |
| A4 | **Исследование:** точки интереса на карте (образцы, кратеры, обломки), материалы для стройки, наука за доставку в лабораторию | Тест: доставка образцов даёт науку только при запитанной лаборатории |
| A5 | **События:** вспышки DONKI (live/архив), пыльная буря, поломки, утечка, урожай, груз, эмоции экипажа, прогноз ЦУП с задержкой связи | Каждое событие возвращает `GameEvent`; тест расписания по сиду |
| A6 | **Прогресс:** очки открывают карты (Земля → Луна 60 → Марс 180 → Орбита 400 → Европа 700), звёзды → скины, 14 нашивок, сохранение профиля | Тест: покупка скина не закрывает карту |
| A7 | **Баланс:** Монте-Карло (аккуратный и небрежный игрок) на новой симуляции | Аккуратный выигрывает ≥ 70% на каждой карте, небрежный заметно хуже |
| A8 | **Тексты RU/EN** для всех событий, подсказок, брифингов, разбора | Нет строк, которые часть B пишет сама |

**Как работать с GPT:**
- Давай ему этот файл и `crates/core/src/lib.rs` целиком.
- Проси по одной задаче из таблицы, а в конце: «напиши тесты и не меняй публичный API без согласования».
- Перед каждым PR запускай `cargo test -p core` и `cargo clippy -p core`.

---

## 5. Часть B — сокомандник с Claude: «Тело игры» (`crates/game` + `assets`)

Всё, что видно, слышно и нажимается. Берёт данные только из `Game::state()` и событий.

| # | Задача | Готово, когда |
|---|---|---|
| B0 | Вместе с A — контракт и `mock`; Cargo workspace; пустое окно Bevy | Окно открывается, `mock` подключён |
| B1 | **Арт-стиль и ассеты:** палитра, тайлсет поверхности (Луна, Марс), спрайты модулей, астронавт (4 направления × ходьба), ровер, иконки | `assets/CREDITS.md` с лицензиями |
| B2 | **Мир:** тайловая карта, камера сверху с плавным следованием, день/ночь (затемнение), пыльная буря, свечение щита | Видно базу, по ней можно ходить |
| B3 | **Игрок:** ходьба, коллизии с модулями, «подойди и нажми F / держи F», подсказка над головой | Ремонт держанием F вызывает `Command::Repair` |
| B4 | **Режим стройки:** выбор модуля в нижней панели, призрак на сетке (зелёный/красный), отказ с понятной причиной из `Refusal` | Можно построить базу мышью и геймпадом |
| B5 | **HUD и окна:** полоски ресурсов, сол-часы, баннер вспышки с меткой LIVE/АРХИВ, радио ЦУП с задержкой, пульт энергии, поддержка экипажа, маяк-дыхание, пауза | Все `GameEvent` что-то показывают |
| B6 | **Меню:** главное меню, выбор карты с замками, брифинг с планом «если — то», магазин скинов, нашивки, страница «Данные NASA и источники», разбор миссии с графиком | Можно пройти круг: меню → миссия → разбор → меню |
| B7 | **Кооператив** на одном экране: разделение экрана или общая камера, 2 набора клавиш / 2 геймпада | Двое играют одновременно |
| B8 | **Звук и музыка**, RU/EN переключатель, настройки | — |
| B9 | **Сборка релиза:** `cargo build --release`, иконка приложения, архивы для Windows/macOS/Linux, скриншоты и видео для судей | Архив запускается на чистом компьютере |

**Как работать с Claude:**
- Открой репозиторий в Claude Code и дай ему этот файл.
- Проси по одной задаче из таблицы, а в конце: «трогай только `crates/game` и `assets`, `core` не меняй».
- Новый экран проверяй запуском игры (`cargo run -p game`) и скриншотом.

---

## 6. Порядок работы и сроки

| Этап | Часть A (GPT) | Часть B (Claude) | Встреча |
|---|---|---|---|
| День 1 | A0, A1 | B0, начать B1 | Утверждаем контракт |
| День 2 | A2, A3 | B1, B2, B3 | Демо: ходим по базе на `mock` |
| День 3 | A4, A5 | B4, B5 | **Переключаем B с `mock` на настоящий `core`** |
| День 4 | A6, A7, A8 | B6, B7 | Полный круг игры |
| День 5 | Правки баланса по игре | B8, B9 | Плейтест на ребёнке 7–12, запись видео |

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
