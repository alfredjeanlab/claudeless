// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::config::{ClaudeConfig, Pattern, ResponseRule, Turn};

fn simple_config(responses: Vec<ResponseRule>) -> ScenarioConfig {
    ScenarioConfig {
        default: None,
        responses,
        tools: None,
        ..Default::default()
    }
}

#[test]
fn test_exact_match() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("hello".to_string()),
        say: Some("Hi!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("hello").is_some());
    assert!(scenario.match_prompt("hello ").is_none());
    assert!(scenario.match_prompt("Hello").is_none());
}

#[test]
fn test_regex_match() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Regexp(r"(?i)^hello\s+\w+$".to_string()),
        say: Some("Matched!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
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
        on: Pattern::Glob("*.txt".to_string()),
        say: Some("File!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("file.txt").is_some());
    assert!(scenario.match_prompt("readme.txt").is_some());
    assert!(scenario.match_prompt("file.md").is_none());
}

#[test]
fn test_contains_match() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Contains("error".to_string()),
        say: Some("Found error!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("there was an error").is_some());
    assert!(scenario.match_prompt("error at line 5").is_some());
    assert!(scenario.match_prompt("everything is fine").is_none());
}

#[test]
fn test_any_match() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("*".to_string()),
        say: Some("Anything!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    assert!(scenario.match_prompt("anything").is_some());
    assert!(scenario.match_prompt("").is_some());
    assert!(scenario.match_prompt("random input 123").is_some());
}

#[test]
fn test_max_matches() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("*".to_string()),
        say: Some("Limited!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: Some(2),
        then: Vec::new(),
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
            on: Pattern::Glob("specific".to_string()),
            say: Some("Exact!".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: Vec::new(),
        },
        ResponseRule {
            on: Pattern::Contains("spec".to_string()),
            say: Some("Contains!".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: Vec::new(),
        },
        ResponseRule {
            on: Pattern::Glob("*".to_string()),
            say: Some("Any!".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: Vec::new(),
        },
    ]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // Exact match takes priority
    let result = scenario.match_prompt("specific").unwrap();
    assert_eq!(result, MatchResult::Response { rule_index: 0 });
    assert_eq!(scenario.get_say(&result), Some("Exact!"));

    // Contains match for non-exact
    let result = scenario.match_prompt("specification").unwrap();
    assert_eq!(result, MatchResult::Response { rule_index: 1 });
    assert_eq!(scenario.get_say(&result), Some("Contains!"));

    // Any match for other
    let result = scenario.match_prompt("other").unwrap();
    assert_eq!(result, MatchResult::Response { rule_index: 2 });
    assert_eq!(scenario.get_say(&result), Some("Any!"));
}

#[test]
fn test_default_response() {
    let config = ScenarioConfig {
        default: Some(Response {
            say: Some("Default!".to_string()),
            ..Default::default()
        }),
        responses: vec![ResponseRule {
            on: Pattern::Glob("match".to_string()),
            say: Some("Matched!".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: Vec::new(),
        }],
        tools: None,
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
        on: Pattern::Glob("*".to_string()),
        say: Some("Limited!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: Some(1),
        then: Vec::new(),
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
        on: Pattern::Regexp("[invalid".to_string()),
        say: Some("Never!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ScenarioError::Regex(_)));
}

#[test]
fn test_invalid_glob() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("[invalid".to_string()),
        say: Some("Never!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let result = Scenario::from_config(config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ScenarioError::Glob(_)));
}

#[test]
fn test_invalid_session_id() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            session_id: Some("not-a-uuid".to_string()),
            ..Default::default()
        },
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
        claude: ClaudeConfig {
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_launch_timestamp() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            launch_timestamp: Some("not-a-timestamp".to_string()),
            ..Default::default()
        },
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
        claude: ClaudeConfig {
            launch_timestamp: Some("2025-01-15T10:30:00Z".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_launch_timestamp_with_timezone() {
    let config = ScenarioConfig {
        claude: ClaudeConfig {
            launch_timestamp: Some("2025-01-15T10:30:00-08:00".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = Scenario::from_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_permission_mode() {
    let config = ScenarioConfig {
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
        on: Pattern::Contains("start".to_string()),
        say: Some("Step 1".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: vec![
            Turn {
                on: Pattern::Glob("*".to_string()),
                say: Some("Step 2".to_string()),
                tools: Vec::new(),
                usage: None,
                delay_ms: None,
                failure: None,
            },
            Turn {
                on: Pattern::Glob("*".to_string()),
                say: Some("Step 3".to_string()),
                tools: Vec::new(),
                usage: None,
                delay_ms: None,
                failure: None,
            },
        ],
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // First prompt activates sequence
    let r1 = scenario.match_prompt("start").unwrap();
    assert_eq!(r1, MatchResult::Response { rule_index: 0 });
    assert_eq!(scenario.get_say(&r1), Some("Step 1"));
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
    assert_eq!(scenario.get_say(&r2), Some("Step 2"));
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
    assert_eq!(scenario.get_say(&r3), Some("Step 3"));
    assert!(!scenario.has_active_sequence());
}

#[test]
fn test_turn_mismatch_deactivates_and_falls_through() {
    let config = simple_config(vec![
        ResponseRule {
            on: Pattern::Contains("start".to_string()),
            say: Some("Started".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: vec![Turn {
                on: Pattern::Contains("continue".to_string()),
                say: Some("Continued".to_string()),
                tools: Vec::new(),
                usage: None,
                delay_ms: None,
                failure: None,
            }],
        },
        ResponseRule {
            on: Pattern::Glob("*".to_string()),
            say: Some("Fallback".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: Vec::new(),
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
    assert_eq!(scenario.get_say(&result), Some("Fallback"));
}

#[test]
fn test_turns_with_failures() {
    use crate::config::FailureSpec;

    let config = simple_config(vec![ResponseRule {
        on: Pattern::Contains("start".to_string()),
        say: Some("Started".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: vec![Turn {
            on: Pattern::Glob("*".to_string()),
            say: None,
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
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
        on: Pattern::Contains("start".to_string()),
        say: Some("Started".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: Some(1),
        then: vec![Turn {
            on: Pattern::Glob("*".to_string()),
            say: Some("Turn 1".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
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
        on: Pattern::Contains("start".to_string()),
        say: Some("Started".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: vec![Turn {
            on: Pattern::Glob("*".to_string()),
            say: Some("Turn 1".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
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

#[test]
fn test_response_text_extracts_text() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("*".to_string()),
        say: Some("Hello!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();
    let result = scenario.match_prompt("anything").unwrap();
    assert_eq!(scenario.response_text(&result), "Hello!");
}

#[test]
fn test_response_text_returns_empty_for_none() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("*".to_string()),
        say: None, // No response
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();
    let result = scenario.match_prompt("anything").unwrap();
    assert_eq!(scenario.response_text(&result), "");
}

#[test]
fn test_response_text_or_default_matched() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Contains("hello".to_string()),
        say: Some("Matched!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();
    assert_eq!(scenario.response_text_or_default("hello world"), "Matched!");
}

#[test]
fn test_response_text_or_default_falls_back() {
    let config = ScenarioConfig {
        default: Some(Response {
            say: Some("Default!".to_string()),
            ..Default::default()
        }),
        responses: vec![ResponseRule {
            on: Pattern::Glob("specific".to_string()),
            say: Some("Matched!".to_string()),
            tools: Vec::new(),
            usage: None,
            delay_ms: None,
            failure: None,
            max: None,
            then: Vec::new(),
        }],
        tools: None,
        ..Default::default()
    };

    let mut scenario = Scenario::from_config(config).unwrap();
    assert_eq!(scenario.response_text_or_default("no match"), "Default!");
}

#[test]
fn test_response_text_or_default_returns_empty_when_no_default() {
    let config = simple_config(vec![ResponseRule {
        on: Pattern::Glob("specific".to_string()),
        say: Some("Matched!".to_string()),
        tools: Vec::new(),
        usage: None,
        delay_ms: None,
        failure: None,
        max: None,
        then: Vec::new(),
    }]);

    let mut scenario = Scenario::from_config(config).unwrap();
    assert_eq!(scenario.response_text_or_default("no match"), "");
}
