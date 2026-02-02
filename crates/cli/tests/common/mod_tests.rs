// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
