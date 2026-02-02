// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Content area rendering: conversation, shortcuts, slash menu, hints.

use iocraft::prelude::*;

use crate::tui::shortcuts::shortcuts_by_column;
use crate::tui::slash_menu::{COMMANDS, MENU_VISIBLE_COUNT};
use crate::tui::spinner;

use crate::tui::app::types::{AppMode, RenderState};

/// Truncate a string to `max_chars` characters, appending "…" if truncated.
fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}\u{2026}")
}

/// Render the shortcuts panel with 3 columns
pub(crate) fn render_shortcuts_panel(_width: usize) -> AnyElement<'static> {
    let columns = shortcuts_by_column();

    // Fixed column widths matching the Claude Code fixture:
    // - Left column: 23 chars total (2-space indent + 21 content)
    // - Center column: 31 chars
    // - Right column: remaining space
    const LEFT_WIDTH: usize = 21; // Content width (after 2-space indent)
    const CENTER_WIDTH: usize = 31;

    // Build the multi-column layout
    // Each row contains entries from all 3 columns
    let max_rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);

    let mut lines = Vec::new();
    for row_idx in 0..max_rows {
        let left = columns[0].get(row_idx).copied().unwrap_or("");
        let center = columns[1].get(row_idx).copied().unwrap_or("");
        let right = columns[2].get(row_idx).copied().unwrap_or("");

        // Format line with 2-space indent and fixed column widths
        let line = format!(
            "  {:<left_w$}{:<center_w$}{}",
            left,
            center,
            right,
            left_w = LEFT_WIDTH,
            center_w = CENTER_WIDTH
        );
        lines.push(line);
    }

    element! {
        View(flex_direction: FlexDirection::Column) {
            #(lines.into_iter().map(|line| {
                element! {
                    Text(content: line)
                }
            }).collect::<Vec<_>>())
        }
    }
    .into()
}

/// Render conversation history area
pub(crate) fn render_conversation_area(state: &RenderState) -> AnyElement<'static> {
    let mut content = String::new();

    // Add compact separator if conversation has been compacted
    if state.display.is_compacted {
        content.push_str("✻ Conversation compacted (ctrl+o for history)");
        content.push_str("\n\n\n");
    }

    // Add conversation display (includes user prompts and past responses)
    if !state.display.conversation_display.is_empty() {
        content.push_str(&state.display.conversation_display);
    }

    // Add current response if present
    if !state.display.response_content.is_empty() || state.is_compacting {
        // Check if this is a compacting-in-progress state (not yet completed)
        if state.is_compacting && !state.display.is_compacted {
            // During compacting, show animated spinner
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&render_spinner(state, "Compacting conversation"));
            content.push_str("…\n  \u{23BF}  Tip: Use /memory to view and manage Claude memory");
        } else if matches!(state.mode, AppMode::Responding | AppMode::Thinking)
            && !state.display.is_command_output
            && state.display.response_content.is_empty()
        {
            // Show animated spinner during thinking/responding with no content yet
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&render_spinner(state, &state.spinner_verb));
            content.push('…');
        } else if state.display.is_command_output {
            // Completed command output uses elbow connector format
            if !content.is_empty() {
                content.push('\n');
            }
            // First line gets elbow connector; subsequent lines pass through as-is
            for (i, line) in state.display.response_content.lines().enumerate() {
                if i > 0 {
                    content.push('\n');
                }
                if i == 0 {
                    content.push_str(&format!("  \u{23BF}  {}", line));
                } else {
                    content.push_str(line);
                }
            }
        } else {
            // Normal response with ⏺ prefix
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&format!("⏺ {}", state.display.response_content));
        }
    }

    // Add trailing empty line if there's content (creates space before separator)
    if !content.is_empty() {
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: content)
                Text(content: "")
            }
        }
        .into()
    } else {
        // Empty element if no content
        element! {
            View {}
        }
        .into()
    }
}

/// Render the slash command autocomplete menu (if open)
pub(crate) fn render_slash_menu(state: &RenderState) -> AnyElement<'static> {
    let Some(ref menu) = state.display.slash_menu else {
        return element! { View {} }.into();
    };

    if menu.filtered_commands.is_empty() {
        return element! { View {} }.into();
    }

    // Build menu content - visible items as a list below the input separator
    let mut content = String::new();
    let visible_end = menu.filtered_commands.len().min(MENU_VISIBLE_COUNT);
    let width = state.display.terminal_width as usize;
    // 2-space indent + 24-char command column = 26 chars before description.
    // Real Claude reserves 2 chars of right margin.
    let desc_max = width.saturating_sub(28);

    for cmd in menu.filtered_commands[..visible_end].iter() {
        // Format: 2-space indent + /command padded to 24 chars + description
        let cmd_display = format!("/{}", cmd.name);
        let desc = truncate_with_ellipsis(cmd.description, desc_max);
        content.push_str(&format!("  {:<24}{}\n", cmd_display, desc));
    }

    // Remove trailing newline
    if content.ends_with('\n') {
        content.pop();
    }

    element! {
        View(flex_direction: FlexDirection::Column) {
            Text(content: content)
        }
    }
    .into()
}

/// Render stash indicator if stash is active
pub(crate) fn render_stash_indicator(state: &RenderState) -> AnyElement<'static> {
    if !state.input.show_stash_indicator {
        return element! { View {} }.into();
    }

    // Use orange accent color for the › character
    use crate::tui::colors::{escape, LOGO_FG};
    let accent_fg = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let reset = escape::RESET;

    let indicator_text = format!(
        "  {}›{} Stashed (auto-restores after submit)",
        accent_fg, reset
    );

    element! {
        View(flex_direction: FlexDirection::Column) {
            Text(content: indicator_text, wrap: TextWrap::NoWrap)
        }
    }
    .into()
}

/// Get argument hint text for completed slash commands (if any).
///
/// Returns the hint string (e.g., "[open]") to be appended inline after the input,
/// or None if no hint should be shown.
pub(crate) fn get_argument_hint(state: &RenderState) -> Option<&'static str> {
    // Only show hint when menu is closed and input starts with a completed command
    if state.display.slash_menu.is_some() || !state.input.buffer.starts_with('/') {
        return None;
    }

    // Extract command name (without leading /, trimmed of trailing whitespace/args)
    let cmd_text = state.input.buffer.trim_start_matches('/');
    let cmd_name = cmd_text.split_whitespace().next().unwrap_or("");

    // Find exact match
    if let Some(cmd) = COMMANDS.iter().find(|c| c.name == cmd_name) {
        return cmd.argument_hint;
    }

    None
}

/// Render animated spinner with text
fn render_spinner(state: &RenderState, text: &str) -> String {
    let cycle = spinner::spinner_cycle();
    let frame = cycle[state.spinner_frame % cycle.len()];
    format!("{} {}", frame, text)
}
