// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests validating simulator behavior matches real Claude.
//!
//! These tests verify the simulator produces output that would be accepted
//! by code expecting real Claude CLI responses.

use claudeless::cli::OutputFormat;
use claudeless::config::ResponseSpec;
use claudeless::output::{OutputWriter, ResultOutput};
use claudeless::validation::{
    AccuracyReport, CliAudit, FeatureCategory, ValidationItem, ValidationStatus,
};

// =============================================================================
// CLI Validation Tests
// =============================================================================

#[test]
fn test_all_needed_cli_flags_implemented() {
    let audit = CliAudit::new();
    let missing = audit.flags_with_status(&claudeless::validation::FlagStatus::MissingNeeded);

    assert!(
        missing.is_empty(),
        "Missing needed flags that should be implemented: {:?}",
        missing.iter().map(|f| f.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_cli_audit_generates_valid_report() {
    let audit = CliAudit::new();
    let md = audit.to_markdown();

    // Report should contain expected sections
    assert!(md.contains("# CLI Flag Audit"));
    assert!(md.contains("## Implemented"));

    // Should have implementation counts
    let counts = audit.count_by_status();
    assert!(counts["implemented"] > 10, "Expected >10 implemented flags");
}

// =============================================================================
// Output Format Validation Tests
// =============================================================================

#[test]
fn test_json_output_has_required_result_fields() {
    let required_fields = [
        "type",
        "subtype",
        "cost_usd",
        "is_error",
        "duration_ms",
        "duration_api_ms",
        "num_turns",
        "session_id",
    ];

    let result = ResultOutput::success("Hello!".to_string(), "session-123".to_string(), 1000);

    let json: serde_json::Value = serde_json::to_value(&result).unwrap();

    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Missing required field: {}",
            field
        );
    }
}

#[test]
fn test_json_error_has_required_fields() {
    let required_fields = [
        "type",
        "subtype",
        "cost_usd",
        "is_error",
        "duration_ms",
        "duration_api_ms",
        "num_turns",
        "error",
    ];

    let result = ResultOutput::error(
        "Something failed".to_string(),
        "session-123".to_string(),
        100,
    );

    let json: serde_json::Value = serde_json::to_value(&result).unwrap();

    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Missing required field: {}",
            field
        );
    }
}

#[test]
fn test_stream_json_produces_valid_ndjson() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Hello, world!".to_string());
    writer
        .write_real_response(&response, "session-123", vec!["Bash".to_string()])
        .unwrap();

    let output = String::from_utf8(buf).unwrap();

    // Verify each line is valid JSON
    for line in output.lines() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "Invalid JSON line: {} - Error: {:?}",
            line,
            parsed.err()
        );
    }
}

#[test]
fn test_stream_json_has_system_init() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Hello!".to_string());
    writer
        .write_real_response(&response, "session-123", vec!["Bash".to_string()])
        .unwrap();

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["type"], "system");
    assert_eq!(first["subtype"], "init");
}

#[test]
fn test_stream_json_ends_with_result() {
    let mut buf = Vec::new();
    let mut writer = OutputWriter::new(
        &mut buf,
        OutputFormat::StreamJson,
        "claude-test".to_string(),
    );

    let response = ResponseSpec::Simple("Hello!".to_string());
    writer
        .write_real_response(&response, "session-123", vec![])
        .unwrap();

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    let last: serde_json::Value = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert_eq!(last["type"], "result");
}

// =============================================================================
// Accuracy Report Validation Tests
// =============================================================================

#[test]
fn test_accuracy_report_covers_all_categories() {
    let mut report = AccuracyReport::new()
        .with_date("2025-01-18")
        .with_claude_version("1.0.0");

    // Add CLI flags
    let audit = CliAudit::new();
    report.add_cli_flags(&audit);

    let by_category = report.items_by_category();
    assert!(
        by_category.contains_key(&FeatureCategory::CliFlags),
        "Report should contain CLI Flags category"
    );
}

