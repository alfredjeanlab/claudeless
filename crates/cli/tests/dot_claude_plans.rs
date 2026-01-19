// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests verifying claudeless produces ~/.claude/plans/ output
//! matching real Claude CLI (v2.1.12).
//!
//! ## Real Claude plans/ format
//! - Directory: `~/.claude/plans/`
//! - File naming: `{adjective}-{verb}-{noun}.md` (e.g., `velvety-crunching-ocean.md`)
//! - Content: Standard markdown with plan structure
//!
//! Plans are created when Claude is in plan mode (`--permission-mode plan`)
//! and uses the ExitPlanMode tool.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Create a scenario that triggers plan mode and plan creation
fn create_plan_scenario(dir: &TempDir) -> PathBuf {
    let scenario_path = dir.path().join("scenario.json");
    let scenario = serde_json::json!({
        "default_response": {
            "text": "I'll create a plan for you.",
            "tool_calls": [
                {
                    "tool": "ExitPlanMode",
                    "input": {
                        "plan_content": "# Test Plan\n\n## Overview\n\nThis is a test plan for the feature.\n\n## Steps\n\n1. First step\n2. Second step\n3. Third step"
                    }
                }
            ]
        },
        "permission_mode": "plan",
        "tool_execution": {
            "mode": "simulated",
            "tools": {
                "ExitPlanMode": {
                    "auto_approve": true
                }
            }
        }
    });
    std::fs::write(
        &scenario_path,
        serde_json::to_string_pretty(&scenario).unwrap(),
    )
    .unwrap();
    scenario_path
}

// =============================================================================
// Plans Directory Tests
// =============================================================================

/// Verify plans directory is created
#[test]
fn test_plans_directory_created() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_plan_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Create a plan for adding a greeting feature",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success(), "claudeless failed: {:?}", output);

    let plans_dir = state_dir.path().join("plans");
    assert!(plans_dir.exists(), "plans/ directory should exist");
}

/// Verify plan file naming convention: {adjective}-{verb}-{noun}.md
#[test]
fn test_plan_file_naming_convention() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_plan_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Plan a feature",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    let plans_dir = state_dir.path().join("plans");
    let plan_files: Vec<_> = std::fs::read_dir(&plans_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();

    assert!(!plan_files.is_empty(), "Should have at least one plan file");

    let filename = plan_files[0].file_name().to_string_lossy().to_string();

    // Format: {word}-{word}-{word}.md
    assert!(
        filename.ends_with(".md"),
        "Plan file should end with .md: {}",
        filename
    );

    let name_without_ext = filename.trim_end_matches(".md");
    let parts: Vec<&str> = name_without_ext.split('-').collect();

    assert_eq!(
        parts.len(),
        3,
        "Plan name should have 3 parts (adjective-verb-noun): {}",
        filename
    );

    // Each part should be lowercase alphabetic
    for part in &parts {
        assert!(
            part.chars().all(|c| c.is_ascii_lowercase()),
            "Plan name parts should be lowercase: {} in {}",
            part,
            filename
        );
    }
}

/// Verify plan file is markdown format
#[test]
fn test_plan_file_is_markdown() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_plan_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Plan something",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    let plans_dir = state_dir.path().join("plans");
    let plan_file = std::fs::read_dir(&plans_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .expect("Should have a plan file");

    let content = std::fs::read_to_string(plan_file.path()).unwrap();

    // Should be non-empty markdown
    assert!(!content.is_empty(), "Plan file should not be empty");

    // Should have markdown heading
    assert!(
        content.contains('#'),
        "Plan should contain markdown headings"
    );
}

