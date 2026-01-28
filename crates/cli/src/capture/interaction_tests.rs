// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;
use proptest::prelude::*;
use std::time::{Duration, SystemTime};

fn make_args(prompt: Option<&str>) -> CapturedArgs {
    CapturedArgs {
        prompt: prompt.map(|s| s.to_string()),
        model: "claude-test".to_string(),
        output_format: "text".to_string(),
        print_mode: true,
        continue_conversation: false,
        resume: None,
        allowed_tools: vec![],
        cwd: None,
    }
}

#[test]
fn test_captured_args_serialization() {
    let args = make_args(Some("hello world"));
    let json = serde_json::to_string(&args).unwrap();
    let parsed: CapturedArgs = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.prompt, Some("hello world".to_string()));
    assert_eq!(parsed.model, "claude-test");
    assert_eq!(parsed.output_format, "text");
}

#[test]
fn test_captured_args_with_all_fields() {
    let args = CapturedArgs {
        prompt: Some("test prompt".to_string()),
        model: "claude-3-opus".to_string(),
        output_format: "json".to_string(),
        print_mode: false,
        continue_conversation: true,
        resume: Some("session-123".to_string()),
        allowed_tools: vec!["Bash".to_string(), "Read".to_string()],
        cwd: Some("/home/user".to_string()),
    };

    let json = serde_json::to_string(&args).unwrap();
    let parsed: CapturedArgs = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.prompt, args.prompt);
    assert_eq!(parsed.resume, Some("session-123".to_string()));
    assert_eq!(parsed.allowed_tools, vec!["Bash", "Read"]);
    assert_eq!(parsed.cwd, Some("/home/user".to_string()));
}

#[test]
fn test_captured_outcome_response_serialization() {
    let outcome = CapturedOutcome::Response {
        text: "Hello!".to_string(),
        matched_rule: Some("greeting".to_string()),
        delay_ms: 100,
    };

    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains(r#""type":"response""#));

    let parsed: CapturedOutcome = serde_json::from_str(&json).unwrap();
    match parsed {
        CapturedOutcome::Response {
            text,
            matched_rule,
            delay_ms,
        } => {
            assert_eq!(text, "Hello!");
            assert_eq!(matched_rule, Some("greeting".to_string()));
            assert_eq!(delay_ms, 100);
        }
        _ => panic!("Expected Response variant"),
    }
}

#[test]
fn test_captured_outcome_failure_serialization() {
    let outcome = CapturedOutcome::Failure {
        failure_type: "network".to_string(),
        message: "Connection refused".to_string(),
    };

    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains(r#""type":"failure""#));

    let parsed: CapturedOutcome = serde_json::from_str(&json).unwrap();
    match parsed {
        CapturedOutcome::Failure {
            failure_type,
            message,
        } => {
            assert_eq!(failure_type, "network");
            assert_eq!(message, "Connection refused");
        }
        _ => panic!("Expected Failure variant"),
    }
}

#[test]
fn test_captured_outcome_no_match_serialization() {
    let outcome = CapturedOutcome::NoMatch { used_default: true };

    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains(r#""type":"no_match""#));

    let parsed: CapturedOutcome = serde_json::from_str(&json).unwrap();
    match parsed {
        CapturedOutcome::NoMatch { used_default } => {
            assert!(used_default);
        }
        _ => panic!("Expected NoMatch variant"),
    }
}

#[test]
fn test_captured_interaction_serialization() {
    let interaction = CapturedInteraction {
        seq: 42,
        timestamp: SystemTime::now(),
        elapsed: Duration::from_millis(100),
        args: make_args(Some("test")),
        outcome: CapturedOutcome::Response {
            text: "response".to_string(),
            matched_rule: Some("rule1".to_string()),
            delay_ms: 50,
        },
    };

    let json = serde_json::to_string(&interaction).unwrap();
    let parsed: CapturedInteraction = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.seq, 42);
    assert_eq!(parsed.args.prompt, Some("test".to_string()));
    assert_eq!(parsed.elapsed, Duration::from_millis(100));
}

// Property-based tests
proptest! {
    #[test]
    fn captured_args_roundtrip(
        prompt in proptest::option::of("[a-zA-Z0-9 ]{0,100}"),
        model in "[a-z-]{1,20}",
        print_mode in proptest::bool::ANY,
        continue_conv in proptest::bool::ANY,
    ) {
        let args = CapturedArgs {
            prompt: prompt.map(|s| s.to_string()),
            model: model.to_string(),
            output_format: "text".to_string(),
            print_mode,
            continue_conversation: continue_conv,
            resume: None,
            allowed_tools: vec![],
            cwd: None,
        };

        let json = serde_json::to_string(&args).unwrap();
        let parsed: CapturedArgs = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(parsed.prompt, args.prompt);
        prop_assert_eq!(parsed.model, args.model);
        prop_assert_eq!(parsed.print_mode, args.print_mode);
        prop_assert_eq!(parsed.continue_conversation, args.continue_conversation);
    }

    #[test]
    fn duration_roundtrip(secs in 0u64..1000000, nanos in 0u32..1000000000) {
        let interaction = CapturedInteraction {
            seq: 0,
            timestamp: SystemTime::now(),
            elapsed: Duration::new(secs, nanos),
            args: make_args(None),
            outcome: CapturedOutcome::NoMatch { used_default: false },
        };

        let json = serde_json::to_string(&interaction).unwrap();
        let parsed: CapturedInteraction = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(parsed.elapsed.as_secs(), secs);
        prop_assert_eq!(parsed.elapsed.subsec_nanos(), nanos);
    }
}
