// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Permission mode enum matching real Claude's --permission-mode flag.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Permission handling modes matching real Claude.
///
/// These modes control how tool execution permissions are handled:
/// - `Default`: Interactive prompts for each tool use
/// - `AcceptEdits`: Auto-allow file edit operations
/// - `BypassPermissions`: Skip all permission checks
/// - `Delegate`: Use hooks for permission decisions
/// - `DontAsk`: Deny operations that would require permission
/// - `Plan`: Plan mode (no execution allowed)
#[derive(Clone, Debug, Default, ValueEnum, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Accept file edits automatically
    AcceptEdits,
    /// Bypass all permission checks (requires allow flag)
    BypassPermissions,
    /// Default interactive permission prompts
    #[default]
    Default,
    /// Delegate decisions to hooks
    Delegate,
    /// Don't ask, deny by default
    DontAsk,
    /// Plan mode (no execution)
    Plan,
}

impl PermissionMode {
    /// Check if this mode allows all operations without prompting.
    pub fn allows_all(&self) -> bool {
        matches!(self, Self::BypassPermissions)
    }

    /// Check if this mode denies all operations by default.
    pub fn denies_all(&self) -> bool {
        matches!(self, Self::DontAsk | Self::Plan)
    }

    /// Check if this mode auto-accepts edit operations.
    pub fn accepts_edits(&self) -> bool {
        matches!(self, Self::AcceptEdits | Self::BypassPermissions)
    }

    /// Cycle to the next permission mode (for TUI shift+tab)
    pub fn cycle_next(&self) -> Self {
        match self {
            Self::Default => Self::Plan,
            Self::Plan => Self::AcceptEdits,
            Self::AcceptEdits => Self::BypassPermissions,
            Self::BypassPermissions => Self::Default,
            Self::Delegate => Self::Default,
            Self::DontAsk => Self::Default,
        }
    }

    /// Get the display name for this mode (for TUI status bar)
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Plan => "plan",
            Self::AcceptEdits => "accept edits",
            Self::BypassPermissions => "bypass permissions",
            Self::Delegate => "delegate",
            Self::DontAsk => "dont ask",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        assert_eq!(PermissionMode::default(), PermissionMode::Default);
    }

    #[test]
    fn test_allows_all() {
        assert!(PermissionMode::BypassPermissions.allows_all());
        assert!(!PermissionMode::Default.allows_all());
        assert!(!PermissionMode::AcceptEdits.allows_all());
    }

    #[test]
    fn test_denies_all() {
        assert!(PermissionMode::DontAsk.denies_all());
        assert!(PermissionMode::Plan.denies_all());
        assert!(!PermissionMode::Default.denies_all());
    }

    #[test]
    fn test_accepts_edits() {
        assert!(PermissionMode::AcceptEdits.accepts_edits());
        assert!(PermissionMode::BypassPermissions.accepts_edits());
        assert!(!PermissionMode::Default.accepts_edits());
        assert!(!PermissionMode::DontAsk.accepts_edits());
    }

    #[test]
    fn test_value_enum_parsing() {
        // Test that all variants can be parsed from their kebab-case names
        assert_eq!(
            PermissionMode::from_str("accept-edits", true).unwrap(),
            PermissionMode::AcceptEdits
        );
        assert_eq!(
            PermissionMode::from_str("bypass-permissions", true).unwrap(),
            PermissionMode::BypassPermissions
        );
        assert_eq!(
            PermissionMode::from_str("default", true).unwrap(),
            PermissionMode::Default
        );
        assert_eq!(
            PermissionMode::from_str("delegate", true).unwrap(),
            PermissionMode::Delegate
        );
        assert_eq!(
            PermissionMode::from_str("dont-ask", true).unwrap(),
            PermissionMode::DontAsk
        );
        assert_eq!(
            PermissionMode::from_str("plan", true).unwrap(),
            PermissionMode::Plan
        );
    }
}
