//! Snapshot tests comparing full screen output against fixtures.
//!
//! These tests verify exact terminal rendering by comparing captured frames
//! against known-good fixture files.

use similar_asserts::assert_eq;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::{NamedTempFile, TempDir};

/// Helper to run capsh and return the latest frame content
fn capture_frame(script: &str, command: &[&str], cols: u16, rows: u16) -> String {
    let dir = TempDir::new().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_capsh"))
        .arg("--frames")
        .arg(dir.path())
        .arg("--cols")
        .arg(cols.to_string())
        .arg("--rows")
        .arg(rows.to_string())
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
    assert!(
        output.status.success(),
        "capsh failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::read_to_string(dir.path().join("latest.txt")).expect("no frame captured")
}

/// Helper to run capsh with vi on a temp file
fn capture_vi_frame(script: &str) -> String {
    let file = NamedTempFile::new().unwrap();
    let file_path = file.path().to_str().unwrap().to_string();

    let dir = TempDir::new().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_capsh"))
        .arg("--frames")
        .arg(dir.path())
        .arg("--cols")
        .arg("80")
        .arg("--rows")
        .arg("24")
        .arg("--")
        .arg("vi")
        .arg(&file_path)
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
    assert!(
        output.status.success(),
        "capsh failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::read_to_string(dir.path().join("latest.txt")).expect("no frame captured")
}

// =============================================================================
// echo snapshot tests
// =============================================================================

#[test]
fn snapshot_echo_hello() {
    let script = r#"
wait "hello"
snapshot
"#;

    let actual = capture_frame(script, &["echo", "hello"], 40, 5);

    // echo outputs "hello" on first line
    assert!(actual.starts_with("hello"), "should start with hello");
    // Should have at least the hello line, may have trailing empty lines
    assert!(actual.lines().count() >= 1, "should have at least one line");
}

// =============================================================================
// vi snapshot tests
// =============================================================================

#[test]
fn snapshot_vi_empty_file() {
    let script = r#"
wait "~"
wait 200
snapshot
send ":q\n"
"#;

    let actual = capture_vi_frame(script);
    let expected = include_str!("fixtures/vi_empty_file.txt");

    assert_eq!(actual, expected, "vi empty file screen mismatch");
}

#[test]
fn snapshot_vi_hello_world() {
    let script = r#"
wait "~"
send "ihello world"
send <Esc>
wait 200
snapshot
send ":q!\n"
"#;

    let actual = capture_vi_frame(script);
    let expected = include_str!("fixtures/vi_hello_world.txt");

    assert_eq!(actual, expected, "vi hello world screen mismatch");
}

#[test]
fn snapshot_vi_multiline() {
    let script = r#"
wait "~"
send "iline one"
send <Esc>
send "oline two"
send <Esc>
send "oline three"
send <Esc>
wait 200
snapshot
send ":q!\n"
"#;

    let actual = capture_vi_frame(script);
    let expected = include_str!("fixtures/vi_multiline.txt");

    assert_eq!(actual, expected, "vi multiline screen mismatch");
}
