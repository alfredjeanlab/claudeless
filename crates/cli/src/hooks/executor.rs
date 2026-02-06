// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hook execution engine.

use super::protocol::{HookEvent, HookMessage, HookPayload, HookResponse};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Configuration for a single hook
#[derive(Clone, Debug, Default)]
pub struct HookConfig {
    /// Path to hook script
    pub script_path: PathBuf,

    /// Timeout in milliseconds
    pub timeout_ms: u64,

    /// Whether hook failure should block execution
    pub blocking: bool,

    /// Optional pipe-separated matcher pattern for sub-event filtering.
    /// For Notification hooks, matches against notification_type.
    pub matcher: Option<String>,
}

impl HookConfig {
    /// Create a new hook config with specified timeout
    pub fn new(script_path: impl Into<PathBuf>, default_timeout_ms: u64) -> Self {
        Self {
            script_path: script_path.into(),
            timeout_ms: default_timeout_ms,
            blocking: false,
            matcher: None,
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

    /// Set matcher pattern
    pub fn with_matcher(mut self, matcher: Option<String>) -> Self {
        self.matcher = matcher;
        self
    }
}

/// Hook executor that runs hook scripts
#[derive(Clone, Debug, Default)]
pub struct HookExecutor {
    /// Registered hooks by event
    hooks: HashMap<HookEvent, Vec<HookConfig>>,
    /// Working directory (common hook field)
    cwd: Option<String>,
    /// Transcript JSONL path (common hook field)
    transcript_path: Option<String>,
    /// Permission mode (common hook field)
    permission_mode: Option<String>,
}

impl HookExecutor {
    /// Create a new hook executor
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
            cwd: None,
            transcript_path: None,
            permission_mode: None,
        }
    }

    /// Set common context fields injected into every hook payload.
    pub fn with_context(
        mut self,
        cwd: Option<String>,
        transcript_path: Option<String>,
        permission_mode: Option<String>,
    ) -> Self {
        self.cwd = cwd;
        self.transcript_path = transcript_path;
        self.permission_mode = permission_mode;
        self
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
            // Matcher filtering: skip hooks whose pattern doesn't match the sub-event
            if let Some(ref matcher_pattern) = hook.matcher {
                let subject = match &message.payload {
                    // For Notification events, match against notification_type
                    HookPayload::Notification {
                        ref notification_type,
                        ..
                    } => Some(notification_type.as_str()),
                    // For tool events, match against tool_name
                    HookPayload::ToolExecution { ref tool_name, .. } => Some(tool_name.as_str()),
                    _ => None,
                };
                if let Some(subject) = subject {
                    let matches = matcher_pattern
                        .split('|')
                        .any(|segment| segment.trim() == subject);
                    if !matches {
                        continue;
                    }
                }
            }

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
        // Use flat wire format matching real Claude Code protocol
        let mut wire = message.to_wire_json();
        if let Some(obj) = wire.as_object_mut() {
            if let Some(ref cwd) = self.cwd {
                obj.insert("cwd".to_string(), serde_json::Value::String(cwd.clone()));
            }
            if let Some(ref tp) = self.transcript_path {
                obj.insert(
                    "transcript_path".to_string(),
                    serde_json::Value::String(tp.clone()),
                );
            }
            if let Some(ref pm) = self.permission_mode {
                obj.insert(
                    "permission_mode".to_string(),
                    serde_json::Value::String(pm.clone()),
                );
            }
        }
        let message_json = wire.to_string();

        let mut child = Command::new("/bin/bash")
            .arg(&config.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Ensure process is killed if we drop the handle
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

        // Wait with timeout, killing the process if it takes too long
        let timeout_duration = std::time::Duration::from_millis(config.timeout_ms);
        let output = match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
            Ok(result) => result.map_err(|e| HookError::Io(e.to_string()))?,
            Err(_) => {
                // Timeout elapsed - kill_on_drop(true) ensures the process is killed
                // when the child handle is dropped (which happens when this function returns)
                return Err(HookError::Timeout);
            }
        };

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Exit code 2 means "block/deny" — return a block response instead of error
            if output.status.code() == Some(2) {
                return Ok(HookResponse::block(stderr.trim().to_string()));
            }
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
