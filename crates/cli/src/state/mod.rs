// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! State management module for Claudeless.
//!
//! This module provides emulation of Claude Code's `~/.claude` directory structure,
//! including todos, projects, plans, settings, and session state.

pub mod directory;
pub mod session;
pub mod settings;
pub mod settings_loader;
pub mod settings_source;
pub mod todos;
pub mod words;

pub(crate) mod index;
pub(crate) mod io;
pub(crate) mod paths;
pub(crate) mod persistence;
pub(crate) mod plans;

pub use directory::{StateDirectory, StateError};
pub use persistence::ContentBlock;
pub use settings::{
    ClaudeSettings, HookCommand, HookDef, HookMatcher, PermissionSettings, Settings,
};
pub use settings_loader::{SettingsLoader, SettingsPaths};
pub use settings_source::SettingSource;
pub use todos::{TodoItem, TodoStatus};

pub(crate) use index::{get_git_branch, SessionIndexEntry, SessionsIndex};
pub(crate) use io::to_io_json;
pub(crate) use persistence::{
    append_assistant_message_jsonl, append_error_jsonl, append_result_jsonl, append_turn_jsonl,
    append_user_message_jsonl, write_queue_operation, AssistantMessageParams, TurnParams,
    UserMessageContent, UserMessageParams,
};
pub(crate) use plans::PlansManager;
pub(crate) use todos::TodoState;

use chrono::{DateTime, Utc};
use io::JsonLoad;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Facade for writing Claude state to JSONL files during execution.
///
/// `StateWriter` provides high-level methods for JSONL persistence used by
/// external watchers (e.g., otters integration tests) that monitor session state.
#[derive(Debug)]
pub struct StateWriter {
    dir: StateDirectory,
    pub session_id: String,
    project_path: PathBuf,
    launch_timestamp: DateTime<Utc>,
    model: String,
    cwd: PathBuf,
    first_prompt: Option<String>,
    message_count: u32,
}

impl StateWriter {
    /// Create a new state writer.
    pub fn new(
        session_id: impl Into<String>,
        project_path: impl Into<PathBuf>,
        launch_timestamp: DateTime<Utc>,
        model: impl Into<String>,
        cwd: impl Into<PathBuf>,
    ) -> std::io::Result<Self> {
        let mut dir = StateDirectory::resolve()?;
        dir.initialize().map_err(std::io::Error::other)?;
        Ok(Self {
            dir,
            session_id: session_id.into(),
            project_path: project_path.into(),
            launch_timestamp,
            model: model.into(),
            cwd: cwd.into(),
            first_prompt: None,
            message_count: 0,
        })
    }

    /// Create a new state writer for resuming a session.
    ///
    /// Initializes with an existing message count and loads the first prompt
    /// from the sessions index if available.
    pub fn new_with_count(
        session_id: impl Into<String>,
        project_path: impl Into<PathBuf>,
        launch_timestamp: DateTime<Utc>,
        model: impl Into<String>,
        cwd: impl Into<PathBuf>,
        message_count: u32,
    ) -> std::io::Result<Self> {
        let mut dir = StateDirectory::resolve()?;
        dir.initialize().map_err(std::io::Error::other)?;

        // Load first_prompt from index if resuming
        let session_id_str = session_id.into();
        let project_path_buf = project_path.into();
        let first_prompt = Self::load_first_prompt(&dir, &project_path_buf, &session_id_str);

        Ok(Self {
            dir,
            session_id: session_id_str,
            project_path: project_path_buf,
            launch_timestamp,
            model: model.into(),
            cwd: cwd.into(),
            first_prompt,
            message_count,
        })
    }

    fn load_first_prompt(
        dir: &StateDirectory,
        project_path: &std::path::Path,
        session_id: &str,
    ) -> Option<String> {
        let index_path = dir.project_dir(project_path).join("sessions-index.json");
        SessionsIndex::load(&index_path)
            .ok()
            .and_then(|idx| idx.get(session_id).map(|e| e.first_prompt.clone()))
    }

    /// Load message count from sessions index for an existing session.
    pub fn load_message_count_from_index(
        project_path: &std::path::Path,
        session_id: &str,
    ) -> Option<u32> {
        let dir = StateDirectory::resolve().ok()?;
        let index_path = dir.project_dir(project_path).join("sessions-index.json");
        SessionsIndex::load(&index_path)
            .ok()
            .and_then(|idx| idx.get(session_id).map(|e| e.message_count))
    }

