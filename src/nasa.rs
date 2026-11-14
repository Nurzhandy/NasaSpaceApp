//! Live NASA DONKI space weather (native: background thread + ureq). Falls back to the
//! embedded archive and never pretends archived data is live.

use crate::data::{Donki, DONKI_ARCHIVE};
use bevy::prelude::*;
use std::sync::{mpsc, Mutex};

#[derive(Clone, Debug)]
pub struct WeatherEvent {
    pub kind: String,
    pub date: String,
    pub label_ru: String,
    pub label_en: String,
    pub power: f32,
}

impl From<&Donki> for WeatherEvent {
    fn from(d: &Donki) -> Self {
        Self { kind: d.kind.into(), date: d.date.into(), label_ru: d.label_ru.into(), label_en: d.label_en.into(), power: d.power }
    }
}

#[derive(Resource, Default)]
pub struct Weather {
    pub live: bool,
    pub loading: bool,
    pub events: Vec<WeatherEvent>,
    pub counts: (usize, usize, usize, usize),
    pub kp_max: f32,
    pub range: String,
}

impl Weather {
    pub fn archive() -> Self {
        Self { live: false, loading: false, events: DONKI_ARCHIVE.iter().map(WeatherEvent::from).collect(), ..default() }
    }
    /// A real event to drive an in-game flare. Prefers recent live events.
    pub fn pick_flare(&self, rng: &mut crate::sim::Rng) -> (WeatherEvent, bool) {
        if self.live {
            let strong: Vec<&WeatherEvent> = self.events.iter().filter(|e| e.power >= 0.3).take(6).collect();
            if !strong.is_empty() && rng.chance(0.7) {
                return (strong[rng.below(strong.len())].clone(), true);
            }
        }
        (WeatherEvent::from(&DONKI_ARCHIVE[rng.below(DONKI_ARCHIVE.len())]), false)
    }
    /// 1 calm, 2 medium, 3 high
    pub fn risk(&self) -> u8 {
        if !self.live {
            return 0;
        }
        let strong = self.events.iter().filter(|e| e.power >= 0.6).count();
        let score = strong * 2 + self.events.len().min(6) / 2 + if self.kp_max >= 6.0 { 2 } else { 0 };
        if score >= 6 { 3 } else if score >= 2 { 2 } else { 1 }
    }
}

/// API key: `NASA_API_KEY` env var, then the profile, then the public `DEMO_KEY` (~30 req/hour).
pub fn api_key(profile: &crate::save::Profile) -> String {
    let env = std::env::var("NASA_API_KEY").unwrap_or_default();
    if !env.trim().is_empty() {
        env.trim().to_string()
    } else if !profile.nasa_key.trim().is_empty() {
        profile.nasa_key.trim().to_string()
    } else {
        "DEMO_KEY".to_string()
    }
}

#[derive(Resource)]
pub struct WeatherRx(pub Mutex<mpsc::Receiver<Weather>>);

pub struct NasaPlugin;
impl Plugin for NasaPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Weather { loading: true, ..Weather::archive() });
        app.add_systems(Startup, start_fetch);
        app.add_systems(Update, poll);
    }
}

fn start_fetch(mut commands: Commands, profile: Res<crate::save::Profile>) {
    let (tx, rx) = mpsc::channel();
    commands.insert_resource(WeatherRx(Mutex::new(rx)));
    let key = api_key(&profile);
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let w = fetch(&key).unwrap_or_else(|_| Weather::archive());
            let _ = tx.send(w);
        });
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = key;
        let _ = tx.send(Weather::archive());
    }
}

fn poll(rx: Option<Res<WeatherRx>>, mut weather: ResMut<Weather>) {
    let Some(rx) = rx else { return };
    let got = match rx.0.lock() {
        Ok(r) => r.try_recv().ok(),
        Err(_) => None,
    };
    if let Some(w) = got {
        *weather = w;
        weather.loading = false;
        info!("DONKI: live={} events={}", weather.live, weather.events.len());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch(key: &str) -> Result<Weather, Box<dyn std::error::Error>> {
    use serde_json::Value;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
    let end = ymd(now);
    let start = ymd(now - 30 * 86400);
    let get = |t: &str| -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let url = format!("https://api.nasa.gov/DONKI/{t}?startDate={start}&endDate={end}&api_key={key}");
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(9)).build();
        match agent.get(&url).call() {
            Ok(r) => {
                let txt = r.into_string()?;
                if txt.trim().is_empty() {
                    return Ok(vec![]);
                }
                Ok(serde_json::from_str::<Vec<Value>>(&txt).unwrap_or_default())
            }
            Err(ureq::Error::Status(404, _)) => Ok(vec![]),
            Err(e) => Err(Box::new(e)),
        }
    };
    let flr = get("FLR")?;
    let gst = get("GST")?;
    let sep = get("SEP").unwrap_or_default();
    let cme = get("CME").unwrap_or_default();

    let mut events = vec![];
    let mut kp_max = 0.0f32;
    for f in &flr {
        let cls = f["classType"].as_str().unwrap_or("C1").to_string();
        let n: f32 = cls[1..].parse().unwrap_or(1.0);
        let power = match cls.chars().next().unwrap_or('C') {
            'X' => (0.6 + n / 25.0).min(1.0),
            'M' => 0.3 + n / 40.0,
            _ => 0.15,
        };
        let date = f["peakTime"].as_str().or(f["beginTime"].as_str()).unwrap_or("").chars().take(10).collect();
        events.push(WeatherEvent { kind: "FLR".into(), date, label_ru: format!("Солнечная вспышка класса {cls}"), label_en: format!("Class {cls} solar flare"), power });
    }
    for g in &gst {
        let kp = g["allKpIndex"].as_array().map(|a| a.iter().filter_map(|k| k["kpIndex"].as_f64()).fold(0.0, f64::max)).unwrap_or(0.0) as f32;
        kp_max = kp_max.max(kp);
        let date = g["startTime"].as_str().unwrap_or("").chars().take(10).collect();
        events.push(WeatherEvent { kind: "GST".into(), date, label_ru: format!("Магнитная буря, Kp {kp}"), label_en: format!("Geomagnetic storm, Kp {kp}"), power: ((kp - 4.0) / 5.0).clamp(0.2, 1.0) });
    }
    for s in &sep {
        let date = s["eventTime"].as_str().unwrap_or("").chars().take(10).collect();
        events.push(WeatherEvent { kind: "SEP".into(), date, label_ru: "Поток солнечных частиц (SEP)".into(), label_en: "Solar particle event (SEP)".into(), power: 0.8 });
    }
    for c in &cme {
        let speed = c["cmeAnalyses"].as_array().map(|a| a.iter().filter_map(|x| x["speed"].as_f64()).fold(0.0, f64::max)).unwrap_or(0.0) as f32;
        if speed > 700.0 {
            let date = c["startTime"].as_str().unwrap_or("").chars().take(10).collect();
            events.push(WeatherEvent { kind: "CME".into(), date, label_ru: format!("Выброс плазмы, {} км/с", speed as i32), label_en: format!("Coronal mass ejection, {} km/s", speed as i32), power: (speed / 2500.0).clamp(0.3, 0.9) });
        }
    }
    events.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(Weather { live: true, loading: false, counts: (flr.len(), gst.len(), sep.len(), cme.len()), events, kp_max, range: format!("{start} — {end}") })
}

#[cfg(not(target_arch = "wasm32"))]
fn ymd(secs: i64) -> String {
    // civil-from-days (Howard Hinnant)
    let days = secs.div_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}
