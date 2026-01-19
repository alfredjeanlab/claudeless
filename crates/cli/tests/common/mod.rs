// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Shared helpers for TUI snapshot tests.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

#![allow(dead_code)]

pub mod tmux;

use std::io::Write;
use tempfile::NamedTempFile;

/// Pattern that indicates the TUI is fully rendered (appears in status bar).
pub const TUI_READY_PATTERN: &str = "? for shortcuts";

// =============================================================================
// Scenario Helpers
// =============================================================================

/// Create a temporary scenario file
/// Detects JSON vs TOML content and uses appropriate extension
pub fn write_scenario(content: &str) -> NamedTempFile {
    // Check if content looks like JSON (starts with { or [)
    let is_json = content.trim().starts_with('{') || content.trim().starts_with('[');

    let mut file = if is_json {
        tempfile::Builder::new().suffix(".json").tempfile().unwrap()
    } else {
        tempfile::Builder::new().suffix(".toml").tempfile().unwrap()
    };

    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

/// Get path to claudeless binary
pub fn claudeless_bin() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../target/debug/claudeless", manifest_dir)
}

/// Start claudeless TUI in a tmux session and wait for it to be ready.
/// Returns the initial capture after the TUI is ready.
/// Uses default dimensions (120x40) and waits for `TUI_READY_PATTERN`.
pub fn start_tui(session: &str, scenario: &tempfile::NamedTempFile) -> String {
    start_tui_ext(session, scenario, 120, 40, TUI_READY_PATTERN)
}

/// Start claudeless TUI with custom dimensions and wait pattern.
/// Returns the capture after the wait pattern appears.
pub fn start_tui_ext(
    session: &str,
    scenario: &tempfile::NamedTempFile,
    width: u16,
    height: u16,
    wait_for: &str,
) -> String {
    tmux::kill_session(session);
    tmux::new_session(session, width, height);

    let cmd = format!(
        "{} --scenario {} --tui",
        claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    tmux::wait_for_content(session, wait_for)
}

/// Helper to start claudeless TUI and capture initial state
pub fn capture_tui_initial(session: &str, extra_args: &str) -> String {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    tmux::kill_session(session);
    tmux::new_session(session, 120, 20);

    let cmd = format!(
        "{} --scenario {} --tui {}",
        claudeless_bin(),
        scenario.path().display(),
        extra_args
    );
    tmux::send_line(session, &cmd);

    let capture = tmux::wait_for_content(session, TUI_READY_PATTERN);

    // Cleanup: first C-c cancels operation, wait for effect, second C-c exits
    let before_cleanup = tmux::capture_pane(session);
    tmux::send_keys(session, "C-c");
    let _ = tmux::wait_for_change(session, &before_cleanup);
    tmux::send_keys(session, "C-c");
    tmux::kill_session(session);

    capture
}

// =============================================================================
// Fixture Comparison Helpers
// =============================================================================

/// Default fixture version (Claude Code version used to capture fixtures)
pub const DEFAULT_FIXTURE_VERSION: &str = "v2.1.12";

/// Path to the TUI fixtures directory
pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tui")
}

/// Load a fixture file by name (uses default version)
pub fn load_fixture(name: &str) -> String {
    load_versioned_fixture(DEFAULT_FIXTURE_VERSION, name)
}

/// Load a versioned fixture file
pub fn load_versioned_fixture(version: &str, name: &str) -> String {
    let path = fixtures_dir().join(version).join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {:?}: {}", path, e))
}

