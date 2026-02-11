// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =========================================================================
// Default impls
// =========================================================================

#[test]
fn scenario_config_defaults() {
    let config = ScenarioConfig::default();

    assert!(config.claude.trusted);
    assert!(config.claude.logged_in);
    assert!(config.claude.permission_mode.is_none());
    assert!(config.default.is_none());
    assert!(config.responses.is_empty());
    assert!(config.tools.is_none());
}

#[test]
fn claude_config_defaults() {
    let config = ClaudeConfig::default();

    assert!(config.session_id.is_none());
    assert!(config.project_path.is_none());
    assert!(config.launch_timestamp.is_none());
    assert!(config.username.is_none());
    assert!(config.version.is_none());
    assert!(config.model.is_none());
    assert!(config.provider.is_none());
    assert!(config.placeholder.is_none());
    assert!(config.working_directory.is_none());
    assert!(config.show_welcome_back.is_none());
    assert!(config.welcome_back_right_panel.is_none());
    assert!(config.timeouts.is_none());
}

#[test]
fn response_defaults() {
    let resp = Response::default();

    assert!(resp.say.is_none());
    assert!(resp.tools.is_empty());
    assert!(resp.usage.is_none());
    assert!(resp.delay_ms.is_none());
}

#[test]
fn pattern_default_is_glob_star() {
    let pat = Pattern::default();
    match pat {
        Pattern::Glob(s) => assert_eq!(s, "*"),
        other => panic!("Expected Glob(\"*\"), got {:?}", other),
    }
}

#[test]
fn tools_config_defaults() {
    let config = ToolsConfig::default();

    assert_eq!(config.mode, ToolExecutionMode::Live);
    assert!(config.per_tool.is_empty());
}

#[test]
fn tool_config_defaults() {
    let config = ToolConfig::default();

    assert!(!config.approve);
    assert!(config.result.is_none());
    assert!(config.error.is_none());
    assert!(config.answers.is_none());
}

// =========================================================================
// Validation
// =========================================================================

#[test]
fn validate_accepts_valid_config() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
            permission_mode: Some("plan".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(config.validate().is_ok());
}

#[test]
fn validate_rejects_invalid_session_id() {
    let config = ScenarioConfig {
        claude: ClaudeConfig { session_id: Some("not-a-uuid".to_string()), ..Default::default() },
        ..Default::default()
    };

    let err = config.validate().unwrap_err();
    assert!(err.contains("session_id"));
}

#[test]
fn validate_rejects_invalid_launch_timestamp() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            launch_timestamp: Some("not-a-timestamp".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let err = config.validate().unwrap_err();
    assert!(err.contains("launch_timestamp"));
}

#[test]
fn validate_rejects_invalid_permission_mode() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            permission_mode: Some("invalid-mode".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let err = config.validate().unwrap_err();
    assert!(err.contains("permission_mode"));
}

#[test]
fn validate_accepts_all_permission_modes() {
    for mode in VALID_PERMISSION_MODES {
        let config = ScenarioConfig {
            claude: ClaudeConfig { permission_mode: Some(mode.to_string()), ..Default::default() },
            ..Default::default()
        };
        assert!(config.validate().is_ok(), "mode '{}' should be valid", mode);
    }
}

#[test]
fn validate_accepts_empty_config() {
    let config = ScenarioConfig::default();
    assert!(config.validate().is_ok());
}

// =========================================================================
// TOML parsing
// =========================================================================

#[test]
fn parse_simple_scenario() {
    let toml_str = r#"
[[responses]]
on = { contains = "hello" }
say = "Hello back!"

[[responses]]
on = "*"
say = "Default response"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.responses.len(), 2);
    assert_eq!(config.responses[0].say, Some("Hello back!".to_string()));
}

#[test]
fn parse_glob_pattern_as_string() {
    let toml_str = r#"
[[responses]]
on = "*.txt"
say = "File!"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    match &config.responses[0].on {
        Pattern::Glob(s) => assert_eq!(s, "*.txt"),
        other => panic!("Expected Glob, got {:?}", other),
    }
}

#[test]
fn parse_contains_pattern() {
    let toml_str = r#"
[[responses]]
on = { contains = "error" }
say = "Found error!"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    match &config.responses[0].on {
        Pattern::Contains(s) => assert_eq!(s, "error"),
        other => panic!("Expected Contains, got {:?}", other),
    }
}

#[test]
fn parse_regexp_pattern() {
    let toml_str = r#"
[[responses]]
on = { regexp = "^test.*" }
say = "Matched!"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    match &config.responses[0].on {
        Pattern::Regexp(s) => assert_eq!(s, "^test.*"),
        other => panic!("Expected Regexp, got {:?}", other),
    }
}

#[test]
fn parse_response_with_tool_calls() {
    let toml_str = r#"
[[responses]]
on = "test"
say = "Response text"
delay_ms = 100

[[responses.tools]]
call = "Bash"
input = { command = "ls" }
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    let rule = &config.responses[0];
    assert_eq!(rule.say, Some("Response text".to_string()));
    assert_eq!(rule.delay_ms, Some(100));
    assert_eq!(rule.tools.len(), 1);
    assert_eq!(rule.tools[0].call, "Bash");
}

