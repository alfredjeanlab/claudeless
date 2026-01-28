// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook registry for test configuration.

use super::executor::{HookConfig, HookExecutor};
use super::protocol::HookEvent;
use std::io::Write;
use tempfile::TempPath;

/// Default hook timeout in milliseconds
const DEFAULT_HOOK_TIMEOUT_MS: u64 = 5000;

/// Hook registry for test configuration
pub struct HookRegistry {
    executor: HookExecutor,
    temp_scripts: Vec<TempPath>,
    default_timeout_ms: u64,
}

impl HookRegistry {
    /// Create a new registry with default timeout
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_HOOK_TIMEOUT_MS)
    }

    /// Create a new registry with custom default timeout
    pub fn with_timeout(default_timeout_ms: u64) -> Self {
        Self {
            executor: HookExecutor::new(),
            temp_scripts: Vec::new(),
            default_timeout_ms,
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
            HookConfig::new(path.to_path_buf(), self.default_timeout_ms).with_blocking(blocking),
        );

        self.temp_scripts.push(path);
        Ok(())
    }

    /// Register a hook that always proceeds
    pub fn register_passthrough(&mut self, event: HookEvent) -> std::io::Result<()> {
        // Must consume stdin first to avoid broken pipe when executor writes
        self.register_script(event, r#"cat > /dev/null; echo '{"proceed": true}'"#, false)
    }

    /// Register a hook that blocks with a reason
    pub fn register_blocking(&mut self, event: HookEvent, reason: &str) -> std::io::Result<()> {
        // Must consume stdin first to avoid broken pipe when executor writes
        let script = format!(
            r#"cat > /dev/null; echo '{{"proceed": false, "error": "{}"}}'"#,
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
        // Must consume stdin first to avoid broken pipe when executor writes
        let script = format!(
            r#"
cat > /dev/null
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
#[path = "registry_tests.rs"]
mod tests;
