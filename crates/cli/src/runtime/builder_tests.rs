// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::Parser;
use std::fs;
use std::sync::Mutex;

use super::*;
use crate::state::{SessionIndexEntry, SessionsIndex};

// Serialize tests that use CLAUDELESS_CONFIG_DIR to avoid race conditions.
// Use current_thread runtime to ensure the lock is held across await points.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Guard that restores/removes env var on drop, ensuring cleanup even on panic
struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvGuard {
    fn new(key: &'static str, value: &std::path::Path) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(prev) => std::env::set_var(self.key, prev),
            None => std::env::remove_var(self.key),
        }
    }
}

#[test]
fn builder_validates_cli() {
    // Create a valid CLI and verify builder creation succeeds
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let builder = RuntimeBuilder::new(cli);
    assert!(builder.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn test_resume_session_not_found() {
    let _lock = ENV_LOCK.lock().unwrap();

    // Create a temp directory for state
    let temp_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Set environment variable for state directory (auto-cleaned on drop)
    let _env = EnvGuard::new("CLAUDELESS_CONFIG_DIR", temp_dir.path());

    // Create project directory but no sessions-index.json
    let project_dir =
        crate::state::directory::StateDirectory::new(temp_dir.path()).project_dir(work_dir.path());
    fs::create_dir_all(&project_dir).unwrap();

    // Create CLI with resume flag
    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "test",
        "--resume",
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
        "--cwd",
        work_dir.path().to_str().unwrap(),
    ])
    .unwrap();

    let builder = RuntimeBuilder::new(cli).unwrap();
    let result = builder.build().await;

    // Should fail with SessionNotFound
    assert!(matches!(result, Err(RuntimeBuildError::SessionNotFound(_))));
}

#[tokio::test(flavor = "current_thread")]
async fn test_resume_session_validation_success() {
    let _lock = ENV_LOCK.lock().unwrap();

    // Create a temp directory for state
    let temp_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Set environment variable for state directory (auto-cleaned on drop)
    let _env = EnvGuard::new("CLAUDELESS_CONFIG_DIR", temp_dir.path());

    // Create project directory with sessions-index.json
    let project_dir =
        crate::state::directory::StateDirectory::new(temp_dir.path()).project_dir(work_dir.path());
    fs::create_dir_all(&project_dir).unwrap();

    let session_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let mut index = SessionsIndex::new();
    index.add_or_update(SessionIndexEntry {
        session_id: session_id.to_string(),
        full_path: project_dir
            .join(format!("{}.jsonl", session_id))
            .to_string_lossy()
            .to_string(),
        file_mtime: 0,
        first_prompt: "test".to_string(),
        message_count: 1,
        created: "2025-01-01T00:00:00Z".to_string(),
        modified: "2025-01-01T00:00:00Z".to_string(),
        git_branch: "".to_string(),
        project_path: work_dir.path().to_string_lossy().to_string(),
        is_sidechain: false,
    });
    index
        .save(&project_dir.join("sessions-index.json"))
        .unwrap();

    // Create CLI with resume flag
    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "test",
        "--resume",
        session_id,
        "--cwd",
        work_dir.path().to_str().unwrap(),
    ])
    .unwrap();

    let builder = RuntimeBuilder::new(cli).unwrap();
    let result = builder.build().await;

    // Should succeed since session exists in index
    assert!(
        result.is_ok(),
        "Expected success but got error: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_resume_session_not_in_index() {
    let _lock = ENV_LOCK.lock().unwrap();

    // Create a temp directory for state
    let temp_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();

    // Set environment variable for state directory (auto-cleaned on drop)
    let _env = EnvGuard::new("CLAUDELESS_CONFIG_DIR", temp_dir.path());

    // Create project directory with sessions-index.json but without our session
    let project_dir =
        crate::state::directory::StateDirectory::new(temp_dir.path()).project_dir(work_dir.path());
    fs::create_dir_all(&project_dir).unwrap();

    let mut index = SessionsIndex::new();
    index.add_or_update(SessionIndexEntry {
        session_id: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb".to_string(),
        full_path: "other.jsonl".to_string(),
        file_mtime: 0,
        first_prompt: "other".to_string(),
        message_count: 1,
        created: "2025-01-01T00:00:00Z".to_string(),
        modified: "2025-01-01T00:00:00Z".to_string(),
        git_branch: "".to_string(),
        project_path: work_dir.path().to_string_lossy().to_string(),
        is_sidechain: false,
    });
    index
        .save(&project_dir.join("sessions-index.json"))
        .unwrap();

    // Create CLI with resume flag for a different session
    let cli = Cli::try_parse_from([
        "claude",
        "-p",
        "test",
        "--resume",
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
        "--cwd",
        work_dir.path().to_str().unwrap(),
    ])
    .unwrap();

    let builder = RuntimeBuilder::new(cli).unwrap();
    let result = builder.build().await;

    // Should fail with SessionNotFound
    assert!(matches!(result, Err(RuntimeBuildError::SessionNotFound(_))));
}
