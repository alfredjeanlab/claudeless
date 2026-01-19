// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook registry for test configuration.

use super::executor::{HookConfig, HookExecutor};
use super::protocol::HookEvent;
use std::io::Write;
use tempfile::TempPath;

/// Hook registry for test configuration
pub struct HookRegistry {
    executor: HookExecutor,
    temp_scripts: Vec<TempPath>,
}

impl HookRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            executor: HookExecutor::new(),
            temp_scripts: Vec::new(),
        }
    }

    /// Register a hook script from a string
    pub fn register_script(
        &mut self,
        event: HookEvent,
        script_content: &str,
        blocking: bool,
    ) -> std::io::Result<()> {
        // Create temp script file
        let mut file = tempfile::Builder::new()
            .prefix("hook_")
            .suffix(".sh")
            .tempfile()?;

        writeln!(file, "#!/bin/bash")?;
        write!(file, "{}", script_content)?;

        let path = file.into_temp_path();

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }

        self.executor.register(
            event,
            HookConfig::new(path.to_path_buf())
                .with_timeout(5000)
                .with_blocking(blocking),
        );

        self.temp_scripts.push(path);
        Ok(())
    }

    /// Register a hook that always proceeds
    pub fn register_passthrough(&mut self, event: HookEvent) -> std::io::Result<()> {
        self.register_script(event, r#"echo '{"proceed": true}'"#, false)
    }

    /// Register a hook that blocks with a reason
    pub fn register_blocking(&mut self, event: HookEvent, reason: &str) -> std::io::Result<()> {
        let script = format!(
            r#"echo '{{"proceed": false, "error": "{}"}}'"#,
            reason.replace('\"', "\\\"").replace('\'', "\\'")
        );
        self.register_script(event, &script, true)
    }

    /// Register a hook that echoes back the input with a modification
    pub fn register_echo(&mut self, event: HookEvent) -> std::io::Result<()> {
        // This hook reads the input and outputs a proceed response with the input as data
        let script = r#"
input=$(cat)
echo "{\"proceed\": true, \"data\": $input}"
"#;
        self.register_script(event, script, false)
    }

    /// Register a hook that logs to a file
    pub fn register_logger(
        &mut self,
        event: HookEvent,
        log_path: &std::path::Path,
    ) -> std::io::Result<()> {
        let script = format!(
            r#"
cat >> {}
echo '{{"proceed": true}}'
"#,
            log_path.display()
        );
        self.register_script(event, &script, false)
    }

    /// Register a hook that delays for a specified duration
    pub fn register_delayed(&mut self, event: HookEvent, delay_secs: f32) -> std::io::Result<()> {
        let script = format!(
            r#"
sleep {}
echo '{{"proceed": true}}'
"#,
            delay_secs
        );
        self.register_script(event, &script, false)
    }

    /// Register a custom inline script
    pub fn register_inline(
        &mut self,
        event: HookEvent,
        script: &str,
        blocking: bool,
    ) -> std::io::Result<()> {
        self.register_script(event, script, blocking)
    }

    /// Get the executor
    pub fn executor(&self) -> &HookExecutor {
        &self.executor
    }

    /// Get mutable executor
    pub fn executor_mut(&mut self) -> &mut HookExecutor {
        &mut self.executor
    }

    /// Check if any hooks are registered for an event
    pub fn has_hooks(&self, event: &HookEvent) -> bool {
        self.executor.has_hooks(event)
    }

    /// Clear all registered hooks
    pub fn clear(&mut self) {
        self.executor.clear();
        self.temp_scripts.clear();
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::protocol::HookMessage;

    #[test]
    fn test_registry_new() {
        let registry = HookRegistry::new();
        assert!(!registry.has_hooks(&HookEvent::PreToolExecution));
    }

    #[tokio::test]
    async fn test_register_passthrough() {
        let mut registry = HookRegistry::new();
        registry
            .register_passthrough(HookEvent::PreToolExecution)
            .unwrap();

        assert!(registry.has_hooks(&HookEvent::PreToolExecution));

        let message = HookMessage::tool_execution(
            "test_session",
            HookEvent::PreToolExecution,
            "Bash",
            serde_json::json!({"command": "ls"}),
            None,
        );

        let responses = registry.executor().execute(&message).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses[0].proceed);
    }

    #[tokio::test]
    async fn test_register_blocking() {
        let mut registry = HookRegistry::new();
        registry
            .register_blocking(HookEvent::PreToolExecution, "Not allowed")
            .unwrap();

        let message = HookMessage::tool_execution(
            "test_session",
            HookEvent::PreToolExecution,
            "Bash",
            serde_json::json!({"command": "rm -rf /"}),
            None,
        );

        let responses = registry.executor().execute(&message).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(!responses[0].proceed);
        assert_eq!(responses[0].error, Some("Not allowed".to_string()));
    }

    #[tokio::test]
    async fn test_register_echo() {
        let mut registry = HookRegistry::new();
        registry.register_echo(HookEvent::PromptSubmit).unwrap();

        let message = HookMessage::prompt_submit("test_session", "hello world");

        let responses = registry.executor().execute(&message).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses[0].proceed);
        assert!(responses[0].data.is_some());
    }

    #[tokio::test]
    async fn test_register_logger() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("hook.log");

        let mut registry = HookRegistry::new();
        registry
            .register_logger(HookEvent::SessionStart, &log_path)
            .unwrap();

        let message = HookMessage::session("test_session", HookEvent::SessionStart, None);

        let responses = registry.executor().execute(&message).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses[0].proceed);

        // Check that log file was written
        let log_content = std::fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("session_start"));
    }

    #[tokio::test]
    async fn test_register_inline() {
        let mut registry = HookRegistry::new();
        registry
            .register_inline(
                HookEvent::PreToolExecution,
                r#"echo '{"proceed": true, "data": {"custom": "value"}}'"#,
                false,
            )
            .unwrap();

        let message = HookMessage::tool_execution(
            "test",
            HookEvent::PreToolExecution,
            "Test",
            serde_json::json!({}),
            None,
        );

        let responses = registry.executor().execute(&message).await.unwrap();
        assert!(responses[0].proceed);
        let data = responses[0].data.as_ref().unwrap();
        assert_eq!(data["custom"], "value");
    }

    #[test]
    fn test_clear() {
        let mut registry = HookRegistry::new();
        registry
            .register_passthrough(HookEvent::PreToolExecution)
            .unwrap();
        registry
            .register_passthrough(HookEvent::PostToolExecution)
            .unwrap();

        assert!(registry.has_hooks(&HookEvent::PreToolExecution));
        registry.clear();
        assert!(!registry.has_hooks(&HookEvent::PreToolExecution));
        assert!(!registry.has_hooks(&HookEvent::PostToolExecution));
    }

    #[tokio::test]
    async fn test_multiple_hooks_same_event() {
        let mut registry = HookRegistry::new();

        // Register two non-blocking hooks
        registry
            .register_passthrough(HookEvent::PreToolExecution)
            .unwrap();
        registry
            .register_passthrough(HookEvent::PreToolExecution)
            .unwrap();

        let message = HookMessage::tool_execution(
            "test",
            HookEvent::PreToolExecution,
            "Test",
            serde_json::json!({}),
            None,
        );

        let responses = registry.executor().execute(&message).await.unwrap();
        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn test_blocking_stops_processing() {
        let mut registry = HookRegistry::new();

        // First hook blocks
        registry
            .register_blocking(HookEvent::PreToolExecution, "Blocked")
            .unwrap();
        // Second hook would proceed, but should never execute
        registry
            .register_passthrough(HookEvent::PreToolExecution)
            .unwrap();

        let message = HookMessage::tool_execution(
            "test",
            HookEvent::PreToolExecution,
            "Test",
            serde_json::json!({}),
            None,
        );

        let responses = registry.executor().execute(&message).await.unwrap();
        // Only one response because blocking hook stopped processing
        assert_eq!(responses.len(), 1);
        assert!(!responses[0].proceed);
    }
}
