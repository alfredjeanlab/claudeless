// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Scenario configuration types for TOML/JSON scenario files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default model to report in output
pub const DEFAULT_MODEL: &str = "claude-opus-4-5-20251101";
/// Default Claude version string
pub const DEFAULT_CLAUDE_VERSION: &str = "2.1.12";
/// Default user display name
pub const DEFAULT_USER_NAME: &str = "Alfred";

fn default_true() -> bool {
    true
}

/// Session identity configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct IdentityConfig {
    /// Model to report in output (default: "claude-opus-4-5-20251101")
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

    /// Placeholder text for the input prompt (default: "Try \"write a test for scenario.rs\"")
    #[serde(default)]
    pub placeholder: Option<String>,

    /// Provider name shown in header (default: "Claude Max")
    #[serde(default)]
    pub provider: Option<String>,

    /// Show "Welcome back!" splash instead of normal header (default: false)
    #[serde(default)]
    pub show_welcome_back: Option<bool>,

    /// Right panel rows for the welcome back box.
    /// Use "---" for a separator line, "" for an empty row.
    /// Defaults to Tips/Recent activity if not specified.
    #[serde(default)]
    pub welcome_back_right_panel: Option<Vec<String>>,
}

impl IdentityConfig {
    /// Validate the identity configuration.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref id) = self.session_id {
            if uuid::Uuid::parse_str(id).is_err() {
                return Err(format!("Invalid session_id '{}': must be a valid UUID", id));
            }
        }
        Ok(())
    }
}

/// Environment configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentConfig {
    /// Override project path for state directory naming
    #[serde(default)]
    pub project_path: Option<String>,

    /// Simulated working directory (default: actual cwd)
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Whether directory is trusted (default: true)
    /// When false, TUI shows trust prompt before proceeding
    #[serde(default = "default_true")]
    pub trusted: bool,

    /// Whether user is logged in (default: true)
    /// When false, TUI shows setup wizard on launch
    #[serde(default = "default_true")]
    pub logged_in: bool,

    /// Permission mode override
    /// Values: "default", "plan", "bypass-permissions", "accept-edits", "dont-ask", "delegate"
    #[serde(default)]
    pub permission_mode: Option<String>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            project_path: None,
            working_directory: None,
            trusted: true,
            logged_in: true,
            permission_mode: None,
        }
    }
}

impl EnvironmentConfig {
    /// Valid permission mode values
    pub const VALID_PERMISSION_MODES: &'static [&'static str] = &[
        "default",
        "plan",
        "bypass-permissions",
        "accept-edits",
        "dont-ask",
        "delegate",
    ];

    /// Validate the environment configuration.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref mode) = self.permission_mode {
            if !Self::VALID_PERMISSION_MODES.contains(&mode.to_lowercase().as_str()) {
                return Err(format!(
                    "Invalid permission_mode '{}': must be one of {:?}",
                    mode,
                    Self::VALID_PERMISSION_MODES
                ));
            }
        }
        Ok(())
    }
}

/// Timing configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TimingConfig {
    /// Session start time as ISO 8601 (default: current time)
    /// Enables deterministic tests with fixed timestamps
    #[serde(default)]
    pub launch_timestamp: Option<String>,

    /// Timeout configuration
    #[serde(default)]
    pub timeouts: Option<TimeoutOverrides>,
}

impl TimingConfig {
    /// Validate the timing configuration.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref ts) = self.launch_timestamp {
            if chrono::DateTime::parse_from_rfc3339(ts).is_err() {
                return Err(format!(
                    "Invalid launch_timestamp '{}': must be ISO 8601 format (e.g., 2025-01-15T10:30:00Z)",
                    ts
                ));
            }
        }
        Ok(())
    }
}

/// Top-level scenario configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

    /// Session identity configuration
    #[serde(flatten)]
    pub identity: IdentityConfig,

    /// Environment configuration
    #[serde(flatten)]
    pub environment: EnvironmentConfig,

    /// Timing configuration
    #[serde(flatten)]
    pub timing: TimingConfig,
}

