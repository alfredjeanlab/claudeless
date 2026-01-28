// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn deserialize_minimal_spec() {
    let toml = r#"
        name = "minimal"
    "#;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.name, "minimal");
    assert_eq!(spec.capture_type, CaptureType::Tui);
    assert_eq!(spec.timeout_ms, 30_000);
}

#[test]
fn deserialize_full_spec() {
    let toml = r#"
        name = "full-capture"
        claude_version = "2.1.12"
        capture_type = "tui"
        retry_count = 3
        timeout_ms = 60000

        [[key_sequences]]
        name = "type hello"
        keys = ["h", "e", "l", "l", "o", "Enter"]
        delay_ms = 100

        [[expected_states]]
        name = "check response"
        after_sequence = 0

        [[expected_states.conditions]]
        type = "text_visible"
        pattern = "Hello"

        [[normalization_rules]]
        type = "strip_ansi"

        [[normalization_rules]]
        type = "normalize_uuids"
    "#;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.name, "full-capture");
    assert_eq!(spec.retry_count, 3);
    assert_eq!(spec.key_sequences.len(), 1);
    assert_eq!(spec.normalization_rules.len(), 2);
}

#[test]
fn deserialize_all_capture_types() {
    let tui: CaptureSpec = toml::from_str(r#"capture_type = "tui""#).unwrap();
    assert_eq!(tui.capture_type, CaptureType::Tui);

    let cli: CaptureSpec = toml::from_str(r#"capture_type = "cli""#).unwrap();
    assert_eq!(cli.capture_type, CaptureType::Cli);

    let dot_claude: CaptureSpec = toml::from_str(r#"capture_type = "dot_claude""#).unwrap();
    assert_eq!(dot_claude.capture_type, CaptureType::DotClaude);
}

#[test]
fn deserialize_state_conditions() {
    let toml = r##"
        [[expected_states]]
        name = "test conditions"
        [[expected_states.conditions]]
        type = "prompt_ready"
        [[expected_states.conditions]]
        type = "response_complete"
        [[expected_states.conditions]]
        type = "text_visible"
        pattern = "hello"
        [[expected_states.conditions]]
        type = "element_visible"
        selector = "#main"
    "##;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.expected_states.len(), 1);
    assert_eq!(spec.expected_states[0].conditions.len(), 4);
}

#[test]
fn deserialize_all_normalization_rules() {
    let toml = r##"
        [[normalization_rules]]
        type = "replace"
        pattern = "foo"
        replacement = "bar"

        [[normalization_rules]]
        type = "replace"
        pattern = "FOO"
        replacement = "bar"
        flags = "i"

        [[normalization_rules]]
        type = "remove_lines"
        pattern = "^#"

        [[normalization_rules]]
        type = "strip_ansi"

        [[normalization_rules]]
        type = "normalize_timestamps"

        [[normalization_rules]]
        type = "normalize_timestamps"
        format = "iso8601"

        [[normalization_rules]]
        type = "normalize_uuids"

        [[normalization_rules]]
        type = "normalize_paths"

        [[normalization_rules]]
        type = "normalize_paths"
        base = "/home/user/project"
    "##;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.normalization_rules.len(), 9);
}

#[test]
fn validate_empty_keys_fails() {
    let spec = CaptureSpec {
        key_sequences: vec![KeySequence {
            name: None,
            keys: vec![], // Empty!
            delay_ms: None,
            wait_for: None,
        }],
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(err, CaptureSpecError::InvalidKeySequence { .. }));
}

#[test]
fn validate_zero_timeout_fails() {
    let spec = CaptureSpec {
        timeout_ms: 0,
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(err, CaptureSpecError::InvalidTimeout(0)));
}

#[test]
fn validate_invalid_regex_fails() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::Replace {
            pattern: "[invalid".to_string(),
            replacement: "x".to_string(),
            flags: None,
        }],
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(err, CaptureSpecError::InvalidRegex { .. }));
}

#[test]
fn validate_invalid_remove_lines_regex_fails() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::RemoveLines {
            pattern: "(unclosed".to_string(),
        }],
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(err, CaptureSpecError::InvalidRegex { .. }));
}

#[test]
fn validate_invalid_sequence_reference_fails() {
    let spec = CaptureSpec {
        key_sequences: vec![KeySequence {
            name: Some("test".to_string()),
            keys: vec!["a".to_string()],
            delay_ms: None,
            wait_for: None,
        }],
        expected_states: vec![ExpectedState {
            name: None,
            after_sequence: Some(5), // References sequence 5, but only 1 exists
            conditions: vec![],
        }],
        ..Default::default()
    };
    let err = spec.validate().unwrap_err();
    assert!(matches!(
        err,
        CaptureSpecError::InvalidSequenceReference { index: 5, max: 0 }
    ));
}

#[test]
fn validate_valid_spec_succeeds() {
    let spec = CaptureSpec {
        name: "valid".to_string(),
        key_sequences: vec![
            KeySequence {
                name: Some("type hello".to_string()),
                keys: vec!["h".to_string(), "e".to_string(), "l".to_string()],
                delay_ms: Some(50),
                wait_for: None,
            },
            KeySequence {
                name: None,
                keys: vec!["Enter".to_string()],
                delay_ms: None,
                wait_for: Some(StateCondition::PromptReady),
            },
        ],
        expected_states: vec![ExpectedState {
            name: Some("check".to_string()),
            after_sequence: Some(1),
            conditions: vec![StateCondition::TextVisible {
                pattern: "test".to_string(),
            }],
        }],
        normalization_rules: vec![
            NormalizationRule::StripAnsi,
            NormalizationRule::Replace {
                pattern: r"\d+".to_string(),
                replacement: "[NUM]".to_string(),
                flags: None,
            },
        ],
        ..Default::default()
    };
    assert!(spec.validate().is_ok());
}

