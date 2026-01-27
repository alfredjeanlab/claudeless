// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI rendering module for the App component.
//!
//! This module contains all rendering logic extracted from app.rs:
//! - `content.rs` - Conversation, shortcuts, slash menu, hints
//! - `dialogs.rs` - Dialog render functions (trust, thinking, tasks, etc.)
//! - `format.rs` - Header, status bar, model name formatting

mod content;
mod dialogs;
mod format;

pub(crate) use content::{
    render_argument_hint, render_conversation_area, render_shortcuts_panel, render_slash_menu,
    render_stash_indicator,
};
pub(crate) use dialogs::{
    render_export_dialog, render_help_dialog, render_hooks_dialog, render_memory_dialog,
    render_model_picker_dialog, render_permission_dialog, render_tasks_dialog,
    render_thinking_dialog, render_trust_prompt,
};
pub(crate) use format::{format_header_lines, format_status_bar, format_status_bar_styled};

use iocraft::prelude::*;

use crate::tui::colors::{
    styled_bash_placeholder, styled_bash_separator, styled_bash_status, styled_placeholder,
    styled_separator,
};
use crate::tui::separator::make_separator;

use super::types::{AppMode, RenderState};

/// Render the main content based on current mode
pub(crate) fn render_main_content(state: &RenderState) -> AnyElement<'static> {
    let width = state.terminal_width as usize;

    // If in trust mode, render trust prompt
    if state.mode == AppMode::Trust {
        if let Some(ref prompt) = state.trust_prompt {
            return render_trust_prompt(prompt, width);
        }
    }

    // If in thinking toggle mode, render just the thinking dialog
    if state.mode == AppMode::ThinkingToggle {
        if let Some(ref dialog) = state.thinking_dialog {
            return render_thinking_dialog(dialog, width);
        }
    }

    // If in tasks dialog mode, render just the tasks dialog
    if state.mode == AppMode::TasksDialog {
        if let Some(ref dialog) = state.tasks_dialog {
            return render_tasks_dialog(dialog, width);
        }
    }

    // If in export dialog mode, render just the export dialog
    if state.mode == AppMode::ExportDialog {
        if let Some(ref dialog) = state.export_dialog {
            return render_export_dialog(dialog, width);
        }
    }

    // If in help dialog mode, render just the help dialog
    if state.mode == AppMode::HelpDialog {
        if let Some(ref dialog) = state.help_dialog {
            return render_help_dialog(dialog, width);
        }
    }

    // If in hooks dialog mode, render just the hooks dialog
    if state.mode == AppMode::HooksDialog {
        if let Some(ref dialog) = state.hooks_dialog {
            return render_hooks_dialog(dialog, width);
        }
    }

    // If in memory dialog mode, render just the memory dialog
    if state.mode == AppMode::MemoryDialog {
        if let Some(ref dialog) = state.memory_dialog {
            return render_memory_dialog(dialog, width);
        }
    }

    // If in model picker mode, render just the model picker dialog
    if state.mode == AppMode::ModelPicker {
        if let Some(ref dialog) = state.model_picker_dialog {
            return render_model_picker_dialog(dialog, width);
        }
    }

    // If in permission mode, render just the permission dialog (full-screen)
    if state.mode == AppMode::Permission {
        if let Some(ref perm) = state.pending_permission {
            return render_permission_dialog(perm, width);
        }
    }

    // Format header lines
    let (header_line1, header_line2, header_line3) = format_header_lines(state);

    // Use styled output when connected to a TTY
    let use_colors = state.is_tty;

    // Format input line
    // Shell mode shows `! Try "..."` in pink, otherwise show normal input or placeholder
    let input_display = if state.shell_mode {
        // Bash mode: show `! ` prefix in pink with suggestion or typed command
        if state.input_buffer.is_empty() {
            if use_colors {
                styled_bash_placeholder("Try \"fix lint errors\"")
            } else {
                "! Try \"fix lint errors\"".to_string()
            }
        } else {
            // User is typing a command - show `! ` followed by their input
            if use_colors {
                let fg_pink = crate::tui::colors::escape::fg(
                    crate::tui::colors::BASH_MODE.0,
                    crate::tui::colors::BASH_MODE.1,
                    crate::tui::colors::BASH_MODE.2,
                );
                format!(
                    "{}{}! {}{}",
                    crate::tui::colors::escape::RESET,
                    fg_pink,
                    state.input_buffer,
                    crate::tui::colors::escape::RESET
                )
            } else {
                format!("! {}", state.input_buffer)
            }
        }
    } else if state.input_buffer.is_empty() {
        if state.conversation_display.is_empty() && state.response_content.is_empty() {
            // Show placeholder only on initial state
            if use_colors {
                styled_placeholder("Try \"write a test for scenario.rs\"")
            } else {
                "❯ Try \"refactor mod.rs\"".to_string()
            }
        } else {
            // After conversation started, show just the cursor
            "❯".to_string()
        }
    } else {
        format!("❯ {}", state.input_buffer)
    };

    // Format separators - pink in bash mode, gray otherwise
    let separator = if state.shell_mode && use_colors {
        styled_bash_separator(width)
    } else if use_colors {
        styled_separator(width)
    } else {
        format!("{}\n", make_separator(width))
    };

    // Format status bar - show `! for bash mode` in bash mode
    let status_bar = if state.shell_mode && use_colors {
        styled_bash_status()
    } else if use_colors {
        format_status_bar_styled(state, width)
    } else {
        format_status_bar(state, width)
    };

    // Main layout matching real Claude CLI
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Header with Claude branding (3 lines)
            // Use NoWrap to preserve trailing ANSI escape codes
            // Note: Empty first element to work around iocraft first-element rendering issue
            Text(content: "")
            Text(content: header_line1, wrap: TextWrap::NoWrap)
            Text(content: header_line2, wrap: TextWrap::NoWrap)
            Text(content: header_line3, wrap: TextWrap::NoWrap)

            // Empty line after header
            Text(content: "")

            // Conversation history area (if any)
            #(render_conversation_area(state))

            // Slash menu (if open)
            #(render_slash_menu(state))

            // Input area with separators (NoWrap to preserve ANSI)
            Text(content: separator.clone(), wrap: TextWrap::NoWrap)
            #(render_stash_indicator(state))
            Text(content: input_display, wrap: TextWrap::NoWrap)
            #(render_argument_hint(state))
            Text(content: separator, wrap: TextWrap::NoWrap)

            // Shortcuts panel or status bar (NoWrap to preserve ANSI)
            #(if state.show_shortcuts_panel {
                render_shortcuts_panel(state.terminal_width as usize)
            } else {
                element! {
                    Text(content: status_bar.clone(), wrap: TextWrap::NoWrap)
                }.into()
            })
        }
    }
    .into()
}
