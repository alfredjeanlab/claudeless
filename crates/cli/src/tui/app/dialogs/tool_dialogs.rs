// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tool-driven dialogs: elicitation (AskUserQuestion) and plan approval (ExitPlanMode).

use iocraft::prelude::*;

use crate::tui::app::state::TuiAppState;
use crate::tui::widgets::elicitation::ElicitationState;
use crate::tui::widgets::plan_approval::PlanApprovalState;

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

        // Check current state
        let on_free_text = inner
            .dialog
            .as_elicitation()
            .is_some_and(|s| s.is_on_free_text());
        let on_submit_tab = inner
            .dialog
            .as_elicitation()
            .is_some_and(|s| s.is_on_submit_tab());

        match key.code {
            KeyCode::Up => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    if state.on_submit_tab {
                        state.submit_cursor_up();
                    } else {
                        state.cursor_up();
                    }
                }
            }
            KeyCode::Down => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    if state.on_submit_tab {
                        state.submit_cursor_down();
                    } else {
                        state.cursor_down();
                    }
                }
            }
            KeyCode::Left if on_submit_tab => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.prev_question();
                }
            }
            KeyCode::Right if on_submit_tab => {
                // Already on last tab, no-op
            }
            KeyCode::Char(' ') if !on_free_text && !on_submit_tab => {
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
            KeyCode::Char(c @ '1'..='9') if !on_free_text && !on_submit_tab => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    let num = (c as u8 - b'0') as usize;
                    state.select_by_number(num);
                    // Auto-advance for single-select; multi-select stays
                    let is_multi = state
                        .questions
                        .get(state.current_question)
                        .is_some_and(|q| q.multi_select);
                    if !is_multi {
                        state.next_question();
                    }
                }
            }
            // Submit tab: number keys for Submit/Cancel
            KeyCode::Char('1') if on_submit_tab => {
                drop(inner);
                self.confirm_elicitation();
            }
            KeyCode::Char('2') if on_submit_tab => {
                drop(inner);
                self.cancel_elicitation();
            }
            // Free-text input: characters when on "Type something."
            KeyCode::Char(c) if on_free_text => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.insert_char(c);
                }
            }
            KeyCode::Backspace if on_free_text => {
                if let Some(state) = inner.dialog.as_elicitation_mut() {
                    state.backspace_char();
                }
            }
            KeyCode::Enter => {
                if on_submit_tab {
                    // On submit tab: cursor 0 = submit, cursor 1 = cancel
                    let submit_cursor = inner
                        .dialog
                        .as_elicitation()
                        .map(|s| s.submit_cursor)
                        .unwrap_or(0);
                    drop(inner);
                    if submit_cursor == 0 {
                        self.confirm_elicitation();
                    } else {
                        self.cancel_elicitation();
                    }
                } else {
                    // Check for "Chat about this" — immediately confirm
                    let on_chat = inner.dialog.as_elicitation().is_some_and(|s| {
                        s.questions
                            .get(s.current_question)
                            .is_some_and(|q| q.cursor == ElicitationState::chat_about_this_index(q))
                    });
                    if on_chat {
                        drop(inner);
                        self.confirm_elicitation();
                        return;
                    }
                    // On a question: single-select → select and advance; multi-select → toggle
                    let is_multi = inner
                        .dialog
                        .as_elicitation()
                        .and_then(|s| s.questions.get(s.current_question))
                        .is_some_and(|q| q.multi_select);
                    if is_multi {
                        // Multi-select: Enter confirms this question's selections, advance
                        if let Some(state) = inner.dialog.as_elicitation_mut() {
                            state.next_question();
                        }
                    } else if on_free_text {
                        // Free-text: Enter submits the free-text answer, advance
                        if let Some(state) = inner.dialog.as_elicitation_mut() {
                            state.next_question();
                        }
                    } else {
                        // Single-select: select at cursor and advance
                        if let Some(state) = inner.dialog.as_elicitation_mut() {
                            state.select_and_advance();
                        }
                    }
                }
            }
            KeyCode::Esc => {
                drop(inner);
                self.cancel_elicitation();
            }
            _ => {}
        }
    }
}

// ── Plan approval dialog ────────────────────────────────────────────────

/// Render plan approval dialog (ExitPlanMode)
pub(crate) fn render_plan_approval_dialog(
    state: &PlanApprovalState,
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
    /// Handle key events in plan approval dialog mode
    pub(in crate::tui::app) fn handle_plan_approval_key(&self, key: KeyEvent) {
        // Handle Ctrl+C before taking the lock (cancel_plan_approval takes its own lock)
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.cancel_plan_approval();
            return;
        }

        let mut inner = self.inner.lock();

        // Check if cursor is on the free-text row
        let on_free_text = inner
            .dialog
            .as_plan_approval()
            .is_some_and(|s| s.is_on_free_text());

        match key.code {
            KeyCode::Up => {
                if let Some(state) = inner.dialog.as_plan_approval_mut() {
                    state.cursor_up();
                }
            }
            KeyCode::Down => {
                if let Some(state) = inner.dialog.as_plan_approval_mut() {
                    state.cursor_down();
                }
            }
            KeyCode::Char(c @ '1'..='3') if !on_free_text => {
                if let Some(state) = inner.dialog.as_plan_approval_mut() {
                    let num = (c as u8 - b'0') as usize;
                    state.select_by_number(num);
                }
                // Number keys immediately select and submit
                drop(inner);
                self.confirm_plan_approval();
            }
            // Free-text input: characters when on "Type here..." option
            KeyCode::Char(c) if on_free_text => {
                if let Some(state) = inner.dialog.as_plan_approval_mut() {
                    state.insert_char(c);
                }
            }
            KeyCode::Backspace if on_free_text => {
                if let Some(state) = inner.dialog.as_plan_approval_mut() {
                    state.backspace_char();
                }
            }
            KeyCode::Enter => {
                drop(inner);
                self.confirm_plan_approval();
            }
            KeyCode::Esc => {
                drop(inner);
                self.cancel_plan_approval();
            }
            _ => {}
        }
    }
}
