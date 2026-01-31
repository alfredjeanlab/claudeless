// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! State management module for Claudeless.
//!
//! This module provides emulation of Claude Code's `~/.claude` directory structure,
//! including todos, projects, plans, settings, and session state.

pub mod directory;
pub mod io;
pub mod plans;
pub mod session;
pub mod sessions_index;
pub mod settings;
pub mod settings_loader;
pub mod settings_source;
pub mod todos;
pub mod words;

pub use directory::{normalize_project_path, project_dir_name, StateDirectory, StateError};
pub use plans::{Plan, PlansManager};
pub use session::{
    append_assistant_message_jsonl, append_error_jsonl, append_result_jsonl, append_turn_jsonl,
    append_user_message_jsonl, write_queue_operation, AssistantMessage, AssistantMessageLine,
    AssistantMessageParams, ContentBlock, ErrorLine, QueueOperationLine, ResultLine, Session,
    SessionManager, ToolResultContent, ToolResultMessageLine, ToolResultUserMessage, Turn,
    TurnParams, TurnToolCall, Usage, UserMessage, UserMessageContent, UserMessageLine,
    UserMessageParams,
};
pub use sessions_index::{get_git_branch, SessionIndexEntry, SessionsIndex};
pub use settings::{ClaudeSettings, McpServerConfig, PermissionSettings, Settings};
pub use settings_loader::{SettingsLoader, SettingsPaths};
pub use settings_source::SettingSource;
pub use todos::{ClaudeTodoItem, TodoItem, TodoState, TodoStatus};

pub use io::{to_io_error, to_io_json, JsonLoad};

use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Facade for writing Claude state during execution.
///
/// `StateWriter` wraps `StateDirectory` and provides high-level methods
/// for the operations main.rs needs during session execution.
#[derive(Debug)]
pub struct StateWriter {
    /// The underlying state directory.
    dir: StateDirectory,
    /// Session ID (UUID).
    session_id: String,
    /// Project path for directory naming.
    project_path: PathBuf,
    /// Session launch timestamp.
    launch_timestamp: DateTime<Utc>,
    /// Model name for messages.
    model: String,
    /// Working directory (cwd).
    cwd: PathBuf,
    /// First prompt (for sessions-index).
    first_prompt: Option<String>,
    /// Message count.
    message_count: u32,
}

impl StateWriter {
    /// Create a new state writer.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session UUID string
    /// * `project_path` - Project path for directory naming
    /// * `launch_timestamp` - Session start time
    /// * `model` - Model name (e.g., "claude-sonnet-4-20250514")
    /// * `cwd` - Working directory
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

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the state directory.
    pub fn state_dir(&self) -> &StateDirectory {
        &self.dir
    }

    /// Get the project directory path.
    pub fn project_dir(&self) -> PathBuf {
        self.dir.project_dir(&self.project_path)
    }

    /// Get the session JSONL file path.
    pub fn session_jsonl_path(&self) -> PathBuf {
        self.project_dir()
            .join(format!("{}.jsonl", self.session_id))
    }

    /// Get the todo file path (Claude format).
    ///
    /// Format: `{sessionId}-agent-{sessionId}.json`
    pub fn todo_path(&self) -> PathBuf {
        self.dir.todos_dir().join(format!(
            "{}-agent-{}.json",
            self.session_id, self.session_id
        ))
    }

    /// Write queue-operation line for `-p` (print) mode.
    ///
    /// This should be called once at the start of a session in print mode.
    pub fn write_queue_operation(&self) -> std::io::Result<()> {
        // Ensure project directory exists
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        write_queue_operation(&jsonl_path, &self.session_id, "dequeue", Utc::now())
    }

    /// Record a conversation turn.
    ///
    /// Writes to the JSONL session file and updates sessions-index.json.
    /// This is a convenience method for simple turns without tool calls.
    pub fn record_turn(&mut self, prompt: &str, response: &str) -> std::io::Result<()> {
        // Ensure project directory exists
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();

        // Generate UUIDs for this turn
        let user_uuid = Uuid::new_v4().to_string();
        let assistant_uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());

