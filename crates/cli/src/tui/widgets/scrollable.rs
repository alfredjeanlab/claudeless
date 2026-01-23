// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Reusable scroll-aware navigation helper for dialog widgets.
//!
//! Provides consistent wrapping navigation and scroll offset management
//! across all dialog widgets that display scrollable lists.

#[cfg(test)]
#[path = "scrollable_tests.rs"]
mod tests;

/// State for scroll-aware list navigation.
///
/// Manages selection index and scroll offset with wrapping behavior
/// at list boundaries.
#[derive(Clone, Debug, Default)]
pub struct ScrollState {
    /// Currently selected item index (0-based)
    pub selected_index: usize,
    /// Scroll offset (first visible item index)
    pub scroll_offset: usize,
    /// Number of items visible at once
    pub visible_count: usize,
    /// Total number of items in the list
    pub total_items: usize,
}

impl ScrollState {
    /// Create a new scroll state with the given visible count.
    pub fn new(visible_count: usize) -> Self {
        Self {
            selected_index: 0,
            scroll_offset: 0,
            visible_count,
            total_items: 0,
        }
    }

    /// Update the total item count (call when list changes).
    pub fn set_total(&mut self, total: usize) {
        self.total_items = total;
        // Clamp selection if list shrunk
        if self.selected_index >= total && total > 0 {
            self.selected_index = total - 1;
        }
        // Adjust scroll offset if needed
        self.adjust_scroll();
    }

    /// Move selection up (wraps at top to bottom).
    pub fn select_prev(&mut self) {
        if self.total_items == 0 {
            return;
        }
        if self.selected_index == 0 {
            // Wrap to bottom
            self.selected_index = self.total_items - 1;
            // Scroll to show bottom item
            if self.total_items > self.visible_count {
                self.scroll_offset = self.total_items - self.visible_count;
            }
        } else {
            self.selected_index -= 1;
            // Scroll up if selection is above visible area
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    /// Move selection down (wraps at bottom to top).
    pub fn select_next(&mut self) {
        if self.total_items == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.total_items;
        // Handle wrap to top
        if self.selected_index == 0 {
            self.scroll_offset = 0;
        }
        // Scroll down if selection is below visible area
        else if self.selected_index >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected_index - self.visible_count + 1;
        }
    }

    /// Check if there are items above the visible area.
    pub fn has_more_above(&self) -> bool {
        self.scroll_offset > 0
    }

    /// Check if there are items below the visible area.
    pub fn has_more_below(&self) -> bool {
        self.scroll_offset + self.visible_count < self.total_items
    }

    /// Adjust scroll offset to keep selection visible.
    fn adjust_scroll(&mut self) {
        if self.total_items == 0 {
            self.scroll_offset = 0;
            return;
        }
        // Ensure selection is in visible range
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected_index - self.visible_count + 1;
        }
        // Clamp scroll offset to valid range
        let max_offset = self.total_items.saturating_sub(self.visible_count);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }
}
