// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Simple list-style dialogs: trust, bypass, thinking, permission, memory.
//!
//! Each dialog's rendering and key handling are colocated here.

use iocraft::prelude::*;

use crate::tui::separator::make_separator;
use crate::tui::widgets::permission::PermissionSelection;
use crate::tui::widgets::thinking::{ThinkingDialog, ThinkingMode};
use crate::tui::widgets::trust::TrustChoice;
use crate::tui::widgets::MemoryDialog;

use crate::tui::app::state::TuiAppState;
use crate::tui::app::types::{
    AppMode, BypassChoice, BypassConfirmState, ExitReason, PermissionRequest, TrustPromptState,
};
use crate::tui::widgets::elicitation::ElicitationState;

use super::{DialogState, SelectionList};

// ── Trust prompt ─────────────────────────────────────────────────────────

/// Render trust prompt dialog
pub(crate) fn render_trust_prompt(prompt: &TrustPromptState, width: usize) -> AnyElement<'static> {
    let option_lines = SelectionList::new(&["Yes, proceed", "No, exit"])
        .selected(prompt.selected as usize)
        .lines();

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
            #(option_lines.into_iter().map(|line| {
                element! { Text(content: line) }
            }))
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · Esc to cancel")
        }
    }.into()
}

impl TuiAppState {
    /// Grant trust and either advance to setup wizard or dismiss to input.
    ///
    /// Returns `Some(prompt)` when a pending initial prompt should be processed
    /// after dropping the lock.
    fn grant_trust(&self, inner: &mut crate::tui::app::state::TuiAppStateInner) -> Option<String> {
        inner.trust_granted = true;
        if !inner.config.logged_in {
            let version = inner
                .config
                .claude_version
                .clone()
                .unwrap_or_else(|| crate::config::DEFAULT_CLAUDE_VERSION.to_string());
            inner.dialog = DialogState::Setup(crate::tui::widgets::setup::SetupState::new(version));
            inner.mode = AppMode::Setup;
            None
        } else {
            inner.dialog.dismiss();
            let initial = inner.pending_initial_prompt.take();
            if initial.is_none() {
                inner.mode = AppMode::Input;
            }
            initial
        }
    }

    /// Handle key events in trust prompt mode
    pub(in crate::tui::app) fn handle_trust_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            // Left/Right/Tab - Toggle selection
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                if let Some(prompt) = inner.dialog.as_trust_mut() {
                    prompt.selected = match prompt.selected {
                        TrustChoice::Yes => TrustChoice::No,
                        TrustChoice::No => TrustChoice::Yes,
                    };
                }
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                if let Some(prompt) = inner.dialog.as_trust() {
                    match prompt.selected {
                        TrustChoice::Yes => {
                            if let Some(initial) = self.grant_trust(&mut inner) {
                                drop(inner);
                                self.process_prompt(initial);
                            }
                        }
                        TrustChoice::No => {
                            inner.exit(ExitReason::UserQuit);
                        }
                    }
                }
            }

            // Y/y - Yes (trust)
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(initial) = self.grant_trust(&mut inner) {
                    drop(inner);
                    self.process_prompt(initial);
                }
            }

            // N/n or Escape - No (exit)
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                inner.exit(ExitReason::UserQuit);
            }

            _ => {}
        }
    }
}

// ── Bypass confirm ───────────────────────────────────────────────────────

/// Render bypass permissions confirmation dialog
pub(crate) fn render_bypass_confirm_dialog(
    state: &BypassConfirmState,
    width: usize,
) -> AnyElement<'static> {
    let option_lines = SelectionList::new(&["No, exit", "Yes, I accept"])
        .selected(state.selected as usize)
        .lines();

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            Text(content: make_separator(width))
            Text(content: " WARNING: Claude Code running in Bypass Permissions mode")
            Text(content: "")
            Text(content: " In Bypass Permissions mode, Claude Code will not ask for your approval before")
            Text(content: " running potentially dangerous commands.")
            Text(content: " This mode should only be used in a sandboxed container/VM that has restricted")
            Text(content: " internet access and can easily be restored if damaged.")
            Text(content: "")
            Text(content: " By proceeding, you accept all responsibility for actions taken while running")
            Text(content: " in Bypass Permissions mode.")
            Text(content: "")
            Text(content: " https://code.claude.com/docs/en/security")
            Text(content: "")
            #(option_lines.into_iter().map(|line| {
                element! { Text(content: line) }
            }))
            Text(content: "")
            Text(content: " Enter to confirm · Esc to cancel")
        }
    }
    .into()
}

