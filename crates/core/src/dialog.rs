//! A6 — NPC dialogs with branches and conditions. Content lives in `assets/content/dialogs/`.

use crate::content::{Content, Dialog, Trigger};
use serde::{Deserialize, Serialize};

/// The dialog on screen: which one and which line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DialogState {
    pub id: String,
    pub node: String,
}

/// Dialogs that should start now: matching trigger, not seen yet, flag condition met.
pub fn triggered<'a>(c: &'a Content, trigger: &Trigger, seen: &[String], flags: &[String]) -> Option<&'a Dialog> {
    c.dialogs.iter().find(|d| &d.trigger == trigger && !seen.contains(&d.id) && d.require_flag.as_ref().is_none_or(|f| flags.contains(f)))
}

pub fn start(d: &Dialog) -> DialogState {
    DialogState { id: d.id.clone(), node: d.nodes[0].id.clone() }
}

/// Go on: `choice` picks a branch (when the line has choices). Returns the new state (`None` = ended)
/// and a flag to set, if the choice sets one.
pub fn advance(c: &Content, s: &DialogState, choice: Option<usize>) -> (Option<DialogState>, Option<String>) {
    let Some(node) = c.dialog(&s.id).and_then(|d| d.nodes.iter().find(|n| n.id == s.node)) else { return (None, None) };
    let (next, flag) = if node.choices.is_empty() {
        (node.next.clone(), None)
    } else {
        match choice.and_then(|i| node.choices.get(i)) {
            Some(ch) => (ch.next.clone(), ch.set_flag.clone()),
            // a line with choices waits for one
            None => return (Some(s.clone()), None),
        }
    };
    (next.map(|n| DialogState { id: s.id.clone(), node: n }), flag)
}
