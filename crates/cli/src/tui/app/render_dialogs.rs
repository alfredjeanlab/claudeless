// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Dialog rendering functions for permission, trust, thinking, tasks, etc.

use iocraft::prelude::*;

use crate::tui::separator::make_separator;
use crate::tui::slash_menu::COMMANDS;
use crate::tui::widgets::export::{ExportDialog, ExportMethod, ExportStep};
use crate::tui::widgets::help::{HelpDialog, HelpTab};
use crate::tui::widgets::tasks::TasksDialog;
use crate::tui::widgets::thinking::{ThinkingDialog, ThinkingMode};
use crate::tui::widgets::trust::TrustChoice;
use crate::tui::widgets::{
    HookType, HooksDialog, HooksView, MemoryDialog, ModelChoice, ModelPickerDialog,
};

use crate::tui::app::types::{PermissionRequest, TrustPromptState};

/// Render trust prompt dialog
pub(crate) fn render_trust_prompt(prompt: &TrustPromptState, width: usize) -> AnyElement<'static> {
    let yes_indicator = if prompt.selected == TrustChoice::Yes {
        " ❯ "
    } else {
        "   "
    };
    let no_indicator = if prompt.selected == TrustChoice::No {
        " ❯ "
    } else {
        "   "
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Horizontal rule separator
            Text(content: make_separator(width))
            // Title
            Text(content: " Do you trust the files in this folder?")
            Text(content: "")
            // Working directory
            Text(content: format!(" {}", prompt.working_directory))
            Text(content: "")
            // Security warning (wrapped text)
            Text(content: " Claude Code may read, write, or execute files contained in this directory. This can pose security risks, so only use")
            Text(content: " files from trusted sources.")
            Text(content: "")
            // Learn more link (plain text)
            Text(content: " Learn more")
            Text(content: "")
            // Options
            Text(content: format!("{}1. Yes, proceed", yes_indicator))
            Text(content: format!("{}2. No, exit", no_indicator))
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · Esc to cancel")
        }
    }.into()
}

/// Render thinking toggle dialog
pub(crate) fn render_thinking_dialog(dialog: &ThinkingDialog, width: usize) -> AnyElement<'static> {
    let enabled_indicator = if dialog.selected == ThinkingMode::Enabled {
        " ❯ "
    } else {
        "   "
    };
    let disabled_indicator = if dialog.selected == ThinkingMode::Disabled {
        " ❯ "
    } else {
        "   "
    };
    let enabled_check = if dialog.current == ThinkingMode::Enabled {
        " ✔"
    } else {
        ""
    };
    let disabled_check = if dialog.current == ThinkingMode::Disabled {
        " ✔"
    } else {
        ""
    };

    // Mid-conversation warning text (shown when toggling after conversation started)
    let warning_text = if dialog.is_mid_conversation {
        " Changing mid-conversation may reduce quality. For best results, set this at the start of a session."
    } else {
        ""
    };

    let subtitle = " Enable or disable thinking for this session.";

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            // Horizontal rule separator at top
            Text(content: make_separator(width))
            // Title
            Text(content: " Toggle thinking mode")
            // Subtitle
            Text(content: subtitle)
            // Mid-conversation warning (only if applicable)
            #(if dialog.is_mid_conversation {
                Some(element! { Text(content: warning_text) })
            } else {
                None
            })
            Text(content: "")
            // Options with descriptions
            Text(content: format!("{}1. Enabled{}  Claude will think before responding", enabled_indicator, enabled_check))
            Text(content: format!("{}2. Disabled{}   Claude will respond without extended thinking", disabled_indicator, disabled_check))
            Text(content: "")
            // Footer (note: lowercase 'escape' per fixture)
            Text(content: " Enter to confirm · escape to exit")
        }
    }.into()
}

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

    // Box drawing chars
    let h_line = "─".repeat(inner_width);
    let top_border = format!("╭{}╮", h_line);
    let bottom_border = format!("╰{}╯", h_line);

    // Pad content lines to fill width
    let pad_line = |s: &str| {
        // Calculate visual width (accounting for multi-byte chars)
        let visible_len = s.chars().count();
        let padding = inner_width.saturating_sub(visible_len);
        format!("│{}{}│", s, " ".repeat(padding))
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            Text(content: top_border)
            Text(content: pad_line(" Background tasks"))
            Text(content: pad_line(&format!(" {}", content)))
            Text(content: bottom_border)
            Text(content: "  ↑/↓ to select · Enter to view · Esc to close")
        }
    }
    .into()
}

