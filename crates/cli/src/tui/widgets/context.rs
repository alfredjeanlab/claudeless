// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Context usage visualization widget.
//!
//! Shown when user executes `/context` to view context allocation.

/// Memory file info
#[derive(Clone, Debug)]
pub struct MemoryFile {
    pub path: String,
    pub tokens: u64,
}

/// Data for context usage display
#[derive(Clone, Debug, Default)]
pub struct ContextUsage {
    pub system_prompt_tokens: u64,
    pub system_tools_tokens: u64,
    pub memory_files_tokens: u64,
    pub messages_tokens: u64,
    pub free_space_tokens: u64,
    pub autocompact_buffer_tokens: u64,
    pub total_tokens: u64,
    pub memory_files: Vec<MemoryFile>,
}

impl ContextUsage {
    /// Create default context usage with typical values
    pub fn new() -> Self {
        Self {
            system_prompt_tokens: 2800,
            system_tools_tokens: 16300,
            memory_files_tokens: 659,
            messages_tokens: 787,
            free_space_tokens: 134_000,
            autocompact_buffer_tokens: 45_000,
            total_tokens: 200_000,
            memory_files: vec![MemoryFile {
                path: "CLAUDE.md".to_string(),
                tokens: 659,
            }],
        }
    }

    /// Calculate percentage for a token count
    pub fn percentage(&self, tokens: u64) -> f64 {
        if self.total_tokens == 0 {
            0.0
        } else {
            (tokens as f64 / self.total_tokens as f64) * 100.0
        }
    }

    /// Format tokens for display (e.g., "2.8k" or "134k")
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

    /// Get grid cells (90 total: 10 columns x 9 rows)
    /// Returns Vec of chars: ⛀ (used marker), ⛶ (free space), ⛝ (autocompact buffer)
    pub fn grid_cells(&self) -> Vec<char> {
        // Calculate proportions out of 90 cells
        let total_used = self.system_prompt_tokens
            + self.system_tools_tokens
            + self.memory_files_tokens
            + self.messages_tokens;
        let used_cells = ((total_used as f64 / self.total_tokens as f64) * 90.0).ceil() as usize;
        let autocompact_cells = ((self.autocompact_buffer_tokens as f64 / self.total_tokens as f64)
            * 90.0)
            .ceil() as usize;
        let free_cells = 90_usize
            .saturating_sub(used_cells)
            .saturating_sub(autocompact_cells);

        let mut cells = Vec::with_capacity(90);

        // First cell is ⛀ (used marker), rest of used space shown as free
        // (the grid is a visual indicator, details are in the legend)
        if used_cells > 0 {
            cells.push('⛀');
            // Remaining used cells shown as free space in grid
            cells.extend(std::iter::repeat_n('⛶', used_cells.saturating_sub(1)));
        }

        // Free space cells
        cells.extend(std::iter::repeat_n('⛶', free_cells));

        // Autocompact buffer cells
        cells.extend(std::iter::repeat_n('⛝', autocompact_cells));

        // Ensure exactly 90 cells
        cells.truncate(90);
        cells.extend(std::iter::repeat_n(
            '⛶',
            90_usize.saturating_sub(cells.len()),
        ));

        cells
    }
}
