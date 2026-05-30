use std::collections::HashMap;

use ron::de::SpannedError;
use serde::Deserialize;

use crate::ul::connection::UpperLayerConnectionState;
use crate::ul::event::{CommandKind, EventKind};

#[derive(Deserialize)]
pub struct TransitionTable {
    transitions: HashMap<UpperLayerConnectionState, HashMap<EventKind, TransitionEntry>>,
}

#[derive(Deserialize)]
pub struct TransitionEntry {
    pub to: UpperLayerConnectionState,
    pub commands: Vec<CommandKind>,
}

impl TransitionTable {
    pub fn new() -> Result<Self, SpannedError> {
        Ok(ron::from_str(include_str!("transitions.ron"))?)
    }

    /// Looks up the transition for a given state and event.
    /// Falls back to the `"*"` wildcard state if no specific entry exists.
    pub fn lookup(&self, state: UpperLayerConnectionState, event: EventKind) -> Option<&TransitionEntry> {
        self.transitions.get(&state).and_then(|m| m.get(&event))
    }
}