impl ScenarioConfig {
    /// Validate all sub-configurations.
    pub fn validate(&self) -> Result<(), String> {
        self.identity.validate()?;
        self.environment.validate()?;
        self.timing.validate()?;
        Ok(())
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

    /// Pre-configured answers for AskUserQuestion tool.
    /// Keys are question text, values are selected option label(s).
    /// For multi-select, join labels with ", ".
    #[serde(default)]
    pub answers: Option<HashMap<String, String>>,
}

/// Tool execution modes
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionMode {
    /// Return pre-configured results from scenario config
    Mock,
    /// Execute built-in tools directly
    #[default]
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

impl ResponseSpec {
    /// Extract just the text content.
    pub fn text(&self) -> &str {
        match self {
            ResponseSpec::Simple(s) => s,
            ResponseSpec::Detailed { text, .. } => text,
        }
    }

    /// Extract text content as owned String.
    pub fn into_text(self) -> String {
        match self {
            ResponseSpec::Simple(s) => s,
            ResponseSpec::Detailed { text, .. } => text,
        }
    }

    /// Get tool calls if any.
    pub fn tool_calls(&self) -> &[ToolCallSpec] {
        match self {
            ResponseSpec::Simple(_) => &[],
            ResponseSpec::Detailed { tool_calls, .. } => tool_calls,
        }
    }

    /// Get delay if specified.
    pub fn delay_ms(&self) -> Option<u64> {
        match self {
            ResponseSpec::Simple(_) => None,
            ResponseSpec::Detailed { delay_ms, .. } => *delay_ms,
        }
    }

    /// Extract text and optional usage from a response.
    pub fn text_and_usage(&self) -> (String, Option<UsageSpec>) {
        match self {
            ResponseSpec::Simple(s) => (s.clone(), None),
            ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
        }
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
pub use crate::usage::TokenCounts as UsageSpec;

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

/// Timeout overrides (scenario [timeouts] section)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutOverrides {
    pub exit_hint_ms: Option<u64>,
    pub compact_delay_ms: Option<u64>,
    pub hook_timeout_ms: Option<u64>,
    pub mcp_timeout_ms: Option<u64>,
    pub response_delay_ms: Option<u64>,
}

/// Resolved timeouts with defaults applied
#[derive(Clone, Debug)]
pub struct ResolvedTimeouts {
    pub exit_hint_ms: u64,
    pub compact_delay_ms: u64,
    pub hook_timeout_ms: u64,
    pub mcp_timeout_ms: u64,
    pub response_delay_ms: u64,
}

impl ResolvedTimeouts {
    pub const DEFAULT_EXIT_HINT_MS: u64 = 2000;
    pub const DEFAULT_COMPACT_DELAY_MS: u64 = 20;
    pub const DEFAULT_HOOK_TIMEOUT_MS: u64 = 5000;
    pub const DEFAULT_MCP_TIMEOUT_MS: u64 = 30000;
    pub const DEFAULT_RESPONSE_DELAY_MS: u64 = 20;

    /// Resolve from optional config with precedence: scenario > env > default
    pub fn resolve(config: Option<&TimeoutOverrides>) -> Self {
        let cfg = config.cloned().unwrap_or_default();
        Self {
            exit_hint_ms: cfg
                .exit_hint_ms
                .or_else(crate::env::exit_hint_timeout_ms)
                .unwrap_or(Self::DEFAULT_EXIT_HINT_MS),
            compact_delay_ms: cfg
                .compact_delay_ms
                .or_else(crate::env::compact_delay_ms)
                .unwrap_or(Self::DEFAULT_COMPACT_DELAY_MS),
            hook_timeout_ms: cfg
                .hook_timeout_ms
                .or_else(crate::env::hook_timeout_ms)
                .unwrap_or(Self::DEFAULT_HOOK_TIMEOUT_MS),
            mcp_timeout_ms: cfg
                .mcp_timeout_ms
                .or_else(crate::env::mcp_timeout_ms)
                .unwrap_or(Self::DEFAULT_MCP_TIMEOUT_MS),
            response_delay_ms: cfg
                .response_delay_ms
                .or_else(crate::env::response_delay_ms)
                .unwrap_or(Self::DEFAULT_RESPONSE_DELAY_MS),
        }
    }
}

impl Default for ResolvedTimeouts {
    fn default() -> Self {
        Self::resolve(None)
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
