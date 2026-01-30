// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

#[test]
fn test_thinking_dialog_mid_conversation() {
    // Default new() is not mid-conversation
    let dialog = ThinkingDialog::new(true);
    assert!(!dialog.is_mid_conversation);

    // with_mid_conversation sets the flag
    let dialog = ThinkingDialog::with_mid_conversation(true, true);
    assert!(dialog.is_mid_conversation);
    assert_eq!(dialog.selected, ThinkingMode::Enabled);
    assert_eq!(dialog.current, ThinkingMode::Enabled);

    // with_mid_conversation false
    let dialog = ThinkingDialog::with_mid_conversation(false, false);
    assert!(!dialog.is_mid_conversation);
    assert_eq!(dialog.selected, ThinkingMode::Disabled);
}
