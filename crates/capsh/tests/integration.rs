// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for capsh.
//!
//! These tests spawn real processes in PTYs and verify end-to-end behavior.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Helper to run capsh with a script and return (exit_code, frames_dir)
fn run_capsh(script: &str, command: &[&str]) -> (i32, TempDir) {
    let dir = TempDir::new().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_capsh"))
        .arg("--frames")
        .arg(dir.path())
        .arg("--")
        .args(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn capsh");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);

    (exit_code, dir)
}

/// Helper to run capsh without frames directory
fn run_capsh_no_frames(script: &str, command: &[&str]) -> i32 {
    let mut child = Command::new(env!("CARGO_BIN_EXE_capsh"))
        .arg("--")
        .args(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn capsh");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    output.status.code().unwrap_or(-1)
}

// =============================================================================
// Echo session tests
// =============================================================================

#[test]
fn echo_basic_capture() {
    let script = r#"
wait "hello"
"#;

    let (exit_code, dir) = run_capsh(script, &["echo", "hello"]);

    assert_eq!(exit_code, 0, "echo should exit 0");

    // Should have at least one frame
    assert!(
        dir.path().join("000001.txt").exists(),
        "should capture frame"
    );
    assert!(
        dir.path().join("000001.ansi.txt").exists(),
        "should capture ANSI frame"
    );

    // Frame should contain "hello"
    let frame = std::fs::read_to_string(dir.path().join("000001.txt")).unwrap();
    assert!(frame.contains("hello"), "frame should contain 'hello'");

    // Recording should exist
    assert!(dir.path().join("recording.jsonl").exists());
    assert!(dir.path().join("raw.bin").exists());
}

#[test]
fn echo_without_frames_dir() {
    let script = r#"
wait "world"
"#;

    let exit_code = run_capsh_no_frames(script, &["echo", "world"]);
    assert_eq!(exit_code, 0);
}

#[test]
fn echo_captures_recording_jsonl() {
    let script = r#"
wait "test"
"#;

    let (exit_code, dir) = run_capsh(script, &["echo", "test"]);
    assert_eq!(exit_code, 0);

    let jsonl = std::fs::read_to_string(dir.path().join("recording.jsonl")).unwrap();

    // Should have at least one frame entry
    assert!(jsonl.contains(r#""frame":"#), "recording should log frames");
    assert!(
        jsonl.contains(r#""ms":"#),
        "recording should have timestamps"
    );
}

#[test]
fn echo_raw_bin_contains_output() {
    let script = r#"
wait "rawtest"
"#;

    let (exit_code, dir) = run_capsh(script, &["echo", "rawtest"]);
    assert_eq!(exit_code, 0);

    let raw = std::fs::read(dir.path().join("raw.bin")).unwrap();
    let raw_str = String::from_utf8_lossy(&raw);

    assert!(
        raw_str.contains("rawtest"),
        "raw.bin should contain PTY output"
    );
}

// =============================================================================
// Timeout and EOF behavior tests
// =============================================================================

#[test]
fn eof_during_wait_returns_child_exit_code() {
    // When echo exits before wait pattern matches, capsh returns child's exit code.
    // This is correct behavior - the drain loop catches EOF and returns early.
    let script = r#"
wait "this_will_never_match"
"#;

    let (exit_code, _dir) = run_capsh(script, &["echo", "hello"]);

    // echo exits 0, so capsh returns 0 (child exited before wait could fail)
    assert_eq!(
        exit_code, 0,
        "should return child exit code when child exits first"
    );
}

#[test]
fn unmatched_wait_returns_child_exit_code() {
    // Pattern won't match, but child exits with known code.
    // capsh should return the child's exit code, not error.
    let script = r#"
wait "this_will_never_appear"
"#;

    let (exit_code, _dir) = run_capsh(script, &["sh", "-c", "echo other; exit 3"]);

    // Should return child's exit code (3), not error (1)
    assert_eq!(
        exit_code, 3,
        "should return child exit code when pattern doesn't match"
    );
}

#[test]
fn nonzero_exit_propagates() {
    // Test that non-zero exit codes from child propagate
    let script = r#"
wait "."
"#;

    let (exit_code, _dir) = run_capsh(script, &["sh", "-c", "echo x; exit 42"]);
    assert_eq!(exit_code, 42, "should propagate child exit code");
}

#[test]
fn exit_code_logged_to_recording() {
    // Now reliable: EOF during wait checks if pattern already matched
    let script = r#"
wait "done"
"#;

    let (exit_code, dir) = run_capsh(script, &["sh", "-c", "echo done; exit 7"]);
    assert_eq!(exit_code, 7);

    let jsonl = std::fs::read_to_string(dir.path().join("recording.jsonl")).unwrap();
    assert!(
        jsonl.contains(r#""exit":7"#),
        "should log exit code, got: {}",
        jsonl
    );
}

// =============================================================================
// vi integration tests
// =============================================================================

#[test]
fn vi_open_and_quit() {
    let script = r#"
wait "~"
send ":q\n"
"#;

    let (exit_code, _dir) = run_capsh(script, &["vi"]);
    assert_eq!(exit_code, 0, "vi should exit cleanly with :q");
}

#[test]
fn vi_insert_text_and_quit() {
    let script = r#"
wait "~"
send "ihello world"
send <Esc>
wait 200
send ":q!\n"
"#;

    let (exit_code, dir) = run_capsh(script, &["vi"]);
    assert_eq!(exit_code, 0, "vi should exit cleanly");

    // Check recording has send events
    let jsonl = std::fs::read_to_string(dir.path().join("recording.jsonl")).unwrap();
    assert!(jsonl.contains(r#""send":"#), "should log send events");
}

#[test]
fn vi_snapshot_captures_state() {
    let script = r#"
wait "~"
send "itest content"
send <Esc>
snapshot
send ":q!\n"
"#;

    let (exit_code, dir) = run_capsh(script, &["vi"]);
    assert_eq!(exit_code, 0);

    // Find the latest frame
    let latest = dir.path().join("latest.txt");
    assert!(latest.exists(), "latest.txt symlink should exist");

    // The snapshot should have captured "test content"
    let frame = std::fs::read_to_string(latest).unwrap();
    assert!(
        frame.contains("test content"),
        "snapshot should capture inserted text, got: {}",
        frame
    );
}

#[test]
fn vi_multiple_lines() {
    let script = r#"
wait "~"
send "iline one"
send <Esc>
send "oline two"
send <Esc>
snapshot
send ":q!\n"
"#;

    let (exit_code, dir) = run_capsh(script, &["vi"]);
    assert_eq!(exit_code, 0);

    let frame = std::fs::read_to_string(dir.path().join("latest.txt")).unwrap();
    assert!(frame.contains("line one"), "should have line one");
    assert!(frame.contains("line two"), "should have line two");
}

#[test]
fn vi_arrow_keys() {
    let script = r#"
wait "~"
send "iABC"
send <Esc>
send "0"
send "iX"
send <Esc>
snapshot
send ":q!\n"
"#;

    let (exit_code, dir) = run_capsh(script, &["vi"]);
    assert_eq!(exit_code, 0);

    let frame = std::fs::read_to_string(dir.path().join("latest.txt")).unwrap();
    // X should be inserted at beginning: "XABC"
    assert!(
        frame.contains("XABC"),
        "X should be at beginning, got: {}",
        frame
    );
}

#[test]
fn vi_ctrl_c_handling() {
    let script = r#"
wait "~"
send "i"
send <C-c>
wait 200
send ":q!\n"
"#;

    let (exit_code, _dir) = run_capsh(script, &["vi"]);
    // vi should handle Ctrl-C in insert mode and exit cleanly
    assert_eq!(exit_code, 0, "vi should handle Ctrl-C gracefully");
}

// =============================================================================
// Signal handling tests
// =============================================================================

#[test]
fn sigterm_terminates_capsh() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Script that waits forever
    let script = r#"
wait "this_will_never_match_so_capsh_blocks_forever"
"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_capsh"))
        .arg("--")
        .arg("sleep")
        .arg("60")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn capsh");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    drop(child.stdin.take()); // Close stdin

    // Give capsh time to start
    std::thread::sleep(Duration::from_millis(100));

    // Send SIGTERM
    let pid = Pid::from_raw(child.id() as i32);
    kill(pid, Signal::SIGTERM).expect("failed to send SIGTERM");

    // Wait for exit with timeout
    let output = child.wait_with_output().unwrap();

    // Should terminate - either via exit code 143 or signal 15
    let terminated = match output.status.code() {
        Some(143) => true, // Exited with 128 + SIGTERM
        Some(_) => false,
        None => {
            // Killed by signal
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                output.status.signal() == Some(15) // SIGTERM
            }
            #[cfg(not(unix))]
            false
        }
    };

    assert!(
        terminated,
        "capsh should terminate on SIGTERM, status: {:?}",
        output.status
    );
}
