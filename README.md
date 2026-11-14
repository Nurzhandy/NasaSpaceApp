# 🚀 Junior Astronaut Mission Trainer / Тренажёр юного астронавта

**NASA Space Apps Challenge 2026 — “Build a Junior Astronaut Mission Trainer”**

A browser game for kids aged 7–12. You run an outpost on the Moon, on Mars, in lunar orbit or on Europa, and split a small power budget between **oxygen, food, radiation shielding and science**. No choice is free. Radiation events are **real solar flares and storms from NASA DONKI** (live when the API is reachable, a labeled archive when it isn’t).

Браузерная игра для детей 7–12 лет: управляй базой на Луне и Марсе и дели энергию между кислородом, едой, щитом и наукой. Вспышки в игре — настоящие события из базы NASA DONKI.

## ▶️ How to run / Как запустить

No build, no account, no install. Open `index.html` in a browser, or serve the folder:

```bash
python3 -m http.server 8765
```

Then open http://localhost:8765. Online multiplayer needs internet (WebRTC via PeerJS). Everything else works offline. Without internet the game uses the embedded DONKI archive.

**NASA API key:** the game uses `DEMO_KEY`, which allows only ~30 requests/hour per IP. For demos and judging, get a free key at https://api.nasa.gov (1,000 requests/hour). Paste it on the **“For adults & judges”** page in the game, or change `DEFAULT_KEY` in `js/nasa.js`. Responses are cached for 6 hours.

## 🎮 What’s inside / Что есть в игре

| Mode | What the player does | What it teaches |
|---|---|---|
| **Outposts** (5 maps) | Splits power blocks every sol and handles random events | Engineering trade-offs, radiation, power and life support |
| **Training center** (8 drills) | Pre-launch checklist, rover re-route, robot debug, crew comms, mood remote, crew day, true/myth quiz, space beacon | Attention, flexibility, debugging, communication, feelings, routine |
| **Multiplayer: Mission Control & Crew** | Mission Control sees the forecast and DONKI data, the crew holds the controls. They talk with preset phrases, delayed like real comms | Communication delay, “say–repeat–confirm”, teamwork |

**Progression:** missions and drills earn points. **🏅 Total points** unlock maps (Moon 60 → Mars 180 → Lunar orbit 400 → Europa 700) and never go down. **⭐ Stars** are a separate balance spent on skins (suits, visors, accessories, rovers). Buying a skin never relocks a map. There are also 14 mission patches (badges) and a private mission log.

**Ages:** “Cadet 7–9” tracks 3 resources, gets accurate forecasts and has read-aloud on. “Pilot 10–12” tracks 4 resources, gets uncertain forecasts (false alarms and surprise flares) and extra reflection questions.

## 🛰️ NASA data used and why / Какие данные NASA и зачем

| Dataset | Where in the game | Why |
|---|---|---|
| **DONKI** (FLR, GST, SEP, CME), live via api.nasa.gov | Home “Space weather” panel, solar flare events, Mission Control radar, “real space weather” mission option | Links the shielding decision to actual current solar activity |
| **Curiosity RAD**: Mars surface 0.64 mSv/day, cruise 1.84 mSv/day (Hassler et al., 2014) | Dose rate on the Mars and Lunar orbit maps | Real radiation ratios between places |
| **Chang’e‑4 LND**: lunar surface 1.369 mSv/day (Zhang et al., 2020) | Dose rate on the Moon map | Shows why the Moon needs more shielding than Mars |
| **NASA OSDR / Veggie / APEX** | Greenhouse harvest event and facts | Real plant growth in space (ISS lettuce, 2015) |
| **MOXIE** (Perseverance) | Life-support facts | Oxygen made from Mars air (122 g) |
| **NASA Space Communications** | 1.3 s Moon delay (real) and 4–24 min Mars delay (sped up and labeled) | The communication-delay mechanic |
| **NASA-STD-3001** (600 mSv career limit) | Dose limit explanation | Why dose has a limit |
| **Mars 2018 dust storm / Opportunity**, lunar day 29.5 days | Dust storm and lunar night events | Power drops that really happened |

**Honesty rules in the code:**
- Game numbers are scaled so kids can do the maths in their heads (dose = real mSv/day × 7.3). Ratios and cause and effect follow the real data.
- Static data is **never** presented as live. The UI always shows *LIVE*, *saved data* (with its fetch date) or *ARCHIVE*.

## 🧠 Psychology (from our research doc) / Психологическая основа

The research behind this is in `Trenazher_yunyh_astronavtov.docx`:
- Pause before acting (“Stop, look, choose”), if–then plans before each mission, a thought check after failure, and debriefs that ask “what helped?” and “where else is this useful?”.
- The **Space Beacon** breathing pause is voluntary and never scored (Obradović et al., 2021).
- Feedback describes actions (“you raised the shield”), not personality.
- **No crew deaths.** On failure the crew evacuates safely and the game explains the cause.
- Hints are free and never reduce rewards. There is no public ranking.
- Multiplayer chat uses preset phrases only, so it is child-safe. No personal data is collected; progress stays in `localStorage`.

## 🏆 How we address the judging criteria

- **Impact:** for kids 7–12, playable in any browser with no account. It works in classrooms too: one shared tablet for two players.
- **Creativity:** an asymmetric co-op mode where Mission Control and the crew are split by a real communication delay. Real solar flares drive the in-game events.
- **Validity:** dose ratios come from measured data, DONKI events are real, the simulation was balance-tested with Monte Carlo runs, and every source is cited in the game.
- **Relevance:** covers every challenge objective: life support, radiation shielding, power budgeting, food production, communication delay and real NASA open data.
- **Presentation:** bilingual (RU/EN), read-aloud, keyboard support, reduced-motion support.

## 📁 Files

- `index.html` — page shell
- `css/style.css` — styles
- `js/data.js` — maps, DONKI archive, skins, badges, quiz, sources
- `js/core.js` — profile/saving, language, sound, read-aloud, avatar
- `js/nasa.js` — live DONKI fetch, cache and archive fallback
- `js/sim.js` — outpost simulation (pure, shared by single and multiplayer)
- `js/ui.js` — screens, outpost game, debrief, beacon, shop, adults page
- `js/missions.js` — training drills
- `js/multi.js` — multiplayer (PeerJS online and one-device modes)
- `js/main.js` — boot and starfield

*Educational game. It does not assess readiness for real spaceflight and is not psychological treatment.*
