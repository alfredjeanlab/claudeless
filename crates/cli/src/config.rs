// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Scenario configuration types for TOML/JSON scenario files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default model to report in output
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
/// Default Claude version string
pub const DEFAULT_CLAUDE_VERSION: &str = "2.1.12";
/// Default user display name
pub const DEFAULT_USER_NAME: &str = "Alfred";

fn default_trusted() -> bool {
    true
}

/// Top-level scenario configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioConfig {
    /// Name for logging/debugging
    #[serde(default)]
    pub name: String,

    /// Default response if no pattern matches
    #[serde(default)]
    pub default_response: Option<ResponseSpec>,

    /// Ordered list of response rules
    #[serde(default)]
    pub responses: Vec<ResponseRule>,

    /// Tool execution configuration
    #[serde(default)]
    pub tool_execution: Option<ToolExecutionConfig>,

    // Session identity fields
    /// Model to report in output (default: "claude-sonnet-4-20250514")
    /// Overridden by --model CLI flag
    #[serde(default)]
    pub default_model: Option<String>,

    /// Claude version string (default: "2.1.12")
    #[serde(default)]
    pub claude_version: Option<String>,

    /// User display name (default: "Alfred")
    #[serde(default)]
    pub user_name: Option<String>,

    /// Fixed session UUID for deterministic file paths (default: random)
    #[serde(default)]
    pub session_id: Option<String>,

    /// Override project path for state directory naming
    #[serde(default)]
    pub project_path: Option<String>,

    // Timing
    /// Session start time as ISO 8601 (default: current time)
    /// Enables deterministic tests with fixed timestamps
    #[serde(default)]
    pub launch_timestamp: Option<String>,

    // Environment
    /// Simulated working directory (default: actual cwd)
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Whether directory is trusted (default: true)
    /// When false, TUI shows trust prompt before proceeding
    #[serde(default = "default_trusted")]
    pub trusted: bool,

    /// Permission mode override
    /// Values: "default", "plan", "bypass-permissions", "accept-edits", "dont-ask", "delegate"
    #[serde(default)]
    pub permission_mode: Option<String>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            default_response: None,
            responses: Vec::new(),
            tool_execution: None,
            default_model: None,
            claude_version: None,
            user_name: None,
            session_id: None,
            project_path: None,
            launch_timestamp: None,
            working_directory: None,
            trusted: true, // Default to trusted
            permission_mode: None,
        }
    }
}

/// Tool execution configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolExecutionConfig {
    /// Execution mode
    #[serde(default)]
    pub mode: ToolExecutionMode,

    /// Per-tool configuration overrides
    #[serde(default)]
    pub tools: HashMap<String, ToolConfig>,
}

/// Configuration for a specific tool
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolConfig {
    /// Skip permission prompt for this tool
    #[serde(default)]
    pub auto_approve: bool,

    /// Canned result for this tool (overrides execution)
    #[serde(default)]
    pub result: Option<String>,

    /// Simulate error for this tool
    #[serde(default)]
    pub error: Option<String>,
}

/// Tool execution modes
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionMode {
    /// No tool execution (backward compatibility)
    #[default]
    Disabled,
    /// Return pre-configured results from scenario config
    Mock,
    /// Execute built-in tools directly
    Live,
}

/// A single response rule
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRule {
    /// Pattern to match against prompt (entry pattern for turn sequences)
    pub pattern: PatternSpec,

    /// Response to return when pattern matches.
    /// Optional when `failure` is set (failures don't produce responses).
    #[serde(default)]
    pub response: Option<ResponseSpec>,

    /// Optional failure to inject instead of responding
    #[serde(default)]
    pub failure: Option<FailureSpec>,

    /// How many times this rule can match (None = unlimited)
    #[serde(default)]
    pub max_matches: Option<u32>,

    /// Optional follow-up turns after initial match.
    /// When present, subsequent prompts match against turns in sequence.
    #[serde(default)]
    pub turns: Vec<ConversationTurn>,
}

/// Pattern specification for matching prompts
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternSpec {
    /// Exact string match
    Exact { text: String },
    /// Regex pattern
    Regex { pattern: String },
    /// Glob pattern (shell-style wildcards)
    Glob { pattern: String },
    /// Contains substring
    Contains { text: String },
    /// Match any prompt
    Any,
}

/// Response specification
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseSpec {
    /// Simple text response
    Simple(String),

    /// Detailed response with metadata
    Detailed {
        /// Response text content
        text: String,

        /// Simulated tool calls in the response
        #[serde(default)]
        tool_calls: Vec<ToolCallSpec>,

        /// Token usage stats (for JSON output)
        #[serde(default)]
        usage: Option<UsageSpec>,

        /// Delay before responding (ms)
        #[serde(default)]
        delay_ms: Option<u64>,
    },
}

impl Default for ResponseSpec {
    fn default() -> Self {
        ResponseSpec::Simple(String::new())
    }
}

/// Simulated tool call
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCallSpec {
    pub tool: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub result: Option<String>,
}

/// Token usage statistics
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UsageSpec {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Failure specification
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FailureSpec {
    NetworkUnreachable,
    ConnectionTimeout { after_ms: u64 },
    AuthError { message: String },
    RateLimit { retry_after: u64 },
    OutOfCredits,
    PartialResponse { partial_text: String },
    MalformedJson { raw: String },
}

/// A single turn in a multi-turn conversation
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationTurn {
    /// Expected prompt pattern for this turn
    pub expect: PatternSpec,
    /// Response for this turn
    pub response: ResponseSpec,
    /// Optional failure for this turn
    #[serde(default)]
    pub failure: Option<FailureSpec>,
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
