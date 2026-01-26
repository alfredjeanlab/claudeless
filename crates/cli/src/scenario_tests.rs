// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::config::{ConversationTurn, ResponseRule, ResponseSpec};

fn simple_config(responses: Vec<ResponseRule>) -> ScenarioConfig {
    ScenarioConfig {
        name: "test".to_string(),
        default_response: None,
        responses,
        tool_execution: None,
        ..Default::default()
    }
}

#[test]
fn test_exact_match() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Exact {
            text: "hello".to_string(),
        },
        response: Some(ResponseSpec::Simple("Hi!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("hello").is_some());
    assert!(scenario.match_prompt("hello ").is_none());
    assert!(scenario.match_prompt("Hello").is_none());
}

#[test]
fn test_regex_match() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Regex {
            pattern: r"(?i)^hello\s+\w+$".to_string(),
        },
        response: Some(ResponseSpec::Simple("Matched!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("hello world").is_some());
    assert!(scenario.match_prompt("Hello World").is_some());
    assert!(scenario.match_prompt("hello").is_none());
    assert!(scenario.match_prompt("hello world!").is_none());
}

#[test]
fn test_glob_match() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Glob {
            pattern: "*.txt".to_string(),
        },
        response: Some(ResponseSpec::Simple("File!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("file.txt").is_some());
    assert!(scenario.match_prompt("readme.txt").is_some());
    assert!(scenario.match_prompt("file.md").is_none());
}

#[test]
fn test_contains_match() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Contains {
            text: "error".to_string(),
        },
        response: Some(ResponseSpec::Simple("Found error!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("there was an error").is_some());
    assert!(scenario.match_prompt("error at line 5").is_some());
    assert!(scenario.match_prompt("everything is fine").is_none());
}

#[test]
fn test_any_match() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Any,
        response: Some(ResponseSpec::Simple("Anything!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("anything").is_some());
    assert!(scenario.match_prompt("").is_some());
    assert!(scenario.match_prompt("random input 123").is_some());
}

#[test]
fn test_max_matches() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Any,
        response: Some(ResponseSpec::Simple("Limited!".to_string())),
        failure: None,
        max_matches: Some(2),
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("first").is_some());
    assert!(scenario.match_prompt("second").is_some());
    assert!(scenario.match_prompt("third").is_none());
}

#[test]
fn test_rule_ordering() {
    let config = simple_config(vec![
        ResponseRule {
            pattern: PatternSpec::Exact {
                text: "specific".to_string(),
            },
            response: Some(ResponseSpec::Simple("Exact!".to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        },
        ResponseRule {
            pattern: PatternSpec::Contains {
                text: "spec".to_string(),
            },
            response: Some(ResponseSpec::Simple("Contains!".to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        },
        ResponseRule {
            pattern: PatternSpec::Any,
            response: Some(ResponseSpec::Simple("Any!".to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        },
    ]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // Exact match takes priority
    let result = scenario.match_prompt("specific").unwrap();
    assert_eq!(result, MatchResult::Response { rule_index: 0 });
    let response = scenario.get_response(&result);
    assert!(matches!(response, Some(ResponseSpec::Simple(s)) if s == "Exact!"));

    // Contains match for non-exact
    let result = scenario.match_prompt("specification").unwrap();
    assert_eq!(result, MatchResult::Response { rule_index: 1 });
    let response = scenario.get_response(&result);
    assert!(matches!(response, Some(ResponseSpec::Simple(s)) if s == "Contains!"));

    // Any match for other
    let result = scenario.match_prompt("other").unwrap();
    assert_eq!(result, MatchResult::Response { rule_index: 2 });
    let response = scenario.get_response(&result);
    assert!(matches!(response, Some(ResponseSpec::Simple(s)) if s == "Any!"));
}

#[test]
fn test_default_response() {
    let config = ScenarioConfig {
        name: "with-default".to_string(),
        default_response: Some(ResponseSpec::Simple("Default!".to_string())),
        responses: vec![ResponseRule {
            pattern: PatternSpec::Exact {
                text: "match".to_string(),
            },
            response: Some(ResponseSpec::Simple("Matched!".to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        }],
        tool_execution: None,
        ..Default::default()
    };

    let mut scenario = Scenario::from_config(config).unwrap();

    // Rule matches
    assert!(scenario.match_prompt("match").is_some());

    // No rule matches, but default exists
    assert!(scenario.match_prompt("no match").is_none());
    assert!(scenario.default_response().is_some());
}

#[test]
fn test_reset_counts() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Any,
        response: Some(ResponseSpec::Simple("Limited!".to_string())),
        failure: None,
        max_matches: Some(1),
        turns: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("first").is_some());
    assert!(scenario.match_prompt("second").is_none());

    scenario.reset_counts();

    assert!(scenario.match_prompt("third").is_some());
}

#[test]
fn test_invalid_regex() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Regex {
            pattern: "[invalid".to_string(),
        },
        response: Some(ResponseSpec::Simple("Never!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ScenarioError::Regex(_)));
}

#[test]
fn test_invalid_glob() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Glob {
            pattern: "[invalid".to_string(),
        },
        response: Some(ResponseSpec::Simple("Never!".to_string())),
        failure: None,
        max_matches: None,
        turns: Vec::new(),
    }]);

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ScenarioError::Glob(_)));
}

#[test]
fn test_invalid_session_id() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        session_id: Some("not-a-uuid".to_string()),
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ScenarioError::Validation(_)));
    assert!(err.to_string().contains("session_id"));
    assert!(err.to_string().contains("not-a-uuid"));
}

#[test]
fn test_valid_session_id() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_launch_timestamp() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        launch_timestamp: Some("not-a-timestamp".to_string()),
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ScenarioError::Validation(_)));
    assert!(err.to_string().contains("launch_timestamp"));
    assert!(err.to_string().contains("not-a-timestamp"));
}

