// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Multi-section panel dialogs: hooks, model picker, setup wizard.
//!
//! Each dialog's rendering and key handling are colocated here.

use iocraft::prelude::*;

use crate::tui::separator::make_section_divider;
use crate::tui::widgets::setup::{
    SetupState, SetupStep, SETUP_ART, SETUP_SEPARATOR, SETUP_SPLIT_SEPARATOR, SYNTAX_PREVIEW,
    THEME_LABELS,
};
use crate::tui::widgets::{HookType, HooksDialog, HooksView, ModelChoice, ModelPickerDialog};

use crate::hooks::NOTIFICATION_AUTH_SUCCESS;
use crate::tui::app::state::TuiAppState;
use crate::tui::app::types::{AppMode, ExitReason};

use super::SelectionList;

// ── Hooks dialog ─────────────────────────────────────────────────────────

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
            Text(content: " Enter to confirm · Esc to cancel")
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
            Text(content: "   2. + Match all (no filter)")
            Text(content: "   No matchers configured yet")
            Text(content: "")
            Text(content: " Enter to confirm · Esc to cancel")
        }
    }
    .into()
}

impl TuiAppState {
    /// Handle key events in hooks dialog mode
    pub(in crate::tui::app) fn handle_hooks_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(dialog) = inner.dialog.as_hooks_mut() else {
            return;
        };

        match dialog.view {
            HooksView::HookList => match key.code {
                KeyCode::Esc => {
                    inner.dismiss_dialog("Hooks dialog");
                }
                KeyCode::Up => dialog.select_prev(),
                KeyCode::Down => dialog.select_next(),
                KeyCode::Enter => dialog.open_matchers(),
                _ => {}
            },
            HooksView::Matchers => match key.code {
                KeyCode::Esc => dialog.close_matchers(),
                KeyCode::Up => {
                    // Navigate matchers (when implemented)
                }
                KeyCode::Down => {
                    // Navigate matchers (when implemented)
                }
                KeyCode::Enter => {
                    // Add new matcher (when implemented)
                }
                _ => {}
            },
        }
    }
}

// ── Model picker ─────────────────────────────────────────────────────────

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
            Text(content: " Switch between Claude models. Applies to this session and future Claude Code")
            Text(content: " sessions. For other/previous model names, specify with --model.")
            // Empty line
            Text(content: "")
            // Options
            #(choices.iter().enumerate().map(|(i, choice)| {
                let is_selected = *choice == dialog.selected;
                let is_current = *choice == dialog.current;

                let cursor = if is_selected { "❯" } else { " " };
                let number = i + 1;

                let label = match choice {
                    ModelChoice::Default => "Default (recommended)",
                    ModelChoice::Opus => "Opus",
                    ModelChoice::Haiku => "Haiku",
                };

                let label_with_check = if is_current {
                    format!("{} ✔", label)
                } else {
                    label.to_string()
                };

                let description = format!(
                    "{} · {}",
                    choice.display_name(),
                    choice.description()
                );

                // Format: " ❯ 1. Label [✔]          Description"
                let content = format!(
                    " {} {}. {:<23}{}",
                    cursor,
                    number,
                    label_with_check,
                    description
                );

                element! {
                    Text(content: content)
                }
            }))
            // Empty line
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · Esc to exit")
        }
    }
    .into_any()
}

impl TuiAppState {
    /// Handle key events in model picker mode
    pub(in crate::tui::app) fn handle_model_picker_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(dialog) = inner.dialog.as_model_picker_mut() {
                    dialog.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                if let Some(dialog) = inner.dialog.as_model_picker_mut() {
                    dialog.move_down();
                }
            }
            KeyCode::Enter => {
                if let Some(dialog) = inner.dialog.as_model_picker() {
                    // Apply selection
                    inner.status.model = dialog.selected.model_id().to_string();
                }
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }
            KeyCode::Esc => {
                // Cancel without changes
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }
            _ => {}
        }
    }
}

// ── Setup wizard ─────────────────────────────────────────────────────────

