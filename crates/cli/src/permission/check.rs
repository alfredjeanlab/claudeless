// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Permission checking logic for tool execution.

use super::bypass::PermissionBypass;
use super::mode::PermissionMode;
use super::pattern::PermissionPatterns;
use crate::config::ToolConfig;
use std::collections::HashMap;

/// Permission check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    /// Permission granted
    Allowed,
    /// Permission denied
    Denied { reason: String },
    /// Need to prompt user/hook
    NeedsPrompt { tool: String, action: String },
}

/// Permission checker for tool execution.
///
/// Combines permission mode, bypass configuration, settings patterns,
/// and scenario overrides to determine whether tool operations should be allowed.
///
/// Priority order (highest to lowest):
/// 1. Bypass flags (--dangerously-skip-permissions)
/// 2. Scenario tool_execution.tools overrides
/// 3. Settings permissions.deny (explicit deny)
/// 4. Settings permissions.allow (auto-approve)
/// 5. Permission mode (default, plan, accept-edits, etc.)
pub struct PermissionChecker {
    mode: PermissionMode,
    bypass: PermissionBypass,
    /// Patterns from settings files
    settings_patterns: PermissionPatterns,
    /// Per-tool overrides from scenario (highest priority after bypass)
    scenario_overrides: HashMap<String, ToolConfig>,
}

impl PermissionChecker {
    /// Create from mode and bypass configuration.
    pub fn new(mode: PermissionMode, bypass: PermissionBypass) -> Self {
        Self {
            mode,
            bypass,
            settings_patterns: PermissionPatterns::default(),
            scenario_overrides: HashMap::new(),
        }
    }

    /// Create with mode, bypass, and settings patterns.
    pub fn with_patterns(
        mode: PermissionMode,
        bypass: PermissionBypass,
        settings_patterns: PermissionPatterns,
    ) -> Self {
        Self {
            mode,
            bypass,
            settings_patterns,
            scenario_overrides: HashMap::new(),
        }
    }

    /// Add scenario tool overrides (highest priority after bypass).
    pub fn with_scenario_overrides(mut self, overrides: HashMap<String, ToolConfig>) -> Self {
        self.scenario_overrides = overrides;
        self
    }

    /// Check if a tool action is allowed.
    ///
    /// Returns:
    /// - `Allowed` if the action is permitted
    /// - `Denied` if the action is not permitted
    /// - `NeedsPrompt` if the action requires user/hook confirmation
    pub fn check(&self, tool_name: &str, action: &str) -> PermissionResult {
        self.check_with_input(tool_name, action, None)
    }

    /// Check if a tool action is allowed, with optional tool input for pattern matching.
    ///
    /// Priority order (highest to lowest):
    /// 1. Bypass flags (--dangerously-skip-permissions)
    /// 2. Scenario tool_execution.tools overrides
    /// 3. Settings permissions.deny (explicit deny)
    /// 4. Settings permissions.allow (auto-approve)
    /// 5. Permission mode (default, plan, accept-edits, etc.)
    pub fn check_with_input(
        &self,
        tool_name: &str,
        action: &str,
        tool_input: Option<&str>,
    ) -> PermissionResult {
        // 1. Bypass overrides everything
        if self.bypass.is_active() {
            return PermissionResult::Allowed;
        }

        // 2. Scenario overrides take next priority
        if let Some(config) = self.scenario_overrides.get(tool_name) {
            if config.auto_approve {
                return PermissionResult::Allowed;
            }
            if let Some(ref error) = config.error {
                return PermissionResult::Denied {
                    reason: error.clone(),
                };
            }
        }

        // 3. Settings deny patterns
        if self.settings_patterns.is_denied(tool_name, tool_input) {
            return PermissionResult::Denied {
                reason: format!("Tool {} is denied by settings", tool_name),
            };
        }

        // 4. Settings allow patterns - auto-approve
        if self.settings_patterns.is_allowed(tool_name, tool_input) {
            return PermissionResult::Allowed;
        }

        // 5. Fall back to mode-based checking
        self.check_by_mode(tool_name, action)
    }

    /// Check permission based on mode only.
    fn check_by_mode(&self, tool_name: &str, action: &str) -> PermissionResult {
        match self.mode {
            PermissionMode::BypassPermissions => PermissionResult::Allowed,
            PermissionMode::AcceptEdits if is_edit_action(action) => PermissionResult::Allowed,
            PermissionMode::DontAsk => PermissionResult::Denied {
                reason: "Permission denied in DontAsk mode".into(),
            },
            PermissionMode::Plan => PermissionResult::Denied {
                reason: "Execution not allowed in Plan mode".into(),
            },
            PermissionMode::Delegate | PermissionMode::Default | PermissionMode::AcceptEdits => {
                PermissionResult::NeedsPrompt {
                    tool: tool_name.into(),
                    action: action.into(),
                }
            }
        }
    }

    /// Check if permissions are entirely bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypass.is_active() || self.mode == PermissionMode::BypassPermissions
    }

    /// Get the current permission mode.
    pub fn mode(&self) -> &PermissionMode {
        &self.mode
    }

    /// Get effective settings patterns (for inspection).
    pub fn settings_patterns(&self) -> &PermissionPatterns {
        &self.settings_patterns
    }
}

/// Check if an action is considered an edit operation.
fn is_edit_action(action: &str) -> bool {
    matches!(
        action.to_lowercase().as_str(),
        "edit" | "write" | "create" | "delete" | "modify"
    )
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;
