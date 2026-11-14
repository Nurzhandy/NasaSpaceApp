//! A5 — one physics card after each episode, in the order of `physics_cards.json`.
//! A card tied to this episode (`after`) comes first; otherwise the next free card.

use crate::content::Content;

pub fn next_card<'a>(c: &'a Content, seen: &[String], episode: &str) -> Option<&'a str> {
    let unseen = |id: &String| !seen.contains(id);
    c.cards
        .iter()
        .find(|k| k.after.as_deref() == Some(episode) && unseen(&k.id))
        .or_else(|| c.cards.iter().find(|k| k.after.is_none() && unseen(&k.id)))
        .map(|k| k.id.as_str())
}
