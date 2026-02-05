// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI rendering module for the App component.
//!
//! This module contains all rendering logic extracted from app.rs:
//! - `content.rs` - Conversation, shortcuts, slash menu, hints
//! - `format.rs` - Header, status bar, model name formatting
//! - Dialog rendering lives in `super::dialogs`

#[path = "content.rs"]
mod content;
#[path = "format.rs"]
mod format;

pub(crate) use content::{
    get_argument_hint, render_conversation_area, render_shortcuts_panel, render_slash_menu,
    render_stash_indicator,
};
pub(crate) use format::{
    format_header_lines, format_status_bar, format_status_bar_styled, format_welcome_back_box,
};

use iocraft::prelude::*;

use crate::tui::colors::{
    styled_bash_placeholder, styled_bash_separator, styled_bash_status, styled_placeholder,
    styled_separator,
};
use crate::tui::separator::make_separator;

use super::dialogs::{
    render_bypass_confirm_dialog, render_elicitation_dialog, render_export_dialog,
    render_help_dialog, render_hooks_dialog, render_memory_dialog, render_model_picker_dialog,
    render_permission_dialog, render_plan_approval_dialog, render_setup_wizard,
    render_tasks_dialog, render_thinking_dialog, render_trust_prompt,
};
use super::types::{AppMode, RenderState};

/// Render modal dialog if one is active, otherwise return None.
fn render_active_dialog(state: &RenderState, width: usize) -> Option<AnyElement<'static>> {
    match state.mode {
        AppMode::Setup => state
            .dialog
            .as_setup()
            .map(|s| render_setup_wizard(s, width)),
        AppMode::Trust => state
            .dialog
            .as_trust()
            .map(|p| render_trust_prompt(p, width)),
        AppMode::BypassConfirm => state
            .dialog
            .as_bypass_confirm()
            .map(|d| render_bypass_confirm_dialog(d, width)),
        AppMode::ThinkingToggle => state
            .dialog
            .as_thinking()
            .map(|d| render_thinking_dialog(d, width)),
        AppMode::TasksDialog => state
            .dialog
            .as_tasks()
            .map(|d| render_tasks_dialog(d, width)),
        AppMode::ExportDialog => state
            .dialog
            .as_export()
            .map(|d| render_export_dialog(d, width)),
        AppMode::HelpDialog => state.dialog.as_help().map(|d| render_help_dialog(d, width)),
        AppMode::HooksDialog => state
            .dialog
            .as_hooks()
            .map(|d| render_hooks_dialog(d, width)),
        AppMode::MemoryDialog => state
            .dialog
            .as_memory()
            .map(|d| render_memory_dialog(d, width)),
        AppMode::ModelPicker => state
            .dialog
            .as_model_picker()
            .map(|d| render_model_picker_dialog(d, width)),
        AppMode::Elicitation => state
            .dialog
            .as_elicitation()
            .map(|d| render_elicitation_dialog(d, width)),
        AppMode::PlanApproval => state
            .dialog
            .as_plan_approval()
            .map(|d| render_plan_approval_dialog(d, width)),
        // Permission is rendered inline, not as a full-screen modal
        _ => None,
    }
}

/// Render the header area: welcome box or standard 3-line header.
///
/// Each line is rendered as a separate Text element with NoWrap to preserve ANSI codes.
fn render_header_area(state: &RenderState, width: usize) -> AnyElement<'static> {
    let show_welcome_box = state.show_welcome_back
        && state.display.conversation_display.is_empty()
        && state.display.response_content.is_empty();

    if show_welcome_box {
        let lines = format_welcome_back_box(state, width);
        element! {
            View(flex_direction: FlexDirection::Column) {
                #(lines.into_iter().map(|line| {
                    element! {
                        Text(content: line, wrap: TextWrap::NoWrap)
                    }
                }).collect::<Vec<_>>())
            }
        }
        .into()
    } else {
        let (h1, h2, h3) = format_header_lines(state);
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: h1, wrap: TextWrap::NoWrap)
                Text(content: h2, wrap: TextWrap::NoWrap)
                Text(content: h3, wrap: TextWrap::NoWrap)
            }
        }
        .into()
    }
}