impl TuiAppState {
    /// Handle key events in bypass confirmation mode
    pub(in crate::tui::app) fn handle_bypass_confirm_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            // Up/Down/Tab - Toggle selection
            KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                if let Some(dialog) = inner.dialog.as_bypass_confirm_mut() {
                    dialog.selected = match dialog.selected {
                        BypassChoice::No => BypassChoice::Yes,
                        BypassChoice::Yes => BypassChoice::No,
                    };
                }
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                if let Some(dialog) = inner.dialog.as_bypass_confirm() {
                    match dialog.selected {
                        BypassChoice::Yes => {
                            // Enable bypass mode
                            inner.permission_mode =
                                crate::permission::PermissionMode::BypassPermissions;
                            inner.allow_bypass_permissions = true;
                            inner.dialog.dismiss();
                            // Check for pending initial prompt
                            if let Some(initial) = inner.pending_initial_prompt.take() {
                                drop(inner);
                                self.process_prompt(initial);
                            } else {
                                inner.mode = AppMode::Input;
                            }
                        }
                        BypassChoice::No => {
                            inner.exit(ExitReason::UserQuit);
                        }
                    }
                }
            }

            // Escape - Exit (same as No)
            KeyCode::Esc => {
                inner.exit(ExitReason::UserQuit);
            }

            _ => {}
        }
    }
}

// ── Thinking toggle ──────────────────────────────────────────────────────

/// Render thinking toggle dialog
pub(crate) fn render_thinking_dialog(dialog: &ThinkingDialog, width: usize) -> AnyElement<'static> {
    let option_lines = SelectionList::new(&["Enabled", "Disabled"])
        .descriptions(&[
            "Claude will think before responding",
            "Claude will respond without extended thinking",
        ])
        .selected(dialog.selected as usize)
        .current(dialog.current as usize)
        .lines();

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
            Text(content: " Enable or disable thinking for this session.")
            Text(content: "")
            // Options with descriptions
            #(option_lines.into_iter().map(|line| {
                element! { Text(content: line) }
            }))
            Text(content: "")
            // Footer
            Text(content: " Enter to confirm · Esc to exit")
        }
    }
    .into()
}

impl TuiAppState {
    /// Handle key events in thinking toggle mode
    pub(in crate::tui::app) fn handle_thinking_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();
        match key.code {
            // Up/Down arrows, Tab - Toggle selection
            KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                if let Some(dialog) = inner.dialog.as_thinking_mut() {
                    dialog.selected = match dialog.selected {
                        ThinkingMode::Enabled => ThinkingMode::Disabled,
                        ThinkingMode::Disabled => ThinkingMode::Enabled,
                    };
                }
            }

            // 1 - Select Enabled and confirm
            KeyCode::Char('1') => {
                inner.thinking_enabled = true;
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }

            // 2 - Select Disabled and confirm
            KeyCode::Char('2') => {
                inner.thinking_enabled = false;
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                if let Some(dialog) = inner.dialog.as_thinking() {
                    inner.thinking_enabled = dialog.selected == ThinkingMode::Enabled;
                }
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }

            // Escape - Cancel (close without changing)
            KeyCode::Esc => {
                inner.dialog.dismiss();
                inner.mode = AppMode::Input;
            }

            _ => {}
        }
    }
}

// ── Permission dialog ────────────────────────────────────────────────────

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

impl TuiAppState {
    /// Handle key events in permission mode
    pub(in crate::tui::app) fn handle_permission_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let select_and_confirm =
            |selection: PermissionSelection,
             inner: &mut crate::tui::app::state::TuiAppStateInner| {
                if let Some(perm) = inner.dialog.as_permission_mut() {
                    perm.dialog.selected = selection;
                }
            };

