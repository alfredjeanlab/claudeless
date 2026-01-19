#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_hook_config_new() {
    let config = HookConfig::new("/path/to/script.sh");
    assert_eq!(config.script_path, PathBuf::from("/path/to/script.sh"));
    assert_eq!(config.timeout_ms, 5000);
    assert!(!config.blocking);
}

#[test]
fn test_hook_config_builder() {
    let config = HookConfig::new("/script.sh")
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
    executor.register(HookEvent::PreToolExecution, HookConfig::new("/script.sh"));

    assert!(executor.has_hooks(&HookEvent::PreToolExecution));
    assert_eq!(executor.hook_count(&HookEvent::PreToolExecution), 1);
}

#[test]
fn test_hook_executor_clear() {
    let mut executor = HookExecutor::new();
    executor.register(HookEvent::PreToolExecution, HookConfig::new("/a.sh"));
    executor.register(HookEvent::PostToolExecution, HookConfig::new("/b.sh"));

    executor.clear_event(&HookEvent::PreToolExecution);
    assert!(!executor.has_hooks(&HookEvent::PreToolExecution));
    assert!(executor.has_hooks(&HookEvent::PostToolExecution));

    executor.clear();
    assert!(!executor.has_hooks(&HookEvent::PostToolExecution));
}

#[test]
fn test_hook_executor_registered_events() {
    let mut executor = HookExecutor::new();
    executor.register(HookEvent::PreToolExecution, HookConfig::new("/a.sh"));
    executor.register(HookEvent::SessionStart, HookConfig::new("/b.sh"));

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
