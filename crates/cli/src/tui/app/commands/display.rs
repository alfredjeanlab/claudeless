// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool call display formatting and permission type conversion.
//!
//! Contains:
//! - `wrap_response_paragraph` - Word-wrap text for display
//! - `format_completed_tool_display` - Format completed tool calls
//! - `format_tool_call_display` - Format tool calls for permission dialogs
//! - `tool_call_to_permission_type` - Convert tool calls to permission types

use crate::config::ToolCallSpec;
use crate::tui::widgets::permission::{DiffKind, DiffLine, PermissionType};

// ============================================================================
// Tool Call Display Formatting
// ============================================================================

/// Word-wrap a text paragraph for display after a `⏺ ` prefix.
///
/// Real Claude Code wraps response text at the terminal width with a 2-space
/// continuation indent. The first line has a 2-char prefix (`⏺ `), and
/// continuation lines use `  ` (2 spaces) indent.
pub(super) fn wrap_response_paragraph(text: &str, terminal_width: usize) -> String {
    // Account for "⏺ " prefix on first line (2 visual columns)
    let first_line_width = terminal_width.saturating_sub(2);
    // Continuation indent is "  " (2 spaces)
    let continuation_width = terminal_width.saturating_sub(2);

    if first_line_width == 0 || text.chars().count() <= first_line_width {
        return text.to_string();
    }

    let mut result = String::new();
    let mut current_line_len = 0;
    let mut is_first_line = true;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let max_width = if is_first_line {
            first_line_width
        } else {
            continuation_width
        };

        if current_line_len == 0 {
            result.push_str(word);
            current_line_len = word_len;
        } else if current_line_len + 1 + word_len <= max_width {
            result.push(' ');
            result.push_str(word);
            current_line_len += 1 + word_len;
        } else {
            result.push_str("\n  ");
            result.push_str(word);
            current_line_len = word_len;
            is_first_line = false;
        }
    }

    result
}

/// Join display parts where the first part is unprefixed (gets ⏺ from display layer)
/// and subsequent parts get their own ⏺ prefix.
pub(super) fn join_display_parts(parts: &[String]) -> String {
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            result.push_str(part);
        } else {
            result.push_str("\n\n⏺ ");
            result.push_str(part);
        }
    }
    result
}

/// Format a completed tool call with its result for display.
pub(super) fn format_completed_tool_display(
    call: &ToolCallSpec,
    result_text: Option<&str>,
) -> String {
    match call.tool.as_str() {
        "Write" => {
            let file_path = call
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mut display = format!("Write({})", file_path);
            if let Some(result) = result_text {
                display.push_str(&format!("\n  \u{23bf} \u{a0}{}", result));
                // Show content lines indented under the result
                if let Some(content) = call.input.get("content").and_then(|v| v.as_str()) {
                    for (i, line) in content.split('\n').enumerate() {
                        display.push_str(&format!("\n      {} {}", i + 1, line));
                    }
                }
            }
            display
        }
        "Read" => {
            if let Some(result) = result_text {
                // Results ending with "…" indicate a streaming/in-progress read
                if result.ends_with('\u{2026}') {
                    format!("Reading {} (ctrl+o to expand)", result)
                } else {
                    format!("Read {} (ctrl+o to expand)", result)
                }
            } else {
                "Read (ctrl+o to expand)".to_string()
            }
        }
        "Bash" => {
            let command = call
                .input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mut display = format!("Bash({})", command);
            if let Some(result) = result_text {
                display.push_str(&format!("\n  \u{23bf} \u{a0}{}", result));
            }
            display
        }
        _ => {
            if let Some(result) = result_text {
                result.to_string()
            } else {
                call.tool.clone()
            }
        }
    }
}

/// Format a tool call for display above the permission dialog.
pub(super) fn format_tool_call_display(call: &ToolCallSpec) -> String {
    match call.tool.as_str() {
        "Bash" => {
            let command = call
                .input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Bash({})\n  \u{23bf} \u{a0}Running\u{2026}", command)
        }
        "Edit" => {
            let file_path = call
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Update({})", file_path)
        }
        "Write" => {
            let file_path = call
                .input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Write({})", file_path)
        }
        _ => call.tool.clone(),
    }
}

// ============================================================================
// Tool Call → Permission Type Conversion
// ============================================================================

/// Convert a `ToolCallSpec` into a `PermissionType` for the TUI permission dialog.
///
/// Returns `None` for unknown tool names (e.g., MCP tools that don't have
/// a corresponding permission dialog).
pub(crate) fn tool_call_to_permission_type(call: &ToolCallSpec) -> Option<PermissionType> {
    match call.tool.as_str() {
        "Bash" => {
            let command = call.input.get("command")?.as_str()?.to_string();
            let description = call
                .input
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some(PermissionType::Bash {
                command,
                description,
            })
        }
        "Write" => {
            let file_path = call.input.get("file_path")?.as_str()?.to_string();
            let content = call
                .input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let content_lines = content.split('\n').map(|s| s.to_string()).collect();
            Some(PermissionType::Write {
                file_path,
                content_lines,
            })
        }
        "Edit" => {
            let file_path = call.input.get("file_path")?.as_str()?.to_string();
            let old_string = call
                .input
                .get("old_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new_string = call
                .input
                .get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut diff_lines = Vec::new();
            let mut line_num: u32 = 1;

            // Removed lines
            for line in old_string.lines() {
                diff_lines.push(DiffLine {
                    line_num: Some(line_num),
                    kind: DiffKind::Removed,
                    content: line.to_string(),
                });
                line_num += 1;
            }
            // NoNewline marker after removed lines if old_string doesn't end with newline
            if !old_string.is_empty() && !old_string.ends_with('\n') {
                diff_lines.push(DiffLine {
                    line_num: Some(line_num - 1),
                    kind: DiffKind::NoNewline,
                    content: "No newline at end of file".to_string(),
                });
            }

            // Added lines (line numbering continues from removed)
            let added_start = line_num;
            for (i, line) in new_string.lines().enumerate() {
                diff_lines.push(DiffLine {
                    line_num: Some(added_start + i as u32),
                    kind: DiffKind::Added,
                    content: line.to_string(),
                });
            }
            // NoNewline marker after added lines
            let added_count = new_string.lines().count();
            if !new_string.is_empty() && !new_string.ends_with('\n') {
                diff_lines.push(DiffLine {
                    line_num: Some(added_start + added_count as u32),
                    kind: DiffKind::NoNewline,
                    content: "No newline at end of file".to_string(),
                });
            }

            Some(PermissionType::Edit {
                file_path,
                diff_lines,
            })
        }
        _ => None,
    }
}
