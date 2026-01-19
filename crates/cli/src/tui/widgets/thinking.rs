// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Thinking toggle dialog widget.
//!
//! Shown when user presses Meta+t to toggle extended thinking mode.
//!
//! Note: Rendering has been moved to app.rs as part of the iocraft migration.
//! This module keeps the types used by the app component.

/// Thinking mode selection
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThinkingMode {
    Enabled,
    Disabled,
}

/// Thinking toggle dialog state
#[derive(Clone, Debug)]
pub struct ThinkingDialog {
    pub selected: ThinkingMode,
    pub current: ThinkingMode,
}

impl ThinkingDialog {
    pub fn new(current_enabled: bool) -> Self {
        let current = if current_enabled {
            ThinkingMode::Enabled
        } else {
            ThinkingMode::Disabled
        };
        Self {
            selected: current.clone(),
            current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinking_dialog_creation_enabled() {
        let dialog = ThinkingDialog::new(true);
        assert_eq!(dialog.selected, ThinkingMode::Enabled);
        assert_eq!(dialog.current, ThinkingMode::Enabled);
    }

    #[test]
    fn test_thinking_dialog_creation_disabled() {
        let dialog = ThinkingDialog::new(false);
        assert_eq!(dialog.selected, ThinkingMode::Disabled);
        assert_eq!(dialog.current, ThinkingMode::Disabled);
    }

    #[test]
    fn test_thinking_mode_toggle() {
        let mut dialog = ThinkingDialog::new(true);
        assert_eq!(dialog.selected, ThinkingMode::Enabled);
        dialog.selected = ThinkingMode::Disabled;
        assert_eq!(dialog.selected, ThinkingMode::Disabled);
    }
}