#[test]
fn normalize_strips_ansi() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::StripAnsi],
        ..Default::default()
    };
    let input = "\x1b[31mred\x1b[0m text";
    assert_eq!(spec.normalize(input), "red text");
}

#[test]
fn normalize_replaces_uuids() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::NormalizeUuids],
        ..Default::default()
    };
    let input = "session: a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    assert_eq!(spec.normalize(input), "session: [UUID]");
}

#[test]
fn normalize_replaces_timestamps() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::NormalizeTimestamps { format: None }],
        ..Default::default()
    };
    let input = "created at 2025-01-15T10:30:00Z";
    assert_eq!(spec.normalize(input), "created at [TIMESTAMP]");

    let input2 = "modified 2025-01-15 10:30:00";
    assert_eq!(spec.normalize(input2), "modified [TIMESTAMP]");
}

#[test]
fn normalize_removes_lines() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::RemoveLines {
            pattern: "^#".to_string(),
        }],
        ..Default::default()
    };
    let input = "# comment\ncode\n# another comment\nmore code";
    assert_eq!(spec.normalize(input), "code\nmore code");
}

#[test]
fn normalize_replaces_pattern() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::Replace {
            pattern: r"\d+".to_string(),
            replacement: "[NUM]".to_string(),
            flags: None,
        }],
        ..Default::default()
    };
    let input = "line 123, col 45";
    assert_eq!(spec.normalize(input), "line [NUM], col [NUM]");
}

#[test]
fn normalize_replaces_case_insensitive() {
    let spec = CaptureSpec {
        normalization_rules: vec![NormalizationRule::Replace {
            pattern: "error".to_string(),
            replacement: "[ERROR]".to_string(),
            flags: Some("i".to_string()),
        }],
        ..Default::default()
    };
    let input = "Error: ERROR in error handler";
    assert_eq!(spec.normalize(input), "[ERROR]: [ERROR] in [ERROR] handler");
}

#[test]
fn normalize_chains_rules() {
    let spec = CaptureSpec {
        normalization_rules: vec![
            NormalizationRule::StripAnsi,
            NormalizationRule::NormalizeUuids,
            NormalizationRule::Replace {
                pattern: r"\d+".to_string(),
                replacement: "[N]".to_string(),
                flags: None,
            },
        ],
        ..Default::default()
    };
    let input = "\x1b[32msession\x1b[0m: a1b2c3d4-e5f6-7890-abcd-ef1234567890, count: 42";
    assert_eq!(spec.normalize(input), "session: [UUID], count: [N]");
}

#[test]
fn serialize_roundtrip() {
    let spec = CaptureSpec {
        name: "roundtrip-test".to_string(),
        capture_type: CaptureType::Cli,
        key_sequences: vec![KeySequence {
            name: Some("test".to_string()),
            keys: vec!["a".to_string(), "Enter".to_string()],
            delay_ms: Some(100),
            wait_for: Some(StateCondition::PromptReady),
        }],
        normalization_rules: vec![NormalizationRule::StripAnsi],
        timeout_ms: 5000,
        retry_count: 2,
        ..Default::default()
    };

    let serialized = toml::to_string(&spec).unwrap();
    let deserialized: CaptureSpec = toml::from_str(&serialized).unwrap();

    assert_eq!(deserialized.name, spec.name);
    assert_eq!(deserialized.capture_type, spec.capture_type);
    assert_eq!(deserialized.key_sequences.len(), spec.key_sequences.len());
    assert_eq!(deserialized.timeout_ms, spec.timeout_ms);
    assert_eq!(deserialized.retry_count, spec.retry_count);
}

#[test]
fn default_capture_spec() {
    let spec = CaptureSpec::default();
    assert_eq!(spec.name, "");
    assert_eq!(spec.capture_type, CaptureType::Tui);
    assert_eq!(spec.timeout_ms, 30_000);
    assert_eq!(spec.retry_count, 0);
    assert!(spec.key_sequences.is_empty());
    assert!(spec.expected_states.is_empty());
    assert!(spec.normalization_rules.is_empty());
}

#[test]
fn key_sequence_with_wait_for() {
    let toml = r#"
        [[key_sequences]]
        name = "wait for prompt"
        keys = ["Enter"]

        [key_sequences.wait_for]
        type = "text_visible"
        pattern = "ready"
    "#;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.key_sequences.len(), 1);
    assert!(spec.key_sequences[0].wait_for.is_some());
    if let Some(StateCondition::TextVisible { pattern }) = &spec.key_sequences[0].wait_for {
        assert_eq!(pattern, "ready");
    } else {
        unreachable!("Expected TextVisible condition");
    }
}

#[test]
fn metadata_field() {
    let toml = r#"
        name = "with-metadata"
        [metadata]
        author = "test"
        version = 1
        features = ["a", "b"]
    "#;
    let spec: CaptureSpec = toml::from_str(toml).unwrap();
    assert_eq!(spec.metadata.get("author").unwrap(), "test");
    assert_eq!(spec.metadata.get("version").unwrap(), 1);
}
