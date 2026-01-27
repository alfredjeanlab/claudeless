// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Content area rendering: conversation, shortcuts, slash menu, hints.

use iocraft::prelude::*;

use crate::tui::separator::make_compact_separator;
use crate::tui::shortcuts::shortcuts_by_column;
use crate::tui::slash_menu::COMMANDS;

use super::super::types::RenderState;

/// Render the shortcuts panel with 3 columns
pub(crate) fn render_shortcuts_panel(_width: usize) -> AnyElement<'static> {
    let columns = shortcuts_by_column();

    // Fixed column widths matching the Claude Code fixture:
    // - Left column: 26 chars total (2-space indent + 24 content)
    // - Center column: 35 chars
    // - Right column: remaining space
    const LEFT_WIDTH: usize = 24; // Content width (after 2-space indent)
    const CENTER_WIDTH: usize = 35;

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
        let compact_text = "Conversation compacted · ctrl+o for history";
        content.push_str(&make_compact_separator(
            compact_text,
            state.display.terminal_width as usize,
        ));
        content.push('\n');
    }

    // Add conversation display (includes user prompts and past responses)
    if !state.display.conversation_display.is_empty() {
        content.push_str(&state.display.conversation_display);
    }

    // Add current response if present
    if !state.display.response_content.is_empty() {
        // Check if this is a compacting-in-progress message (✻ symbol)
        let is_compacting_in_progress = state.display.response_content.starts_with('✻');

        if is_compacting_in_progress {
            // During compacting, show message on its own line after blank line
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&state.display.response_content);
        } else if state.display.is_command_output {
            // Completed command output uses elbow connector format
            if !content.is_empty() {
                content.push('\n');
            }
            // Format each line with elbow connector (2 spaces + ⎿ + 2 spaces)
            for (i, line) in state.display.response_content.lines().enumerate() {
                if i > 0 {
                    content.push('\n');
                }
                content.push_str(&format!("  ⎿  {}", line));
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

    // Build menu content
    let max_visible = 10; // Show at most 10 commands
    let mut content = String::new();

    for (i, cmd) in menu.filtered_commands.iter().take(max_visible).enumerate() {
        let is_selected = i == menu.selected_index;
        let indicator = if is_selected { " ❯ " } else { "   " };

        // Format: indicator + /command + spaces + description
        // Use 14 chars for command name (including /) to align descriptions
        let cmd_display = format!("/{}", cmd.name);
        content.push_str(&format!(
            "{}{:<14}  {}\n",
            indicator, cmd_display, cmd.description
        ));
    }

    // Remove trailing newline
    if content.ends_with('\n') {
        content.pop();
    }

    element! {
        View(flex_direction: FlexDirection::Column) {
            Text(content: content)
            Text(content: "")
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

/// Render argument hint for completed slash commands
pub(crate) fn render_argument_hint(state: &RenderState) -> AnyElement<'static> {
    // Only show hint when menu is closed and input starts with a completed command
    if state.display.slash_menu.is_some() || !state.input.buffer.starts_with('/') {
        return element! { View {} }.into();
    }

    // Extract command name (without leading /)
    let cmd_text = state.input.buffer.trim_start_matches('/');

    // Find exact match
    if let Some(cmd) = COMMANDS.iter().find(|c| c.name == cmd_text) {
        if let Some(hint) = cmd.argument_hint {
            let hint_text = format!("     {}  {}", " ".repeat(cmd_text.len()), hint);
            return element! {
                View(flex_direction: FlexDirection::Column) {
                    Text(content: hint_text)
                }
            }
            .into();
        }
    }

    element! { View {} }.into()
}
