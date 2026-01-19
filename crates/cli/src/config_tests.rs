#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
        Some(ResponseSpec::Detailed {
            text,
            delay_ms,
            tool_calls,
            ..
        }) => {
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

    // Check tool call has result
    let rule = &config.responses[0];
    if let Some(ResponseSpec::Detailed { tool_calls, .. }) = &rule.response {
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
fn test_parse_tool_execution_live() {
    let toml_str = r#"
[tool_execution]
mode = "live"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    let tool_exec = config.tool_execution.unwrap();
    assert_eq!(tool_exec.mode, ToolExecutionMode::Live);
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
mode = "live"

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

    assert_eq!(tool_exec.mode, ToolExecutionMode::Live);
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
