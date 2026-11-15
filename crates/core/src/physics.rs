//! A5 — a physics card after an episode — only the card tied to that episode (`after`), so the
//! card always matches what the crew just did. Cards without a task are read in the encyclopedia.

use crate::content::Content;

pub fn next_card<'a>(c: &'a Content, seen: &[String], episode: &str) -> Option<&'a str> {
    let unseen = |id: &String| !seen.contains(id);
    c.cards
        .iter()
        .find(|k| k.after.as_deref() == Some(episode) && unseen(&k.id))
        .map(|k| k.id.as_str())
}
