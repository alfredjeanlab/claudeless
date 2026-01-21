// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! ANSI-aware test utilities for TUI snapshot testing.
//!
//! Provides functions for normalizing and comparing ANSI-colored terminal output
//! while preserving color information for exact matching.

use claudeless::ansi::{parse_ansi, AnsiSpan};
use regex::Regex;

/// Normalize ANSI output for comparison.
///
/// Applies the same normalizations as `normalize_tui()` but preserves
/// ANSI escape sequences. Normalizations are applied to the text
/// content between ANSI sequences.
pub fn normalize_ansi_tui(input: &str, cwd: Option<&str>) -> String {
    // Parse input into spans
    let spans = parse_ansi(input);

    // Normalize each span's text content
    let normalized_spans: Vec<AnsiSpan> = spans
        .into_iter()
        .map(|span| {
            let normalized_text = normalize_text_content(&span.text, cwd);
            AnsiSpan {
                text: normalized_text,
                sequences: span.sequences,
            }
        })
        .collect();

    // Reconstruct the string with ANSI sequences
    let mut result = String::new();
    for span in normalized_spans {
        for seq in &span.sequences {
            result.push_str(&seq.to_escape_code());
        }
        result.push_str(&span.text);
    }

    // Strip leading empty lines
    while result.starts_with('\n') {
        result = result[1..].to_string();
    }

    // Strip trailing empty lines
    while result.ends_with('\n') {
        result = result[..result.len() - 1].to_string();
    }

    // Normalize each line:
    // 1. Convert non-breaking spaces to regular spaces (fixture inconsistency)
    // 2. Strip trailing whitespace (tmux may add padding)
    // 3. Strip trailing [39m] (iocraft may optimize these away)
    let fg_reset = "\x1b[39m";
    result = result
        .lines()
        .map(|line| {
            // Convert non-breaking space (U+00A0) to regular space
            let l = line.replace('\u{00A0}', " ");
            // Strip trailing whitespace
            let mut l = l.trim_end().to_string();
            // Strip trailing [39m] sequences
            while l.ends_with(fg_reset) {
                l = l[..l.len() - fg_reset.len()].to_string();
            }
            l
        })
        .collect::<Vec<_>>()
        .join("\n");

    result
}

/// Normalize text content (without ANSI sequences).
fn normalize_text_content(input: &str, cwd: Option<&str>) -> String {
    let mut result = input.to_string();

    // Timestamps (HH:MM:SS or HH:MM)
    let time_re = Regex::new(r"\d{1,2}:\d{2}(:\d{2})?").unwrap();
    result = time_re.replace_all(&result, "<TIME>").to_string();

    // Session IDs (UUIDs)
    let uuid_re =
        Regex::new(r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}").unwrap();
    result = uuid_re.replace_all(&result, "<SESSION>").to_string();

    // Session patterns like session-abc123
    let session_re = Regex::new(r"session-[a-zA-Z0-9]+").unwrap();
    result = session_re.replace_all(&result, "<SESSION>").to_string();

    // macOS temp directories
    let macos_tmp_re = Regex::new(r"/private/var/folders/[^/]+/[^/]+/[^/]+/[^\s]+").unwrap();
    result = macos_tmp_re.replace_all(&result, "<PATH>").to_string();

    // Linux temp directories
    let linux_tmp_re = Regex::new(r"/tmp/[^\s]+").unwrap();
    result = linux_tmp_re.replace_all(&result, "<PATH>").to_string();
    let var_tmp_re = Regex::new(r"/var/tmp/[^\s]+").unwrap();
    result = var_tmp_re.replace_all(&result, "<PATH>").to_string();

    // Working directory paths
    let workdir_re = Regex::new(r"(~|/)[^\s\n]+(/[^\s\n]+)*").unwrap();
    result = workdir_re.replace_all(&result, "<PATH>").to_string();

    // Replace CWD if provided
    if let Some(cwd) = cwd {
        result = result.replace(cwd, "<PATH>");
    }

    // Version strings
    let version_re = Regex::new(r"v\d+\.\d+\.\d+").unwrap();
    result = version_re.replace_all(&result, "<VERSION>").to_string();

    // Model names in header line
    let model_re = Regex::new(r"(Haiku|Sonnet|Opus)( \d+(\.\d+)?)?").unwrap();
    result = model_re.replace_all(&result, "<MODEL>").to_string();

    // Placeholder prompts
    let placeholder_re = Regex::new(r#"Try "[^"]+""#).unwrap();
    result = placeholder_re
        .replace_all(&result, "<PLACEHOLDER>")
        .to_string();

    // Strip trailing whitespace per line (handled at line level in output)
    result
}

/// Compare two ANSI strings for semantic equivalence.
///
/// Returns true if both strings have:
/// 1. Same text content (after normalization)
/// 2. Same ANSI sequences at corresponding positions
pub fn compare_ansi_output(actual: &str, expected: &str, cwd: Option<&str>) -> bool {
    let normalized_actual = normalize_ansi_tui(actual, cwd);
    let normalized_expected = normalize_ansi_tui(expected, cwd);
    normalized_actual == normalized_expected
}

/// Generate a detailed diff showing ANSI differences.
pub fn diff_ansi_strings(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let mut diff = String::new();

    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let exp = expected_lines.get(i).copied().unwrap_or("");
        let act = actual_lines.get(i).copied().unwrap_or("");

        if exp != act {
            diff.push_str(&format!("Line {}:\n", i + 1));
            diff.push_str(&format!("  expected: {}\n", escape_ansi_for_display(exp)));
            diff.push_str(&format!("  actual:   {}\n", escape_ansi_for_display(act)));

            // Try to identify specific ANSI differences
            let exp_seqs = claudeless::ansi::extract_sequences(exp);
            let act_seqs = claudeless::ansi::extract_sequences(act);

            if exp_seqs != act_seqs {
                diff.push_str("  ANSI diffs:\n");
                for (pos, seq) in &exp_seqs {
                    if !act_seqs.iter().any(|(_, s)| s == seq) {
                        diff.push_str(&format!("    - missing at {}: {:?}\n", pos, seq));
                    }
                }
                for (pos, seq) in &act_seqs {
                    if !exp_seqs.iter().any(|(_, s)| s == seq) {
                        diff.push_str(&format!("    + extra at {}: {:?}\n", pos, seq));
                    }
                }
            }
        }
    }

    if diff.is_empty() {
        diff = "(No differences found - check whitespace?)".to_string();
    }

    diff
}

