// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Memory dialog widget.
//!
//! Shown when user executes `/memory` to view and manage CLAUDE.md instruction files.

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;

/// Memory source types displayed in the dialog
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemorySource {
    /// Project-level CLAUDE.md (.claude/CLAUDE.md or CLAUDE.md)
    Project,
    /// User-level CLAUDE.md (~/.claude/CLAUDE.md)
    User,
    /// Enterprise/Organization level
    Enterprise,
}

impl MemorySource {
    /// All memory sources in display order
    pub fn all() -> &'static [MemorySource] {
        &[
            MemorySource::Project,
            MemorySource::User,
            MemorySource::Enterprise,
        ]
    }

    /// Display name for the source type
    pub fn name(self) -> &'static str {
        match self {
            MemorySource::Project => "Project",
            MemorySource::User => "User",
            MemorySource::Enterprise => "Enterprise",
        }
    }

    /// Description for the source type
    pub fn description(self) -> &'static str {
        match self {
            MemorySource::Project => "Project-specific instructions (.claude/CLAUDE.md)",
            MemorySource::User => "User-level instructions (~/.claude/CLAUDE.md)",
            MemorySource::Enterprise => "Organization-level instructions",
        }
    }
}

/// A loaded memory entry
#[derive(Clone, Debug)]
pub struct MemoryEntry {
    /// Source type
    pub source: MemorySource,
    /// File path (if available)
    pub path: Option<String>,
    /// Whether this entry exists/is active
    pub is_active: bool,
    /// Preview of content (first N chars)
    pub preview: Option<String>,
}

/// State for the /memory dialog
#[derive(Clone, Debug)]
pub struct MemoryDialog {
    /// Currently selected entry index (0-based)
    pub selected_index: usize,
    /// Memory entries (loaded from filesystem)
    pub entries: Vec<MemoryEntry>,
    /// Scroll offset for the list
    pub scroll_offset: usize,
    /// Visible item count (based on terminal height)
    pub visible_count: usize,
}

impl Default for MemoryDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryDialog {
    pub fn new() -> Self {
        // Create placeholder entries for all memory sources
        // In production, this would scan for actual CLAUDE.md files
        let entries = vec![
            MemoryEntry {
                source: MemorySource::Project,
                path: Some(".claude/CLAUDE.md".to_string()),
                is_active: true,
                preview: Some("Project-specific instructions...".to_string()),
            },
            MemoryEntry {
                source: MemorySource::User,
                path: Some("~/.claude/CLAUDE.md".to_string()),
                is_active: false,
                preview: None,
            },
            MemoryEntry {
                source: MemorySource::Enterprise,
                path: None,
                is_active: false,
                preview: None,
            },
        ];

        Self {
            selected_index: 0,
            entries,
            scroll_offset: 0,
            visible_count: 5,
        }
    }

    /// Move selection up (wraps at boundaries)
    pub fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.entries.len() - 1;
            // Scroll to bottom
            if self.entries.len() > self.visible_count {
                self.scroll_offset = self.entries.len() - self.visible_count;
            }
        } else {
            self.selected_index -= 1;
            // Scroll up if needed
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    /// Move selection down (wraps at boundaries)
    pub fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.entries.len();
        // Handle wrap to top
        if self.selected_index == 0 {
            self.scroll_offset = 0;
        }
        // Scroll down if needed
        else if self.selected_index >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected_index - self.visible_count + 1;
        }
    }

    /// Get currently selected entry
    pub fn selected_entry(&self) -> Option<&MemoryEntry> {
        self.entries.get(self.selected_index)
    }

    /// Check if we should show scroll indicator above
    pub fn has_more_above(&self) -> bool {
        self.scroll_offset > 0
    }

    /// Check if we should show scroll indicator below
    pub fn has_more_below(&self) -> bool {
        self.scroll_offset + self.visible_count < self.entries.len()
    }
}
