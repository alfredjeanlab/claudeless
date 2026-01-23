// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// MemorySource Tests
// =============================================================================

#[test]
fn test_memory_source_all_returns_expected_sources() {
    let sources = MemorySource::all();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0], MemorySource::Project);
    assert_eq!(sources[1], MemorySource::User);
    assert_eq!(sources[2], MemorySource::Enterprise);
}

#[test]
fn test_memory_source_name() {
    assert_eq!(MemorySource::Project.name(), "Project");
    assert_eq!(MemorySource::User.name(), "User");
    assert_eq!(MemorySource::Enterprise.name(), "Enterprise");
}

#[test]
fn test_memory_source_description() {
    assert!(MemorySource::Project
        .description()
        .contains(".claude/CLAUDE.md"));
    assert!(MemorySource::User
        .description()
        .contains("~/.claude/CLAUDE.md"));
    assert!(MemorySource::Enterprise
        .description()
        .contains("Organization"));
}

// =============================================================================
// MemoryDialog Creation Tests
// =============================================================================

#[test]
fn test_memory_dialog_new_initializes_correctly() {
    let dialog = MemoryDialog::new();
    assert_eq!(dialog.selected_index, 0);
    assert_eq!(dialog.scroll_offset, 0);
    assert!(!dialog.entries.is_empty());
}

#[test]
fn test_memory_dialog_default_matches_new() {
    let default = MemoryDialog::default();
    let new = MemoryDialog::new();
    assert_eq!(default.selected_index, new.selected_index);
    assert_eq!(default.entries.len(), new.entries.len());
}

// =============================================================================
// Navigation Tests
// =============================================================================

#[test]
fn test_select_next_increments() {
    let mut dialog = MemoryDialog::new();
    assert_eq!(dialog.selected_index, 0);

    dialog.select_next();
    assert_eq!(dialog.selected_index, 1);

    dialog.select_next();
    assert_eq!(dialog.selected_index, 2);
}

#[test]
fn test_select_next_wraps_at_end() {
    let mut dialog = MemoryDialog::new();
    let total = dialog.entries.len();

    // Navigate to last entry
    for _ in 0..total - 1 {
        dialog.select_next();
    }
    assert_eq!(dialog.selected_index, total - 1);

    // Next should wrap to first
    dialog.select_next();
    assert_eq!(dialog.selected_index, 0);
    assert_eq!(dialog.scroll_offset, 0);
}

#[test]
fn test_select_prev_decrements() {
    let mut dialog = MemoryDialog::new();
    dialog.selected_index = 2;

    dialog.select_prev();
    assert_eq!(dialog.selected_index, 1);

    dialog.select_prev();
    assert_eq!(dialog.selected_index, 0);
}

#[test]
fn test_select_prev_wraps_at_beginning() {
    let mut dialog = MemoryDialog::new();
    assert_eq!(dialog.selected_index, 0);

    // Previous should wrap to last
    dialog.select_prev();
    assert_eq!(dialog.selected_index, dialog.entries.len() - 1);
}

#[test]
fn test_navigation_with_empty_entries() {
    let mut dialog = MemoryDialog::new();
    dialog.entries.clear();

    // Should not panic with empty entries
    dialog.select_next();
    assert_eq!(dialog.selected_index, 0);

    dialog.select_prev();
    assert_eq!(dialog.selected_index, 0);
}

// =============================================================================
// Selected Entry Tests
// =============================================================================

#[test]
fn test_selected_entry_returns_correct_entry() {
    let dialog = MemoryDialog::new();
    let entry = dialog.selected_entry();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().source, MemorySource::Project);
}

#[test]
fn test_selected_entry_after_navigation() {
    let mut dialog = MemoryDialog::new();

    dialog.select_next();
    let entry = dialog.selected_entry();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().source, MemorySource::User);

    dialog.select_next();
    let entry = dialog.selected_entry();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().source, MemorySource::Enterprise);
}

#[test]
fn test_selected_entry_with_empty_entries() {
    let mut dialog = MemoryDialog::new();
    dialog.entries.clear();

    assert!(dialog.selected_entry().is_none());
}

// =============================================================================
// Scroll Indicator Tests
// =============================================================================

#[test]
fn test_has_more_below_when_many_entries() {
    let mut dialog = MemoryDialog::new();
    dialog.visible_count = 2; // Only show 2 items

    // With 3 entries and 2 visible, should have more below at offset 0
    assert!(dialog.has_more_below());
}

#[test]
fn test_has_more_below_when_few_entries() {
    let dialog = MemoryDialog::new();
    // Default has 3 entries, visible_count is 5
    // Should not have more below since all fit
    assert!(!dialog.has_more_below());
}

#[test]
fn test_has_more_above_when_scrolled() {
    let mut dialog = MemoryDialog::new();
    dialog.scroll_offset = 1;

    assert!(dialog.has_more_above());
}

#[test]
fn test_has_more_above_when_at_top() {
    let dialog = MemoryDialog::new();
    assert_eq!(dialog.scroll_offset, 0);
    assert!(!dialog.has_more_above());
}

// =============================================================================
// MemoryEntry Tests
// =============================================================================

#[test]
fn test_memory_entry_active_status() {
    let dialog = MemoryDialog::new();
    let project_entry = &dialog.entries[0];
    let user_entry = &dialog.entries[1];

    assert!(project_entry.is_active);
    assert!(!user_entry.is_active);
}

#[test]
fn test_memory_entry_path() {
    let dialog = MemoryDialog::new();
    let project_entry = &dialog.entries[0];

    assert!(project_entry.path.is_some());
    assert!(project_entry.path.as_ref().unwrap().contains("CLAUDE.md"));
}
