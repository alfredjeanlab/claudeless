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
fn test_state_paths() {
    let paths = StatePaths::new("/home/user/.claude");

    assert_eq!(paths.root(), Path::new("/home/user/.claude"));
    assert_eq!(paths.todos_dir(), PathBuf::from("/home/user/.claude/todos"));
    assert_eq!(
        paths.projects_dir(),
        PathBuf::from("/home/user/.claude/projects")
    );
    assert_eq!(paths.plans_dir(), PathBuf::from("/home/user/.claude/plans"));
    assert_eq!(
        paths.sessions_dir(),
        PathBuf::from("/home/user/.claude/sessions")
    );
    assert_eq!(
        paths.settings_path(),
        PathBuf::from("/home/user/.claude/settings.json")
    );
}

#[test]
fn test_state_paths_session() {
    let paths = StatePaths::new("/tmp/state");
    assert_eq!(
        paths.session_path("abc-123"),
        PathBuf::from("/tmp/state/sessions/abc-123.json")
    );
}

#[test]
fn test_state_paths_todo() {
    let paths = StatePaths::new("/tmp/state");
    assert_eq!(
        paths.todo_path("context1"),
        PathBuf::from("/tmp/state/todos/context1.json")
    );
}
