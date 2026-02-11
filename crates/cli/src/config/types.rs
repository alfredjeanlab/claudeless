// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Scenario configuration types for TOML/JSON scenario files.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// Token usage statistics.
pub use crate::usage::TokenCounts as UsageSpec;

/// Default model to report in output
pub const DEFAULT_MODEL: &str = "claude-opus-4-5-20251101";
/// Default Claude version string
pub const DEFAULT_CLAUDE_VERSION: &str = "2.1.12";
/// Default user display name
pub const DEFAULT_USER_NAME: &str = "Alfred";

fn default_true() -> bool {
    true
}

/// Top-level scenario configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioConfig {
    #[serde(default)]
    pub claude: ClaudeConfig,
    #[serde(default)]
    pub default: Option<Response>,
    #[serde(default)]
    pub responses: Vec<ResponseRule>,
    #[serde(default)]
    pub tools: Option<ToolsConfig>,
}

/// Claude identity/environment configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeConfig {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub project_path: Option<String>,
    #[serde(default)]
    pub launch_timestamp: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub show_welcome_back: Option<bool>,
    #[serde(default)]
    pub welcome_back_right_panel: Option<Vec<String>>,
    #[serde(default)]
    pub timeouts: Option<TimeoutOverrides>,
    #[serde(default = "default_true")]
    pub trusted: bool,
    #[serde(default = "default_true")]
    pub logged_in: bool,
    #[serde(default)]
    pub permission_mode: Option<String>,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            session_id: None,
            project_path: None,
            launch_timestamp: None,
            username: None,
            version: None,
            model: None,
            provider: None,
            placeholder: None,
            working_directory: None,
            show_welcome_back: None,
            welcome_back_right_panel: None,
            timeouts: None,
            trusted: true,
            logged_in: true,
            permission_mode: None,
        }
    }
}

/// A response rule matching prompts to responses.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRule {
    pub on: Pattern,
    #[serde(default)]
    pub say: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolCall>,
    #[serde(default)]
    pub usage: Option<UsageSpec>,
    #[serde(default)]
    pub delay_ms: Option<u64>,
    #[serde(default)]
    pub failure: Option<FailureSpec>,
    #[serde(default)]
    pub max: Option<u32>,
    #[serde(default)]
    pub then: Vec<Turn>,
}

/// A follow-up turn in a multi-turn conversation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Turn {
    pub on: Pattern,
    #[serde(default)]
    pub say: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolCall>,
    #[serde(default)]
    pub usage: Option<UsageSpec>,
    #[serde(default)]
    pub delay_ms: Option<u64>,
    #[serde(default)]
    pub failure: Option<FailureSpec>,
}

/// Default response / match result fields.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    #[serde(default)]
    pub say: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolCall>,
    #[serde(default)]
    pub usage: Option<UsageSpec>,
    #[serde(default)]
    pub delay_ms: Option<u64>,
}

/// Pattern for matching prompts.
///
/// Serializes as a string for glob patterns, or a table for contains/regexp:
/// - `"*"` or `"hello"` → `Glob`
/// - `{ contains = "hello" }` → `Contains`
/// - `{ regexp = "^test" }` → `Regexp`
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

/// Helper for Pattern deserialization from a table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PatternTable {
    #[serde(default)]
    contains: Option<String>,
    #[serde(default)]
    regexp: Option<String>,
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct PatternVisitor;

        impl<'de> de::Visitor<'de> for PatternVisitor {
            type Value = Pattern;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string (glob pattern) or table with 'contains' or 'regexp' key")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Pattern, E> {
                Ok(Pattern::Glob(v.to_string()))
            }

            fn visit_map<M: de::MapAccess<'de>>(self, map: M) -> Result<Pattern, M::Error> {
                let table: PatternTable =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                match (table.contains, table.regexp) {
                    (Some(text), None) => Ok(Pattern::Contains(text)),
                    (None, Some(pattern)) => Ok(Pattern::Regexp(pattern)),
                    (Some(_), Some(_)) => Err(de::Error::custom(
                        "pattern must have either 'contains' or 'regexp', not both",
                    )),
                    (None, None) => {
                        Err(de::Error::custom("pattern table must have 'contains' or 'regexp' key"))
                    }
                }
            }
        }

        deserializer.deserialize_any(PatternVisitor)
    }
}

impl Serialize for Pattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Pattern::Glob(s) => serializer.serialize_str(s),
            Pattern::Contains(s) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("contains", s)?;
                map.end()
            }
            Pattern::Regexp(s) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("regexp", s)?;
                map.end()
            }
        }
    }
}

/// A tool call in a response.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub call: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub result: Option<String>,
}

/// Tool execution configuration.
///
/// Tool names are flattened as sibling keys of `mode`:
/// ```toml
/// [tools]
/// mode = "live"
///
/// [tools.Bash]
/// approve = true
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolsConfig {
    #[serde(default)]
    pub mode: ToolExecutionMode,
    #[serde(flatten, default)]
    pub per_tool: HashMap<String, ToolConfig>,
}

/// Per-tool configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolConfig {
    #[serde(default)]
    pub approve: bool,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub answers: Option<HashMap<String, String>>,
}

/// Tool execution modes.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionMode {
    /// Return pre-configured results from scenario config
    Mock,
    /// Execute built-in tools directly
    #[default]
    Live,
}

/// Failure specification.
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

/// Timeout overrides (scenario [timeouts] section).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutOverrides {
    pub exit_hint_ms: Option<u64>,
    pub compact_delay_ms: Option<u64>,
    pub hook_timeout_ms: Option<u64>,
    pub mcp_timeout_ms: Option<u64>,
    pub response_delay_ms: Option<u64>,
}

/// Resolved timeouts with defaults applied.
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
        if let Some(ref mode) = self.claude.permission_mode {
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
