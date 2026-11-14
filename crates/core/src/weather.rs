//! Space weather from NASA DONKI. PART A (A1) ports the live fetch from the 3D version
//! (`src/nasa.rs`). Until then `fetch` fails and the honest archive is used.

use crate::catalog::DONKI_ARCHIVE;

#[derive(Clone, Debug)]
pub struct WeatherEvent {
    pub kind: String,
    pub date: String,
    pub label_ru: String,
    pub label_en: String,
    pub power: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Weather {
    /// true only for data fetched from the live API
    pub live: bool,
    pub events: Vec<WeatherEvent>,
    /// FLR, GST, SEP, CME counts over the fetched window
    pub counts: (usize, usize, usize, usize),
    pub kp_max: f32,
    pub range: String,
}

impl Weather {
    pub fn archive() -> Self {
        Self {
            live: false,
            events: DONKI_ARCHIVE
                .iter()
                .map(|d| WeatherEvent { kind: d.kind.into(), date: d.date.into(), label_ru: d.label_ru.into(), label_en: d.label_en.into(), power: d.power })
                .collect(),
            ..Default::default()
        }
    }
}

/// Blocking fetch of the last 30 days of DONKI. The app calls it on a background thread.
pub fn fetch(_api_key: &str) -> Result<Weather, String> {
    Err("live DONKI fetch not implemented yet (part A1)".into())
}
