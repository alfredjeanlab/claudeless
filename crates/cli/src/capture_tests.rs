// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

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
fn test_record_and_retrieve() {
    let log = CaptureLog::new();

    log.record(
        make_args(Some("hello")),
        CapturedOutcome::Response {
            text: "Hi!".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );

    assert_eq!(log.len(), 1);
    let interactions = log.interactions();
    assert_eq!(interactions[0].seq, 0);
    assert_eq!(interactions[0].args.prompt, Some("hello".to_string()));
}

#[test]
fn test_last_n() {
    let log = CaptureLog::new();

    for i in 0..5 {
        log.record(
            make_args(Some(&format!("prompt {}", i))),
            CapturedOutcome::Response {
                text: format!("response {}", i),
                matched_rule: None,
                delay_ms: 0,
            },
        );
    }

    let last2 = log.last(2);
    assert_eq!(last2.len(), 2);
    assert_eq!(last2[0].args.prompt, Some("prompt 3".to_string()));
    assert_eq!(last2[1].args.prompt, Some("prompt 4".to_string()));
}

#[test]
fn test_count() {
    let log = CaptureLog::new();

    log.record(
        make_args(Some("test")),
        CapturedOutcome::Response {
            text: "ok".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );
    log.record(
        make_args(Some("other")),
        CapturedOutcome::Failure {
            failure_type: "test".to_string(),
            message: "error".to_string(),
        },
    );
    log.record(
        make_args(Some("test again")),
        CapturedOutcome::Response {
            text: "ok".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );

    let response_count = log.count(|i| matches!(i.outcome, CapturedOutcome::Response { .. }));
    assert_eq!(response_count, 2);

    let failure_count = log.count(|i| matches!(i.outcome, CapturedOutcome::Failure { .. }));
    assert_eq!(failure_count, 1);
}

#[test]
fn test_find_by_prompt() {
    let log = CaptureLog::new();

    log.record(
        make_args(Some("hello world")),
        CapturedOutcome::Response {
            text: "hi".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );
    log.record(
        make_args(Some("goodbye")),
        CapturedOutcome::Response {
            text: "bye".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );
    log.record(
        make_args(Some("hello again")),
        CapturedOutcome::Response {
            text: "hi again".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );

    let matches = log.find_by_prompt("hello");
    assert_eq!(matches.len(), 2);
}

#[test]
fn test_find_responses_and_failures() {
    let log = CaptureLog::new();

    log.record(
        make_args(Some("a")),
        CapturedOutcome::Response {
            text: "ok".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );
    log.record(
        make_args(Some("b")),
        CapturedOutcome::Failure {
            failure_type: "error".to_string(),
            message: "failed".to_string(),
        },
    );

    assert_eq!(log.find_responses().len(), 1);
    assert_eq!(log.find_failures().len(), 1);
}

#[test]
fn test_clear() {
    let log = CaptureLog::new();

    log.record(
        make_args(Some("test")),
        CapturedOutcome::Response {
            text: "ok".to_string(),
            matched_rule: None,
            delay_ms: 0,
        },
    );

    assert!(!log.is_empty());
    log.clear();
    assert!(log.is_empty());
}

#[test]
fn test_serialization() {
    let interaction = CapturedInteraction {
        seq: 0,
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

    assert_eq!(parsed.seq, 0);
    assert_eq!(parsed.args.prompt, Some("test".to_string()));
}

#[test]
fn test_file_capture() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.jsonl");

    {
        let log = CaptureLog::with_file(&path).unwrap();
        log.record(
            make_args(Some("prompt1")),
            CapturedOutcome::Response {
                text: "response1".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );
        log.record(
            make_args(Some("prompt2")),
            CapturedOutcome::Response {
                text: "response2".to_string(),
                matched_rule: None,
                delay_ms: 0,
            },
        );
    }

    // Read back the file
    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    // Parse each line
    let i1: CapturedInteraction = serde_json::from_str(lines[0]).unwrap();
    let i2: CapturedInteraction = serde_json::from_str(lines[1]).unwrap();

    assert_eq!(i1.args.prompt, Some("prompt1".to_string()));
    assert_eq!(i2.args.prompt, Some("prompt2".to_string()));
}
