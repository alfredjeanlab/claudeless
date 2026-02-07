// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Elicitation dialog widget for AskUserQuestion tool.
//!
//! Presents questions with selectable options for user input.

use std::collections::HashMap;

/// A parsed question ready for display.
#[derive(Clone, Debug)]
pub struct ElicitationQuestion {
    pub header: String,
    pub question: String,
    pub options: Vec<ElicitationOption>,
    pub multi_select: bool,
    /// Index of the cursor (highlighted option).
    /// Options 0..N are defined options, N is "Type something.", N+1 is "Chat about this".
    pub cursor: usize,
    /// Indices of selected options (for multi-select).
    pub selected: Vec<usize>,
    /// Free-text input buffer for the "Type something." option.
    pub free_text: String,
}

/// A single option in a question.
#[derive(Clone, Debug)]
pub struct ElicitationOption {
    pub label: String,
    pub description: String,
}

/// Result of user interaction with the elicitation dialog.
#[derive(Clone, Debug)]
pub enum ElicitationResult {
    Answered(HashMap<String, String>),
    Cancelled,
    /// User selected "Chat about this" — rejection with clarification request.
    ChatAboutThis,
}

/// State for the elicitation dialog.
#[derive(Clone, Debug)]
pub struct ElicitationState {
    pub questions: Vec<ElicitationQuestion>,
    /// Index of the currently displayed question.
    pub current_question: usize,
    /// Whether the user is on the submit review tab.
    pub on_submit_tab: bool,
    /// Cursor position on the submit tab (0 = Submit, 1 = Cancel).
    pub submit_cursor: usize,
    /// The original tool call input (for re-execution with answers).
    pub tool_input: serde_json::Value,
    /// The tool use ID from the pending permission.
    pub tool_use_id: String,
}

impl ElicitationState {
    /// Parse from AskUserQuestion tool call input.
    pub fn from_tool_input(input: &serde_json::Value, tool_use_id: String) -> Self {
        let questions = input
            .get("questions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(parse_question).collect())
            .unwrap_or_default();

        Self {
            questions,
            current_question: 0,
            on_submit_tab: false,
            submit_cursor: 0,
            tool_input: input.clone(),
            tool_use_id,
        }
    }

    /// Total number of selectable rows: defined options + "Type something." + "Chat about this".
    fn total_rows(q: &ElicitationQuestion) -> usize {
        q.options.len() + 2
    }

    /// Index of the "Type something." row.
    fn type_something_index(q: &ElicitationQuestion) -> usize {
        q.options.len()
    }

    /// Index of the "Chat about this" row.
    pub(crate) fn chat_about_this_index(q: &ElicitationQuestion) -> usize {
        q.options.len() + 1
    }

    /// Whether the user is viewing the submit review tab.
    pub fn is_on_submit_tab(&self) -> bool {
        self.on_submit_tab
    }

    /// Whether a question has been answered (has a selection or free-text).
    pub fn is_question_answered(&self, idx: usize) -> bool {
        self.questions.get(idx).is_some_and(|q| {
            !q.selected.is_empty()
                || (q.cursor == Self::type_something_index(q) && !q.free_text.is_empty())
        })
    }

    /// Whether the cursor is on the "Type something." row.
    pub fn is_on_free_text(&self) -> bool {
        !self.on_submit_tab
            && self
                .questions
                .get(self.current_question)
                .is_some_and(|q| q.cursor == Self::type_something_index(q))
    }

    /// Move cursor up in current question.
    ///
    /// Wraps from first defined option (0) to "Type something." — matching real Claude Code
    /// where Up from position 0 skips "Chat about this" and lands on "Type something.".
    pub fn cursor_up(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            if q.cursor > 0 {
                q.cursor -= 1;
            } else {
                // Wrap to "Type something." (skip "Chat about this")
                q.cursor = Self::type_something_index(q);
            }
        }
    }

