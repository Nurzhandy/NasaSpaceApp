//! NASA DONKI space weather: live download (feature `live`, native only) with an honest fallback
//! to the embedded archive. Archived data is never presented as live.

use crate::data::{Donki, DONKI_ARCHIVE};
use crate::rng::Rng;

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherEvent {
    /// FLR, GST, SEP or CME
    pub kind: String,
    pub date: String,
    pub label_ru: String,
    pub label_en: String,
    /// 0..1 strength used for the in-game flare
    pub power: f32,
}

impl From<&Donki> for WeatherEvent {
    fn from(d: &Donki) -> Self {
        Self { kind: d.kind.into(), date: d.date.into(), label_ru: d.label_ru.into(), label_en: d.label_en.into(), power: d.power }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Weather {
    pub live: bool,
    pub events: Vec<WeatherEvent>,
    /// (flares, storms, particle events, CMEs) in the last 30 days
    pub counts: (usize, usize, usize, usize),
    pub kp_max: f32,
    /// "YYYY-MM-DD — YYYY-MM-DD" of the live query
    pub range: String,
}

impl Weather {
    pub fn archive() -> Self {
        Self { live: false, events: DONKI_ARCHIVE.iter().map(WeatherEvent::from).collect(), ..Default::default() }
    }

    /// A real event to drive an in-game flare. Prefers recent strong live events.
    pub fn pick_flare(&self, rng: &mut Rng) -> (WeatherEvent, bool) {
        if self.live {
            let strong: Vec<&WeatherEvent> = self.events.iter().filter(|e| e.power >= 0.3).take(6).collect();
            if !strong.is_empty() && rng.chance(0.7) {
                return (strong[rng.below(strong.len())].clone(), true);
            }
        }
        (WeatherEvent::from(&DONKI_ARCHIVE[rng.below(DONKI_ARCHIVE.len())]), false)
    }

    /// 0 unknown (archive), 1 calm, 2 medium, 3 high
    pub fn risk(&self) -> u8 {
        if !self.live {
            return 0;
        }
        let strong = self.events.iter().filter(|e| e.power >= 0.6).count();
        let score = strong * 2 + self.events.len().min(6) / 2 + if self.kp_max >= 6.0 { 2 } else { 0 };
        if score >= 6 {
            3
        } else if score >= 2 {
            2
        } else {
            1
        }
    }
}

/// API key: `NASA_API_KEY` env var, then the saved profile key, then the public `DEMO_KEY` (~30 req/hour).
pub fn api_key(profile_key: &str) -> String {
    let env = std::env::var("NASA_API_KEY").unwrap_or_default();
    if !env.trim().is_empty() {
        env.trim().to_string()
    } else if !profile_key.trim().is_empty() {
        profile_key.trim().to_string()
    } else {
        "DEMO_KEY".to_string()
    }
}

/// Starts the download in a background thread. The receiver gets exactly one `Weather`
/// (live, or the archive if anything fails). Poll it with `try_recv` once per frame.
pub fn spawn_fetch(key: String) -> std::sync::mpsc::Receiver<Weather> {
    let (tx, rx) = std::sync::mpsc::channel();
    #[cfg(all(feature = "live", not(target_arch = "wasm32")))]
    std::thread::spawn(move || {
        let _ = tx.send(fetch(&key).unwrap_or_else(|_| Weather::archive()));
    });
    #[cfg(not(all(feature = "live", not(target_arch = "wasm32"))))]
    {
        let _ = key;
        let _ = tx.send(Weather::archive());
    }
    rx
}

/// Parse DONKI JSON lists into a `Weather` (separate from the download so it can be tested offline).
pub fn parse(flr: &[serde_json::Value], gst: &[serde_json::Value], sep: &[serde_json::Value], cme: &[serde_json::Value], range: String) -> Weather {
    let mut events = vec![];
    let mut kp_max = 0.0f32;
    for f in flr {
        let cls = f["classType"].as_str().unwrap_or("C1").to_string();
        let n: f32 = cls.get(1..).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let power = match cls.chars().next().unwrap_or('C') {
            'X' => (0.6 + n / 25.0).min(1.0),
            'M' => 0.3 + n / 40.0,
            _ => 0.15,
        };
        let date = f["peakTime"].as_str().or(f["beginTime"].as_str()).unwrap_or("").chars().take(10).collect();
        events.push(WeatherEvent { kind: "FLR".into(), date, label_ru: format!("Солнечная вспышка класса {cls}"), label_en: format!("Class {cls} solar flare"), power });
    }
    for g in gst {
        let kp = g["allKpIndex"].as_array().map(|a| a.iter().filter_map(|k| k["kpIndex"].as_f64()).fold(0.0, f64::max)).unwrap_or(0.0) as f32;
        kp_max = kp_max.max(kp);
        let date = g["startTime"].as_str().unwrap_or("").chars().take(10).collect();
        events.push(WeatherEvent { kind: "GST".into(), date, label_ru: format!("Магнитная буря, Kp {kp}"), label_en: format!("Geomagnetic storm, Kp {kp}"), power: ((kp - 4.0) / 5.0).clamp(0.2, 1.0) });
    }
    for s in sep {
        let date = s["eventTime"].as_str().unwrap_or("").chars().take(10).collect();
        events.push(WeatherEvent { kind: "SEP".into(), date, label_ru: "Поток солнечных частиц (SEP)".into(), label_en: "Solar particle event (SEP)".into(), power: 0.8 });
    }
    for c in cme {
        let speed = c["cmeAnalyses"].as_array().map(|a| a.iter().filter_map(|x| x["speed"].as_f64()).fold(0.0, f64::max)).unwrap_or(0.0) as f32;
        if speed > 700.0 {
            let date = c["startTime"].as_str().unwrap_or("").chars().take(10).collect();
            events.push(WeatherEvent { kind: "CME".into(), date, label_ru: format!("Выброс плазмы, {} км/с", speed as i32), label_en: format!("Coronal mass ejection, {} km/s", speed as i32), power: (speed / 2500.0).clamp(0.3, 0.9) });
        }
    }
    events.sort_by(|a, b| b.date.cmp(&a.date));
    Weather { live: true, counts: (flr.len(), gst.len(), sep.len(), cme.len()), events, kp_max, range }
}

#[cfg(all(feature = "live", not(target_arch = "wasm32")))]
pub fn fetch(key: &str) -> Result<Weather, String> {
    use serde_json::Value;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs() as i64;
    let (start, end) = (ymd(now - 30 * 86400), ymd(now));
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(9)).build();
    let get = |t: &str| -> Result<Vec<Value>, String> {
        let url = format!("https://api.nasa.gov/DONKI/{t}?startDate={start}&endDate={end}&api_key={key}");
        match agent.get(&url).call() {
            Ok(r) => {
                let txt = r.into_string().map_err(|e| e.to_string())?;
                Ok(if txt.trim().is_empty() { vec![] } else { serde_json::from_str::<Vec<Value>>(&txt).unwrap_or_default() })
            }
            Err(ureq::Error::Status(404, _)) => Ok(vec![]),
            Err(e) => Err(e.to_string()),
        }
    };
    let flr = get("FLR")?;
    let gst = get("GST")?;
    let sep = get("SEP").unwrap_or_default();
    let cme = get("CME").unwrap_or_default();
    Ok(parse(&flr, &gst, &sep, &cme, format!("{start} — {end}")))
}

/// Unix seconds → "YYYY-MM-DD" (civil-from-days, Howard Hinnant).
pub fn ymd(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(1_727_913_600), "2024-10-03");
    }

    #[test]
    fn parses_donki_and_never_marks_archive_live() {
        let flr: Vec<serde_json::Value> = serde_json::from_str(r#"[{"classType":"X9.0","peakTime":"2024-10-03T12:18Z"},{"classType":"M2.1","beginTime":"2024-10-01T00:00Z"}]"#).unwrap();
        let gst: Vec<serde_json::Value> = serde_json::from_str(r#"[{"startTime":"2024-10-10T15:00Z","allKpIndex":[{"kpIndex":8.67}]}]"#).unwrap();
        let w = parse(&flr, &gst, &[], &[], "x".into());
        assert!(w.live);
        assert_eq!(w.counts, (2, 1, 0, 0));
        assert_eq!(w.events[0].date, "2024-10-10");
        assert!(w.events.iter().any(|e| e.kind == "FLR" && e.power >= 0.9));
        assert!(!Weather::archive().live);
        let mut rng = Rng::new(7);
        for _ in 0..20 {
            assert!(!Weather::archive().pick_flare(&mut rng).1, "archive flares are never live");
        }
    }
}
