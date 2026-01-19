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

    /// Multi-turn conversation sequences
    #[serde(default)]
    pub conversations: HashMap<String, ConversationSpec>,

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
            conversations: HashMap::new(),
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

    /// Root directory for sandboxed execution (simulated mode)
    #[serde(default)]
    pub sandbox_root: Option<String>,

    /// Allow real bash execution in simulated mode
    #[serde(default)]
    pub allow_real_bash: bool,

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
    /// Execute built-in tools in a sandbox
    Simulated,
    /// Spawn real MCP servers for tool execution
    RealMcp,
}

/// A single response rule
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRule {
    /// Pattern to match against prompt
    pub pattern: PatternSpec,

    /// Response to return when pattern matches
    pub response: ResponseSpec,

    /// Optional failure to inject instead of responding
    #[serde(default)]
    pub failure: Option<FailureSpec>,

    /// How many times this rule can match (None = unlimited)
    #[serde(default)]
    pub max_matches: Option<u32>,
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

/// Multi-turn conversation specification
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationSpec {
    /// Ordered turns in the conversation
    pub turns: Vec<ConversationTurn>,
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_scenario() {
        let toml_str = r#"
name = "test-scenario"

[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hello back!"

[[responses]]
pattern = { type = "any" }
response = "Default response"
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "test-scenario");
        assert_eq!(config.responses.len(), 2);
    }

    #[test]
    fn test_parse_detailed_response() {
        let toml_str = r#"
[[responses]]
pattern = { type = "exact", text = "test" }

[responses.response]
text = "Response text"
delay_ms = 100

[[responses.response.tool_calls]]
tool = "Bash"
input = { command = "ls" }
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        let rule = &config.responses[0];
        match &rule.response {
            ResponseSpec::Detailed {
                text,
                delay_ms,
                tool_calls,
                ..
            } => {
                assert_eq!(text, "Response text");
                assert_eq!(*delay_ms, Some(100));
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].tool, "Bash");
            }
            _ => panic!("Expected Detailed response"),
        }
    }

    #[test]
    fn test_parse_failure_spec() {
        let toml_str = r#"
[[responses]]
pattern = { type = "contains", text = "fail" }
response = ""
failure = { type = "rate_limit", retry_after = 30 }
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        let rule = &config.responses[0];
        match &rule.failure {
            Some(FailureSpec::RateLimit { retry_after }) => {
                assert_eq!(*retry_after, 30);
            }
            _ => panic!("Expected RateLimit failure"),
        }
    }

    #[test]
    fn test_parse_conversation() {
        let toml_str = r#"
[conversations.auth-flow]
turns = [
    { expect = { type = "contains", text = "login" }, response = "Enter password:" },
    { expect = { type = "any" }, response = "Logged in successfully" }
]
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        assert!(config.conversations.contains_key("auth-flow"));
        let conv = &config.conversations["auth-flow"];
        assert_eq!(conv.turns.len(), 2);
    }

    #[test]
    fn test_parse_json_scenario() {
        let json_str = r#"{
            "name": "json-test",
            "responses": [
                {
                    "pattern": { "type": "contains", "text": "hello" },
                    "response": "Hi there!"
                }
            ]
        }"#;
        let config: ScenarioConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.name, "json-test");
        assert_eq!(config.responses.len(), 1);
    }

    #[test]
    fn test_parse_default_response() {
        let toml_str = r#"
name = "with-default"

[default_response]
text = "I don't understand"
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        assert!(config.default_response.is_some());
    }

    #[test]
    fn test_parse_max_matches() {
        let toml_str = r#"
[[responses]]
pattern = { type = "any" }
response = "Once only"
max_matches = 1
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.responses[0].max_matches, Some(1));
    }

    #[test]
    fn test_all_pattern_types() {
        let toml_str = r#"
[[responses]]
pattern = { type = "exact", text = "exact match" }
response = "1"

[[responses]]
pattern = { type = "regex", pattern = "^test.*" }
response = "2"

[[responses]]
pattern = { type = "glob", pattern = "*.txt" }
response = "3"

[[responses]]
pattern = { type = "contains", text = "substring" }
response = "4"

[[responses]]
pattern = { type = "any" }
response = "5"
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.responses.len(), 5);

        assert!(matches!(
            &config.responses[0].pattern,
            PatternSpec::Exact { text } if text == "exact match"
        ));
        assert!(matches!(
            &config.responses[1].pattern,
            PatternSpec::Regex { pattern } if pattern == "^test.*"
        ));
        assert!(matches!(
            &config.responses[2].pattern,
            PatternSpec::Glob { pattern } if pattern == "*.txt"
        ));
        assert!(matches!(
            &config.responses[3].pattern,
            PatternSpec::Contains { text } if text == "substring"
        ));
        assert!(matches!(&config.responses[4].pattern, PatternSpec::Any));
    }

    #[test]
    fn test_parse_tool_execution_config() {
        let toml_str = r#"
name = "tool-execution-test"

[tool_execution]
mode = "mock"

[[responses]]
pattern = { type = "contains", text = "list files" }

[responses.response]
text = "Here are the files:"

[[responses.response.tool_calls]]
tool = "Bash"
input = { command = "ls" }
result = "file1.txt\nfile2.txt"
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        assert!(config.tool_execution.is_some());
        let tool_exec = config.tool_execution.unwrap();
        assert_eq!(tool_exec.mode, ToolExecutionMode::Mock);
        assert!(tool_exec.sandbox_root.is_none());
        assert!(!tool_exec.allow_real_bash);

        // Check tool call has result
        let rule = &config.responses[0];
        if let ResponseSpec::Detailed { tool_calls, .. } = &rule.response {
            assert_eq!(tool_calls.len(), 1);
            assert_eq!(
                tool_calls[0].result,
                Some("file1.txt\nfile2.txt".to_string())
            );
        } else {
            panic!("Expected Detailed response");
        }
    }

    #[test]
    fn test_parse_tool_execution_simulated() {
        let toml_str = r#"
[tool_execution]
mode = "simulated"
sandbox_root = "/tmp/claudeless-sandbox"
allow_real_bash = true
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        let tool_exec = config.tool_execution.unwrap();
        assert_eq!(tool_exec.mode, ToolExecutionMode::Simulated);
        assert_eq!(
            tool_exec.sandbox_root,
            Some("/tmp/claudeless-sandbox".to_string())
        );
        assert!(tool_exec.allow_real_bash);
    }

    #[test]
    fn test_tool_execution_mode_default() {
        let mode = ToolExecutionMode::default();
        assert_eq!(mode, ToolExecutionMode::Disabled);
    }

    #[test]
    fn test_unknown_field_rejected() {
        let toml_str = r#"
name = "test"
unknown_field = "should fail"
"#;
        let result: Result<ScenarioConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_unknown_tool_execution_field_rejected() {
        let toml_str = r#"
[tool_execution]
mode = "mock"
unknwon_field = "should fail"
"#;
        let result: Result<ScenarioConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_parse_new_scenario_fields() {
        let toml_str = r#"
name = "full-featured"

# Session identity
default_model = "claude-opus-4-20250514"
claude_version = "3.0.0"
user_name = "TestUser"
session_id = "550e8400-e29b-41d4-a716-446655440000"
project_path = "/test/project"

# Timing
launch_timestamp = "2025-01-15T10:30:00Z"

# Environment
working_directory = "/work/dir"
trusted = false
permission_mode = "plan"
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.name, "full-featured");
        assert_eq!(
            config.default_model,
            Some("claude-opus-4-20250514".to_string())
        );
        assert_eq!(config.claude_version, Some("3.0.0".to_string()));
        assert_eq!(config.user_name, Some("TestUser".to_string()));
        assert_eq!(
            config.session_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
        assert_eq!(config.project_path, Some("/test/project".to_string()));
        assert_eq!(
            config.launch_timestamp,
            Some("2025-01-15T10:30:00Z".to_string())
        );
        assert_eq!(config.working_directory, Some("/work/dir".to_string()));
        assert!(!config.trusted);
        assert_eq!(config.permission_mode, Some("plan".to_string()));
    }

    #[test]
    fn test_parse_per_tool_config() {
        let toml_str = r#"
[tool_execution]
mode = "simulated"

[tool_execution.tools.Bash]
auto_approve = true

[tool_execution.tools.Read]
auto_approve = true
result = "file contents here"

[tool_execution.tools.Write]
error = "Permission denied"
"#;
        let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
        let tool_exec = config.tool_execution.unwrap();

        assert_eq!(tool_exec.mode, ToolExecutionMode::Simulated);
        assert_eq!(tool_exec.tools.len(), 3);

        let bash = tool_exec.tools.get("Bash").unwrap();
        assert!(bash.auto_approve);
        assert!(bash.result.is_none());
        assert!(bash.error.is_none());

        let read = tool_exec.tools.get("Read").unwrap();
        assert!(read.auto_approve);
        assert_eq!(read.result, Some("file contents here".to_string()));
        assert!(read.error.is_none());

        let write = tool_exec.tools.get("Write").unwrap();
        assert!(!write.auto_approve);
        assert!(write.result.is_none());
        assert_eq!(write.error, Some("Permission denied".to_string()));
    }

    #[test]
    fn test_default_trusted_value() {
        let config = ScenarioConfig::default();
        assert!(config.trusted);
    }
}