    /// Move cursor down in current question.
    ///
    /// Clamps at "Chat about this" (the last row) — no wrap on Down.
    pub fn cursor_down(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            let max = Self::total_rows(q).saturating_sub(1);
            if q.cursor < max {
                q.cursor += 1;
            }
        }
    }

    /// Insert a character into the free-text buffer (only when on "Type something." row).
    pub fn insert_char(&mut self, c: char) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            if q.cursor == Self::type_something_index(q) {
                q.free_text.push(c);
            }
        }
    }

    /// Delete the last character from the free-text buffer.
    pub fn backspace_char(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            if q.cursor == Self::type_something_index(q) {
                q.free_text.pop();
            }
        }
    }

    /// Toggle selection at cursor (for multi-select) or select (for single-select).
    pub fn toggle_or_select(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            let idx = q.cursor;
            if q.multi_select {
                if let Some(pos) = q.selected.iter().position(|&i| i == idx) {
                    q.selected.remove(pos);
                } else {
                    q.selected.push(idx);
                }
            } else {
                q.selected = vec![idx];
            }
        }
    }

    /// Select a specific option by number (1-based).
    ///
    /// Numbers map to defined options only (not "Type something." or "Chat about this").
    pub fn select_by_number(&mut self, num: usize) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            let idx = num.saturating_sub(1);
            if idx < q.options.len() {
                q.cursor = idx;
                if q.multi_select {
                    if let Some(pos) = q.selected.iter().position(|&i| i == idx) {
                        q.selected.remove(pos);
                    } else {
                        q.selected.push(idx);
                    }
                } else {
                    q.selected = vec![idx];
                }
            }
        }
    }

    /// Select current option and advance to next question (single-select only).
    pub fn select_and_advance(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            if !q.multi_select {
                let idx = q.cursor;
                q.selected = vec![idx];
                // Advance to next question or submit tab
                if self.current_question + 1 < self.questions.len() {
                    self.current_question += 1;
                } else {
                    self.on_submit_tab = true;
                }
            }
        }
    }

    /// Move to next question (or submit tab from last question).
    pub fn next_question(&mut self) {
        if self.on_submit_tab {
            return;
        }
        if self.current_question + 1 < self.questions.len() {
            self.current_question += 1;
        } else {
            self.on_submit_tab = true;
        }
    }

    /// Move to previous question (or back from submit tab).
    pub fn prev_question(&mut self) {
        if self.on_submit_tab {
            self.on_submit_tab = false;
            return;
        }
        if self.current_question > 0 {
            self.current_question -= 1;
        }
    }

    /// Move submit tab cursor up.
    pub fn submit_cursor_up(&mut self) {
        if self.submit_cursor > 0 {
            self.submit_cursor -= 1;
        }
    }

    /// Move submit tab cursor down.
    pub fn submit_cursor_down(&mut self) {
        if self.submit_cursor < 1 {
            self.submit_cursor += 1;
        }
    }

    /// Collect answers from all questions.
    pub fn collect_answers(&self) -> ElicitationResult {
        let mut answers = HashMap::new();
        for q in &self.questions {
            // Check if cursor is on "Chat about this"
            if q.cursor == Self::chat_about_this_index(q) {
                return ElicitationResult::ChatAboutThis;
            }
            // Check if cursor is on "Type something."
            if q.cursor == Self::type_something_index(q) {
                if q.free_text.is_empty() {
                    return ElicitationResult::Cancelled;
                }
                answers.insert(q.question.clone(), q.free_text.clone());
                continue;
            }
            if q.selected.is_empty() {
                // Default to first option if nothing selected
                if let Some(opt) = q.options.first() {
                    answers.insert(q.question.clone(), opt.label.clone());
                }
            } else if q.multi_select {
                let labels: Vec<&str> = q
                    .selected
                    .iter()
                    .filter_map(|&i| q.options.get(i))
                    .map(|o| o.label.as_str())
                    .collect();
                answers.insert(q.question.clone(), labels.join(", "));
            } else if let Some(&idx) = q.selected.first() {
                if let Some(opt) = q.options.get(idx) {
                    answers.insert(q.question.clone(), opt.label.clone());
                }
            }
        }
        ElicitationResult::Answered(answers)
    }

    /// Render the tab bar showing all questions + Submit.
    fn render_tab_bar(&self) -> String {
        if self.questions.len() <= 1 {
            return String::new();
        }
        let mut tabs = Vec::new();
        for (i, q) in self.questions.iter().enumerate() {
            let check = if self.is_question_answered(i) {
                "☒"
            } else {
                "☐"
            };
            if i == self.current_question && !self.on_submit_tab {
                tabs.push(format!("[{} {}]", check, q.header));
            } else {
                tabs.push(format!("{} {}", check, q.header));
            }
        }
        // Submit tab
        if self.on_submit_tab {
            tabs.push("[✔ Submit]".to_string());
        } else {
            tabs.push("✔ Submit".to_string());
        }
        format!("← {} →", tabs.join("  "))
    }

    /// Render the submit review page.
    fn render_submit_page(&self, width: usize) -> String {
        let sep = "─".repeat(width);
        let mut lines = vec![sep.clone()];

        // Tab bar
        let tab_bar = self.render_tab_bar();
        if !tab_bar.is_empty() {
            lines.push(format!(" {}", tab_bar));
        }
        lines.push(String::new());

        lines.push("Review your answers".to_string());
        lines.push(String::new());

        let all_answered = (0..self.questions.len()).all(|i| self.is_question_answered(i));

        for q in &self.questions {
            lines.push(format!(" ● {}", q.question));
            if !q.selected.is_empty() {
                let labels: Vec<&str> = q
                    .selected
                    .iter()
                    .filter_map(|&i| q.options.get(i))
                    .map(|o| o.label.as_str())
                    .collect();
                if !labels.is_empty() {
                    lines.push(format!("   → {}", labels.join(", ")));
                } else {
                    lines.push("   (no answer)".to_string());
                }
            } else if q.cursor == Self::type_something_index(q) && !q.free_text.is_empty() {
                lines.push(format!("   → {}", q.free_text));
            } else {
                lines.push("   (no answer)".to_string());
            }
        }
        lines.push(String::new());

        if !all_answered {
            lines.push("⚠ You have not answered all questions".to_string());
            lines.push(String::new());
        }

        lines.push("Ready to submit your answers?".to_string());
        lines.push(String::new());

        // Submit/Cancel options
        let submit_cursor = if self.submit_cursor == 0 { "❯" } else { " " };
        let cancel_cursor = if self.submit_cursor == 1 { "❯" } else { " " };
        lines.push(format!("{} 1. Submit answers", submit_cursor));
        lines.push(format!("{} 2. Cancel", cancel_cursor));

        // Footer
        lines.push("  Enter to select · Tab/Arrow keys to navigate · Esc to cancel".to_string());

        lines.join("\n")
    }

    /// Render the current question for display.
    ///
    /// Layout matches real Claude Code elicitation dialog:
    /// ```text
    /// ────────────────────────────
    /// ← ☐ Language  ☒ Project  [✔ Submit] →
    ///
    ///  ☐ Header
    ///
    /// Question text?
    ///
    /// ❯ 1. Label ✔
    ///      Description
    ///   2. Label
    ///      Description
    /// ────────────────────────────
    ///
    /// Enter to select · Tab/Arrow keys to navigate · Esc to cancel
    /// ```
    pub fn render(&self, width: usize) -> String {
        if self.on_submit_tab {
            return self.render_submit_page(width);
        }

        let sep = "─".repeat(width);
        let mut lines = vec![sep.clone()];

        // Tab bar (only for multi-question)
        let tab_bar = self.render_tab_bar();
        if !tab_bar.is_empty() {
            lines.push(format!(" {}", tab_bar));
        }

        if let Some(q) = self.questions.get(self.current_question) {
            // Header line: ☐/☒ Header
            if q.multi_select {
                let checked = q.selected.len();
                let total = q.options.len();
                if checked == total {
                    lines.push(format!(" ☑ {}", q.header));
                } else if checked > 0 {
                    lines.push(format!(" ☒ {}", q.header));
                } else {
                    lines.push(format!(" ☐ {}", q.header));
                }
            } else if !q.selected.is_empty() {
                lines.push(format!(" ☒ {}", q.header));
            } else {
                lines.push(format!(" ☐ {}", q.header));
            }
            lines.push(String::new());

            // Question text
            lines.push(q.question.clone());
            lines.push(String::new());

            // Options with cursor indicator and ✔ for selected
            let type_idx = Self::type_something_index(q);
            let chat_idx = Self::chat_about_this_index(q);
            for (i, opt) in q.options.iter().enumerate() {
                let cursor = if i == q.cursor { "❯" } else { " " };
                let check = if q.selected.contains(&i) { " ✔" } else { "" };
                lines.push(format!("{} {}. {}{}", cursor, i + 1, opt.label, check));
                if !opt.description.is_empty() {
                    lines.push(format!("     {}", opt.description));
                }
            }

            // "Type something." option
            let type_cursor = if q.cursor == type_idx { "❯" } else { " " };
            let type_label = if q.free_text.is_empty() {
                "Type something.".to_string()
            } else {
                q.free_text.clone()
            };
            lines.push(format!(
                "{} {}. {}",
                type_cursor,
                q.options.len() + 1,
                type_label
            ));

            // Separator before "Chat about this"
            lines.push(sep.clone());

            // "Chat about this" option
            let chat_cursor = if q.cursor == chat_idx { "❯" } else { " " };
            lines.push(format!(
                "{} {}. Chat about this",
                chat_cursor,
                q.options.len() + 2,
            ));

            // Footer
            let action = if q.multi_select {
                "  Space to toggle · Enter to confirm · Tab/Arrow keys to navigate · Esc to cancel"
            } else {
                "  Enter to select · Tab/Arrow keys to navigate · Esc to cancel"
            };
            lines.push(action.to_string());
        }

        lines.join("\n")
    }
}

fn parse_question(value: &serde_json::Value) -> ElicitationQuestion {
    let header = value
        .get("header")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let question = value
        .get("question")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let multi_select = value
        .get("multiSelect")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let options = value
        .get("options")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|o| ElicitationOption {
                    label: o
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    description: o
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    ElicitationQuestion {
        header,
        question,
        options,
        multi_select,
        cursor: 0,
        selected: vec![],
        free_text: String::new(),
    }
}

#[cfg(test)]
#[path = "elicitation_tests.rs"]
mod tests;
