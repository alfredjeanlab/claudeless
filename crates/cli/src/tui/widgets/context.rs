// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Context usage visualization widget.
//!
//! Shown when user executes `/context` to view context allocation.

/// Data for context usage display
#[derive(Clone, Debug, Default)]
pub struct ContextUsage {
    pub system_prompt_tokens: u64,
    pub system_tools_tokens: u64,
    pub messages_tokens: u64,
    pub free_space_tokens: u64,
    pub autocompact_buffer_tokens: u64,
    pub total_tokens: u64,
    pub model_name: String,
}

impl ContextUsage {
    /// Create default context usage with typical values
    pub fn new() -> Self {
        Self {
            system_prompt_tokens: 2300,
            system_tools_tokens: 16700,
            messages_tokens: 8,
            free_space_tokens: 148_000,
            autocompact_buffer_tokens: 33_000,
            total_tokens: 200_000,
            model_name: "claude-haiku-4-5-20251001".to_string(),
        }
    }

    /// Create context usage with a specific model name
    pub fn new_with_model(model_name: String) -> Self {
        Self {
            model_name,
            ..Self::new()
        }
    }

    /// Total used tokens (system prompt + system tools + messages)
    pub fn total_used(&self) -> u64 {
        self.system_prompt_tokens + self.system_tools_tokens + self.messages_tokens
    }

    /// Used percentage of total tokens
    pub fn used_percentage(&self) -> f64 {
        self.percentage(self.total_used())
    }

    /// Calculate percentage for a token count
    pub fn percentage(&self, tokens: u64) -> f64 {
        if self.total_tokens == 0 {
            0.0
        } else {
            (tokens as f64 / self.total_tokens as f64) * 100.0
        }
    }

    /// Format tokens for display (e.g., "2.3k" or "148k")
    pub fn format_tokens(tokens: u64) -> String {
        if tokens >= 1000 {
            let k = tokens as f64 / 1000.0;
            if k >= 100.0 {
                format!("{:.0}k", k)
            } else {
                format!("{:.1}k", k)
            }
        } else {
            tokens.to_string()
        }
    }

    /// Format tokens concisely for summary display (e.g., "19k" instead of "19.0k")
    pub fn format_tokens_short(tokens: u64) -> String {
        if tokens >= 1000 {
            let k = tokens as f64 / 1000.0;
            if k >= 100.0 || (k - k.round()).abs() < 0.05 {
                format!("{:.0}k", k)
            } else {
                format!("{:.1}k", k)
            }
        } else {
            tokens.to_string()
        }
    }

    /// Get grid cells (100 total: 10 columns x 10 rows)
    /// Returns Vec of chars: ⛁ (used), ⛀ (last used/boundary), ⛶ (free space), ⛝ (autocompact)
    pub fn grid_cells(&self) -> Vec<char> {
        let total_used = self.total_used();
        let used_cells = ((total_used as f64 / self.total_tokens as f64) * 100.0).ceil() as usize;
        let autocompact_cells = ((self.autocompact_buffer_tokens as f64 / self.total_tokens as f64)
            * 100.0)
            .ceil() as usize;
        let free_cells = 100_usize
            .saturating_sub(used_cells)
            .saturating_sub(autocompact_cells);

        let mut cells = Vec::with_capacity(100);
        if used_cells > 1 {
            cells.extend(std::iter::repeat_n('\u{26C1}', used_cells - 1));
            cells.push('\u{26C0}'); // boundary marker is last used cell
        } else if used_cells == 1 {
            cells.push('\u{26C0}');
        }
        cells.extend(std::iter::repeat_n('\u{26F6}', free_cells));
        cells.extend(std::iter::repeat_n('\u{26DD}', autocompact_cells));
        cells.truncate(100);
        cells.extend(std::iter::repeat_n(
            '\u{26F6}',
            100_usize.saturating_sub(cells.len()),
        ));
        cells
    }
}