#[test]
fn test_valid_launch_timestamp() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_launch_timestamp_with_timezone() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        launch_timestamp: Some("2025-01-15T10:30:00-08:00".to_string()),
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_permission_mode() {
    let config = ScenarioConfig {
        name: "test".to_string(),
        permission_mode: Some("invalid-mode".to_string()),
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ScenarioError::Validation(_)));
    assert!(err.to_string().contains("permission_mode"));
    assert!(err.to_string().contains("invalid-mode"));
}

#[test]
fn test_valid_permission_modes() {
    for mode in [
        "default",
        "plan",
        "bypass-permissions",
        "accept-edits",
        "dont-ask",
        "delegate",
    ] {
        let config = ScenarioConfig {
            name: "test".to_string(),
            permission_mode: Some(mode.to_string()),
            ..Default::default()
        };

        let result = Scenario::from_config(config);
        assert!(result.is_ok(), "Failed for mode: {}", mode);
    }
}

// Turn sequence tests

#[test]
fn test_turn_sequence_advances() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Contains {
            text: "start".to_string(),
        },
        response: Some(ResponseSpec::Simple("Step 1".to_string())),
        failure: None,
        max_matches: None,
        turns: vec![
            ConversationTurn {
                expect: PatternSpec::Any,
                response: ResponseSpec::Simple("Step 2".to_string()),
                failure: None,
            },
            ConversationTurn {
                expect: PatternSpec::Any,
                response: ResponseSpec::Simple("Step 3".to_string()),
                failure: None,
            },
        ],
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // First prompt activates sequence
    let r1 = scenario.match_prompt("start").unwrap();
    assert_eq!(r1, MatchResult::Response { rule_index: 0 });
    let resp1 = scenario.get_response(&r1);
    assert!(matches!(resp1, Some(ResponseSpec::Simple(s)) if s == "Step 1"));
    assert!(scenario.has_active_sequence());

    // Second prompt advances to turn 0
    let r2 = scenario.match_prompt("anything").unwrap();
    assert_eq!(
        r2,
        MatchResult::Turn {
            rule_index: 0,
            turn_index: 0
        }
    );
    let resp2 = scenario.get_response(&r2);
    assert!(matches!(resp2, Some(ResponseSpec::Simple(s)) if s == "Step 2"));
    assert!(scenario.has_active_sequence());

    // Third prompt advances to turn 1 and completes
    let r3 = scenario.match_prompt("anything").unwrap();
    assert_eq!(
        r3,
        MatchResult::Turn {
            rule_index: 0,
            turn_index: 1
        }
    );
    let resp3 = scenario.get_response(&r3);
    assert!(matches!(resp3, Some(ResponseSpec::Simple(s)) if s == "Step 3"));
    assert!(!scenario.has_active_sequence());
}

