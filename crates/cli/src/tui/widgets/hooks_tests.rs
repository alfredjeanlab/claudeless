// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn hook_type_all_returns_14_types() {
    let all = HookType::all();
    assert_eq!(all.len(), 14);
    // Verify first and last
    assert_eq!(all[0], HookType::PreToolUse);
    assert_eq!(all[13], HookType::DisableAllHooks);
}

#[test]
fn hook_type_names_are_correct() {
    assert_eq!(HookType::PreToolUse.name(), "PreToolUse");
    assert_eq!(HookType::PostToolUse.name(), "PostToolUse");
    assert_eq!(HookType::DisableAllHooks.name(), "Disable all hooks");
}

#[test]
fn hook_type_descriptions_are_correct() {
    assert_eq!(HookType::PreToolUse.description(), "Before tool execution");
    assert_eq!(HookType::PostToolUse.description(), "After tool execution");
    assert_eq!(
        HookType::UserPromptSubmit.description(),
        "When the user submits a prompt"
    );
}

#[test]
fn hook_type_has_matchers() {
    assert!(HookType::PreToolUse.has_matchers());
    assert!(HookType::PostToolUse.has_matchers());
    assert!(HookType::PostToolUseFailure.has_matchers());
    assert!(!HookType::Notification.has_matchers());
    assert!(!HookType::SessionStart.has_matchers());
}

#[test]
fn hooks_dialog_default_state() {
    let dialog = HooksDialog::default();
    assert_eq!(dialog.selected_index, 0);
    assert_eq!(dialog.view, HooksView::HookList);
    assert_eq!(dialog.selected_hook, None);
    assert_eq!(dialog.active_hook_count, 4);
}

#[test]
fn hooks_dialog_select_next_increments() {
    let mut dialog = HooksDialog::new(4);
    assert_eq!(dialog.selected_index, 0);

    dialog.select_next();
    assert_eq!(dialog.selected_index, 1);

    dialog.select_next();
    assert_eq!(dialog.selected_index, 2);
}

#[test]
fn hooks_dialog_select_next_wraps() {
    let mut dialog = HooksDialog::new(4);
    dialog.selected_index = 13; // Last item

    dialog.select_next();
    assert_eq!(dialog.selected_index, 0); // Wraps to first
    assert_eq!(dialog.scroll_offset, 0); // Scroll resets
}

#[test]
fn hooks_dialog_select_prev_decrements() {
    let mut dialog = HooksDialog::new(4);
    dialog.selected_index = 2;

    dialog.select_prev();
    assert_eq!(dialog.selected_index, 1);

    dialog.select_prev();
    assert_eq!(dialog.selected_index, 0);
}

#[test]
fn hooks_dialog_select_prev_wraps() {
    let mut dialog = HooksDialog::new(4);
    assert_eq!(dialog.selected_index, 0);

    dialog.select_prev();
    assert_eq!(dialog.selected_index, 13); // Wraps to last
                                           // Scroll should be at the bottom
    assert_eq!(dialog.scroll_offset, 14 - dialog.visible_count);
}

#[test]
fn hooks_dialog_scroll_offset_updates_on_navigate_down() {
    let mut dialog = HooksDialog::new(4);
    dialog.visible_count = 5;

    // Navigate down past visible area
    for _ in 0..5 {
        dialog.select_next();
    }
    // selected_index is now 5, visible is 0-4, so scroll offset should update
    assert!(dialog.scroll_offset > 0);
}

#[test]
fn hooks_dialog_scroll_offset_updates_on_navigate_up() {
    let mut dialog = HooksDialog::new(4);
    dialog.visible_count = 5;
    dialog.selected_index = 6;
    dialog.scroll_offset = 2;

    // Navigate up to item below visible area
    dialog.select_prev(); // Now at 5
    dialog.select_prev(); // Now at 4
    dialog.select_prev(); // Now at 3
    dialog.select_prev(); // Now at 2
    dialog.select_prev(); // Now at 1, should scroll

    assert!(dialog.scroll_offset <= dialog.selected_index);
}

#[test]
fn hooks_dialog_open_matchers() {
    let mut dialog = HooksDialog::new(4);
    assert_eq!(dialog.view, HooksView::HookList);
    assert_eq!(dialog.selected_hook, None);

    dialog.open_matchers();

    assert_eq!(dialog.view, HooksView::Matchers);
    assert_eq!(dialog.selected_hook, Some(HookType::PreToolUse));
    assert_eq!(dialog.matcher_selected, 0);
}

#[test]
fn hooks_dialog_close_matchers() {
    let mut dialog = HooksDialog::new(4);
    dialog.open_matchers();
    assert_eq!(dialog.view, HooksView::Matchers);

    dialog.close_matchers();

    assert_eq!(dialog.view, HooksView::HookList);
    assert_eq!(dialog.selected_hook, None);
}

#[test]
fn hooks_dialog_has_more_below() {
    let mut dialog = HooksDialog::new(4);
    dialog.visible_count = 5;
    dialog.scroll_offset = 0;

    // With 14 items and 5 visible, should have more below
    assert!(dialog.has_more_below());

    // Scroll to bottom
    dialog.scroll_offset = 14 - 5;
    assert!(!dialog.has_more_below());
}

#[test]
fn hooks_dialog_selected_hook_type() {
    let mut dialog = HooksDialog::new(4);
    assert_eq!(dialog.selected_hook_type(), HookType::PreToolUse);

    dialog.selected_index = 1;
    assert_eq!(dialog.selected_hook_type(), HookType::PostToolUse);

    dialog.selected_index = 4;
    assert_eq!(dialog.selected_hook_type(), HookType::UserPromptSubmit);
}
