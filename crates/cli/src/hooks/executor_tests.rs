// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_hook_config_new() {
    let config = HookConfig::new("/path/to/script.sh", 5000);
    assert_eq!(config.script_path, PathBuf::from("/path/to/script.sh"));
    assert_eq!(config.timeout_ms, 5000);
    assert!(!config.blocking);
    assert!(config.matcher.is_none());
}

#[test]
fn test_hook_config_with_matcher() {
    let config = HookConfig::new("/path/to/script.sh", 5000)
        .with_matcher(Some("idle_prompt|permission_prompt".to_string()));
    assert_eq!(
        config.matcher,
        Some("idle_prompt|permission_prompt".to_string())
    );
}

#[test]
fn test_hook_config_builder() {
    let config = HookConfig::new("/script.sh", 5000)
        .with_timeout(10000)
        .with_blocking(true);

    assert_eq!(config.timeout_ms, 10000);
    assert!(config.blocking);
}

#[test]
fn test_hook_executor_new() {
    let executor = HookExecutor::new();
    assert!(!executor.has_hooks(&HookEvent::PreToolExecution));
}

#[test]
fn test_hook_executor_register() {
    let mut executor = HookExecutor::new();
    executor.register(
        HookEvent::PreToolExecution,
        HookConfig::new("/script.sh", 5000),
    );

    assert!(executor.has_hooks(&HookEvent::PreToolExecution));
    assert_eq!(executor.hook_count(&HookEvent::PreToolExecution), 1);
}

#[test]
fn test_hook_executor_clear() {
    let mut executor = HookExecutor::new();
    executor.register(HookEvent::PreToolExecution, HookConfig::new("/a.sh", 5000));
    executor.register(HookEvent::PostToolExecution, HookConfig::new("/b.sh", 5000));

    executor.clear_event(&HookEvent::PreToolExecution);
    assert!(!executor.has_hooks(&HookEvent::PreToolExecution));
    assert!(executor.has_hooks(&HookEvent::PostToolExecution));

    executor.clear();
    assert!(!executor.has_hooks(&HookEvent::PostToolExecution));
}

#[test]
fn test_hook_executor_registered_events() {
    let mut executor = HookExecutor::new();
    executor.register(HookEvent::PreToolExecution, HookConfig::new("/a.sh", 5000));
    executor.register(HookEvent::SessionStart, HookConfig::new("/b.sh", 5000));

    let events = executor.registered_events();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_execute_no_hooks() {
    let executor = HookExecutor::new();
    let message = HookMessage::session("test", HookEvent::SessionStart, None);

    let responses = executor.execute(&message).await.unwrap();
    assert!(responses.is_empty());
}

#[test]
fn test_with_context_sets_fields() {
    let executor = HookExecutor::new().with_context(
        Some("/home/user/project".to_string()),
        Some("/tmp/session.jsonl".to_string()),
        Some("default".to_string()),
    );
    assert_eq!(executor.cwd.as_deref(), Some("/home/user/project"));
    assert_eq!(
        executor.transcript_path.as_deref(),
        Some("/tmp/session.jsonl")
    );
    assert_eq!(executor.permission_mode.as_deref(), Some("default"));
}

#[test]
fn test_with_context_defaults_to_none() {
    let executor = HookExecutor::new();
    assert!(executor.cwd.is_none());
    assert!(executor.transcript_path.is_none());
    assert!(executor.permission_mode.is_none());
}

#[tokio::test]
async fn test_context_fields_in_wire_json() {
    // Create a script that echoes stdin to stdout as a JSON response
    let dir = tempfile::tempdir().unwrap();
    let script_path = dir.path().join("echo_hook.sh");
    std::fs::write(
        &script_path,
        "#!/bin/bash\n# Read stdin and write it to a file, then respond with proceed\ncat > \"$0.input\"\necho '{\"proceed\": true}'\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut executor = HookExecutor::new().with_context(
        Some("/work/dir".to_string()),
        Some("/tmp/transcript.jsonl".to_string()),
        Some("acceptEdits".to_string()),
    );
    executor.register(HookEvent::SessionStart, HookConfig::new(&script_path, 5000));

    let message = HookMessage::session("test-session", HookEvent::SessionStart, None);
    let responses = executor.execute(&message).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].proceed);

    // Read what the hook script received on stdin
    let input_path = format!("{}.input", script_path.display());
    let input = std::fs::read_to_string(input_path).unwrap();
    let wire: serde_json::Value = serde_json::from_str(&input).unwrap();

    assert_eq!(wire["hook_event_name"], "SessionStart");
    assert_eq!(wire["session_id"], "test-session");
    assert_eq!(wire["cwd"], "/work/dir");
    assert_eq!(wire["transcript_path"], "/tmp/transcript.jsonl");
    assert_eq!(wire["permission_mode"], "acceptEdits");
}

#[test]
fn test_matcher_filtering_no_matcher_registers_for_all() {
    let mut executor = HookExecutor::new();
    executor.register(
        HookEvent::Notification,
        HookConfig::new("/script.sh", 5000).with_matcher(None),
    );
    // Without matcher, the hook is registered for all Notification events
    assert!(executor.has_hooks(&HookEvent::Notification));
    assert_eq!(executor.hook_count(&HookEvent::Notification), 1);
}
