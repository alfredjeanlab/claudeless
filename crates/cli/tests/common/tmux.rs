// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Tmux wrapper for TUI tests.
//!
//! Provides a clean API for tmux operations used in testing.
//! All functions accept `impl AsRef<str>` for session names, allowing
//! both `String` and `&str` to be passed without explicit borrowing.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Cached result of tmux availability check.
static TMUX_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if tmux is available on the system.
fn is_tmux_available() -> bool {
    *TMUX_AVAILABLE.get_or_init(|| {
        Command::new("tmux")
            .arg("-V")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Ensure tmux is available, panicking with a clear message if not.
/// Call this at the start of tests that require tmux.
pub fn require_tmux() {
    if !is_tmux_available() {
        panic!(
            "tmux is required for TUI tests but was not found. Install tmux to run these tests."
        );
    }
}

/// Default timeout for waiting for TUI to be ready (in milliseconds).
/// Override with `TMUX_TEST_TIMEOUT_MS` environment variable.
///
/// KEEP COMMENT: Measured timings (2025-01):
/// - tmux session create: ~25ms
/// - tmux send-keys: ~22ms
/// - tmux capture-pane: ~5ms
/// - claudeless startup + render: ~40-50ms
/// - Total typical ready time: ~90ms with 10ms polling
///   Default of 2000ms provides headroom for parallel test execution
///   and slow CI environments where resource contention causes delays.
const DEFAULT_TIMEOUT_MS: u64 = 2000;

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

/// Counter for generating unique session names.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique tmux session name from a prefix.
///
/// This prevents race conditions when tests run in parallel by ensuring
/// each test gets a unique session name. The generated name includes:
/// - The provided prefix (for debugging/identification)
/// - The process ID (isolates parallel test processes)
/// - A monotonic counter (isolates tests within the same process)
///
/// # Example
/// ```ignore
/// let session = unique_session("hooks-test");
/// // Returns something like "hooks-test-12345-1"
/// ```
pub fn unique_session(prefix: &str) -> String {
    let pid = std::process::id();
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}-{}", prefix, pid, counter)
}

/// Create a new detached tmux session with specified dimensions.
/// Waits for the shell to be ready before returning.
pub fn new_session(session: impl AsRef<str>, width: u16, height: u16) {
    let session = session.as_ref();
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

    // Wait for shell to be ready by checking that the pane has content
    // This prevents race conditions where commands are sent before the shell initializes
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    loop {
        let capture = capture_pane(session);
        // Shell is ready when there's any content (prompt, etc.)
        if !capture.trim().is_empty() {
            break;
        }
        if start.elapsed() >= timeout {
            panic!(
                "Timeout waiting for shell to initialize in tmux session '{}'",
                session
            );
        }
        sleep(Duration::from_millis(10));
    }
}

/// Kill a tmux session, ignoring errors if it doesn't exist.
pub fn kill_session(session: impl AsRef<str>) {
    let _ = Command::new("tmux")
        .args(["kill-session", "-t", session.as_ref()])
        .output();
}

/// Send keys to a tmux session (without pressing Enter).
pub fn send_keys(session: impl AsRef<str>, keys: &str) {
    Command::new("tmux")
        .args(["send-keys", "-t", session.as_ref(), keys])
        .status()
        .expect("Failed to send keys");
}

/// Send a literal control character to a tmux session.
/// Uses tmux's -H flag to send hexadecimal keys.
pub fn send_ctrl_char(session: impl AsRef<str>, ascii_code: u8) {
    let hex = format!("{:02X}", ascii_code);
    Command::new("tmux")
        .args(["send-keys", "-H", "-t", session.as_ref(), &hex])
        .status()
        .expect("Failed to send control character");
}

/// Send keys followed by Enter to a tmux session.
pub fn send_line(session: impl AsRef<str>, line: &str) {
    Command::new("tmux")
        .args(["send-keys", "-t", session.as_ref(), line, "Enter"])
        .status()
        .expect("Failed to send line");
}

/// Capture the current pane content (plain text, no ANSI sequences).
pub fn capture_pane(session: impl AsRef<str>) -> String {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session.as_ref(), "-p"])
        .output()
        .expect("Failed to capture tmux pane");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Capture pane content with ANSI escape sequences preserved.
///
/// Uses `tmux capture-pane -e` flag to include escape sequences in the output.
/// This is useful for color comparison testing.
pub fn capture_pane_ansi(session: impl AsRef<str>) -> String {
    let output = Command::new("tmux")
        .args(["capture-pane", "-e", "-p", "-t", session.as_ref()])
        .output()
        .expect("Failed to capture tmux pane with ANSI");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Wait for specific content to appear in the tmux pane.
/// Returns the captured pane content when found, or panics on timeout.
pub fn wait_for_content(session: impl AsRef<str>, pattern: &str) -> String {
    wait_for_content_timeout(session, pattern, timeout())
}

/// Wait for specific content to appear, then return capture with ANSI sequences.
///
/// Waits for the pattern using plain text matching (for reliability),
/// then captures with ANSI escape sequences included.
pub fn wait_for_content_ansi(session: impl AsRef<str>, pattern: &str) -> String {
    let session = session.as_ref();
    wait_for_content_timeout(session, pattern, timeout());
    capture_pane_ansi(session)
}

/// Wait for specific content to appear in the tmux pane with a custom timeout.
/// Returns the captured pane content when found, or panics on timeout.
pub fn wait_for_content_timeout(
    session: impl AsRef<str>,
    pattern: &str,
    timeout: Duration,
) -> String {
    let session = session.as_ref();
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
pub fn wait_for_any(session: impl AsRef<str>, patterns: &[&str]) -> String {
    let session = session.as_ref();
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
pub fn assert_unchanged_ms(session: impl AsRef<str>, previous: &str, duration_ms: u64) -> String {
    let session = session.as_ref();
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
pub fn wait_for_change(session: impl AsRef<str>, previous: &str) -> String {
    let session = session.as_ref();
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

/// Wait for pane content to change, then return capture with ANSI sequences.
///
/// Waits for content to change using plain text matching (for reliability),
/// then captures with ANSI escape sequences included.
pub fn wait_for_change_ansi(session: impl AsRef<str>, previous: &str) -> String {
    let session = session.as_ref();
    wait_for_change(session, previous);
    capture_pane_ansi(session)
}