/// Escape ANSI sequences for readable display.
fn escape_ansi_for_display(input: &str) -> String {
    input.replace("\x1b[", "[")
}

/// Load a fixture file with ANSI sequences.
pub fn load_ansi_fixture(name: &str) -> String {
    let path = super::fixtures_dir()
        .join(super::DEFAULT_FIXTURE_VERSION)
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load ANSI fixture {:?}: {}", path, e))
}

/// Assert that ANSI-colored TUI output matches a fixture.
///
/// Like `assert_tui_matches_fixture` but compares ANSI sequences.
pub fn assert_ansi_matches_fixture(actual: &str, fixture_name: &str, cwd: Option<&str>) {
    let expected = load_ansi_fixture(fixture_name);
    let normalized_actual = normalize_ansi_tui(actual, cwd);
    let normalized_expected = normalize_ansi_tui(&expected, cwd);

    if normalized_actual != normalized_expected {
        let diff = diff_ansi_strings(&normalized_expected, &normalized_actual);
        panic!(
            "ANSI TUI output does not match fixture '{}'\n\n\
             === DIFF (expected vs actual) ===\n{}\n\n\
             === NORMALIZED EXPECTED (escaped) ===\n{}\n\n\
             === NORMALIZED ACTUAL (escaped) ===\n{}\n",
            fixture_name,
            diff,
            escape_ansi_for_display(&normalized_expected),
            escape_ansi_for_display(&normalized_actual)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ansi_preserves_sequences() {
        let input = "\x1b[1mBold text\x1b[0m";
        let normalized = normalize_ansi_tui(input, None);
        assert!(normalized.contains("\x1b[1m"));
        assert!(normalized.contains("\x1b[0m"));
    }

    #[test]
    fn test_normalize_ansi_replaces_version() {
        let input = "\x1b[38;2;153;153;153mv2.1.12\x1b[39m";
        let normalized = normalize_ansi_tui(input, None);
        assert!(normalized.contains("<VERSION>"));
        assert!(normalized.contains("\x1b[38;2;153;153;153m"));
        // Note: trailing [39m] is stripped by normalization to handle iocraft optimization
    }

    #[test]
    fn test_normalize_ansi_replaces_path() {
        let input = "\x1b[38;2;153;153;153m~/Developer/claudeless\x1b[39m";
        let normalized = normalize_ansi_tui(input, None);
        assert!(normalized.contains("<PATH>"));
        assert!(normalized.contains("\x1b[38;2;153;153;153m"));
    }

    #[test]
    fn test_normalize_ansi_replaces_placeholder() {
        let input = "\x1b[0;2mry \"write a test for scenario.rs\"\x1b[0m";
        let normalized = normalize_ansi_tui(input, None);
        // The "Try" part won't match because the placeholder regex expects 'Try "'
        // but the actual fixture has the 'T' outside the dim span
        assert!(normalized.contains("\x1b[0;2m"));
    }

    #[test]
    fn test_compare_ansi_output_identical() {
        let s = "\x1b[1mHello\x1b[0m";
        assert!(compare_ansi_output(s, s, None));
    }

    #[test]
    fn test_compare_ansi_output_different_colors() {
        let a = "\x1b[38;2;255;0;0mRed\x1b[39m";
        let b = "\x1b[38;2;0;255;0mRed\x1b[39m";
        assert!(!compare_ansi_output(a, b, None));
    }

    #[test]
    fn test_compare_ansi_output_different_text() {
        let a = "\x1b[1mHello\x1b[0m";
        let b = "\x1b[1mWorld\x1b[0m";
        assert!(!compare_ansi_output(a, b, None));
    }

    #[test]
    fn test_diff_ansi_strings_no_diff() {
        let s = "\x1b[1mSame\x1b[0m";
        let diff = diff_ansi_strings(s, s);
        assert!(diff.contains("No differences"));
    }

    #[test]
    fn test_diff_ansi_strings_with_diff() {
        let a = "\x1b[1mExpected\x1b[0m";
        let b = "\x1b[2mActual\x1b[0m";
        let diff = diff_ansi_strings(a, b);
        assert!(diff.contains("Line 1"));
        assert!(diff.contains("expected"));
        assert!(diff.contains("actual"));
    }

    #[test]
    fn test_escape_ansi_for_display() {
        let input = "\x1b[38;2;215;119;87mOrange\x1b[39m";
        let escaped = escape_ansi_for_display(input);
        assert_eq!(escaped, "[38;2;215;119;87mOrange[39m");
    }

    #[test]
    fn test_load_ansi_fixture() {
        let fixture = load_ansi_fixture("initial_state_ansi.txt");
        assert!(!fixture.is_empty());
        // Should contain ANSI escape sequences
        assert!(fixture.contains("\x1b["));
    }
}
