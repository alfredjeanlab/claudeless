// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Permission bypass flag handling.
//!
//! This module implements the validation logic for `--dangerously-skip-permissions`
//! and `--allow-dangerously-skip-permissions` flags, matching real Claude's behavior.

/// Result of permission bypass validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BypassValidation {
    /// Bypass enabled and allowed
    Enabled,
    /// Bypass not requested
    Disabled,
    /// Bypass requested but not allowed (error condition)
    NotAllowed,
}

/// Permission bypass handler.
///
/// Validates the combination of `--allow-dangerously-skip-permissions` and
/// `--dangerously-skip-permissions` flags. The bypass flag requires the allow
/// flag as a safety measure.
#[derive(Debug, Clone)]
pub struct PermissionBypass {
    allow_bypass: bool,
    bypass_requested: bool,
}

impl PermissionBypass {
    /// Create a new bypass handler from flag values.
    pub fn new(allow_bypass: bool, bypass_requested: bool) -> Self {
        Self {
            allow_bypass,
            bypass_requested,
        }
    }

    /// Validate bypass configuration.
    ///
    /// Returns:
    /// - `Enabled` if both flags are set
    /// - `Disabled` if bypass not requested (regardless of allow flag)
    /// - `NotAllowed` if bypass requested without allow flag (error)
    pub fn validate(&self) -> BypassValidation {
        match (self.bypass_requested, self.allow_bypass) {
            (true, true) => BypassValidation::Enabled,
            (true, false) => BypassValidation::NotAllowed,
            (false, _) => BypassValidation::Disabled,
        }
    }

    /// Check if bypass is active (enabled and allowed).
    pub fn is_active(&self) -> bool {
        matches!(self.validate(), BypassValidation::Enabled)
    }

    /// Check if bypass was requested but not allowed.
    pub fn is_not_allowed(&self) -> bool {
        matches!(self.validate(), BypassValidation::NotAllowed)
    }

    /// Error message for NotAllowed state.
    pub fn error_message() -> &'static str {
        "Error: --dangerously-skip-permissions requires --allow-dangerously-skip-permissions to be set.\n\
         This is a safety measure. Only use this in sandboxed environments with no internet access."
    }
}

impl Default for PermissionBypass {
    fn default() -> Self {
        Self::new(false, false)
    }
}

#[cfg(test)]
#[path = "bypass_tests.rs"]
mod tests;