#[test]
fn parse_failure_spec() {
    let toml_str = r#"
[[responses]]
on = { contains = "fail" }
failure = { type = "rate_limit", retry_after = 30 }
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    match &config.responses[0].failure {
        Some(FailureSpec::RateLimit { retry_after }) => {
            assert_eq!(*retry_after, 30);
        }
        _ => unreachable!("Expected RateLimit failure"),
    }
}

#[test]
fn parse_turns() {
    let toml_str = r#"
[[responses]]
on = { contains = "login" }
say = "Enter username:"
then = [
    { on = "*", say = "Enter password:" },
    { on = "*", say = "Logged in successfully" }
]
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.responses[0].then.len(), 2);
    assert_eq!(config.responses[0].then[0].say, Some("Enter password:".to_string()));
}

#[test]
fn parse_json_scenario() {
    let json_str = r#"{
        "responses": [
            {
                "on": "*",
                "say": "Hi there!"
            }
        ]
    }"#;
    let config: ScenarioConfig = serde_json::from_str(json_str).unwrap();
    assert_eq!(config.responses.len(), 1);
    assert_eq!(config.responses[0].say, Some("Hi there!".to_string()));
}

#[test]
fn parse_default_response() {
    let toml_str = r#"
[default]
say = "I don't understand"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.default.unwrap().say, Some("I don't understand".to_string()));
}

#[test]
fn parse_max() {
    let toml_str = r#"
[[responses]]
on = "*"
say = "Once only"
max = 1
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.responses[0].max, Some(1));
}

#[test]
fn parse_tools_config() {
    let toml_str = r#"
[tools]
mode = "mock"

[[responses]]
on = { contains = "list files" }
say = "Here are the files:"

[[responses.tools]]
call = "Bash"
input = { command = "ls" }
result = "file1.txt\nfile2.txt"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.tools.as_ref().unwrap().mode, ToolExecutionMode::Mock);
    assert_eq!(config.responses[0].tools[0].result, Some("file1.txt\nfile2.txt".to_string()));
}

#[test]
fn parse_per_tool_config_flattened() {
    let toml_str = r#"
[tools]
mode = "live"

[tools.Bash]
approve = true

[tools.Read]
approve = true
result = "file contents here"

[tools.Write]
error = "Permission denied"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    let tools = config.tools.unwrap();

    assert_eq!(tools.mode, ToolExecutionMode::Live);
    assert_eq!(tools.per_tool.len(), 3);

    assert!(tools.per_tool.get("Bash").unwrap().approve);
    assert!(tools.per_tool.get("Read").unwrap().approve);
    assert_eq!(tools.per_tool.get("Read").unwrap().result, Some("file contents here".to_string()));
    assert_eq!(tools.per_tool.get("Write").unwrap().error, Some("Permission denied".to_string()));
}

#[test]
fn parse_claude_config() {
    let toml_str = r#"
[claude]
model = "claude-opus-4-20250514"
version = "3.0.0"
username = "TestUser"
session_id = "550e8400-e29b-41d4-a716-446655440000"
project_path = "/test/project"
launch_timestamp = "2025-01-15T10:30:00Z"
working_directory = "/work/dir"
trusted = false
permission_mode = "plan"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.claude.model, Some("claude-opus-4-20250514".to_string()));
    assert_eq!(config.claude.version, Some("3.0.0".to_string()));
    assert_eq!(config.claude.username, Some("TestUser".to_string()));
    assert_eq!(config.claude.session_id, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    assert_eq!(config.claude.project_path, Some("/test/project".to_string()));
    assert_eq!(config.claude.launch_timestamp, Some("2025-01-15T10:30:00Z".to_string()));
    assert_eq!(config.claude.working_directory, Some("/work/dir".to_string()));
    assert!(!config.claude.trusted);
    assert_eq!(config.claude.permission_mode, Some("plan".to_string()));
}

#[test]
fn parse_ask_user_question_tool_config() {
    let toml_str = r#"
[tools]
mode = "live"

[tools.AskUserQuestion]
approve = true

[tools.AskUserQuestion.answers]
"What language?" = "Rust"
"Which sections?" = "Introduction, Conclusion"
"#;
    let config: ScenarioConfig = toml::from_str(toml_str).unwrap();
    let tools = config.tools.unwrap();

    let ask = tools.per_tool.get("AskUserQuestion").unwrap();
    assert!(ask.approve);
    let answers = ask.answers.as_ref().unwrap();
    assert_eq!(answers.get("What language?").unwrap(), "Rust");
    assert_eq!(answers.get("Which sections?").unwrap(), "Introduction, Conclusion");
}

#[test]
fn parse_default_trusted_value() {
    let config: ScenarioConfig = toml::from_str("").unwrap();
    assert!(config.claude.trusted);
}

#[test]
fn unknown_field_rejected() {
    let toml_str = r#"
unknown_field = "should fail"
"#;
    let result: Result<ScenarioConfig, _> = toml::from_str(toml_str);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown field"));
}

#[test]
fn name_field_rejected() {
    let toml_str = r#"
name = "should fail"
[[responses]]
on = "*"
say = "ok"
"#;
    let result: Result<ScenarioConfig, _> = toml::from_str(toml_str);
    assert!(result.is_err(), "v2 scenarios should not accept a 'name' field");
}