/// Render setup wizard dialog (theme selection or login method)
pub(crate) fn render_setup_wizard(state: &SetupState, width: usize) -> AnyElement<'static> {
    // Build all lines as individual elements
    let mut lines: Vec<String> = Vec::new();

    // Welcome banner + separator + art
    lines.push(format!("Welcome to Claude Code v{}", state.claude_version));
    lines.push(SETUP_SEPARATOR.to_string());
    lines.push(String::new());
    for art_line in SETUP_ART {
        lines.push(art_line.to_string());
    }
    lines.push(SETUP_SPLIT_SEPARATOR.to_string());

    match state.step {
        SetupStep::ThemeSelection => {
            lines.push(String::new());
            lines.push(" Let's get started.".to_string());
            lines.push(String::new());
            lines.push(" Choose the text style that looks best with your terminal".to_string());
            lines.push(" To change this later, run /theme".to_string());
            lines.push(String::new());

            // Theme options
            lines.extend(
                SelectionList::new(THEME_LABELS)
                    .selected(state.selected_theme)
                    .current(0)
                    .lines(),
            );
            lines.push(String::new());

            // Syntax preview
            let divider = make_section_divider(width);
            lines.push(divider.clone());
            for preview_line in SYNTAX_PREVIEW {
                lines.push(preview_line.to_string());
            }
            lines.push(divider);

            if state.syntax_highlighting {
                lines.push(format!(
                    " Syntax theme: {} (ctrl+t to disable)",
                    state.theme_choice().syntax_theme_name()
                ));
            } else {
                lines.push(" Syntax highlighting disabled (ctrl+t to enable)".to_string());
            }
        }
        SetupStep::LoginMethod => {
            lines.push(String::new());
            lines.push(String::new());
            lines.push(
                " Claude Code can be used with your Claude subscription or billed based on API"
                    .to_string(),
            );
            lines.push(" usage through your Console account.".to_string());
            lines.push(String::new());
            lines.push(" Select login method:".to_string());
            lines.push(String::new());

            let login_labels: &[&str] = &[
                "Claude account with subscription \u{00B7} Pro, Max, Team, or Enterprise",
                "Anthropic Console account \u{00B7} API usage billing",
                "3rd-party platform \u{00B7} Amazon Bedrock, Microsoft Foundry, or Vertex AI",
            ];
            for line in SelectionList::new(login_labels)
                .selected(state.selected_login)
                .lines()
            {
                lines.push(line);
                lines.push(String::new());
            }
        }
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            #(lines.into_iter().map(|line| {
                element! { Text(content: line) }
            }))
        }
    }
    .into()
}

impl TuiAppState {
    /// Handle key events in setup wizard mode
    pub(in crate::tui::app) fn handle_setup_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(dialog) = inner.dialog.as_setup_mut() else {
            return;
        };

        match dialog.step {
            SetupStep::ThemeSelection => match (key.modifiers, key.code) {
                // Ctrl+T - Toggle syntax highlighting
                (m, KeyCode::Char('t')) if m.contains(KeyModifiers::CONTROL) => {
                    dialog.syntax_highlighting = !dialog.syntax_highlighting;
                }
                // Ctrl+C - Exit
                (m, KeyCode::Char('c')) if m.contains(KeyModifiers::CONTROL) => {
                    inner.exit(ExitReason::UserQuit);
                }
                // Esc - Exit
                (_, KeyCode::Esc) => {
                    inner.exit(ExitReason::UserQuit);
                }
                // Up - Move selection up
                (_, KeyCode::Up) => {
                    dialog.theme_up();
                }
                // Down - Move selection down
                (_, KeyCode::Down) => {
                    dialog.theme_down();
                }
                // 1-6 - Jump to specific theme
                (_, KeyCode::Char(c @ '1'..='6')) => {
                    dialog.selected_theme = (c as usize) - ('1' as usize);
                }
                // Enter - Accept theme, advance to login method
                (_, KeyCode::Enter) => {
                    dialog.advance_to_login();
                }
                _ => {}
            },
            SetupStep::LoginMethod => match (key.modifiers, key.code) {
                // Ctrl+C - Exit
                (m, KeyCode::Char('c')) if m.contains(KeyModifiers::CONTROL) => {
                    inner.exit(ExitReason::UserQuit);
                }
                // Esc - Exit
                (_, KeyCode::Esc) => {
                    inner.exit(ExitReason::UserQuit);
                }
                // Up - Move selection up
                (_, KeyCode::Up) => {
                    dialog.login_up();
                }
                // Down - Move selection down
                (_, KeyCode::Down) => {
                    dialog.login_down();
                }
                // 1-3 - Jump to specific option
                (_, KeyCode::Char(c @ '1'..='3')) => {
                    dialog.selected_login = (c as usize) - ('1' as usize);
                }
                // Enter - Accept login method, go to input mode
                (_, KeyCode::Enter) => {
                    inner.dialog.dismiss();
                    inner.mode = AppMode::Input;
                    drop(inner);
                    self.fire_notification(
                        NOTIFICATION_AUTH_SUCCESS,
                        "Auth Success",
                        "Login completed",
                    );
                }
                _ => {}
            },
        }
    }
}
