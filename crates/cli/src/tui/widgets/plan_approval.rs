// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Plan approval dialog widget for ExitPlanMode tool.
//!
//! Presents the plan with bordered display and 4 approval options,
//! matching real Claude Code behavior.

/// Approval mode selected by the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalMode {
    /// Clear context and auto-accept edits (shift+tab)
    ClearContext,
    /// Auto-accept edits
    AutoAccept,
    /// Manually approve edits
    ManualApprove,
}

/// Result of user interaction with the plan approval dialog.
#[derive(Clone, Debug)]
pub enum PlanApprovalResult {
    /// User approved the plan with the selected mode.
    Approved(ApprovalMode),
    /// User provided free-text feedback to revise the plan.
    Revised(String),
    /// User cancelled (Escape).
    Cancelled,
}

/// State for the plan approval dialog.
#[derive(Clone, Debug)]
pub struct PlanApprovalState {
    /// The plan content (markdown) to display.
    pub plan_content: String,
    /// Path to the saved plan file.
    pub plan_file_path: String,
    /// Current cursor position (0..3 for 4 options).
    pub cursor: usize,
    /// Free-text input buffer for option 4.
    pub free_text: String,
    /// The tool use ID from the pending permission.
    pub tool_use_id: String,
    /// The original tool call input (for re-execution with approval).
    pub tool_input: serde_json::Value,
}

impl PlanApprovalState {
    /// Total number of selectable rows (4 options).
    const TOTAL_ROWS: usize = 4;

    /// Index of the free-text option.
    const FREE_TEXT_INDEX: usize = 3;

    /// Create from ExitPlanMode tool call input.
    pub fn from_tool_input(
        input: &serde_json::Value,
        tool_use_id: String,
        plan_file_path: String,
    ) -> Self {
        let plan_content = input
            .get("plan")
            .or_else(|| input.get("plan_content"))
            .or_else(|| input.get("planContent"))
            .or_else(|| input.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("# Plan\n\nNo content provided.")
            .to_string();

        Self {
            plan_content,
            plan_file_path,
            cursor: 0,
            free_text: String::new(),
            tool_use_id,
            tool_input: input.clone(),
        }
    }

    /// Whether the cursor is on the free-text option.
    pub fn is_on_free_text(&self) -> bool {
        self.cursor == Self::FREE_TEXT_INDEX
    }

    /// Move cursor up.
    ///
    /// Wraps from first option (0) to free-text (3), matching elicitation behavior.
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        } else {
            // Wrap to free-text option
            self.cursor = Self::FREE_TEXT_INDEX;
        }
    }

    /// Move cursor down.
    ///
    /// Clamps at the last option (3).
    pub fn cursor_down(&mut self) {
        if self.cursor < Self::TOTAL_ROWS - 1 {
            self.cursor += 1;
        }
    }

    /// Insert a character into the free-text buffer (only when on free-text row).
    pub fn insert_char(&mut self, c: char) {
        if self.is_on_free_text() {
            self.free_text.push(c);
        }
    }

    /// Delete the last character from the free-text buffer.
    pub fn backspace_char(&mut self) {
        if self.is_on_free_text() {
            self.free_text.pop();
        }
    }

    /// Select a specific option by number (1-based).
    ///
    /// Numbers 1-3 map to approval options; 4 selects free-text.
    pub fn select_by_number(&mut self, num: usize) {
        let idx = num.saturating_sub(1);
        if idx < Self::TOTAL_ROWS {
            self.cursor = idx;
        }
    }

    /// Collect the result from the current state.
    pub fn collect_result(&self) -> PlanApprovalResult {
        match self.cursor {
            0 => PlanApprovalResult::Approved(ApprovalMode::ClearContext),
            1 => PlanApprovalResult::Approved(ApprovalMode::AutoAccept),
            2 => PlanApprovalResult::Approved(ApprovalMode::ManualApprove),
            3 => {
                if self.free_text.is_empty() {
                    PlanApprovalResult::Cancelled
                } else {
                    PlanApprovalResult::Revised(self.free_text.clone())
                }
            }
            _ => PlanApprovalResult::Cancelled,
        }
    }

    /// Render the plan approval dialog for display.
    ///
    /// Layout matches real Claude Code ExitPlanMode dialog:
    /// ```text
    /// ────────────────────────────
    ///  Ready to code?
    ///  Here is Claude's plan:
    ///
    ///  ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
    ///  Plan content here...
    ///  ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
    ///
    /// ❯ 1. Yes, clear context and auto-accept edits (shift+tab)
    ///   2. Yes, auto-accept edits
    ///   3. Yes, manually approve edits
    ///   4. Type here to tell Claude what to change
    /// ────────────────────────────
    ///
    ///  ctrl-g to edit in VS Code · ~/.claude/plans/{name}.md
    ///  Enter to select · ↑/↓ to navigate · Esc to cancel
    /// ```
    pub fn render(&self, width: usize) -> String {
        let sep = "─".repeat(width);
        let plan_border = " ".to_string() + &"╌".repeat(width.saturating_sub(2));
        let mut lines = vec![sep.clone()];

        // Header
        lines.push(" Ready to code?".to_string());
        lines.push(" Here is Claude's plan:".to_string());
        lines.push(String::new());

        // Plan content with bordered display
        lines.push(plan_border.clone());
        for line in self.plan_content.lines() {
            lines.push(format!(" {}", line));
        }
        lines.push(plan_border);
        lines.push(String::new());

        // Options
        let options = [
            "Yes, clear context and auto-accept edits (shift+tab)",
            "Yes, auto-accept edits",
            "Yes, manually approve edits",
        ];

        for (i, label) in options.iter().enumerate() {
            let cursor = if i == self.cursor { "❯" } else { " " };
            lines.push(format!("{} {}. {}", cursor, i + 1, label));
        }

        // Free-text option
        let free_cursor = if self.cursor == Self::FREE_TEXT_INDEX {
            "❯"
        } else {
            " "
        };
        let free_label = if self.free_text.is_empty() {
            "Type here to tell Claude what to change".to_string()
        } else {
            self.free_text.clone()
        };
        lines.push(format!(
            "{} {}. {}",
            free_cursor,
            Self::FREE_TEXT_INDEX + 1,
            free_label
        ));

        lines.push(sep);
        lines.push(String::new());

        // Footer
        lines.push(format!(
            "  ctrl-g to edit in VS Code · {}",
            self.plan_file_path
        ));
        lines.push("  Enter to select · ↑/↓ to navigate · Esc to cancel".to_string());

        lines.join("\n")
    }
}

#[cfg(test)]
#[path = "plan_approval_tests.rs"]
mod tests;