/// Render the main content based on current mode
pub(crate) fn render_main_content(state: &RenderState) -> AnyElement<'static> {
    let width = state.display.terminal_width as usize;

    // Modal dialogs take over the full screen
    if let Some(dialog) = render_active_dialog(state, width) {
        return dialog;
    }

    // Use styled output when connected to a TTY
    let use_colors = state.is_tty;

    // Format input line
    // Shell mode shows `! Try "..."` in pink, otherwise show normal input or placeholder
    let input_display = if state.input.shell_mode {
        // Bash mode: show `! ` prefix in pink with suggestion or typed command
        if state.input.buffer.is_empty() {
            let ph = state
                .placeholder
                .as_deref()
                .unwrap_or("Try \"fix lint errors\"");
            if use_colors {
                styled_bash_placeholder(ph)
            } else {
                format!("!\u{00A0}{}", ph)
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
                    "{}{}!\u{00A0}{}{}",
                    crate::tui::colors::escape::RESET,
                    fg_pink,
                    state.input.buffer,
                    crate::tui::colors::escape::RESET
                )
            } else {
                format!("!\u{00A0}{}", state.input.buffer)
            }
        }
    } else if state.input.buffer.is_empty() {
        if state.display.conversation_display.is_empty()
            && state.display.response_content.is_empty()
        {
            // Show placeholder only on initial state
            let ph = state
                .placeholder
                .as_deref()
                .unwrap_or("Try \"write a test for scenario.rs\"");
            if use_colors {
                styled_placeholder(ph)
            } else {
                format!("❯\u{00A0}{}", ph)
            }
        } else {
            // After conversation started, show just the cursor
            // Reset to clear dim/gray from separator so chevron is white
            if use_colors {
                format!("{}❯", crate::tui::colors::escape::RESET)
            } else {
                "❯".to_string()
            }
        }
    } else {
        // User is typing - reset to clear dim/gray from separator
        // so chevron and input text are white
        if use_colors {
            format!(
                "{}❯\u{00A0}{}",
                crate::tui::colors::escape::RESET,
                state.input.buffer
            )
        } else {
            format!("❯\u{00A0}{}", state.input.buffer)
        }
    };

    // Append argument hint inline (e.g., "[open]" after "/plan ")
    let has_argument_hint = get_argument_hint(state).is_some();
    let input_display = if let Some(hint) = get_argument_hint(state) {
        format!("{} {}", input_display, hint)
    } else {
        input_display
    };

    // Format separators - pink in bash mode, gray otherwise
    let separator = if state.input.shell_mode && use_colors {
        styled_bash_separator(width)
    } else if use_colors {
        styled_separator(width)
    } else {
        format!("{}\n", make_separator(width))
    };

    // Format status bar - show `! for bash mode` in bash mode
    let status_bar = if state.input.shell_mode && use_colors {
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
            // Header area: welcome box OR standard 3-line header
            // Note: Empty first element to work around iocraft first-element rendering issue
            Text(content: "")
            #(render_header_area(state, width))

            // Empty line after header/box
            Text(content: "")

            // Conversation history area (if any)
            #(render_conversation_area(state))

            // Permission dialog (replaces input area when active)
            #(if state.mode == AppMode::Permission {
                if let Some(perm) = state.dialog.as_permission() {
                    render_permission_dialog(perm, width)
                } else {
                    element! { View {} }.into()
                }
            } else {
                element! {
                    View(flex_direction: FlexDirection::Column, width: 100pct) {
                        // Input area with separators (NoWrap to preserve ANSI)
                        #(render_stash_indicator(state))
                        Text(content: separator.clone(), wrap: TextWrap::NoWrap)
                        Text(content: input_display, wrap: TextWrap::NoWrap)
                        Text(content: separator, wrap: TextWrap::NoWrap)

                        // Slash menu (below input separator)
                        #(render_slash_menu(state))

                        // Shortcuts panel or status bar (NoWrap to preserve ANSI)
                        // Hide status bar when argument hint is shown (matches real CLI behavior)
                        #(if state.display.show_shortcuts_panel {
                            render_shortcuts_panel(state.display.terminal_width as usize)
                        } else if has_argument_hint {
                            element! { View {} }.into()
                        } else {
                            element! {
                                Text(content: status_bar.clone(), wrap: TextWrap::NoWrap)
                            }.into()
                        })
                    }
                }.into()
            })
        }
    }
    .into()
}