        match key.code {
            // Up - Move selection up
            KeyCode::Up => {
                if let Some(perm) = inner.dialog.as_permission_mut() {
                    perm.dialog.selected = perm.dialog.selected.prev();
                }
            }

            // Down - Move selection down
            KeyCode::Down => {
                if let Some(perm) = inner.dialog.as_permission_mut() {
                    perm.dialog.selected = perm.dialog.selected.next();
                }
            }

            // Enter - Confirm selection
            KeyCode::Enter => {
                drop(inner);
                self.confirm_permission();
            }

            // 1 - Select Yes and confirm
            KeyCode::Char('1') => {
                select_and_confirm(PermissionSelection::Yes, &mut inner);
                drop(inner);
                self.confirm_permission();
            }

            // Y/y - Select Yes and confirm
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                select_and_confirm(PermissionSelection::Yes, &mut inner);
                drop(inner);
                self.confirm_permission();
            }

            // 2 - Select Yes for session and confirm
            KeyCode::Char('2') => {
                select_and_confirm(PermissionSelection::YesSession, &mut inner);
                drop(inner);
                self.confirm_permission();
            }

            // 3 - Select No and confirm
            KeyCode::Char('3') => {
                select_and_confirm(PermissionSelection::No, &mut inner);
                drop(inner);
                self.confirm_permission();
            }

            // N/n - Select No and confirm
            KeyCode::Char('n') | KeyCode::Char('N') => {
                select_and_confirm(PermissionSelection::No, &mut inner);
                drop(inner);
                self.confirm_permission();
            }

            // Escape - Cancel (select No)
            KeyCode::Esc => {
                select_and_confirm(PermissionSelection::No, &mut inner);
                drop(inner);
                self.confirm_permission();
            }

            _ => {}
        }
    }
}

// ── Memory dialog ────────────────────────────────────────────────────────

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

    let footer = " Enter to view · Esc to cancel".to_string();

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

impl TuiAppState {
    /// Handle key events in memory dialog mode
    pub(in crate::tui::app) fn handle_memory_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(dialog) = inner.dialog.as_memory_mut() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                inner.dismiss_dialog("Memory dialog");
            }
            KeyCode::Up => dialog.select_prev(),
            KeyCode::Down => dialog.select_next(),
            KeyCode::Enter => {
                // Open selected memory file for viewing/editing
                // For now, just show the path of the selected entry
                if let Some(entry) = dialog.selected_entry() {
                    let path = entry.path.as_deref().unwrap_or("(not configured)");
                    inner.display.response_content =
                        format!("Selected: {} - {}", entry.source.name(), path);
                    inner.display.is_command_output = true;
                    inner.dialog.dismiss();
                    inner.mode = AppMode::Input;
                }
            }
            _ => {}
        }
    }
}

// ── Elicitation dialog ──────────────────────────────────────────────────

/// Render elicitation dialog (AskUserQuestion)
pub(crate) fn render_elicitation_dialog(
    state: &ElicitationState,
    width: usize,
) -> AnyElement<'static> {
    let content = state.render(width);

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

impl TuiAppState {
    /// Handle key events in elicitation dialog mode
    pub(in crate::tui::app) fn handle_elicitation_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        match key.code {
            KeyCode::Up => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.cursor_up();
                }
            }
            KeyCode::Down => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.cursor_down();
                }
            }
            KeyCode::Char(' ') => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.toggle_or_select();
                }
            }
            KeyCode::Tab => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.next_question();
                }
            }
            KeyCode::BackTab => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.prev_question();
                }
            }
            KeyCode::Char(c @ '1'..='4') => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    let num = (c as u8 - b'0') as usize;
                    state.select_by_number(num);
                }
            }
            KeyCode::Enter => {
                drop(inner);
                self.confirm_elicitation();
            }
            KeyCode::Esc => {
                drop(inner);
                self.cancel_elicitation();
            }
            _ => {}
        }
    }
}