#[test]
fn test_accuracy_report_generates_valid_markdown() {
    let mut report = AccuracyReport::new()
        .with_date("2025-01-18")
        .with_claude_version("1.0.0");

    let audit = CliAudit::new();
    report.add_cli_flags(&audit);

    let md = report.to_markdown();

    assert!(md.contains("# Claudelessulator Accuracy Report"));
    assert!(md.contains("Last validated: 2025-01-18"));
    assert!(md.contains("Claude Code version: 1.0.0"));
    assert!(md.contains("## Summary"));
    assert!(md.contains("## CLI Flags"));
}

#[test]
fn test_accuracy_report_counts_match_items() {
    let mut report = AccuracyReport::new();

    report.add_item(ValidationItem {
        name: "test1".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: None,
    });
    report.add_item(ValidationItem {
        name: "test2".to_string(),
        category: FeatureCategory::CliFlags,
        status: ValidationStatus::Match,
        notes: None,
    });
    report.add_item(ValidationItem {
        name: "test3".to_string(),
        category: FeatureCategory::OutputFormats,
        status: ValidationStatus::Partial("note".to_string()),
        notes: None,
    });

    let counts = report.count_by_status();
    assert_eq!(counts["match"], 2);
    assert_eq!(counts["partial"], 1);
}

// =============================================================================
// State Directory Validation Tests
// =============================================================================

#[test]
fn test_state_directory_structure_matches_real_claude() {
    use claudeless::state::StateDirectory;

    let mut dir = StateDirectory::temp().unwrap();
    dir.initialize().unwrap();

    // Real ~/.claude has these directories
    assert!(dir.projects_dir().exists());
    assert!(dir.todos_dir().exists());
    assert!(dir.settings_path().exists());

    // Validate structure
    let warnings = dir.validate_structure().unwrap();
    assert!(
        warnings.is_empty(),
        "Structure validation warnings: {:?}",
        warnings
    );
}

// =============================================================================
// Hook Protocol Validation Tests
// =============================================================================

#[test]
fn test_hook_message_matches_spec() {
    use claudeless::hooks::{HookEvent, HookMessage};

    let msg = HookMessage::tool_execution(
        "session-123",
        HookEvent::PreToolExecution,
        "Bash",
        serde_json::json!({"command": "ls -la"}),
        None,
        None,
    );

    let json = serde_json::to_value(&msg).unwrap();

    // Verify required fields
    assert!(json.get("event").is_some());
    assert!(json.get("session_id").is_some());
    assert!(json.get("payload").is_some());

    // Verify payload structure
    assert_eq!(json["payload"]["tool_name"], "Bash");
    assert!(json["payload"]["tool_input"].is_object());
}

#[test]
fn test_hook_response_parsing_is_lenient() {
    use claudeless::hooks::HookResponse;

    // Real Claude accepts various response formats
    let test_cases = [
        r#"{"proceed": true}"#,
        r#"{"proceed": false, "error": "blocked"}"#,
        r#"{"proceed": true, "modified_payload": {}}"#,
        r#"{"proceed": true, "data": {"custom": "field"}}"#,
        r#"{}"#, // Should default to proceed: true
    ];

    for json in test_cases {
        let result: Result<HookResponse, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Failed to parse: {}", json);
    }
}

// =============================================================================
// Error Format Validation Tests
// =============================================================================

#[test]
fn test_error_result_is_parseable_as_success_result() {
    // Error and success results should have the same base structure
    let success: serde_json::Value = serde_json::to_value(ResultOutput::success(
        "ok".to_string(),
        "s1".to_string(),
        100,
    ))
    .unwrap();

    let error: serde_json::Value = serde_json::to_value(ResultOutput::error(
        "fail".to_string(),
        "s2".to_string(),
        100,
    ))
    .unwrap();

    // Both should have these fields
    for field in ["type", "subtype", "cost_usd", "is_error", "duration_ms"] {
        assert!(success.get(field).is_some());
        assert!(error.get(field).is_some());
    }
}

#[test]
fn test_exit_codes_are_correct() {
    use claudeless::failure::exit_codes;

    // Document expected exit codes
    assert_eq!(exit_codes::SUCCESS, 0);
    assert_eq!(exit_codes::ERROR, 1);
    assert_eq!(exit_codes::PARTIAL, 2);
    assert_eq!(exit_codes::INTERRUPTED, 130);
}