/// Render export dialog
pub(crate) fn render_export_dialog(dialog: &ExportDialog, width: usize) -> AnyElement<'static> {
    let inner_width = width.saturating_sub(2);
    let h_line = "─".repeat(inner_width);
    let top_border = format!("╭{}╮", h_line);
    let bottom_border = format!("╰{}╯", h_line);

    let pad_line = |s: &str| {
        let visible_len = s.chars().count();
        let padding = inner_width.saturating_sub(visible_len);
        format!("│{}{}│", s, " ".repeat(padding))
    };

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

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                ) {
                    Text(content: top_border)
                    Text(content: pad_line(" Export Conversation"))
                    Text(content: pad_line(""))
                    Text(content: pad_line(" Select export method:"))
                    Text(content: pad_line(&format!(" {} 1. Copy to clipboard", clipboard_cursor)))
                    Text(content: pad_line(&format!(" {} 2. Save to file", file_cursor)))
                    Text(content: bottom_border)
                    Text(content: "  ↑/↓ to select · Enter to confirm · Esc to cancel")
                }
            }
            .into()
        }
        ExportStep::FilenameInput => element! {
            View(
                flex_direction: FlexDirection::Column,
                width: 100pct,
            ) {
                Text(content: top_border)
                Text(content: pad_line(" Export Conversation"))
                Text(content: pad_line(""))
                Text(content: pad_line(" Enter filename:"))
                Text(content: pad_line(&format!(" {}", dialog.filename)))
                Text(content: bottom_border)
                Text(content: "  Enter to save · esc to go back")
            }
        }
        .into(),
    }
}

/// Render help dialog
pub(crate) fn render_help_dialog(dialog: &HelpDialog, width: usize) -> AnyElement<'static> {
    let inner_width = width.saturating_sub(2);

    // Build tab header line
    let version_part = format!("─Claude Code v{}─", dialog.version);
    let tabs_part = format!(
        " {} ─ {} ─ {} ─",
        HelpTab::General.name(),
        HelpTab::Commands.name(),
        HelpTab::CustomCommands.name()
    );
    let hint = "(←/→ or tab to cycle)";
    let used = version_part.len() + tabs_part.len() + hint.len() + 1;
    let remaining = inner_width.saturating_sub(used);
    let tab_header = format!(
        " {}{}{}{}",
        version_part,
        tabs_part,
        hint,
        "─".repeat(remaining)
    );

    let footer = " For more help: https://code.claude.com/docs/en/overview";

    match dialog.active_tab {
        HelpTab::General => {
            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "")
                    Text(content: "  Claude understands your codebase, makes edits with your permission, and executes commands — right from your terminal.")
                    Text(content: "  / for commands    ctrl + o for verbose output              cmd + v to paste images")
                    Text(content: "  & for background  backslash (\\) + return (⏎) for newline   ctrl + s to stash prompt")
                    Text(content: "")
                    Text(content: footer)
                }
            }
            .into()
        }
        HelpTab::Commands => {
            let selected = dialog.commands_selected;
            let cmd = COMMANDS.get(selected);
            let next_cmd = COMMANDS.get(selected + 1);

            let selected_line = format!(
                "  ❯ /{}",
                cmd.map(|c| c.name).unwrap_or("")
            );
            let description_line = format!(
                "    {}",
                cmd.map(|c| c.description).unwrap_or("")
            );
            let next_line = if let Some(next) = next_cmd {
                format!("  ↓ /{}", next.name)
            } else {
                String::new()
            };

            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "  Browse default commands:")
                    Text(content: selected_line)
                    Text(content: description_line)
                    Text(content: next_line)
                    Text(content: "")
                    Text(content: footer)
                }
            }
            .into()
        }
        HelpTab::CustomCommands => {
            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "  Browse custom commands:")
                    Text(content: "  (no custom commands configured)")
                    Text(content: "")
                    Text(content: footer)
                }
            }
            .into()
        }
    }
}

/// Render memory dialog
pub(crate) fn render_memory_dialog(dialog: &MemoryDialog, _width: usize) -> AnyElement<'static> {
    // Build visible items
    let items: Vec<_> = dialog
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == dialog.selected_index();
            let prefix = if is_selected { "❯" } else { " " };
            let status = if entry.is_active { "✓" } else { " " };
            let path = entry.path.as_deref().unwrap_or("(not configured)");

            format!(
                " {} {} {}. {} - {}",
                prefix,
                status,
                i + 1,
                entry.source.name(),
                path
            )
        })
        .collect();

    // Count active entries
    let active_count = dialog.entries.iter().filter(|e| e.is_active).count();
    let header = if active_count == 1 {
        " Memory · 1 file".to_string()
    } else {
        format!(" Memory · {} files", active_count)
    };

    let footer = " Enter to view · esc to cancel".to_string();

    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            Text(content: header)
            Text(content: "")
            #(items.into_iter().map(|item| {
                element! { Text(content: item) }
            }))
            Text(content: "")
            Text(content: footer)
        }
    }
    .into()
}

