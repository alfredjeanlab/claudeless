// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Memory dialog widget.
//!
//! Shown when user executes `/memory` to view and manage CLAUDE.md instruction files.

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;

use super::scrollable::ScrollState;

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
    /// Memory entries (loaded from filesystem)
    pub entries: Vec<MemoryEntry>,
    /// Scroll-aware navigation state
    scroll: ScrollState,
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

        let mut scroll = ScrollState::new(5); // Default visible items
        scroll.set_total(entries.len());

        Self { entries, scroll }
    }

    /// Get the currently selected index
    pub fn selected_index(&self) -> usize {
        self.scroll.selected_index
    }

    /// Get the scroll offset for rendering
    pub fn scroll_offset(&self) -> usize {
        self.scroll.scroll_offset
    }

    /// Get the visible item count
    pub fn visible_count(&self) -> usize {
        self.scroll.visible_count
    }

    /// Set the visible item count (call when terminal resizes)
    pub fn set_visible_count(&mut self, count: usize) {
        self.scroll.visible_count = count;
    }

    /// Update entries list
    pub fn set_entries(&mut self, entries: Vec<MemoryEntry>) {
        self.entries = entries;
        self.scroll.set_total(self.entries.len());
    }

    /// Move selection up (wraps at boundaries)
    pub fn select_prev(&mut self) {
        self.scroll.select_prev();
    }

    /// Move selection down (wraps at boundaries)
    pub fn select_next(&mut self) {
        self.scroll.select_next();
    }

    /// Get currently selected entry
    pub fn selected_entry(&self) -> Option<&MemoryEntry> {
        self.entries.get(self.scroll.selected_index)
    }

    /// Check if we should show scroll indicator above
    pub fn has_more_above(&self) -> bool {
        self.scroll.has_more_above()
    }

    /// Check if we should show scroll indicator below
    pub fn has_more_below(&self) -> bool {
        self.scroll.has_more_below()
    }
}
