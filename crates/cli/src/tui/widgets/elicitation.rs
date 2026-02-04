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
    pub cursor: usize,
    /// Indices of selected options (for multi-select).
    pub selected: Vec<usize>,
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
}

/// State for the elicitation dialog.
#[derive(Clone, Debug)]
pub struct ElicitationState {
    pub questions: Vec<ElicitationQuestion>,
    /// Index of the currently displayed question.
    pub current_question: usize,
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
            tool_input: input.clone(),
            tool_use_id,
        }
    }

    /// Move cursor up in current question.
    pub fn cursor_up(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            if q.cursor > 0 {
                q.cursor -= 1;
            }
        }
    }

    /// Move cursor down in current question.
    pub fn cursor_down(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            if q.cursor + 1 < q.options.len() {
                q.cursor += 1;
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

    /// Move to next question.
    pub fn next_question(&mut self) {
        if self.current_question + 1 < self.questions.len() {
            self.current_question += 1;
        }
    }

    /// Move to previous question.
    pub fn prev_question(&mut self) {
        if self.current_question > 0 {
            self.current_question -= 1;
        }
    }

    /// Collect answers from all questions.
    pub fn collect_answers(&self) -> ElicitationResult {
        let mut answers = HashMap::new();
        for q in &self.questions {
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

    /// Render the current question for display.
    ///
    /// Layout matches real Claude Code elicitation dialog:
    /// ```text
    /// ────────────────────────────
    ///  ☐ Header
    ///
    /// Question text?
    ///
    /// ❯ 1. Label
    ///      Description
    ///   2. Label
    ///      Description
    /// ────────────────────────────
    ///
    /// Enter to select · ↑/↓ to navigate · Esc to cancel
    /// ```
    pub fn render(&self, width: usize) -> String {
        let sep = "─".repeat(width);
        let mut lines = vec![sep.clone()];

        if let Some(q) = self.questions.get(self.current_question) {
            // Header line: ☐ Header (matches real Claude Code)
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
            } else {
                lines.push(format!(" ☐ {}", q.header));
            }
            lines.push(String::new());

            // Question text
            lines.push(q.question.clone());
            lines.push(String::new());

            // Options with cursor indicator
            for (i, opt) in q.options.iter().enumerate() {
                let cursor = if i == q.cursor { "❯" } else { " " };
                lines.push(format!("{} {}. {}", cursor, i + 1, opt.label));
                if !opt.description.is_empty() {
                    lines.push(format!("     {}", opt.description));
                }
            }
            lines.push(sep.clone());

            // Footer
            let nav = if self.questions.len() > 1 {
                format!(
                    "  Question {}/{} · Tab for next · ",
                    self.current_question + 1,
                    self.questions.len()
                )
            } else {
                "  ".to_string()
            };
            let action = if q.multi_select {
                "Space to toggle · Enter to confirm · Esc to cancel"
            } else {
                "Enter to select · ↑/↓ to navigate · Esc to cancel"
            };
            lines.push(format!("{}{}", nav, action));
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
    }
}

#[cfg(test)]
#[path = "elicitation_tests.rs"]
mod tests;
