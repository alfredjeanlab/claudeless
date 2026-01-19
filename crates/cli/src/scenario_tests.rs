#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::config::ResponseSpec;

fn simple_config(responses: Vec<ResponseRule>) -> ScenarioConfig {
    ScenarioConfig {
        name: "test".to_string(),
        default_response: None,
        responses,
        conversations: Default::default(),
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
        response: ResponseSpec::Simple("Hi!".to_string()),
        failure: None,
        max_matches: None,
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
        response: ResponseSpec::Simple("Matched!".to_string()),
        failure: None,
        max_matches: None,
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
        response: ResponseSpec::Simple("File!".to_string()),
        failure: None,
        max_matches: None,
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
        response: ResponseSpec::Simple("Found error!".to_string()),
        failure: None,
        max_matches: None,
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
        response: ResponseSpec::Simple("Anything!".to_string()),
        failure: None,
        max_matches: None,
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
        response: ResponseSpec::Simple("Limited!".to_string()),
        failure: None,
        max_matches: Some(2),
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
            response: ResponseSpec::Simple("Exact!".to_string()),
            failure: None,
            max_matches: None,
        },
        ResponseRule {
            pattern: PatternSpec::Contains {
                text: "spec".to_string(),
            },
            response: ResponseSpec::Simple("Contains!".to_string()),
            failure: None,
            max_matches: None,
        },
        ResponseRule {
            pattern: PatternSpec::Any,
            response: ResponseSpec::Simple("Any!".to_string()),
            failure: None,
            max_matches: None,
        },
    ]);

    let mut scenario = Scenario::from_config(config).unwrap();

    // Exact match takes priority
    let rule = scenario.match_prompt("specific").unwrap();
    assert!(matches!(&rule.response, ResponseSpec::Simple(s) if s == "Exact!"));

    // Contains match for non-exact
    let rule = scenario.match_prompt("specification").unwrap();
    assert!(matches!(&rule.response, ResponseSpec::Simple(s) if s == "Contains!"));

    // Any match for other
    let rule = scenario.match_prompt("other").unwrap();
    assert!(matches!(&rule.response, ResponseSpec::Simple(s) if s == "Any!"));
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
            response: ResponseSpec::Simple("Matched!".to_string()),
            failure: None,
            max_matches: None,
        }],
        conversations: Default::default(),
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
        response: ResponseSpec::Simple("Limited!".to_string()),
        failure: None,
        max_matches: Some(1),
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
        response: ResponseSpec::Simple("Never!".to_string()),
        failure: None,
        max_matches: None,
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
        response: ResponseSpec::Simple("Never!".to_string()),
        failure: None,
        max_matches: None,
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
