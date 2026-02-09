// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Canonical scenario configuration types (v2).
//!
//! These are the types used by all downstream code. The v1 TOML types are
//! parsed first and converted into these canonical types at load time.

use std::collections::HashMap;

/// Reused from v1 (unchanged).
pub use super::v1::{
    FailureSpec, ResolvedTimeouts, TimeoutOverrides, ToolExecutionMode, UsageSpec,
};

/// Default model to report in output
pub const DEFAULT_MODEL: &str = "claude-opus-4-5-20251101";
/// Default Claude version string
pub const DEFAULT_CLAUDE_VERSION: &str = "2.1.12";
/// Default user display name
pub const DEFAULT_USER_NAME: &str = "Alfred";

/// Top-level scenario configuration.
#[derive(Clone, Debug)]
pub struct ScenarioConfig {
    pub claude: ClaudeConfig,
    pub trusted: bool,
    pub logged_in: bool,
    pub permission_mode: Option<String>,
    pub default: Option<Response>,
    pub responses: Vec<ResponseRule>,
    pub tools: Option<ToolsConfig>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            claude: ClaudeConfig::default(),
            trusted: true,
            logged_in: true,
            permission_mode: None,
            default: None,
            responses: Vec::new(),
            tools: None,
        }
    }
}

/// Claude identity/environment configuration.
#[derive(Clone, Debug, Default)]
pub struct ClaudeConfig {
    pub session_id: Option<String>,
    pub project_path: Option<String>,
    pub launch_timestamp: Option<String>,
    pub username: Option<String>,
    pub version: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub placeholder: Option<String>,
    pub working_directory: Option<String>,
    pub show_welcome_back: Option<bool>,
    pub welcome_back_right_panel: Option<Vec<String>>,
    pub timeouts: Option<TimeoutOverrides>,
}

/// A response rule matching prompts to responses.
#[derive(Clone, Debug)]
pub struct ResponseRule {
    pub on: Pattern,
    pub say: Option<String>,
    pub tools: Vec<ToolCall>,
    pub usage: Option<UsageSpec>,
    pub delay_ms: Option<u64>,
    pub failure: Option<FailureSpec>,
    pub max: Option<u32>,
    pub then: Vec<Turn>,
}

/// A follow-up turn in a multi-turn conversation.
#[derive(Clone, Debug)]
pub struct Turn {
    pub on: Pattern,
    pub say: Option<String>,
    pub tools: Vec<ToolCall>,
    pub usage: Option<UsageSpec>,
    pub delay_ms: Option<u64>,
    pub failure: Option<FailureSpec>,
}

/// Default response / match result fields.
#[derive(Clone, Debug, Default)]
pub struct Response {
    pub say: Option<String>,
    pub tools: Vec<ToolCall>,
    pub usage: Option<UsageSpec>,
    pub delay_ms: Option<u64>,
}

/// Pattern for matching prompts.
#[derive(Clone, Debug)]
pub enum Pattern {
    /// Glob pattern (also covers "any" via `*` and "exact" via literal text).
    Glob(String),
    /// Substring match.
    Contains(String),
    /// Regular expression.
    Regexp(String),
}

impl Default for Pattern {
    fn default() -> Self {
        Pattern::Glob("*".to_string())
    }
}

/// A tool call in a response.
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub call: String,
    pub input: serde_json::Value,
    pub result: Option<String>,
}

/// Tool execution configuration.
#[derive(Clone, Debug, Default)]
pub struct ToolsConfig {
    pub mode: ToolExecutionMode,
    pub tools: HashMap<String, ToolConfig>,
}

/// Per-tool configuration.
#[derive(Clone, Debug, Default)]
pub struct ToolConfig {
    pub approve: bool,
    pub result: Option<String>,
    pub error: Option<String>,
    pub answers: Option<HashMap<String, String>>,
}

// =============================================================================
// Validation
// =============================================================================

/// Valid permission mode values.
pub const VALID_PERMISSION_MODES: &[&str] =
    &["default", "plan", "bypass-permissions", "accept-edits", "dont-ask", "delegate"];

impl ScenarioConfig {
    /// Validate the scenario configuration.
    pub fn validate(&self) -> Result<(), String> {
        // Validate session_id
        if let Some(ref id) = self.claude.session_id {
            if uuid::Uuid::parse_str(id).is_err() {
                return Err(format!("Invalid session_id '{}': must be a valid UUID", id));
            }
        }

        // Validate launch_timestamp
        if let Some(ref ts) = self.claude.launch_timestamp {
            if chrono::DateTime::parse_from_rfc3339(ts).is_err() {
                return Err(format!(
                    "Invalid launch_timestamp '{}': must be ISO 8601 format (e.g., 2025-01-15T10:30:00Z)",
                    ts
                ));
            }
        }

        // Validate permission_mode
        if let Some(ref mode) = self.permission_mode {
            if !VALID_PERMISSION_MODES.contains(&mode.to_lowercase().as_str()) {
                return Err(format!(
                    "Invalid permission_mode '{}': must be one of {:?}",
                    mode, VALID_PERMISSION_MODES
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