/// Render hooks dialog
pub(crate) fn render_hooks_dialog(dialog: &HooksDialog, _width: usize) -> AnyElement<'static> {
    match dialog.view {
        HooksView::HookList => render_hooks_list(dialog),
        HooksView::Matchers => render_hooks_matchers(dialog),
    }
}

/// Render the main hooks list view
fn render_hooks_list(dialog: &HooksDialog) -> AnyElement<'static> {
    let hooks = HookType::all();
    let visible_start = dialog.scroll_offset();
    let visible_end = (visible_start + dialog.visible_count()).min(hooks.len());

    // Build visible items
    let items: Vec<_> = hooks
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(visible_end - visible_start)
        .map(|(i, hook)| {
            let is_selected = i == dialog.selected_index();
            let is_last_visible = i == visible_end - 1 && dialog.has_more_below();

            let prefix = if is_selected {
                "❯"
            } else if is_last_visible {
                "↓"
            } else {
                " "
            };

            format!(
                " {} {}.  {} - {}",
                prefix,
                i + 1,
                hook.name(),
                hook.description()
            )
        })
        .collect();

    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            Text(content: " Hooks")
            Text(content: format!(" {} hooks", dialog.active_hook_count))
            Text(content: "")
            #(items.into_iter().map(|item| {
                element! { Text(content: item) }
            }).collect::<Vec<_>>())
            Text(content: "")
            Text(content: " Enter to confirm · esc to cancel")
        }
    }
    .into()
}

/// Render the matchers view for a selected hook type
fn render_hooks_matchers(dialog: &HooksDialog) -> AnyElement<'static> {
    let hook = dialog.selected_hook.unwrap_or(HookType::PreToolUse);

    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            Text(content: format!(" {} - Tool Matchers", hook.name()))
            Text(content: " Input to command is JSON of tool call arguments.")
            Text(content: " Exit code 0 - stdout/stderr not shown")
            Text(content: " Exit code 2 - show stderr to model and block tool call")
            Text(content: " Other exit codes - show stderr to user only but continue with tool call")
            Text(content: "")
            Text(content: " ❯ 1. + Add new matcher…")
            Text(content: "   No matchers configured yet")
            Text(content: "")
            Text(content: " Enter to confirm · esc to cancel")
        }
    }
    .into()
}

/// Render model picker dialog
pub(crate) fn render_model_picker_dialog(
    dialog: &ModelPickerDialog,
    _width: usize,
) -> AnyElement<'static> {
    let choices = ModelChoice::all();

    element! {
        View(flex_direction: FlexDirection::Column) {
            // Title
            Text(content: " Select model")
            // Description
            Text(content: " Switch between Claude models. Applies to this session and future Claude Code sessions. For other/previous model names,")
            Text(content: "  specify with --model.")
            // Empty line
            Text(content: "")
            // Options
            #(choices.iter().enumerate().map(|(i, choice)| {
                let is_selected = *choice == dialog.selected;
                let is_current = *choice == dialog.current;

                let cursor = if is_selected { "❯" } else { " " };
                let checkmark = if is_current { " ✔" } else { "" };
                let number = i + 1;

                let label = match choice {
                    ModelChoice::Default => "Default (recommended)",
                    ModelChoice::Sonnet => "Sonnet",
                    ModelChoice::Haiku => "Haiku",
                };

                let description = format!(
                    "{} · {}",
                    choice.display_name(),
                    choice.description()
                );

                // Format: " ❯ 1. Label checkmark           Description"
                let content = format!(
                    " {} {}. {:<22}{} {}",
                    cursor,
                    number,
                    label,
                    checkmark,
                    description
                );

                element! {
                    Text(content: content)
                }
            }))
            // Empty line
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · esc to exit")
        }
    }
    .into_any()
}

/// Render rich permission dialog
pub(crate) fn render_permission_dialog(
    perm: &PermissionRequest,
    width: usize,
) -> AnyElement<'static> {
    // Render the dialog content using the widget
    let content = perm.dialog.render(width);

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
