//! Interface texts in three languages from `assets/content/ui.json` (B10).
//!
//! Every entry has `ru` and `en`; `kk` may be `null` until the team's native speakers
//! translate it — then the Russian text is shown and the key goes into `missing_kk()`.
//! Part A's content system (`content.rs`) can take this file over later: the entry shape is
//! the contract's `Text { ru, kk: Option, en }`.

use crate::i18n::Lang;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Deserialize, Clone, Debug)]
pub struct Text {
    pub ru: String,
    #[serde(default)]
    pub kk: Option<String>,
    pub en: String,
}

impl Text {
    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::Ru => &self.ru,
            Lang::En => &self.en,
            Lang::Kk => self.kk.as_deref().filter(|s| !s.is_empty()).unwrap_or(&self.ru),
        }
    }
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    #[allow(dead_code)]
    draft: bool,
    texts: BTreeMap<String, Text>,
}

pub struct UiText {
    texts: BTreeMap<String, Text>,
}

impl UiText {
    /// Parse and check: errors name the key that is broken.
    pub fn parse(json: &str) -> Result<UiText, Vec<String>> {
        let f: File = serde_json::from_str(json).map_err(|e| vec![format!("ui.json: {e}")])?;
        let errors: Vec<String> = f
            .texts
            .iter()
            .filter(|(_, t)| t.ru.trim().is_empty() || t.en.trim().is_empty())
            .map(|(k, _)| format!("ui.json: \"{k}\" needs both ru and en"))
            .collect();
        if errors.is_empty() {
            Ok(UiText { texts: f.texts })
        } else {
            Err(errors)
        }
    }

    /// The text for `key`; an unknown key shows itself, so a typo is visible on screen.
    pub fn get<'a>(&'a self, key: &'a str, lang: Lang) -> &'a str {
        self.texts.get(key).map(|t| t.get(lang)).unwrap_or(key)
    }

    pub fn has(&self, key: &str) -> bool {
        self.texts.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// Keys still waiting for a Kazakh translation (the content check report).
    pub fn missing_kk(&self) -> Vec<&str> {
        self.texts.iter().filter(|(_, t)| t.kk.as_deref().map_or(true, |s| s.trim().is_empty())).map(|(k, _)| k.as_str()).collect()
    }
}

const BUILTIN: &str = include_str!("../../../assets/content/ui.json");

/// The texts shipped with the game (checked by the tests below, so this never fails).
pub fn builtin() -> &'static UiText {
    static UI: OnceLock<UiText> = OnceLock::new();
    UI.get_or_init(|| UiText::parse(BUILTIN).unwrap_or_else(|e| panic!("{}", e.join("\n"))))
}

/// Shortcut: `ui_text::t("menu.start", lang)`.
pub fn t(key: &str, lang: Lang) -> &str {
    builtin().get(key, lang)
}

/// Text with `{name}` placeholders filled in.
pub fn tf(key: &str, lang: Lang, args: &[(&str, &str)]) -> String {
    let mut s = t(key, lang).to_string();
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_ui_texts_are_valid() {
        let ui = UiText::parse(BUILTIN).unwrap_or_else(|e| panic!("{}", e.join("\n")));
        assert!(ui.len() > 50);
        assert_eq!(ui.get("menu.start", Lang::Ru), "Начать");
        assert_eq!(ui.get("menu.start", Lang::En), "Start");
        let missing = ui.missing_kk();
        if !missing.is_empty() {
            eprintln!("ui.json: {} of {} texts wait for a Kazakh translation: {}", missing.len(), ui.len(), missing.join(", "));
        }
    }

    #[test]
    fn kazakh_falls_back_to_russian() {
        let ui = UiText::parse(r#"{"texts": {"a.b": {"ru": "Да", "kk": null, "en": "Yes"}, "c.d": {"ru": "Нет", "kk": "Жоқ", "en": "No"}}}"#).unwrap();
        assert_eq!(ui.get("a.b", Lang::Kk), "Да");
        assert_eq!(ui.get("c.d", Lang::Kk), "Жоқ");
        assert_eq!(ui.get("nope.key", Lang::En), "nope.key");
        assert_eq!(ui.missing_kk(), vec!["a.b"]);
        assert!(UiText::parse(r#"{"texts": {"x": {"ru": "", "en": "X"}}}"#).is_err());
    }

    /// Every `tr(l, "…")` / `trf(l, "…"` key used by the game screens exists in ui.json.
    #[test]
    fn every_key_the_game_uses_exists() {
        let ui = builtin();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/src");
        let mut files = vec![];
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map_or(false, |x| x == "rs") {
                    files.push(p);
                }
            }
        }
        let mut missing = vec![];
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap_or_default();
            for pat in ["tr(l, \"", "trf(l, \""] {
                let mut rest = src.as_str();
                while let Some(i) = rest.find(pat) {
                    rest = &rest[i + pat.len()..];
                    let key = &rest[..rest.find('"').unwrap_or(0)];
                    if !ui.has(key) {
                        missing.push(format!("{}: {key}", f.file_name().unwrap().to_string_lossy()));
                    }
                }
            }
        }
        assert!(missing.is_empty(), "keys missing from ui.json:\n{}", missing.join("\n"));
    }
}
