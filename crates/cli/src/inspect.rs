// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! State inspection API for test assertions.
//!
//! This module provides `StateInspector` which allows tests to query and assert
//! on simulator state, including todos, sessions, hooks, and directory structure.

use crate::hooks::protocol::{HookEvent, HookMessage};
use crate::state::directory::StateDirectory;
use crate::state::session::SessionManager;
use crate::state::todos::{TodoState, TodoStatus};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

/// State inspector for test assertions
pub struct StateInspector {
    /// State directory
    state_dir: Arc<Mutex<StateDirectory>>,

    /// Session manager
    sessions: Arc<Mutex<SessionManager>>,

    /// Todo state
    todos: Arc<Mutex<TodoState>>,

    /// Recorded hook invocations
    hook_history: Arc<Mutex<Vec<HookMessage>>>,
}

impl StateInspector {
    /// Create a new state inspector
    pub fn new(
        state_dir: Arc<Mutex<StateDirectory>>,
        sessions: Arc<Mutex<SessionManager>>,
        todos: Arc<Mutex<TodoState>>,
    ) -> Self {
        Self {
            state_dir,
            sessions,
            todos,
            hook_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a state inspector with new instances of each component
    pub fn with_temp_dir() -> std::io::Result<Self> {
        let mut state_dir = StateDirectory::temp()?;
        state_dir.initialize().map_err(std::io::Error::other)?;

        let sessions = SessionManager::new().with_storage(state_dir.sessions_dir());

        Ok(Self {
            state_dir: Arc::new(Mutex::new(state_dir)),
            sessions: Arc::new(Mutex::new(sessions)),
            todos: Arc::new(Mutex::new(TodoState::new())),
            hook_history: Arc::new(Mutex::new(Vec::new())),
        })
    }

    // ---- Todo Queries ----

    /// Get todo count
    pub fn todo_count(&self) -> usize {
        self.todos.lock().len()
    }

    /// Get pending todo count
    pub fn pending_count(&self) -> usize {
        self.todos.lock().pending().count()
    }

    /// Get in-progress todo count
    pub fn in_progress_count(&self) -> usize {
        self.todos.lock().in_progress().count()
    }

    /// Get completed todo count
    pub fn completed_count(&self) -> usize {
        self.todos.lock().completed().count()
    }

    /// Check if a todo exists with given content substring
    pub fn todo_exists(&self, content: &str) -> bool {
        self.todos
            .lock()
            .items
            .iter()
            .any(|t| t.content.contains(content))
    }

    /// Get todo status by content substring
    pub fn todo_status(&self, content: &str) -> Option<TodoStatus> {
        self.todos
            .lock()
            .items
            .iter()
            .find(|t| t.content.contains(content))
            .map(|t| t.status.clone())
    }

    // ---- Todo Assertions ----

    /// Assert todo count
    pub fn assert_todo_count(&self, expected: usize) {
        let actual = self.todo_count();
        assert_eq!(
            actual, expected,
            "Expected {} todos, got {}",
            expected, actual
        );
    }

    /// Assert pending todo count
    pub fn assert_pending_count(&self, expected: usize) {
        let actual = self.pending_count();
        assert_eq!(
            actual, expected,
            "Expected {} pending todos, got {}",
            expected, actual
        );
    }

    /// Assert in-progress todo count
    pub fn assert_in_progress_count(&self, expected: usize) {
        let actual = self.in_progress_count();
        assert_eq!(
            actual, expected,
            "Expected {} in-progress todos, got {}",
            expected, actual
        );
    }

    /// Assert completed todo count
    pub fn assert_completed_count(&self, expected: usize) {
        let actual = self.completed_count();
        assert_eq!(
            actual, expected,
            "Expected {} completed todos, got {}",
            expected, actual
        );
    }

    /// Assert a todo exists with given content
    pub fn assert_todo_exists(&self, content: &str) {
        assert!(
            self.todo_exists(content),
            "Expected todo containing '{}' but none found",
            content
        );
    }

    /// Assert a todo does not exist with given content
    pub fn assert_todo_not_exists(&self, content: &str) {
        assert!(
            !self.todo_exists(content),
            "Expected no todo containing '{}' but one was found",
            content
        );
    }

    /// Assert a todo has given status
    pub fn assert_todo_status(&self, content: &str, expected_status: TodoStatus) {
        let status = self.todo_status(content);
        assert!(status.is_some(), "Todo containing '{}' not found", content);
        let status = status.unwrap_or(TodoStatus::Pending); // Won't reach due to assert above
        assert_eq!(
            status, expected_status,
            "Todo '{}' has status {:?}, expected {:?}",
            content, status, expected_status
        );
    }

    // ---- Session Queries ----

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.lock().len()
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<String> {
        self.sessions.lock().current_id().map(String::from)
    }

    /// Get turn count for current session
    pub fn turn_count(&self) -> usize {
        self.sessions
            .lock()
            .get_current()
            .map(|s| s.turn_count())
            .unwrap_or(0)
    }

    /// Get last prompt in current session
    pub fn last_prompt(&self) -> Option<String> {
        self.sessions
            .lock()
            .get_current()
            .and_then(|s| s.last_turn())
            .map(|t| t.prompt.clone())
    }

    /// Get last response in current session
    pub fn last_response(&self) -> Option<String> {
        self.sessions
            .lock()
            .get_current()
            .and_then(|s| s.last_turn())
            .map(|t| t.response.clone())
    }

    // ---- Session Assertions ----

    /// Assert session count
    pub fn assert_session_count(&self, expected: usize) {
        let actual = self.session_count();
        assert_eq!(
            actual, expected,
            "Expected {} sessions, got {}",
            expected, actual
        );
    }

    /// Assert current session has N turns
    pub fn assert_turn_count(&self, expected: usize) {
        let actual = self.turn_count();
        assert_eq!(
            actual, expected,
            "Expected {} turns, got {}",
            expected, actual
        );
    }

    /// Assert last prompt in current session contains text
    pub fn assert_last_prompt_contains(&self, expected: &str) {
        let prompt = self.last_prompt();
        assert!(prompt.is_some(), "No turns in current session");
        let prompt = prompt.unwrap_or_default();
        assert!(
            prompt.contains(expected),
            "Expected last prompt to contain '{}', got '{}'",
            expected,
            prompt
        );
    }

    /// Assert last response in current session contains text
    pub fn assert_last_response_contains(&self, expected: &str) {
        let response = self.last_response();
        assert!(response.is_some(), "No turns in current session");
        let response = response.unwrap_or_default();
        assert!(
            response.contains(expected),
            "Expected last response to contain '{}', got '{}'",
            expected,
            response
        );
    }

    // ---- Hook Queries ----

    /// Record a hook invocation
    pub fn record_hook(&self, message: HookMessage) {
        self.hook_history.lock().push(message);
    }

    /// Get hook invocation count
    pub fn hook_count(&self) -> usize {
        self.hook_history.lock().len()
    }

    /// Get hook invocation count for a specific event
    pub fn hook_count_for(&self, event: &HookEvent) -> usize {
        self.hook_history
            .lock()
            .iter()
            .filter(|m| &m.event == event)
            .count()
    }

    /// Check if a hook was invoked
    pub fn hook_invoked(&self, event: &HookEvent) -> bool {
        self.hook_history.lock().iter().any(|m| &m.event == event)
    }

    /// Get hook invocations for an event
    pub fn hook_invocations(&self, event: &HookEvent) -> Vec<HookMessage> {
        self.hook_history
            .lock()
            .iter()
            .filter(|m| &m.event == event)
            .cloned()
            .collect()
    }

    // ---- Hook Assertions ----

    /// Assert hook was invoked
    pub fn assert_hook_invoked(&self, event: &HookEvent) {
        assert!(
            self.hook_invoked(event),
            "Expected hook event {:?} to be invoked",
            event
        );
    }

    /// Assert hook was not invoked
    pub fn assert_hook_not_invoked(&self, event: &HookEvent) {
        assert!(
            !self.hook_invoked(event),
            "Expected hook event {:?} not to be invoked",
            event
        );
    }

    /// Assert hook count for event
    pub fn assert_hook_count(&self, event: &HookEvent, expected: usize) {
        let actual = self.hook_count_for(event);
        assert_eq!(
            actual, expected,
            "Expected {} invocations of {:?}, got {}",
            expected, event, actual
        );
    }

    // ---- Directory Queries ----

    /// Get state directory root
    pub fn state_root(&self) -> std::path::PathBuf {
        self.state_dir.lock().root().to_path_buf()
    }

    /// Check if state directory is initialized
    pub fn is_initialized(&self) -> bool {
        self.state_dir.lock().is_initialized()
    }

    /// Check if a project directory exists
    pub fn project_dir_exists(&self, project_path: &Path) -> bool {
        self.state_dir.lock().project_dir(project_path).exists()
    }

    // ---- Directory Assertions ----

    /// Assert state directory is initialized
    pub fn assert_initialized(&self) {
        let state_dir = self.state_dir.lock();
        assert!(state_dir.root().exists(), "State directory not created");
        assert!(
            state_dir.todos_dir().exists(),
            "Todos directory not created"
        );
        assert!(
            state_dir.projects_dir().exists(),
            "Projects directory not created"
        );
        assert!(
            state_dir.plans_dir().exists(),
            "Plans directory not created"
        );
        assert!(
            state_dir.sessions_dir().exists(),
            "Sessions directory not created"
        );
    }

    /// Assert project directory exists
    pub fn assert_project_dir_exists(&self, project_path: &Path) {
        let dir = self.state_dir.lock().project_dir(project_path);
        assert!(dir.exists(), "Project directory {:?} not created", dir);
    }

    // ---- Mutators for testing ----

    /// Get mutable access to todos
    pub fn todos(&self) -> &Arc<Mutex<TodoState>> {
        &self.todos
    }

    /// Get mutable access to sessions
    pub fn sessions(&self) -> &Arc<Mutex<SessionManager>> {
        &self.sessions
    }

    /// Get mutable access to state directory
    pub fn state_dir(&self) -> &Arc<Mutex<StateDirectory>> {
        &self.state_dir
    }

    // ---- State Reset ----

    /// Reset all state
    pub fn reset(&self) {
        self.todos.lock().clear();
        self.sessions.lock().clear();
        self.hook_history.lock().clear();
        // Note: Directory reset is more complex and may leave files
        let _ = self.state_dir.lock().reset();
    }

    /// Clear hook history only
    pub fn clear_hooks(&self) {
        self.hook_history.lock().clear();
    }
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