/// Verify plan file has expected structure
#[test]
fn test_plan_file_structure() {
    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let scenario = create_plan_scenario(&state_dir);

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario.to_str().unwrap(),
            "-p",
            "Plan a feature implementation",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(output.status.success());

    let plans_dir = state_dir.path().join("plans");
    let plan_file = std::fs::read_dir(&plans_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .expect("Should have a plan file");

    let content = std::fs::read_to_string(plan_file.path()).unwrap();

    // Real Claude plans typically have these sections
    // (we don't require all, but at least a title)
    assert!(
        content.starts_with('#') || content.contains("\n#"),
        "Plan should start with or contain a markdown heading"
    );
}

/// Compare plan.md content against captured fixture
/// Since the scenario uses $file reference to load plan.md, content should match exactly
#[test]
fn test_plan_md_matches_fixture_content() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dotclaude/v2.1.12/plan-mode");
    let fixture_path = fixture_dir.join("plan.md");
    let scenario_path = fixture_dir.join("scenario.toml");

    if !fixture_path.exists() {
        panic!(
            "Fixture not found: {:?}\n\
             Run `./scripts/capture-state.sh` to capture fixtures from real Claude CLI",
            fixture_path
        );
    }

    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario_path.to_str().unwrap(),
            "-p",
            // Same prompt used by capture-state.sh
            "Plan a simple feature to add user authentication. Write the plan and exit.",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(
        output.status.success(),
        "claudeless failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read simulator output
    let plans_dir = state_dir.path().join("plans");
    let plan_file = std::fs::read_dir(&plans_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .expect("Should have a plan file");

    let actual_content = std::fs::read_to_string(plan_file.path()).unwrap();

    // Read fixture
    let expected_content = std::fs::read_to_string(&fixture_path).unwrap();

    // Since scenario loads plan.md via $file reference, content should match exactly
    assert_eq!(
        actual_content, expected_content,
        "Plan content should match fixture exactly"
    );
}

/// Normalize JSON for comparison by replacing variable fields with placeholders
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            // UUID pattern
            if s.len() == 36
                && s.chars().enumerate().all(|(i, c)| match i {
                    8 | 13 | 18 | 23 => c == '-',
                    _ => c.is_ascii_hexdigit(),
                })
            {
                return serde_json::Value::String("<UUID>".to_string());
            }
            // ISO timestamp pattern
            if s.len() >= 19 && s.chars().nth(4) == Some('-') && s.chars().nth(10) == Some('T') {
                return serde_json::Value::String("<TIMESTAMP>".to_string());
            }
            // Temp path patterns
            if s.starts_with("/tmp/")
                || s.starts_with("/var/folders/")
                || s.starts_with("/private/var/folders/")
            {
                return serde_json::Value::String("<TEMP_PATH>".to_string());
            }
            // Message ID pattern
            if s.starts_with("msg_") {
                return serde_json::Value::String("<MESSAGE_ID>".to_string());
            }
            // Tool use ID pattern
            if s.starts_with("toolu_") {
                return serde_json::Value::String("<TOOL_USE_ID>".to_string());
            }
            // Request ID pattern
            if s.starts_with("req_") {
                return serde_json::Value::String("<REQUEST_ID>".to_string());
            }
            serde_json::Value::String(s.clone())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_json).collect())
        }
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Validate message sequence order matches fixture
fn validate_message_sequence(actual: &[serde_json::Value], expected: &[serde_json::Value]) {
    let actual_types: Vec<_> = actual.iter().filter_map(|m| m["type"].as_str()).collect();
    let expected_types: Vec<_> = expected.iter().filter_map(|m| m["type"].as_str()).collect();

    assert_eq!(
        actual_types, expected_types,
        "Message sequence mismatch.\nActual: {:?}\nExpected: {:?}",
        actual_types, expected_types
    );
}

/// Compare session.jsonl against captured fixture for plan-mode
#[test]
fn test_plan_mode_session_jsonl_matches_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dotclaude/v2.1.12/plan-mode");
    let fixture_path = fixture_dir.join("session.jsonl");
    let scenario_path = fixture_dir.join("scenario.toml");

    if !fixture_path.exists() {
        panic!(
            "Fixture not found: {:?}\n\
             Run `./scripts/capture-state.sh` to capture fixtures from real Claude CLI",
            fixture_path
        );
    }

    let state_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    let output = Command::new(claudeless_bin())
        .env("CLAUDELESS_STATE_DIR", state_dir.path())
        .current_dir(work_dir.path())
        .args([
            "--scenario",
            scenario_path.to_str().unwrap(),
            "-p",
            "Plan a simple feature to add user authentication. Write the plan and exit.",
        ])
        .output()
        .expect("Failed to run claudeless");

    assert!(
        output.status.success(),
        "claudeless failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read simulator output
    let projects_dir = state_dir.path().join("projects");
    let project_dir = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .expect("Should have project directory");

    let session_file = std::fs::read_dir(project_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .expect("Should have session.jsonl file");

    let actual_content = std::fs::read_to_string(session_file.path()).unwrap();
    let actual_lines: Vec<serde_json::Value> = actual_content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("Invalid JSON in session"))
        .collect();

    // Read fixture
    let expected_content = std::fs::read_to_string(&fixture_path).unwrap();
    let expected_lines: Vec<serde_json::Value> = expected_content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("Invalid JSON in fixture"))
        .collect();

    // Normalize both for comparison
    let actual_normalized: Vec<_> = actual_lines.iter().map(normalize_json).collect();
    let expected_normalized: Vec<_> = expected_lines.iter().map(normalize_json).collect();

    // Validate message sequence (types in order)
    validate_message_sequence(&actual_normalized, &expected_normalized);

    // Validate message count
    assert_eq!(
        actual_normalized.len(),
        expected_normalized.len(),
        "Message count mismatch. Actual: {}, Expected: {}",
        actual_normalized.len(),
        expected_normalized.len()
    );
}
