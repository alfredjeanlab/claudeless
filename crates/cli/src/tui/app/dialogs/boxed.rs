// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Box-bordered dialogs: tasks and export.
//!
//! Each dialog's rendering and key handling are colocated here.

use iocraft::prelude::*;

use crate::tui::widgets::export::{ExportDialog, ExportMethod, ExportStep};
use crate::tui::widgets::tasks::TasksDialog;

use crate::tui::app::commands::{do_clipboard_export, do_file_export};
use crate::tui::app::state::TuiAppState;
use crate::tui::app::types::AppMode;

/// Build top/bottom box borders for a given total width.
fn box_borders(width: usize) -> (String, String) {
    let inner = width.saturating_sub(2);
    let h = "─".repeat(inner);
    (format!("╭{}╮", h), format!("╰{}╯", h))
}

/// Helper to pad a line to fit inside box borders: `│content   │`
fn pad_line(s: &str, inner_width: usize) -> String {
    let visible_len = s.chars().count();
    let padding = inner_width.saturating_sub(visible_len);
    format!("│{}{}│", s, " ".repeat(padding))
}

// ── Tasks dialog ─────────────────────────────────────────────────────────

/// Render tasks dialog with border
pub(crate) fn render_tasks_dialog(dialog: &TasksDialog, width: usize) -> AnyElement<'static> {
    // Inner width accounts for box borders (│ on each side)
    let inner_width = width.saturating_sub(2);

    // Build content string
    let content = if dialog.is_empty() {
        "No tasks currently running".to_string()
    } else {
        // Format task list with selection indicator
        dialog
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let indicator = if i == dialog.selected_index() {
                    "❯ "
                } else {
                    "  "
                };
                format!("{}{}", indicator, task.description)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let (top_border, bottom_border) = box_borders(width);

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            Text(content: top_border)
            Text(content: pad_line(" Background tasks", inner_width))
            Text(content: pad_line(&format!(" {}", content), inner_width))
            Text(content: bottom_border)
            Text(content: "  ↑/↓ to select · Enter to view · Esc to close")
        }
    }
    .into()
}

impl TuiAppState {
    /// Handle key events in tasks dialog mode
    pub(in crate::tui::app) fn handle_tasks_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            KeyCode::Esc => {
                inner.dismiss_dialog("Background tasks dialog");
            }
            KeyCode::Up => {
                if let Some(dialog) = inner.dialog.as_tasks_mut() {
                    dialog.move_selection_up();
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = inner.dialog.as_tasks_mut() {
                    dialog.move_selection_down();
                }
            }
            KeyCode::Enter => {
                // Future: view selected task details
                // For now, just close the dialog
                inner.mode = AppMode::Input;
                inner.dialog.dismiss();
            }
            _ => {}
        }
    }
}

// ── Export dialog ────────────────────────────────────────────────────────

