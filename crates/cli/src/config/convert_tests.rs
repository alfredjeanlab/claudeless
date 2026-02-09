// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::super::types::{FailureSpec, ToolExecutionMode, UsageSpec};
use super::*;
use serde_json::json;
use std::collections::HashMap;

// =========================================================================
// Pattern conversion
// =========================================================================

#[test]
fn convert_pattern_any() {
    let pat = convert_pattern(v1::PatternSpec::Any);
    match pat {
        Pattern::Glob(s) => assert_eq!(s, "*"),
        other => panic!("Expected Glob(\"*\"), got {:?}", other),
    }
}

#[test]
fn convert_pattern_exact() {
    let pat = convert_pattern(v1::PatternSpec::Exact {
        text: "hello".to_string(),
    });
    match pat {
        Pattern::Glob(s) => assert_eq!(s, "hello"),
        other => panic!("Expected Glob(\"hello\"), got {:?}", other),
    }
}

#[test]
fn convert_pattern_glob() {
    let pat = convert_pattern(v1::PatternSpec::Glob {
        pattern: "*.rs".to_string(),
    });
    match pat {
        Pattern::Glob(s) => assert_eq!(s, "*.rs"),
        other => panic!("Expected Glob(\"*.rs\"), got {:?}", other),
    }
}

#[test]
fn convert_pattern_contains() {
    let pat = convert_pattern(v1::PatternSpec::Contains {
        text: "error".to_string(),
    });
    match pat {
        Pattern::Contains(s) => assert_eq!(s, "error"),
        other => panic!("Expected Contains(\"error\"), got {:?}", other),
    }
}

#[test]
fn convert_pattern_regex() {
    let pat = convert_pattern(v1::PatternSpec::Regex {
        pattern: r"^\d+$".to_string(),
    });
    match pat {
        Pattern::Regexp(s) => assert_eq!(s, r"^\d+$"),
        other => panic!("Expected Regexp, got {:?}", other),
    }
}

// =========================================================================
// Response spec conversion
// =========================================================================

#[test]
fn convert_simple_response() {
    let resp = convert_response_spec(v1::ResponseSpec::Simple("hello".to_string()));

    assert_eq!(resp.say, Some("hello".to_string()));
    assert!(resp.tools.is_empty());
    assert!(resp.usage.is_none());
    assert!(resp.delay_ms.is_none());
}

#[test]
fn convert_detailed_response() {
    let resp = convert_response_spec(v1::ResponseSpec::Detailed {
        text: "response text".to_string(),
        tool_calls: vec![v1::ToolCallSpec {
            tool: "Read".to_string(),
            input: json!({"file_path": "test.rs"}),
            result: Some("contents".to_string()),
        }],
        usage: Some(UsageSpec {
            input_tokens: 100,
            output_tokens: 50,
        }),
        delay_ms: Some(500),
    });

    assert_eq!(resp.say, Some("response text".to_string()));
    assert_eq!(resp.tools.len(), 1);
    assert_eq!(resp.tools[0].call, "Read");
    assert_eq!(resp.tools[0].result, Some("contents".to_string()));
    assert_eq!(resp.usage.unwrap().input_tokens, 100);
    assert_eq!(resp.delay_ms, Some(500));
}

// =========================================================================
// Tool call conversion
// =========================================================================

#[test]
fn convert_tool_call_renames_fields() {
    let tc = convert_tool_call(v1::ToolCallSpec {
        tool: "Bash".to_string(),
        input: json!({"command": "ls"}),
        result: Some("output".to_string()),
    });

    assert_eq!(tc.call, "Bash");
    assert_eq!(tc.input, json!({"command": "ls"}));
    assert_eq!(tc.result, Some("output".to_string()));
}

// =========================================================================
// Tool config conversion
// =========================================================================

