use std::collections::HashMap;

use serde::Deserialize;

use crate::ul::event::CommandKind;

#[derive(Deserialize)]
pub struct TransitionTable {
    transitions: HashMap<String, HashMap<String, TransitionEntry>>,
}

#[derive(Deserialize)]
pub struct TransitionEntry {
    pub to: String,
    pub commands: Vec<CommandKind>,
}

impl TransitionTable {
    pub fn new() -> Self {
        toml::from_str(include_str!("transitions.toml")).expect("invalid transitions.toml")
    }

    /// Looks up the transition for a given state and event.
    /// Falls back to the `"*"` wildcard state if no specific entry exists.
    pub fn lookup(&self, state: &str, event: &str) -> Option<&TransitionEntry> {
        self.transitions.get(state).and_then(|m| m.get(event))
    }
}