/// Render export dialog
pub(crate) fn render_export_dialog(dialog: &ExportDialog, width: usize) -> AnyElement<'static> {
    let inner_width = width.saturating_sub(2);
    let (top_border, bottom_border) = box_borders(width);

    match dialog.step {
        ExportStep::MethodSelection => {
            let clipboard_cursor = if dialog.selected_method == ExportMethod::Clipboard {
                "❯"
            } else {
                " "
            };
            let file_cursor = if dialog.selected_method == ExportMethod::File {
                "❯"
            } else {
                " "
            };

            // Format options with descriptions, wrapping long descriptions
            let clipboard_label = "1. Copy to clipboard";
            let file_label = "2. Save to file";
            let clipboard_desc = "Copy the conversation to your system clipboard";
            let file_desc = "Save the conversation to a file in the current directory";

            // Calculate column alignment: both labels padded to same width
            let label_width = clipboard_label.len().max(file_label.len());
            let desc_col = 3 + label_width + 2; // " X " + label + "  "

            let format_option_lines = |cursor: &str, label: &str, desc: &str| -> Vec<String> {
                let label_padded = format!("{:<width$}", label, width = label_width);
                let first_line = format!(" {} {}  {}", cursor, label_padded, desc);

                // Check if description needs wrapping
                let first_visible_len = first_line.chars().count();
                if first_visible_len <= inner_width {
                    vec![first_line]
                } else {
                    // Wrap: find how much fits on first line
                    let avail = inner_width.saturating_sub(desc_col);
                    let desc_words: Vec<&str> = desc.split_whitespace().collect();
                    let mut first_desc = String::new();
                    let mut rest_start = 0;
                    for (i, word) in desc_words.iter().enumerate() {
                        let trial = if first_desc.is_empty() {
                            word.to_string()
                        } else {
                            format!("{} {}", first_desc, word)
                        };
                        if trial.len() <= avail {
                            first_desc = trial;
                            rest_start = i + 1;
                        } else {
                            break;
                        }
                    }
                    let line1 = format!(" {} {}  {}", cursor, label_padded, first_desc);
                    let rest_desc: String = desc_words[rest_start..].join(" ");
                    let indent = " ".repeat(desc_col);
                    let line2 = format!("{}{}", indent, rest_desc);
                    vec![line1, line2]
                }
            };

            let clipboard_lines =
                format_option_lines(clipboard_cursor, clipboard_label, clipboard_desc);
            let file_lines = format_option_lines(file_cursor, file_label, file_desc);

            let mut lines = vec![
                top_border.clone(),
                pad_line("", inner_width),
                pad_line(" Export Conversation", inner_width),
                pad_line("", inner_width),
                pad_line(" Select export method:", inner_width),
                pad_line("", inner_width),
            ];
            for l in &clipboard_lines {
                lines.push(pad_line(l, inner_width));
            }
            for l in &file_lines {
                lines.push(pad_line(l, inner_width));
            }
            lines.push(pad_line("", inner_width));
            lines.push(bottom_border.clone());
            lines.push("  Esc to cancel".to_string());

            let content = lines.join("\n");
            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                ) {
                    Text(content: content)
                }
            }
            .into()
        }
        ExportStep::FilenameInput => {
            let lines = vec![
                top_border.clone(),
                pad_line("", inner_width),
                pad_line(" Export Conversation", inner_width),
                pad_line("", inner_width),
                pad_line(" Enter filename:", inner_width),
                pad_line("", inner_width),
                pad_line(&format!(" > {}", dialog.filename), inner_width),
                pad_line("", inner_width),
                bottom_border.clone(),
                "  Enter to save · Esc to go back".to_string(),
            ];

            let content = lines.join("\n");
            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                ) {
                    Text(content: content)
                }
            }
            .into()
        }
    }
}

impl TuiAppState {
    /// Handle key events in export dialog mode
    pub(in crate::tui::app) fn handle_export_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(dialog) = inner.dialog.as_export_mut() else {
            return;
        };

        match dialog.step {
            ExportStep::MethodSelection => match key.code {
                KeyCode::Esc => {
                    inner.mode = AppMode::Input;
                    inner.dialog.dismiss();
                    inner.display.response_content = "Export cancelled".to_string();
                    inner.display.is_command_output = true;
                }
                KeyCode::Up => dialog.move_selection_up(),
                KeyCode::Down => dialog.move_selection_down(),
                KeyCode::Enter => {
                    if dialog.confirm_selection() {
                        // Clipboard export
                        do_clipboard_export(&mut inner);
                    }
                    // else: moved to filename input, dialog updated
                }
                // 1 - Select clipboard and confirm
                KeyCode::Char('1') => {
                    dialog.selected_method = ExportMethod::Clipboard;
                    do_clipboard_export(&mut inner);
                }
                // 2 - Select file and advance to filename input
                KeyCode::Char('2') => {
                    dialog.selected_method = ExportMethod::File;
                    dialog.step = ExportStep::FilenameInput;
                }
                _ => {}
            },
            ExportStep::FilenameInput => match key.code {
                KeyCode::Esc => {
                    if let Some(dialog) = inner.dialog.as_export_mut() {
                        dialog.go_back();
                    }
                }
                KeyCode::Enter => {
                    do_file_export(&mut inner);
                }
                KeyCode::Backspace => {
                    if let Some(dialog) = inner.dialog.as_export_mut() {
                        dialog.pop_char();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(dialog) = inner.dialog.as_export_mut() {
                        dialog.push_char(c);
                    }
                }
                _ => {}
            },
        }
    }
}