#[test]
fn test_turn_mismatch_deactivates_and_falls_through() {
    let config = simple_config(vec![
        ResponseRule {
            pattern: PatternSpec::Contains {
                text: "start".to_string(),
            },
            response: Some(ResponseSpec::Simple("Started".to_string())),
            failure: None,
            max_matches: None,
            turns: vec![ConversationTurn {
                expect: PatternSpec::Contains {
                    text: "continue".to_string(),
                },
                response: ResponseSpec::Simple("Continued".to_string()),
                failure: None,
            }],
        },
        ResponseRule {
            pattern: PatternSpec::Any,
            response: Some(ResponseSpec::Simple("Fallback".to_string())),
            failure: None,
            max_matches: None,
            turns: Vec::new(),
        },
    ]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // Activate sequence
    scenario.match_prompt("start");
    assert!(scenario.has_active_sequence());

    // Mismatch - should deactivate and fall through to "any" rule
    let result = scenario.match_prompt("wrong input").unwrap();
    assert!(!scenario.has_active_sequence());
    assert_eq!(result, MatchResult::Response { rule_index: 1 });
    let response = scenario.get_response(&result);
    assert!(matches!(response, Some(ResponseSpec::Simple(s)) if s == "Fallback"));
}

#[test]
fn test_turns_with_failures() {
    use crate::config::FailureSpec;

    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Contains {
            text: "start".to_string(),
        },
        response: Some(ResponseSpec::Simple("Started".to_string())),
        failure: None,
        max_matches: None,
        turns: vec![ConversationTurn {
            expect: PatternSpec::Any,
            response: ResponseSpec::Simple(String::new()),
            failure: Some(FailureSpec::AuthError {
                message: "Session expired".to_string(),
            }),
        }],
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    scenario.match_prompt("start");
    let result = scenario.match_prompt("next").unwrap();
    assert!(scenario.get_failure(&result).is_some());
}

#[test]
fn test_max_matches_applies_to_sequence_entry() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Contains {
            text: "start".to_string(),
        },
        response: Some(ResponseSpec::Simple("Started".to_string())),
        failure: None,
        max_matches: Some(1),
        turns: vec![ConversationTurn {
            expect: PatternSpec::Any,
            response: ResponseSpec::Simple("Turn 1".to_string()),
            failure: None,
        }],
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // First entry works
    assert!(scenario.match_prompt("start").is_some());
    scenario.match_prompt("next"); // Complete sequence

    // Second entry blocked by max_matches
    assert!(scenario.match_prompt("start").is_none());
}

#[test]
fn test_reset_counts_also_resets_turns() {
    let config = simple_config(vec![ResponseRule {
        pattern: PatternSpec::Contains {
            text: "start".to_string(),
        },
        response: Some(ResponseSpec::Simple("Started".to_string())),
        failure: None,
        max_matches: None,
        turns: vec![ConversationTurn {
            expect: PatternSpec::Any,
            response: ResponseSpec::Simple("Turn 1".to_string()),
            failure: None,
        }],
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // Activate a sequence
    scenario.match_prompt("start");
    assert!(scenario.has_active_sequence());

    // Reset should clear both match counts and turn state
    scenario.reset_counts();
    assert!(!scenario.has_active_sequence());
}
