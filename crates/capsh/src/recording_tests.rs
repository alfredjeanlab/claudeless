#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use tempfile::TempDir;

#[test]
fn recording_creates_files() {
    let dir = TempDir::new().unwrap();
    let _rec = Recording::new(dir.path()).unwrap();

    assert!(dir.path().join("recording.jsonl").exists());
    assert!(dir.path().join("raw.bin").exists());
}

#[test]
fn recording_logs_frame() {
    let dir = TempDir::new().unwrap();
    let mut rec = Recording::new(dir.path()).unwrap();

    rec.log_frame(1).unwrap();
    rec.log_frame(42).unwrap();
    rec.flush().unwrap();

    let content = std::fs::read_to_string(dir.path().join("recording.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains(r#""frame":"000001""#));
    assert!(lines[1].contains(r#""frame":"000042""#));
    // Both should have ms field
    assert!(lines[0].contains(r#""ms":"#));
}

#[test]
fn recording_logs_send() {
    let dir = TempDir::new().unwrap();
    let mut rec = Recording::new(dir.path()).unwrap();

    rec.log_send("hello").unwrap();
    rec.log_send("with\nnewline").unwrap();
    rec.log_send(r#"with"quote"#).unwrap();
    rec.flush().unwrap();

    let content = std::fs::read_to_string(dir.path().join("recording.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains(r#""send":"hello""#));
    assert!(lines[1].contains(r#""send":"with\nnewline""#));
    assert!(lines[2].contains(r#""send":"with\"quote""#));
}

#[test]
fn recording_appends_raw() {
    let dir = TempDir::new().unwrap();
    let mut rec = Recording::new(dir.path()).unwrap();

    rec.append_raw(b"hello").unwrap();
    rec.append_raw(b" world").unwrap();
    rec.flush().unwrap();

    let content = std::fs::read(dir.path().join("raw.bin")).unwrap();
    assert_eq!(content, b"hello world");
}

#[test]
fn recording_elapsed_ms_increases() {
    let dir = TempDir::new().unwrap();
    let rec = Recording::new(dir.path()).unwrap();

    let t1 = rec.elapsed_ms();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let t2 = rec.elapsed_ms();

    assert!(t2 > t1);
}

#[test]
fn recording_logs_exit() {
    let dir = TempDir::new().unwrap();
    let mut rec = Recording::new(dir.path()).unwrap();

    rec.log_exit(0).unwrap();
    rec.log_exit(42).unwrap();
    rec.flush().unwrap();

    let content = std::fs::read_to_string(dir.path().join("recording.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains(r#""exit":0"#));
    assert!(lines[1].contains(r#""exit":42"#));
}