/// Normalize TUI output for comparison
///
/// Applies the following normalizations:
/// - Timestamps (HH:MM:SS or HH:MM) -> `<TIME>`
/// - Session IDs (UUIDs) -> `<SESSION>`
/// - All paths (temp dirs, working dirs) -> `<PATH>`
/// - Version strings (vX.Y.Z) -> `<VERSION>`
/// - Model names in header -> `<MODEL>`
/// - Placeholder prompts -> `<PLACEHOLDER>`
/// - Strip trailing whitespace per line (preserve leading and interior)
/// - Strip leading and trailing empty lines
pub fn normalize_tui(input: &str, cwd: Option<&str>) -> String {
    use regex::Regex;

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

    // All paths (temp dirs, working dirs, etc.) - normalize to <PATH>
    // macOS temp directories
    let macos_tmp_re = Regex::new(r"/private/var/folders/[^/]+/[^/]+/[^/]+/[^\s]+").unwrap();
    result = macos_tmp_re.replace_all(&result, "<PATH>").to_string();

    // Linux temp directories
    let linux_tmp_re = Regex::new(r"/tmp/[^\s]+").unwrap();
    result = linux_tmp_re.replace_all(&result, "<PATH>").to_string();
    let var_tmp_re = Regex::new(r"/var/tmp/[^\s]+").unwrap();
    result = var_tmp_re.replace_all(&result, "<PATH>").to_string();

    // Working directory paths (e.g., "~/Developer/claudeless" or "/Users/...")
    let workdir_re = Regex::new(r"(~|/)[^\s\n]+(/[^\s\n]+)*").unwrap();
    result = workdir_re.replace_all(&result, "<PATH>").to_string();

    // Replace CWD if provided (now redundant but kept for explicit replacements)
    if let Some(cwd) = cwd {
        result = result.replace(cwd, "<PATH>");
    }

    // Version strings (e.g., "v2.1.12", "v0.1.0")
    let version_re = Regex::new(r"v\d+\.\d+\.\d+").unwrap();
    result = version_re.replace_all(&result, "<VERSION>").to_string();

    // Model names in header line (e.g., "Haiku 4.5 · Claude Max")
    let model_re = Regex::new(r"(Haiku|Sonnet|Opus)( \d+(\.\d+)?)?").unwrap();
    result = model_re.replace_all(&result, "<MODEL>").to_string();

    // Placeholder prompts (e.g., 'Try "refactor mod.rs"', 'Try "fix lint errors"')
    let placeholder_re = Regex::new(r#"Try "[^"]+""#).unwrap();
    result = placeholder_re
        .replace_all(&result, "<PLACEHOLDER>")
        .to_string();

    // User prompts after ❯ (normalize different prompt content)
    result = result
        .lines()
        .map(|line| {
            if line.starts_with("❯ ") && !line.contains("<PLACEHOLDER>") && line.len() > 2 {
                "❯ <PROMPT>".to_string()
            } else if line == "❯" {
                // Empty input line
                line.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Response content after ⏺ (normalize different response content)
    result = result
        .lines()
        .map(|line| {
            if line.starts_with("⏺ ") {
                "⏺ <RESPONSE>".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Strip trailing whitespace per line (preserve leading and interior)
    result = result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    // Strip leading empty lines only (not leading whitespace on first content line)
    while result.starts_with('\n') {
        result = result[1..].to_string();
    }

    // Strip trailing empty lines
    while result.ends_with('\n') {
        result = result[..result.len() - 1].to_string();
    }

    result
}

/// Compare TUI output against a fixture
///
/// Returns true if normalized versions match
pub fn compare_tui_output(actual: &str, expected: &str, cwd: Option<&str>) -> bool {
    let normalized_actual = normalize_tui(actual, cwd);
    let normalized_expected = normalize_tui(expected, cwd);
    normalized_actual == normalized_expected
}

/// Assert that TUI output matches a fixture, with detailed diff on failure
pub fn assert_tui_matches_fixture(actual: &str, fixture_name: &str, cwd: Option<&str>) {
    let expected = load_fixture(fixture_name);
    let normalized_actual = normalize_tui(actual, cwd);
    let normalized_expected = normalize_tui(&expected, cwd);

    if normalized_actual != normalized_expected {
        // Generate a helpful diff
        let diff = diff_strings(&normalized_expected, &normalized_actual);
        panic!(
            "TUI output does not match fixture '{}'\n\n\
             === DIFF (expected vs actual) ===\n{}\n\n\
             === NORMALIZED EXPECTED ===\n{}\n\n\
             === NORMALIZED ACTUAL ===\n{}\n",
            fixture_name, diff, normalized_expected, normalized_actual
        );
    }
}

/// Assert that TUI output matches a versioned fixture
pub fn assert_tui_matches_versioned_fixture(
    actual: &str,
    version: &str,
    fixture_name: &str,
    cwd: Option<&str>,
) {
    let expected = load_versioned_fixture(version, fixture_name);
    let normalized_actual = normalize_tui(actual, cwd);
    let normalized_expected = normalize_tui(&expected, cwd);

    if normalized_actual != normalized_expected {
        let diff = diff_strings(&normalized_expected, &normalized_actual);
        panic!(
            "TUI output does not match fixture '{}/{}'\n\n\
             === DIFF (expected vs actual) ===\n{}\n\n\
             === NORMALIZED EXPECTED ===\n{}\n\n\
             === NORMALIZED ACTUAL ===\n{}\n",
            version, fixture_name, diff, normalized_expected, normalized_actual
        );
    }
}

/// Generate a simple line-by-line diff
fn diff_strings(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let mut diff = String::new();

    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let exp = expected_lines.get(i).copied().unwrap_or("");
        let act = actual_lines.get(i).copied().unwrap_or("");

        if exp != act {
            diff.push_str(&format!("Line {}:\n", i + 1));
            diff.push_str(&format!("  - {}\n", exp));
            diff.push_str(&format!("  + {}\n", act));
        }
    }

    if diff.is_empty() {
        diff = "(No differences found - check whitespace?)".to_string();
    }

    diff
}

/// Helper to run claudeless and execute a sequence of keys, capturing at each step.
/// After each key press, waits for the screen content to change before capturing.
pub fn capture_key_sequence(session: &str, keys: &[&str]) -> Vec<String> {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    tmux::kill_session(session);
    tmux::new_session(session, 120, 25);

    let cmd = format!(
        "{} --scenario {} --tui",
        claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);

    tmux::wait_for_content(session, TUI_READY_PATTERN);

    let mut captures = Vec::new();

    // Capture initial state
    let mut previous = tmux::capture_pane(session);
    captures.push(previous.clone());

    // Execute each key and capture after screen changes
    for key in keys {
        tmux::send_keys(session, key);
        let capture = tmux::wait_for_change(session, &previous);
        captures.push(capture.clone());
        previous = capture;
    }

    // Cleanup: first C-c cancels operation, wait for effect, second C-c exits
    tmux::send_keys(session, "C-c");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "C-c");
    tmux::kill_session(session);

    captures
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
