// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;
use crate::capture::interaction::{CapturedArgs, CapturedOutcome};
use proptest::prelude::*;
use rstest::rstest;
use std::thread;

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

fn make_response(text: &str) -> CapturedOutcome {
    CapturedOutcome::Response {
        text: text.to_string(),
        matched_rule: None,
        delay_ms: 0,
    }
}

#[test]
fn test_record_and_retrieve() {
    let log = CaptureLog::new();

    log.record(make_args(Some("hello")), make_response("Hi!"));

    assert_eq!(log.len(), 1);
    let interactions = log.interactions();
    assert_eq!(interactions[0].seq, 0);
    assert_eq!(interactions[0].args.prompt, Some("hello".to_string()));
}

#[rstest]
#[case(1, 1)]
#[case(5, 2)]
#[case(10, 5)]
#[case(3, 10)]
fn test_last_n(#[case] total: usize, #[case] n: usize) {
    let log = CaptureLog::new();

    for i in 0..total {
        log.record(
            make_args(Some(&format!("prompt {}", i))),
            make_response(&format!("response {}", i)),
        );
    }

    let last = log.last(n);
    let expected_len = n.min(total);
    assert_eq!(last.len(), expected_len);

    // Verify the last items are in order
    if expected_len > 0 {
        let start = total.saturating_sub(n);
        for (i, interaction) in last.iter().enumerate() {
            assert_eq!(
                interaction.args.prompt,
                Some(format!("prompt {}", start + i))
            );
        }
    }
}

#[test]
fn test_count() {
    let log = CaptureLog::new();

    log.record(make_args(Some("test")), make_response("ok"));
    log.record(
        make_args(Some("other")),
        CapturedOutcome::Failure {
            failure_type: "test".to_string(),
            message: "error".to_string(),
        },
    );
    log.record(make_args(Some("test again")), make_response("ok"));

    let response_count = log.count(|i| matches!(i.outcome, CapturedOutcome::Response { .. }));
    assert_eq!(response_count, 2);

    let failure_count = log.count(|i| matches!(i.outcome, CapturedOutcome::Failure { .. }));
    assert_eq!(failure_count, 1);
}

#[test]
fn test_find_by_prompt() {
    let log = CaptureLog::new();

    log.record(make_args(Some("hello world")), make_response("hi"));
    log.record(make_args(Some("goodbye")), make_response("bye"));
    log.record(make_args(Some("hello again")), make_response("hi again"));

    let matches = log.find_by_prompt("hello");
    assert_eq!(matches.len(), 2);
}

#[test]
fn test_find_by_prompt_no_match() {
    let log = CaptureLog::new();

    log.record(make_args(Some("hello")), make_response("hi"));

    let matches = log.find_by_prompt("nonexistent");
    assert!(matches.is_empty());
}

#[test]
fn test_find_by_prompt_none_prompt() {
    let log = CaptureLog::new();

    log.record(make_args(None), make_response("response"));
    log.record(make_args(Some("hello")), make_response("hi"));

    let matches = log.find_by_prompt("hello");
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_find_responses_and_failures() {
    let log = CaptureLog::new();

    log.record(make_args(Some("a")), make_response("ok"));
    log.record(
        make_args(Some("b")),
        CapturedOutcome::Failure {
            failure_type: "error".to_string(),
            message: "failed".to_string(),
        },
    );
    log.record(
        make_args(Some("c")),
        CapturedOutcome::NoMatch { used_default: true },
    );

    assert_eq!(log.find_responses().len(), 1);
    assert_eq!(log.find_failures().len(), 1);
}

#[test]
fn test_clear() {
    let log = CaptureLog::new();

    log.record(make_args(Some("test")), make_response("ok"));

    assert!(!log.is_empty());
    log.clear();
    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
}

#[test]
fn test_default() {
    let log = CaptureLog::default();
    assert!(log.is_empty());
}

#[test]
fn test_clone_shares_state() {
    let log1 = CaptureLog::new();
    let log2 = log1.clone();

    log1.record(make_args(Some("from log1")), make_response("ok"));

    // Both should see the same data
    assert_eq!(log1.len(), 1);
    assert_eq!(log2.len(), 1);

    log2.record(make_args(Some("from log2")), make_response("ok"));

    assert_eq!(log1.len(), 2);
    assert_eq!(log2.len(), 2);
}

#[test]
fn test_file_capture() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.jsonl");

    {
        let log = CaptureLog::with_file(&path).unwrap();
        log.record(make_args(Some("prompt1")), make_response("response1"));
        log.record(make_args(Some("prompt2")), make_response("response2"));
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

#[test]
fn test_file_capture_invalid_path() {
    let result = CaptureLog::with_file(std::path::Path::new("/nonexistent/dir/file.jsonl"));
    assert!(result.is_err());
}

#[test]
fn test_thread_safety() {
    let log = CaptureLog::new();
    let log_clone = log.clone();

    let handle = thread::spawn(move || {
        for i in 0..100 {
            log_clone.record(
                make_args(Some(&format!("thread1-{}", i))),
                make_response("ok"),
            );
        }
    });

    for i in 0..100 {
        log.record(make_args(Some(&format!("main-{}", i))), make_response("ok"));
    }

    handle.join().unwrap();

    assert_eq!(log.len(), 200);
}

#[test]
fn test_sequence_numbers() {
    let log = CaptureLog::new();

    for i in 0..5 {
        log.record(
            make_args(Some(&format!("prompt {}", i))),
            make_response("ok"),
        );
    }

    let interactions = log.interactions();
    for (i, interaction) in interactions.iter().enumerate() {
        assert_eq!(interaction.seq, i as u64);
    }
}

// Property-based tests
proptest! {
    #[test]
    fn len_equals_record_count(count in 0usize..100) {
        let log = CaptureLog::new();
        for _ in 0..count {
            log.record(make_args(None), make_response("ok"));
        }
        prop_assert_eq!(log.len(), count);
    }

    #[test]
    fn find_responses_consistent(
        response_count in 0usize..20,
        failure_count in 0usize..20,
    ) {
        let log = CaptureLog::new();

        for _ in 0..response_count {
            log.record(make_args(None), make_response("ok"));
        }

        for _ in 0..failure_count {
            log.record(
                make_args(None),
                CapturedOutcome::Failure {
                    failure_type: "test".to_string(),
                    message: "error".to_string(),
                },
            );
        }

        prop_assert_eq!(log.find_responses().len(), response_count);
        prop_assert_eq!(log.find_failures().len(), failure_count);
        prop_assert_eq!(log.len(), response_count + failure_count);
    }
}
