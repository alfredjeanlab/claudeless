#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Unit Tests for Fixture Comparison Helpers
use super::*;

#[test]
fn test_normalize_removes_timestamps() {
    let input = "Last updated at 14:30:45";
    let normalized = normalize_tui(input, None);
    assert!(normalized.contains("<TIME>"));
    assert!(!normalized.contains("14:30:45"));
}

#[test]
fn test_normalize_removes_session_ids() {
    let input = "Session: a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    let normalized = normalize_tui(input, None);
    assert!(normalized.contains("<SESSION>"));
    assert!(!normalized.contains("a1b2c3d4"));
}

#[test]
fn test_normalize_removes_temp_dirs() {
    let input = "/private/var/folders/ab/cd123/T/test.txt";
    let normalized = normalize_tui(input, None);
    assert!(normalized.contains("<PATH>"));
    assert!(!normalized.contains("/private/var/folders"));
}

#[test]
fn test_normalize_strips_trailing_whitespace() {
    let input = "line1   \nline2\t\nline3";
    let normalized = normalize_tui(input, None);
    assert_eq!(normalized, "line1\nline2\nline3");
}

#[test]
fn test_normalize_preserves_leading_whitespace() {
    // Leading whitespace within lines is preserved, but leading/trailing empty lines are trimmed
    let input = "  indented\n    more indented";
    let normalized = normalize_tui(input, None);
    assert!(normalized.starts_with("  "));
    assert!(normalized.contains("    more"));
}

#[test]
fn test_normalize_trims_empty_lines() {
    let input = "\n\n  content\n  more content\n\n";
    let normalized = normalize_tui(input, None);
    // Leading and trailing empty lines are trimmed
    assert!(normalized.starts_with("  content"));
    assert!(normalized.ends_with("more content"));
}

#[test]
fn test_normalize_replaces_cwd() {
    let input = "Working in /home/user/project";
    let normalized = normalize_tui(input, Some("/home/user/project"));
    assert!(normalized.contains("<PATH>"));
    assert!(!normalized.contains("/home/user/project"));
}

#[test]
fn test_load_fixture_trust_prompt() {
    let fixture = load_fixture("trust_prompt.txt");
    assert!(fixture.contains("trust"));
    assert!(fixture.contains("files"));
}

#[test]
fn test_load_fixture_initial_state() {
    let fixture = load_fixture("initial_state.txt");
    // Initial state should contain some TUI elements
    assert!(!fixture.is_empty());
}

#[test]
fn test_all_fixtures_loadable() {
    // Verify all documented fixtures can be loaded
    let fixtures = [
        // Initial and basic states
        "initial_state.txt",
        "with_input.txt",
        "after_response.txt",
        // Model variants
        "model_haiku.txt",
        "model_sonnet.txt",
        "model_opus.txt",
        // Trust prompt
        "trust_prompt.txt",
        // Permission modes
        "permission_default.txt",
        "permission_plan.txt",
        "permission_accept_edits.txt",
        "permission_bypass.txt",
        "permission_bash_command.txt",
        "permission_edit_file.txt",
        "permission_write_file.txt",
        "permission_trust_folder.txt",
        // Thinking dialog
        "thinking_dialog.txt",
        "thinking_dialog_enabled_selected.txt",
        "thinking_dialog_disabled_selected.txt",
        "thinking_dialog_mid_conversation.txt",
        "thinking_off_status.txt",
        // Compacting
        "compact_before.txt",
        "compact_during.txt",
        "compact_after.txt",
        // Clear
        "clear_before.txt",
        "clear_after.txt",
        // Status bar
        "status_bar_extended.txt",
    ];

    for fixture in &fixtures {
        let content = load_fixture(fixture);
        assert!(
            !content.is_empty(),
            "Fixture {} should not be empty",
            fixture
        );
    }
}
