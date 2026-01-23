// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn new_dialog_defaults_to_clipboard_selection() {
    let dialog = ExportDialog::new();
    assert_eq!(dialog.step, ExportStep::MethodSelection);
    assert_eq!(dialog.selected_method, ExportMethod::Clipboard);
}

#[test]
fn new_dialog_has_default_filename() {
    let dialog = ExportDialog::new();
    assert!(dialog.filename.starts_with("conversation_"));
    assert!(dialog.filename.ends_with(".txt"));
}

#[test]
fn toggle_method_switches_between_options() {
    let mut dialog = ExportDialog::new();
    assert_eq!(dialog.selected_method, ExportMethod::Clipboard);

    dialog.toggle_method();
    assert_eq!(dialog.selected_method, ExportMethod::File);

    dialog.toggle_method();
    assert_eq!(dialog.selected_method, ExportMethod::Clipboard);
}

#[test]
fn move_selection_up_toggles_method() {
    let mut dialog = ExportDialog::new();
    assert_eq!(dialog.selected_method, ExportMethod::Clipboard);

    dialog.move_selection_up();
    assert_eq!(dialog.selected_method, ExportMethod::File);
}

#[test]
fn move_selection_down_toggles_method() {
    let mut dialog = ExportDialog::new();
    assert_eq!(dialog.selected_method, ExportMethod::Clipboard);

    dialog.move_selection_down();
    assert_eq!(dialog.selected_method, ExportMethod::File);
}

#[test]
fn confirm_clipboard_returns_true() {
    let mut dialog = ExportDialog::new();
    dialog.selected_method = ExportMethod::Clipboard;

    let ready = dialog.confirm_selection();
    assert!(ready);
    assert_eq!(dialog.step, ExportStep::MethodSelection);
}

#[test]
fn confirm_file_advances_to_filename_input() {
    let mut dialog = ExportDialog::new();
    dialog.selected_method = ExportMethod::File;

    let ready = dialog.confirm_selection();
    assert!(!ready);
    assert_eq!(dialog.step, ExportStep::FilenameInput);
}

#[test]
fn confirm_in_filename_step_returns_true() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;

    let ready = dialog.confirm_selection();
    assert!(ready);
}

#[test]
fn go_back_from_filename_returns_to_method_selection() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;

    let cancel = dialog.go_back();
    assert!(!cancel);
    assert_eq!(dialog.step, ExportStep::MethodSelection);
}

#[test]
fn go_back_from_method_selection_cancels_dialog() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::MethodSelection;

    let cancel = dialog.go_back();
    assert!(cancel);
}

#[test]
fn push_char_adds_to_filename() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;
    dialog.filename = "test".to_string();

    dialog.push_char('X');
    assert_eq!(dialog.filename, "testX");
}

#[test]
fn push_char_ignored_in_method_selection() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::MethodSelection;
    let original = dialog.filename.clone();

    dialog.push_char('X');
    assert_eq!(dialog.filename, original);
}

#[test]
fn pop_char_removes_from_filename() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;
    dialog.filename = "test".to_string();

    dialog.pop_char();
    assert_eq!(dialog.filename, "tes");
}

#[test]
fn pop_char_ignored_in_method_selection() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::MethodSelection;
    let original = dialog.filename.clone();

    dialog.pop_char();
    assert_eq!(dialog.filename, original);
}

#[test]
fn filename_accepts_special_characters() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;
    dialog.filename = String::new();

    // Test various special characters that might appear in filenames
    for c in "my-file_2024.01.txt".chars() {
        dialog.push_char(c);
    }
    assert_eq!(dialog.filename, "my-file_2024.01.txt");
}

#[test]
fn filename_handles_unicode_characters() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;
    dialog.filename = String::new();

    // Test Unicode characters
    for c in "対話_记录.txt".chars() {
        dialog.push_char(c);
    }
    assert_eq!(dialog.filename, "対話_记录.txt");

    // Pop should remove last character (which is multibyte)
    dialog.pop_char();
    assert_eq!(dialog.filename, "対話_记录.tx");
}

#[test]
fn filename_handles_empty_after_all_pops() {
    let mut dialog = ExportDialog::new();
    dialog.step = ExportStep::FilenameInput;
    dialog.filename = "ab".to_string();

    dialog.pop_char();
    dialog.pop_char();
    assert_eq!(dialog.filename, "");

    // Further pops should not panic
    dialog.pop_char();
    assert_eq!(dialog.filename, "");
}