        // Append to JSONL file
        let git_branch = get_git_branch();
        let message_id = format!("msg_{}", Uuid::new_v4().simple());
        let params = TurnParams {
            session_id: &self.session_id,
            user_uuid: &user_uuid,
            assistant_uuid: &assistant_uuid,
            request_id: &request_id,
            prompt,
            response,
            model: &self.model,
            cwd: &self.cwd.to_string_lossy(),
            version: env!("CARGO_PKG_VERSION"),
            git_branch: &git_branch,
            message_id: &message_id,
            timestamp: Utc::now(),
        };
        append_turn_jsonl(&jsonl_path, &params)?;

        // Track first prompt and message count
        if self.first_prompt.is_none() {
            self.first_prompt = Some(prompt.to_string());
        }
        self.message_count += 2; // user + assistant

        // Update sessions-index.json
        self.update_sessions_index()?;

        Ok(())
    }

    /// Record a user message.
    ///
    /// Returns the user message UUID for linking to assistant responses.
    pub fn record_user_message(&mut self, prompt: &str) -> std::io::Result<String> {
        // Ensure project directory exists
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        let user_uuid = Uuid::new_v4().to_string();
        let git_branch = get_git_branch();

        let params = UserMessageParams {
            session_id: &self.session_id,
            user_uuid: &user_uuid,
            parent_uuid: None,
            content: UserMessageContent::Text(prompt),
            cwd: &self.cwd.to_string_lossy(),
            version: env!("CARGO_PKG_VERSION"),
            git_branch: &git_branch,
            timestamp: Utc::now(),
        };
        append_user_message_jsonl(&jsonl_path, &params)?;

        // Track first prompt and message count
        if self.first_prompt.is_none() {
            self.first_prompt = Some(prompt.to_string());
        }
        self.message_count += 1;

        Ok(user_uuid)
    }

    /// Record an assistant response (text only, no tool calls).
    ///
    /// This is for simple text responses without tool_use blocks.
    /// Use `record_assistant_response_final` for the final response in a turn.
    pub fn record_assistant_response(
        &mut self,
        parent_user_uuid: &str,
        response: &str,
    ) -> std::io::Result<String> {
        self.record_assistant_response_inner(parent_user_uuid, response, None)
    }

    /// Record a final assistant response (end of turn).
    ///
    /// This sets `stop_reason: "end_turn"` for turn completion detection.
    pub fn record_assistant_response_final(
        &mut self,
        parent_user_uuid: &str,
        response: &str,
    ) -> std::io::Result<String> {
        self.record_assistant_response_inner(parent_user_uuid, response, Some("end_turn"))
    }

    /// Internal helper for recording assistant responses.
    fn record_assistant_response_inner(
        &mut self,
        parent_user_uuid: &str,
        response: &str,
        stop_reason: Option<&str>,
    ) -> std::io::Result<String> {
        let jsonl_path = self.session_jsonl_path();
        let assistant_uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());
        let message_id = format!("msg_{}", Uuid::new_v4().simple());
        let git_branch = get_git_branch();

        let params = AssistantMessageParams {
            session_id: &self.session_id,
            assistant_uuid: &assistant_uuid,
            parent_uuid: parent_user_uuid,
            request_id: &request_id,
            message_id: &message_id,
            content: vec![ContentBlock::Text {
                text: response.to_string(),
            }],
            model: &self.model,
            stop_reason,
            cwd: &self.cwd.to_string_lossy(),
            version: env!("CARGO_PKG_VERSION"),
            git_branch: &git_branch,
            timestamp: Utc::now(),
        };
        append_assistant_message_jsonl(&jsonl_path, &params)?;

        self.message_count += 1;
        self.update_sessions_index()?;

        Ok(assistant_uuid)
    }

    /// Record an assistant message with tool_use blocks.
    ///
    /// Returns the assistant message UUID for linking tool results.
    /// Sets `stop_reason: "tool_use"` automatically for tool call messages.
    pub fn record_assistant_tool_use(
        &mut self,
        parent_user_uuid: &str,
        content: Vec<ContentBlock>,
    ) -> std::io::Result<String> {
        let jsonl_path = self.session_jsonl_path();
        let assistant_uuid = Uuid::new_v4().to_string();
        let request_id = format!("req_{}", Uuid::new_v4().simple());
        let message_id = format!("msg_{}", Uuid::new_v4().simple());
        let git_branch = get_git_branch();

        let params = AssistantMessageParams {
            session_id: &self.session_id,
            assistant_uuid: &assistant_uuid,
            parent_uuid: parent_user_uuid,
            request_id: &request_id,
            message_id: &message_id,
            content,
            model: &self.model,
            stop_reason: Some("tool_use"),
            cwd: &self.cwd.to_string_lossy(),
            version: env!("CARGO_PKG_VERSION"),
            git_branch: &git_branch,
            timestamp: Utc::now(),
        };
        append_assistant_message_jsonl(&jsonl_path, &params)?;

        self.message_count += 1;

        Ok(assistant_uuid)
    }

    /// Record a tool result message.
    ///
    /// This records the tool result as:
    /// 1. A user message with tool_result content (for Claude API compatibility)
    /// 2. A result record (for log extraction)
    pub fn record_tool_result(
        &mut self,
        tool_use_id: &str,
        result_content: &str,
        assistant_uuid: &str,
        tool_use_result: serde_json::Value,
    ) -> std::io::Result<String> {
        let jsonl_path = self.session_jsonl_path();
        let result_uuid = Uuid::new_v4().to_string();
        let git_branch = get_git_branch();
        let timestamp = Utc::now();

        // Write user message with tool_result content
        let params = UserMessageParams {
            session_id: &self.session_id,
            user_uuid: &result_uuid,
            parent_uuid: Some(assistant_uuid),
            content: UserMessageContent::ToolResult {
                tool_use_id,
                content: result_content,
                tool_use_result,
                source_tool_assistant_uuid: assistant_uuid,
            },
            cwd: &self.cwd.to_string_lossy(),
            version: env!("CARGO_PKG_VERSION"),
            git_branch: &git_branch,
            timestamp,
        };
        append_user_message_jsonl(&jsonl_path, &params)?;

        // Also write result record for log extraction
        append_result_jsonl(&jsonl_path, tool_use_id, result_content, timestamp)?;

        self.message_count += 1;
        self.update_sessions_index()?;

        Ok(result_uuid)
    }

    /// Update the sessions-index.json file.
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
    ///
    /// Creates a todo file in Claude CLI format at:
    /// `{state_dir}/todos/{sessionId}-agent-{sessionId}.json`
    pub fn write_todos(&self, items: &[TodoItem]) -> std::io::Result<()> {
        std::fs::create_dir_all(self.dir.todos_dir())?;

        let state = TodoState {
            items: items.to_vec(),
        };
        state.save_claude_format(&self.todo_path())
    }

    /// Create a plan file (called by ExitPlanMode tool).
    ///
    /// Creates a markdown plan file with word-based naming at:
    /// `{state_dir}/plans/{adjective}-{verb}-{noun}.md`
    ///
    /// Returns the generated plan name (without extension).
    pub fn create_plan(&self, content: &str) -> std::io::Result<String> {
        let manager = PlansManager::new(self.dir.plans_dir());
        manager.create_markdown(content)
    }

    /// Record an error to the session JSONL file.
    ///
    /// Writes an error entry in the result wrapper format with `subtype: "error"`.
    /// This is used by failure modes to record errors before exiting.
    ///
    /// # Arguments
    /// * `error` - Error message
    /// * `error_type` - Optional error type (e.g., "rate_limit_error", "network_error")
    /// * `retry_after` - Optional retry delay in seconds (for rate limits)
    /// * `duration_ms` - Time elapsed before error
    pub fn record_error(
        &self,
        error: &str,
        error_type: Option<&str>,
        retry_after: Option<u64>,
        duration_ms: u64,
    ) -> std::io::Result<()> {
        let project_dir = self.project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let jsonl_path = self.session_jsonl_path();
        append_error_jsonl(
            &jsonl_path,
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