#[test]
fn convert_tool_config_renames_auto_approve() {
    let tc = convert_tool_config(v1::ToolConfig {
        auto_approve: true,
        result: Some("result".to_string()),
        error: Some("error".to_string()),
        answers: Some(HashMap::from([("q".to_string(), "a".to_string())])),
    });

    assert!(tc.approve);
    assert_eq!(tc.result, Some("result".to_string()));
    assert_eq!(tc.error, Some("error".to_string()));
    assert_eq!(tc.answers.unwrap().get("q"), Some(&"a".to_string()));
}

// =========================================================================
// Response rule conversion
// =========================================================================

#[test]
fn convert_response_rule_maps_fields() {
    let rule = convert_response_rule(v1::ResponseRule {
        pattern: v1::PatternSpec::Contains {
            text: "hello".to_string(),
        },
        response: Some(v1::ResponseSpec::Simple("world".to_string())),
        failure: None,
        max_matches: Some(3),
        turns: vec![],
    });

    match &rule.on {
        Pattern::Contains(s) => assert_eq!(s, "hello"),
        other => panic!("Expected Contains, got {:?}", other),
    }
    assert_eq!(rule.say, Some("world".to_string()));
    assert!(rule.tools.is_empty());
    assert!(rule.usage.is_none());
    assert!(rule.delay_ms.is_none());
    assert!(rule.failure.is_none());
    assert_eq!(rule.max, Some(3));
    assert!(rule.then.is_empty());
}

#[test]
fn convert_response_rule_with_turns() {
    let rule = convert_response_rule(v1::ResponseRule {
        pattern: v1::PatternSpec::Any,
        response: Some(v1::ResponseSpec::Simple("first".to_string())),
        failure: None,
        max_matches: None,
        turns: vec![v1::ConversationTurn {
            expect: v1::PatternSpec::Contains {
                text: "follow-up".to_string(),
            },
            response: v1::ResponseSpec::Simple("second".to_string()),
            failure: None,
        }],
    });

    assert_eq!(rule.then.len(), 1);
    match &rule.then[0].on {
        Pattern::Contains(s) => assert_eq!(s, "follow-up"),
        other => panic!("Expected Contains, got {:?}", other),
    }
    assert_eq!(rule.then[0].say, Some("second".to_string()));
}

#[test]
fn convert_response_rule_with_no_response() {
    let rule = convert_response_rule(v1::ResponseRule {
        pattern: v1::PatternSpec::Any,
        response: None,
        failure: Some(v1::FailureSpec::NetworkUnreachable),
        max_matches: None,
        turns: vec![],
    });

    assert!(rule.say.is_none());
    assert!(rule.tools.is_empty());
    assert!(matches!(
        rule.failure,
        Some(FailureSpec::NetworkUnreachable)
    ));
}

// =========================================================================
// Tool execution conversion
// =========================================================================

#[test]
fn convert_tool_execution_maps_mode_and_tools() {
    let mut v1_tools = HashMap::new();
    v1_tools.insert(
        "Bash".to_string(),
        v1::ToolConfig {
            auto_approve: true,
            result: None,
            error: None,
            answers: None,
        },
    );

    let te = convert_tool_execution(v1::ToolExecutionConfig {
        mode: ToolExecutionMode::Mock,
        tools: v1_tools,
    });

    assert_eq!(te.mode, ToolExecutionMode::Mock);
    assert!(te.tools.get("Bash").unwrap().approve);
}

// =========================================================================
// Full ScenarioConfig conversion
// =========================================================================

