//! Tiny bilingual string table. The game is icon-first; text stays short.

use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
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

/// Helper macro: `tr!(lang, "рус", "eng")`
#[macro_export]
macro_rules! tr {
    ($l:expr, $ru:expr, $en:expr) => {
        $l.t($ru, $en)
    };
}
