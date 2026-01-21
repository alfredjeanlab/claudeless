// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Tmux wrapper for TUI tests.
//!
//! Provides a clean API for tmux operations used in testing.

use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Default timeout for waiting for TUI to be ready (in milliseconds).
/// Override with `TMUX_TEST_TIMEOUT_MS` environment variable.
///
/// KEEP COMMENT: Measured timings (2025-01):
/// - tmux session create: ~25ms
/// - tmux send-keys: ~22ms
/// - tmux capture-pane: ~5ms
/// - claudeless startup + render: ~40-50ms
/// - Total typical ready time: ~90ms with 10ms polling
///   Default of 1000ms provides headroom for slow CI environments.
const DEFAULT_TIMEOUT_MS: u64 = 1000;

/// Default poll interval when waiting for TUI (in milliseconds).
/// Override with `TMUX_TEST_POLL_MS` environment variable.
///
/// KEEP COMMENT: With 50ms poll interval and ~90ms typical ready time,
/// tests complete in 2-3 poll iterations under normal conditions.
const DEFAULT_POLL_MS: u64 = 50;

/// Get the test timeout from env var or default.
pub fn timeout() -> Duration {
    let ms = std::env::var("TMUX_TEST_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    Duration::from_millis(ms)
}

/// Get the poll interval from env var or default.
pub fn poll_interval() -> Duration {
    let ms = std::env::var("TMUX_TEST_POLL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_POLL_MS);
    Duration::from_millis(ms)
}

/// Create a new detached tmux session with specified dimensions.
pub fn new_session(session: &str, width: u16, height: u16) {
    let status = Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            session,
            "-x",
            &width.to_string(),
            "-y",
            &height.to_string(),
        ])
        .status()
        .expect("Failed to create tmux session");

    assert!(
        status.success(),
        "Failed to create tmux session '{}'",
        session
    );
}

/// Kill a tmux session, ignoring errors if it doesn't exist.
pub fn kill_session(session: &str) {
    let _ = Command::new("tmux")
        .args(["kill-session", "-t", session])
        .output();
}

/// Send keys to a tmux session (without pressing Enter).
pub fn send_keys(session: &str, keys: &str) {
    Command::new("tmux")
        .args(["send-keys", "-t", session, keys])
        .status()
        .expect("Failed to send keys");
}

/// Send keys followed by Enter to a tmux session.
pub fn send_line(session: &str, line: &str) {
    Command::new("tmux")
        .args(["send-keys", "-t", session, line, "Enter"])
        .status()
        .expect("Failed to send line");
}

/// Capture the current pane content (plain text, no ANSI sequences).
pub fn capture_pane(session: &str) -> String {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p"])
        .output()
        .expect("Failed to capture tmux pane");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Capture pane content with ANSI escape sequences preserved.
///
/// Uses `tmux capture-pane -e` flag to include escape sequences in the output.
/// This is useful for color comparison testing.
pub fn capture_pane_ansi(session: &str) -> String {
    let output = Command::new("tmux")
        .args(["capture-pane", "-e", "-p", "-t", session])
        .output()
        .expect("Failed to capture tmux pane with ANSI");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Wait for specific content to appear in the tmux pane.
/// Returns the captured pane content when found, or panics on timeout.
pub fn wait_for_content(session: &str, pattern: &str) -> String {
    wait_for_content_timeout(session, pattern, timeout())
}

/// Wait for specific content to appear, then return capture with ANSI sequences.
///
/// Waits for the pattern using plain text matching (for reliability),
/// then captures with ANSI escape sequences included.
pub fn wait_for_content_ansi(session: &str, pattern: &str) -> String {
    wait_for_content_timeout(session, pattern, timeout());
    capture_pane_ansi(session)
}

/// Wait for specific content to appear in the tmux pane with a custom timeout.
/// Returns the captured pane content when found, or panics on timeout.
pub fn wait_for_content_timeout(session: &str, pattern: &str, timeout: Duration) -> String {
    let poll = poll_interval();
    let start = Instant::now();

    loop {
        let capture = capture_pane(session);

        if capture.contains(pattern) {
            return capture;
        }

        if start.elapsed() >= timeout {
            panic!(
                "Timeout waiting for '{}' in tmux session '{}' after {:?}\n\
                 Last capture:\n{}",
                pattern, session, timeout, capture
            );
        }

        sleep(poll);
    }
}

/// Wait for any of the specified patterns to appear in the tmux pane.
/// Returns the captured pane content when any pattern is found, or panics on timeout.
pub fn wait_for_any(session: &str, patterns: &[&str]) -> String {
    let timeout = timeout();
    let poll = poll_interval();
    let start = Instant::now();

    loop {
        let capture = capture_pane(session);

        if patterns.iter().any(|p| capture.contains(p)) {
            return capture;
        }

        if start.elapsed() >= timeout {
            panic!(
                "Timeout waiting for any of {:?} in tmux session '{}' after {:?}\n\
                 Last capture:\n{}",
                patterns, session, timeout, capture
            );
        }

        sleep(poll);
    }
}

/// Assert that pane content remains unchanged for the specified duration.
/// Polls periodically and panics if content changes during the wait period.
/// Returns the final capture (which should match `previous`).
pub fn assert_unchanged_ms(session: &str, previous: &str, duration_ms: u64) -> String {
    let duration = Duration::from_millis(duration_ms);
    let poll = poll_interval();
    let start = Instant::now();

    while start.elapsed() < duration {
        let capture = capture_pane(session);

        if capture != previous {
            panic!(
                "Content unexpectedly changed in tmux session '{}' after {:?}\n\
                 Expected:\n{}\n\n\
                 Got:\n{}",
                session,
                start.elapsed(),
                previous,
                capture
            );
        }

        sleep(poll);
    }

    capture_pane(session)
}

/// Wait for pane content to change from a previous state.
/// Returns the new captured pane content when it differs, or panics on timeout.
pub fn wait_for_change(session: &str, previous: &str) -> String {
    let timeout = timeout();
    let poll = poll_interval();
    let start = Instant::now();

    loop {
        let capture = capture_pane(session);

        if capture != previous {
            return capture;
        }

        if start.elapsed() >= timeout {
            panic!(
                "Timeout waiting for content change in tmux session '{}' after {:?}\n\
                 Content unchanged:\n{}",
                session, timeout, capture
            );
        }

        sleep(poll);
    }
}
