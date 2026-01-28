// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =============================================================================
// HelpTab Tests
// =============================================================================

#[test]
fn test_help_tab_next_cycles_forward() {
    assert_eq!(HelpTab::General.next(), HelpTab::Commands);
    assert_eq!(HelpTab::Commands.next(), HelpTab::CustomCommands);
    assert_eq!(HelpTab::CustomCommands.next(), HelpTab::General);
}

#[test]
fn test_help_tab_prev_cycles_backward() {
    assert_eq!(HelpTab::General.prev(), HelpTab::CustomCommands);
    assert_eq!(HelpTab::Commands.prev(), HelpTab::General);
    assert_eq!(HelpTab::CustomCommands.prev(), HelpTab::Commands);
}

#[test]
fn test_help_tab_name() {
    assert_eq!(HelpTab::General.name(), "general");
    assert_eq!(HelpTab::Commands.name(), "commands");
    assert_eq!(HelpTab::CustomCommands.name(), "custom-commands");
}

#[test]
fn test_help_tab_all() {
    let all = HelpTab::all();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0], HelpTab::General);
    assert_eq!(all[1], HelpTab::Commands);
    assert_eq!(all[2], HelpTab::CustomCommands);
}

#[test]
fn test_help_tab_default_is_general() {
    assert_eq!(HelpTab::default(), HelpTab::General);
}

// =============================================================================
// HelpDialog Tests
// =============================================================================

#[test]
fn test_help_dialog_new() {
    let dialog = HelpDialog::new("1.2.3".to_string());
    assert_eq!(dialog.version, "1.2.3");
    assert_eq!(dialog.active_tab, HelpTab::General);
    assert_eq!(dialog.commands_selected, 0);
    assert_eq!(dialog.custom_selected, 0);
}

#[test]
fn test_help_dialog_default() {
    let dialog = HelpDialog::default();
    assert_eq!(dialog.version, "2.1.12");
    assert_eq!(dialog.active_tab, HelpTab::General);
}

#[test]
fn test_help_dialog_next_tab() {
    let mut dialog = HelpDialog::default();
    assert_eq!(dialog.active_tab, HelpTab::General);

    dialog.next_tab();
    assert_eq!(dialog.active_tab, HelpTab::Commands);

    dialog.next_tab();
    assert_eq!(dialog.active_tab, HelpTab::CustomCommands);

    dialog.next_tab();
    assert_eq!(dialog.active_tab, HelpTab::General);
}

#[test]
fn test_help_dialog_prev_tab() {
    let mut dialog = HelpDialog::default();
    assert_eq!(dialog.active_tab, HelpTab::General);

    dialog.prev_tab();
    assert_eq!(dialog.active_tab, HelpTab::CustomCommands);

    dialog.prev_tab();
    assert_eq!(dialog.active_tab, HelpTab::Commands);

    dialog.prev_tab();
    assert_eq!(dialog.active_tab, HelpTab::General);
}

#[test]
fn test_help_dialog_select_next_wraps() {
    let mut dialog = HelpDialog {
        active_tab: HelpTab::Commands,
        ..Default::default()
    };

    // Start at 0
    assert_eq!(dialog.commands_selected, 0);

    // Move to 1
    dialog.select_next(5);
    assert_eq!(dialog.commands_selected, 1);

    // Move to 4
    dialog.select_next(5);
    dialog.select_next(5);
    dialog.select_next(5);
    assert_eq!(dialog.commands_selected, 4);

    // Wrap to 0
    dialog.select_next(5);
    assert_eq!(dialog.commands_selected, 0);
}

#[test]
fn test_help_dialog_select_prev_wraps() {
    let mut dialog = HelpDialog {
        active_tab: HelpTab::Commands,
        ..Default::default()
    };

    // Start at 0
    assert_eq!(dialog.commands_selected, 0);

    // Wrap to last (4)
    dialog.select_prev(5);
    assert_eq!(dialog.commands_selected, 4);

    // Move back to 3
    dialog.select_prev(5);
    assert_eq!(dialog.commands_selected, 3);
}

#[test]
fn test_help_dialog_select_does_nothing_on_general_tab() {
    let mut dialog = HelpDialog::default();
    assert_eq!(dialog.active_tab, HelpTab::General);
    assert_eq!(dialog.commands_selected, 0);

    // These should have no effect on General tab
    dialog.select_next(5);
    assert_eq!(dialog.commands_selected, 0);

    dialog.select_prev(5);
    assert_eq!(dialog.commands_selected, 0);
}

#[test]
fn test_help_dialog_select_with_zero_commands() {
    let mut dialog = HelpDialog {
        active_tab: HelpTab::Commands,
        ..Default::default()
    };

    // With 0 commands, selection should not change
    dialog.select_next(0);
    assert_eq!(dialog.commands_selected, 0);

    dialog.select_prev(0);
    assert_eq!(dialog.commands_selected, 0);
}

#[test]
fn test_help_dialog_selection_persists_across_tab_changes() {
    let mut dialog = HelpDialog::default();

    // Go to Commands tab and select item 3
    dialog.next_tab(); // General -> Commands
    assert_eq!(dialog.active_tab, HelpTab::Commands);
    dialog.select_next(10);
    dialog.select_next(10);
    dialog.select_next(10);
    assert_eq!(dialog.commands_selected, 3);

    // Switch tabs and back
    dialog.next_tab();
    dialog.next_tab();
    assert_eq!(dialog.active_tab, HelpTab::General);

    dialog.next_tab();
    assert_eq!(dialog.active_tab, HelpTab::Commands);

    // Selection should persist
    assert_eq!(dialog.commands_selected, 3);
}
