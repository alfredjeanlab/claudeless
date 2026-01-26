// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