#[test]
fn convert_full_scenario_identity_fields() {
    let v1_config = v1::ScenarioConfig {
        identity: v1::IdentityConfig {
            default_model: Some("custom-model".to_string()),
            claude_version: Some("3.0.0".to_string()),
            user_name: Some("TestUser".to_string()),
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            provider: Some("Custom Provider".to_string()),
            placeholder: Some("placeholder text".to_string()),
            show_welcome_back: Some(true),
            welcome_back_right_panel: Some(vec!["line1".to_string()]),
        },
        ..Default::default()
    };

    let config: ScenarioConfig = v1_config.into();

    assert_eq!(config.claude.model, Some("custom-model".to_string()));
    assert_eq!(config.claude.version, Some("3.0.0".to_string()));
    assert_eq!(config.claude.username, Some("TestUser".to_string()));
    assert_eq!(
        config.claude.session_id,
        Some("550e8400-e29b-41d4-a716-446655440000".to_string())
    );
    assert_eq!(config.claude.provider, Some("Custom Provider".to_string()));
    assert_eq!(
        config.claude.placeholder,
        Some("placeholder text".to_string())
    );
    assert_eq!(config.claude.show_welcome_back, Some(true));
    assert_eq!(
        config.claude.welcome_back_right_panel,
        Some(vec!["line1".to_string()])
    );
}

#[test]
fn convert_full_scenario_environment_fields() {
    let v1_config = v1::ScenarioConfig {
        environment: v1::EnvironmentConfig {
            project_path: Some("/project".to_string()),
            working_directory: Some("/work".to_string()),
            trusted: false,
            logged_in: false,
            permission_mode: Some("plan".to_string()),
        },
        ..Default::default()
    };

    let config: ScenarioConfig = v1_config.into();

    assert_eq!(config.claude.project_path, Some("/project".to_string()));
    assert_eq!(config.claude.working_directory, Some("/work".to_string()));
    assert!(!config.trusted);
    assert!(!config.logged_in);
    assert_eq!(config.permission_mode, Some("plan".to_string()));
}

#[test]
fn convert_full_scenario_timing_fields() {
    let v1_config = v1::ScenarioConfig {
        timing: v1::TimingConfig {
            launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
            timeouts: Some(v1::TimeoutOverrides {
                exit_hint_ms: Some(1000),
                ..Default::default()
            }),
        },
        ..Default::default()
    };

    let config: ScenarioConfig = v1_config.into();

    assert_eq!(
        config.claude.launch_timestamp,
        Some("2025-01-15T10:30:00Z".to_string())
    );
    assert_eq!(config.claude.timeouts.unwrap().exit_hint_ms, Some(1000));
}

#[test]
fn convert_full_scenario_default_response() {
    let v1_config = v1::ScenarioConfig {
        default_response: Some(v1::ResponseSpec::Simple("fallback".to_string())),
        ..Default::default()
    };

    let config: ScenarioConfig = v1_config.into();

    assert_eq!(config.default.unwrap().say, Some("fallback".to_string()));
}

#[test]
fn convert_full_scenario_responses() {
    let v1_config = v1::ScenarioConfig {
        responses: vec![v1::ResponseRule {
            pattern: v1::PatternSpec::Contains {
                text: "test".to_string(),
            },
            response: Some(v1::ResponseSpec::Simple("matched".to_string())),
            failure: None,
            max_matches: None,
            turns: vec![],
        }],
        ..Default::default()
    };

    let config: ScenarioConfig = v1_config.into();

    assert_eq!(config.responses.len(), 1);
    assert_eq!(config.responses[0].say, Some("matched".to_string()));
}

#[test]
fn convert_full_scenario_tool_execution() {
    let mut v1_tools = HashMap::new();
    v1_tools.insert(
        "Read".to_string(),
        v1::ToolConfig {
            auto_approve: true,
            result: None,
            error: None,
            answers: None,
        },
    );

    let v1_config = v1::ScenarioConfig {
        tool_execution: Some(v1::ToolExecutionConfig {
            mode: ToolExecutionMode::Mock,
            tools: v1_tools,
        }),
        ..Default::default()
    };

    let config: ScenarioConfig = v1_config.into();

    let tools = config.tools.unwrap();
    assert_eq!(tools.mode, ToolExecutionMode::Mock);
    assert!(tools.tools.get("Read").unwrap().approve);
}
