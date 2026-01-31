// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Session/conversation state tracking.

mod jsonl;

pub use jsonl::*;

use super::io::{json_files_in, JsonLoad};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A tool call within a turn
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnToolCall {
    pub tool: String,
    pub input: serde_json::Value,
    pub output: Option<String>,
}

/// A conversation turn in a session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Turn {
    /// Turn sequence number
    pub seq: u32,

    /// User prompt
    pub prompt: String,

    /// Assistant response
    pub response: String,

    /// Timestamp (millis since epoch for serialization compatibility)
    pub timestamp_ms: u64,

    /// Tool calls made during this turn
    #[serde(default)]
    pub tool_calls: Vec<TurnToolCall>,
}

impl Turn {
    pub fn new(seq: u32, prompt: String, response: String) -> Self {
        Self {
            seq,
            prompt,
            response,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            tool_calls: Vec::new(),
        }
    }

    pub fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.timestamp_ms)
    }
}

/// Session state for a conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at_ms: u64,
    pub last_active_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub turns: Vec<Turn>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            id: id.into(),
            created_at_ms: now_ms,
            last_active_ms: now_ms,
            project_path: None,
            turns: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn new_at(id: impl Into<String>, timestamp_ms: u64) -> Self {
        Self {
            id: id.into(),
            created_at_ms: timestamp_ms,
            last_active_ms: timestamp_ms,
            project_path: None,
            turns: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_project(mut self, path: impl Into<String>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    pub fn add_turn(&mut self, prompt: String, response: String) -> &Turn {
        let turn = Turn::new(self.turns.len() as u32, prompt, response);
        self.turns.push(turn);
        self.last_active_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        &self.turns[self.turns.len() - 1]
    }

    pub fn add_turn_at(
        &mut self,
        prompt: String,
        response: String,
        timestamp_ms: u64,
    ) -> &mut Turn {
        let mut turn = Turn::new(self.turns.len() as u32, prompt, response);
        turn.timestamp_ms = timestamp_ms;
        self.turns.push(turn);
        self.last_active_ms = timestamp_ms;
        let len = self.turns.len();
        &mut self.turns[len - 1]
    }

    pub fn last_turn(&self) -> Option<&Turn> {
        self.turns.last()
    }

    pub fn last_turn_mut(&mut self) -> Option<&mut Turn> {
        self.turns.last_mut()
    }

    pub fn is_expired(&self, max_age: Duration) -> bool {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - self.last_active_ms;
        elapsed > max_age.as_millis() as u64
    }

    pub fn is_expired_at(&self, max_age: Duration, current_ms: u64) -> bool {
        let elapsed = current_ms.saturating_sub(self.last_active_ms);
        elapsed > max_age.as_millis() as u64
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

impl JsonLoad for Session {}

/// Session manager for multiple sessions
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    current: Option<String>,
    storage_dir: Option<PathBuf>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            current: None,
            storage_dir: None,
        }
    }

    pub fn with_storage(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = Some(dir.into());
        self
    }

    pub fn storage_dir(&self) -> Option<&PathBuf> {
        self.storage_dir.as_ref()
    }

    pub fn create_session(&mut self) -> &mut Session {
        let id = generate_session_id();
        let session = Session::new(&id);
        self.current = Some(id.clone());
        match self.sessions.entry(id) {
            Entry::Vacant(e) => e.insert(session),
            Entry::Occupied(e) => e.into_mut(),
        }
    }

    pub fn create_session_with_id(&mut self, id: impl Into<String>) -> &mut Session {
        let id = id.into();
        let session = Session::new(&id);
        self.current = Some(id.clone());
        match self.sessions.entry(id) {
            Entry::Vacant(e) => e.insert(session),
            Entry::Occupied(e) => e.into_mut(),
        }
    }

    pub fn current_id(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn current_session(&mut self) -> &mut Session {
        if self.current.is_none() {
            return self.create_session();
        }
        let id = self.current.clone().unwrap_or_default();
        match self.sessions.entry(id.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(Session::new(&id)),
        }
    }

    pub fn get_current(&self) -> Option<&Session> {
        self.current.as_ref().and_then(|id| self.sessions.get(id))
    }

    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    pub fn resume(&mut self, id: &str) -> Option<&mut Session> {
        if self.sessions.contains_key(id) {
            self.current = Some(id.to_string());
            self.sessions.get_mut(id)
        } else if let Some(session) = self.load_session(id) {
            self.sessions.insert(id.to_string(), session);
            self.current = Some(id.to_string());
            self.sessions.get_mut(id)
        } else {
            None
        }
    }

    pub fn continue_session(&mut self) -> Option<&mut Session> {
        let most_recent = self
            .sessions
            .values()
            .max_by_key(|s| s.last_active_ms)
            .map(|s| s.id.clone());

        if let Some(id) = most_recent {
            self.current = Some(id.clone());
            self.sessions.get_mut(&id)
        } else {
            self.load_most_recent()
        }
    }

    pub fn save_current(&self) -> std::io::Result<()> {
        let Some(ref dir) = self.storage_dir else {
            return Ok(());
        };
        let Some(ref id) = self.current else {
            return Ok(());
        };
        let Some(session) = self.sessions.get(id) else {
            return Ok(());
        };

        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", id));
        session.save(&path)
    }

    fn load_session(&self, id: &str) -> Option<Session> {
        let dir = self.storage_dir.as_ref()?;
        let path = dir.join(format!("{}.json", id));
        Session::load(&path).ok()
    }

    fn load_most_recent(&mut self) -> Option<&mut Session> {
        let dir = self.storage_dir.as_ref()?;
        if !dir.exists() {
            return None;
        }

        let most_recent = json_files_in(dir)
            .filter_map(|path| Session::load(&path).ok())
            .max_by_key(|s| s.last_active_ms);

        if let Some(session) = most_recent {
            let id = session.id.clone();
            self.sessions.insert(id.clone(), session);
            self.current = Some(id.clone());
            self.sessions.get_mut(&id)
        } else {
            None
        }
    }

    pub fn list_ids(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
        self.current = None;
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("session_{:x}", ts)
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
