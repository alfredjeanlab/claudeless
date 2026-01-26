// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_harness_creation() {
    let harness = TuiTestHarness::new();
    assert!(matches!(harness.app_state.mode, AppMode::Input));
    assert!(harness.app_state.input_buffer.is_empty());
}

#[test]
fn test_harness_type_input() {
    let mut harness = TuiTestHarness::new();
    harness.type_input("test");
    assert_eq!(harness.app_state.input_buffer, "test");
    assert_eq!(harness.app_state.cursor_pos, 4);
}

#[test]
fn test_harness_enter_adds_to_history() {
    let mut harness = TuiTestHarness::new();
    harness.type_input("command");
    harness.press_enter();
    assert_eq!(harness.app_state.history.len(), 1);
    assert_eq!(harness.app_state.history[0], "command");
    assert!(harness.app_state.input_buffer.is_empty());
}

#[test]
fn test_ctrl_c_clears_input_and_shows_hint() {
    let mut harness = TuiTestHarness::new();

    harness.type_input("Some text");
    assert!(!harness.app_state.input_buffer.is_empty());

    harness.press_ctrl_c();
    assert!(harness.app_state.input_buffer.is_empty());
    assert!(!harness.app_state.should_exit);
    assert_eq!(harness.app_state.exit_hint, Some(ExitHint::CtrlC));
}

#[test]
fn test_ctrl_c_shows_hint_on_empty() {
    let mut harness = TuiTestHarness::new();

    // First Ctrl+C shows exit hint
    harness.press_ctrl_c();
    assert!(!harness.app_state.should_exit);
    assert_eq!(harness.app_state.exit_hint, Some(ExitHint::CtrlC));
}

#[test]
fn test_double_ctrl_c_exits() {
    let mut harness = TuiTestHarness::new();

    // First Ctrl+C shows exit hint
    harness.press_ctrl_c();
    assert!(!harness.app_state.should_exit);

    // Second Ctrl+C exits
    harness.press_ctrl_c();
    assert!(harness.app_state.should_exit);
    assert!(matches!(
        harness.app_state.exit_reason,
        Some(ExitReason::Interrupted)
    ));
}

#[test]
fn test_history_navigation() {
    let mut harness = TuiTestHarness::new();

    harness.type_input("first");
    harness.press_enter();

    harness.type_input("second");
    harness.press_enter();

    assert!(harness.app_state.input_buffer.is_empty());
    assert_eq!(harness.app_state.history.len(), 2);
}

#[test]
fn test_backspace() {
    let mut harness = TuiTestHarness::new();

    harness.type_input("hello");
    harness.press_backspace();
    assert_eq!(harness.app_state.input_buffer, "hell");
    assert_eq!(harness.app_state.cursor_pos, 4);
}

#[test]
fn test_cursor_movement() {
    let mut harness = TuiTestHarness::new();

    harness.type_input("hello");
    assert_eq!(harness.app_state.cursor_pos, 5);

    harness.press_left();
    assert_eq!(harness.app_state.cursor_pos, 4);

    harness.press_left();
    harness.press_left();
    assert_eq!(harness.app_state.cursor_pos, 2);

    harness.press_right();
    assert_eq!(harness.app_state.cursor_pos, 3);

    harness.press_home();
    assert_eq!(harness.app_state.cursor_pos, 0);

    harness.press_end();
    assert_eq!(harness.app_state.cursor_pos, 5);
}