    pub fn state_dir(&self) -> &StateDirectory {
        &self.dir
    }

    pub fn project_dir(&self) -> PathBuf {
        self.dir.project_dir(&self.project_path)
    }

    pub fn session_jsonl_path(&self) -> PathBuf {
        self.project_dir()
            .join(format!("{}.jsonl", self.session_id))
    }

    /// Get the todo file path (Claude format: `{sessionId}-agent-{sessionId}.json`).
    pub fn todo_path(&self) -> PathBuf {
        self.dir.todos_dir().join(format!(
            "{}-agent-{}.json",
            self.session_id, self.session_id
        ))
    }

    fn on_message_written(&mut self, prompt: Option<&str>) {
        if let Some(p) = prompt {
            if self.first_prompt.is_none() {
                self.first_prompt = Some(p.to_string());
            }
        }
        self.message_count += 1;
    }

    /// Write queue-operation line for `-p` (print) mode.
    pub fn write_queue_operation(&self) -> std::io::Result<()> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;
        write_queue_operation(
            &self.session_jsonl_path(),
            &self.session_id,
            "dequeue",
            Utc::now(),
        )
    }

    /// Record a conversation turn.
    pub fn record_turn(&mut self, prompt: &str, response: &str) -> std::io::Result<()> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        let git_branch = get_git_branch();
        let timestamp = Utc::now();
        let cwd = self.cwd.to_string_lossy().into_owned();
        let version = env!("CARGO_PKG_VERSION");

        let user_uuid = Uuid::new_v4().to_string();
        let assistant_uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());
        let message_id = format!("msg_{}", Uuid::new_v4().simple());

        let params = TurnParams {
            session_id: &self.session_id,
            user_uuid: &user_uuid,
            assistant_uuid: &assistant_uuid,
            request_id: &request_id,
            prompt,
            response,
            model: &self.model,
            cwd: &cwd,
            version,
            git_branch: &git_branch,
            message_id: &message_id,
            timestamp,
        };
        append_turn_jsonl(&jsonl_path, &params)?;

        self.on_message_written(Some(prompt));
        self.message_count += 1;
        self.update_sessions_index()?;

        Ok(())
    }

    /// Record a user message. Returns the user message UUID.
    pub fn record_user_message(&mut self, prompt: &str) -> std::io::Result<String> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        let git_branch = get_git_branch();
        let timestamp = Utc::now();
        let cwd = self.cwd.to_string_lossy().into_owned();
        let version = env!("CARGO_PKG_VERSION");
        let uuid = Uuid::new_v4().to_string();

        let params = UserMessageParams {
            session_id: &self.session_id,
            user_uuid: &uuid,
            parent_uuid: None,
            content: UserMessageContent::Text(prompt),
            cwd: &cwd,
            version,
            git_branch: &git_branch,
            timestamp,
        };
        append_user_message_jsonl(&jsonl_path, &params)?;

        self.on_message_written(Some(prompt));
        Ok(uuid)
    }

    /// Record an assistant response (text only, no tool calls).
    pub fn record_assistant_response(
        &mut self,
        parent_user_uuid: &str,
        response: &str,
    ) -> std::io::Result<String> {
        self.record_assistant_response_inner(parent_user_uuid, response, None)
    }

    /// Record a final assistant response (end of turn).
    pub fn record_assistant_response_final(
        &mut self,
        parent_user_uuid: &str,
        response: &str,
    ) -> std::io::Result<String> {
        self.record_assistant_response_inner(parent_user_uuid, response, Some("end_turn"))
    }

    fn record_assistant_response_inner(
        &mut self,
        parent_user_uuid: &str,
        response: &str,
        stop_reason: Option<&str>,
    ) -> std::io::Result<String> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        let git_branch = get_git_branch();
        let timestamp = Utc::now();
        let cwd = self.cwd.to_string_lossy().into_owned();
        let version = env!("CARGO_PKG_VERSION");

        let uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());
        let message_id = format!("msg_{}", Uuid::new_v4().simple());

        let params = AssistantMessageParams {
            session_id: &self.session_id,
            assistant_uuid: &uuid,
            parent_uuid: parent_user_uuid,
            request_id: &request_id,
            message_id: &message_id,
            content: vec![ContentBlock::Text {
                text: response.to_string(),
            }],
            model: &self.model,
            stop_reason,
            cwd: &cwd,
            version,
            git_branch: &git_branch,
            timestamp,
        };
        append_assistant_message_jsonl(&jsonl_path, &params)?;

        self.on_message_written(None);
        self.update_sessions_index()?;

        Ok(uuid)
    }

    /// Record an assistant message with tool_use blocks.
    pub fn record_assistant_tool_use(
        &mut self,
        parent_user_uuid: &str,
        content: Vec<ContentBlock>,
    ) -> std::io::Result<String> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        let git_branch = get_git_branch();
        let timestamp = Utc::now();
        let cwd = self.cwd.to_string_lossy().into_owned();
        let version = env!("CARGO_PKG_VERSION");

        let uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());
        let message_id = format!("msg_{}", Uuid::new_v4().simple());

        let params = AssistantMessageParams {
            session_id: &self.session_id,
            assistant_uuid: &uuid,
            parent_uuid: parent_user_uuid,
            request_id: &request_id,
            message_id: &message_id,
            content,
            model: &self.model,
            stop_reason: Some("tool_use"),
            cwd: &cwd,
            version,
            git_branch: &git_branch,
            timestamp,
        };
        append_assistant_message_jsonl(&jsonl_path, &params)?;

        self.on_message_written(None);
        Ok(uuid)
    }

    /// Record a tool result message.
    pub fn record_tool_result(
        &mut self,
        tool_use_id: &str,
        result_content: &str,
        assistant_uuid: &str,
        tool_use_result: serde_json::Value,
    ) -> std::io::Result<String> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        let git_branch = get_git_branch();
        let timestamp = Utc::now();
        let cwd = self.cwd.to_string_lossy().into_owned();
        let version = env!("CARGO_PKG_VERSION");
        let uuid = Uuid::new_v4().to_string();

        let params = UserMessageParams {
            session_id: &self.session_id,
            user_uuid: &uuid,
            parent_uuid: Some(assistant_uuid),
            content: UserMessageContent::ToolResult {
                tool_use_id,
                content: result_content,
                tool_use_result,
                source_tool_assistant_uuid: assistant_uuid,
            },
            cwd: &cwd,
            version,
            git_branch: &git_branch,
            timestamp,
        };
        append_user_message_jsonl(&jsonl_path, &params)?;
        append_result_jsonl(&jsonl_path, tool_use_id, result_content, timestamp)?;

        self.on_message_written(None);
        self.update_sessions_index()?;

        Ok(uuid)
    }

    fn update_sessions_index(&self) -> std::io::Result<()> {
        let project_dir = self.project_dir();
        let index_path = project_dir.join("sessions-index.json");

        let mut index = if index_path.exists() {
            SessionsIndex::load(&index_path)?
        } else {
            SessionsIndex::new()
        };

        let file_mtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let entry = SessionIndexEntry {
            session_id: self.session_id.clone(),
            full_path: self.session_jsonl_path().to_string_lossy().into(),
            file_mtime,
            first_prompt: self.first_prompt.clone().unwrap_or_default(),
            message_count: self.message_count,
            created: self.launch_timestamp.to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            git_branch: get_git_branch(),
            project_path: self.project_path.to_string_lossy().into(),
            is_sidechain: false,
        };

        index.add_or_update(entry);
        index.save(&index_path)
    }

    /// Write todo list (called by TodoWrite tool).
    pub fn write_todos(&self, items: &[TodoItem]) -> std::io::Result<()> {
        std::fs::create_dir_all(self.dir.todos_dir())?;

        let state = TodoState {
            items: items.to_vec(),
        };
        state.save_claude_format(&self.todo_path())
    }

    /// Create a plan file (called by ExitPlanMode tool).
    /// Returns the generated plan name (without extension).
    pub fn create_plan(&self, content: &str) -> std::io::Result<String> {
        let manager = PlansManager::new(self.dir.plans_dir());
        manager.create_markdown(content)
    }

    /// Record an error to the session JSONL file.
    pub fn record_error(
        &self,
        error: &str,
        error_type: Option<&str>,
        retry_after: Option<u64>,
        duration_ms: u64,
    ) -> std::io::Result<()> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        append_error_jsonl(
            &self.session_jsonl_path(),
            &self.session_id,
            error,
            error_type,
            retry_after,
            duration_ms,
            Utc::now(),
        )
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
