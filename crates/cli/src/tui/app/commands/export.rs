// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Export functionality for conversations.
//!
//! Contains:
//! - `do_clipboard_export` - Export conversation to clipboard
//! - `do_file_export` - Export conversation to file

use super::super::state::TuiAppStateInner;
use super::super::types::AppMode;

/// Export conversation to clipboard
pub(in crate::tui::app) fn do_clipboard_export(inner: &mut TuiAppStateInner) {
    // Get conversation content
    let content = format_conversation_for_export(inner);

    // Copy to clipboard
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(&content) {
            Ok(()) => {
                inner.display.response_content = "Conversation copied to clipboard".to_string();
            }
            Err(e) => {
                inner.display.response_content = format!("Failed to copy to clipboard: {}", e);
            }
        },
        Err(e) => {
            inner.display.response_content = format!("Failed to access clipboard: {}", e);
        }
    }

    inner.mode = AppMode::Input;
    inner.dialog.dismiss();
    inner.display.is_command_output = true;
}

/// Export conversation to file
pub(in crate::tui::app) fn do_file_export(inner: &mut TuiAppStateInner) {
    let filename = inner
        .dialog
        .as_export()
        .map(|d| d.filename.clone())
        .unwrap_or_else(|| "conversation.txt".to_string());

    let content = format_conversation_for_export(inner);

    match std::fs::write(&filename, &content) {
        Ok(()) => {
            inner.display.response_content = format!("Conversation exported to: {}", filename);
        }
        Err(e) => {
            inner.display.response_content = format!("Failed to write file: {}", e);
        }
    }

    inner.mode = AppMode::Input;
    inner.dialog.dismiss();
    inner.display.is_command_output = true;
}

/// Format conversation for export
fn format_conversation_for_export(inner: &TuiAppStateInner) -> String {
    // Export the conversation display content
    // This includes the visible conversation history
    inner.display.conversation_display.clone()
}
