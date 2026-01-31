// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn test_normalize_project_path() {
    assert_eq!(
        normalize_project_path(Path::new("/Users/user/project")),
        "-Users-user-project"
    );
    assert_eq!(
        normalize_project_path(Path::new("/tmp/test.txt")),
        "-tmp-test-txt"
    );
    assert_eq!(
        normalize_project_path(Path::new("relative/path")),
        "relative-path"
    );
}

#[test]
fn test_path_functions() {
    let root = Path::new("/home/user/.claude");

    assert_eq!(todos_dir(root), PathBuf::from("/home/user/.claude/todos"));
    assert_eq!(
        projects_dir(root),
        PathBuf::from("/home/user/.claude/projects")
    );
    assert_eq!(plans_dir(root), PathBuf::from("/home/user/.claude/plans"));
    assert_eq!(
        sessions_dir(root),
        PathBuf::from("/home/user/.claude/sessions")
    );
    assert_eq!(
        settings_path(root),
        PathBuf::from("/home/user/.claude/settings.json")
    );
}

#[test]
fn test_session_path() {
    let root = Path::new("/tmp/state");
    assert_eq!(
        session_path(root, "abc-123"),
        PathBuf::from("/tmp/state/sessions/abc-123.json")
    );
}

#[test]
fn test_todo_path() {
    let root = Path::new("/tmp/state");
    assert_eq!(
        todo_path(root, "context1"),
        PathBuf::from("/tmp/state/todos/context1.json")
    );
}
