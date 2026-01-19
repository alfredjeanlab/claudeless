// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook execution engine.

use super::protocol::{HookEvent, HookMessage, HookResponse};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Configuration for a single hook
#[derive(Clone, Debug)]
pub struct HookConfig {
    /// Path to hook script
    pub script_path: PathBuf,

    /// Timeout in milliseconds
    pub timeout_ms: u64,

    /// Whether hook failure should block execution
    pub blocking: bool,
}

impl HookConfig {
    /// Create a new hook config
    pub fn new(script_path: impl Into<PathBuf>) -> Self {
        Self {
            script_path: script_path.into(),
            timeout_ms: 5000,
            blocking: false,
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set blocking
    pub fn with_blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }
}

/// Hook executor that runs hook scripts
pub struct HookExecutor {
    /// Registered hooks by event
    hooks: HashMap<HookEvent, Vec<HookConfig>>,
}

impl HookExecutor {
    /// Create a new hook executor
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Register a hook for an event
    pub fn register(&mut self, event: HookEvent, config: HookConfig) {
        self.hooks.entry(event).or_default().push(config);
    }

    /// Execute all hooks for an event
    pub async fn execute(&self, message: &HookMessage) -> Result<Vec<HookResponse>, HookError> {
        let hooks = match self.hooks.get(&message.event) {
            Some(h) => h,
            None => return Ok(vec![]),
        };

        let mut responses = Vec::new();
        for hook in hooks {
            let response = self.execute_hook(hook, message).await?;

            // If blocking hook returns proceed=false, stop processing
            if hook.blocking && !response.proceed {
                responses.push(response);
                break;
            }

            responses.push(response);
        }

        Ok(responses)
    }

    /// Execute a single hook
    async fn execute_hook(
        &self,
        config: &HookConfig,
        message: &HookMessage,
    ) -> Result<HookResponse, HookError> {
        let message_json =
            serde_json::to_string(message).map_err(|e| HookError::Serialization(e.to_string()))?;

        let mut child = Command::new(&config.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| HookError::Spawn(e.to_string()))?;

        // Write message to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(message_json.as_bytes())
                .await
                .map_err(|e| HookError::Io(e.to_string()))?;
            drop(stdin); // Close stdin to signal EOF
        }

        // Wait with timeout
        let timeout = std::time::Duration::from_millis(config.timeout_ms);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| HookError::Timeout)?
            .map_err(|e| HookError::Io(e.to_string()))?;

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HookError::NonZeroExit {
                code: output.status.code(),
                stderr: stderr.to_string(),
            });
        }

        // Parse response
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            // No output means proceed
            Ok(HookResponse::proceed())
        } else {
            serde_json::from_str(stdout.trim())
                .map_err(|e| HookError::InvalidResponse(e.to_string()))
        }
    }

    /// Check if any hooks are registered for an event
    pub fn has_hooks(&self, event: &HookEvent) -> bool {
        self.hooks.get(event).is_some_and(|h| !h.is_empty())
    }

    /// Get hook count for an event
    pub fn hook_count(&self, event: &HookEvent) -> usize {
        self.hooks.get(event).map(|h| h.len()).unwrap_or(0)
    }

    /// Get all registered events
    pub fn registered_events(&self) -> Vec<&HookEvent> {
        self.hooks.keys().collect()
    }

    /// Clear all hooks
    pub fn clear(&mut self) {
        self.hooks.clear();
    }

    /// Clear hooks for a specific event
    pub fn clear_event(&mut self, event: &HookEvent) {
        self.hooks.remove(event);
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("Failed to serialize hook message: {0}")]
    Serialization(String),

    #[error("Failed to spawn hook script: {0}")]
    Spawn(String),

    #[error("Hook I/O error: {0}")]
    Io(String),

    #[error("Hook execution timed out")]
    Timeout,

    #[error("Hook exited with non-zero status (code: {code:?}): {stderr}")]
    NonZeroExit { code: Option<i32>, stderr: String },

    #[error("Invalid hook response: {0}")]
    InvalidResponse(String),
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
