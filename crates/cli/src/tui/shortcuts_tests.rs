// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn test_shortcuts_by_column_left() {
    let columns = shortcuts_by_column();
    let left = &columns[0];

    assert!(
        left.contains(&"! for bash mode"),
        "Left column should have '! for bash mode'"
    );
    assert!(
        left.contains(&"/ for commands"),
        "Left column should have '/ for commands'"
    );
    assert!(
        left.contains(&"@ for file paths"),
        "Left column should have '@ for file paths'"
    );
    assert!(
        left.contains(&"& for background"),
        "Left column should have '& for background'"
    );
    assert_eq!(left.len(), 4, "Left column should have 4 shortcuts");
}

#[test]
fn test_shortcuts_by_column_center() {
    let columns = shortcuts_by_column();
    let center = &columns[1];

    assert!(
        center.contains(&"double tap esc to clear input"),
        "Center column should have 'double tap esc to clear input'"
    );
    assert!(
        center.contains(&"shift + tab to auto-accept edits"),
        "Center column should have 'shift + tab to auto-accept edits'"
    );
    assert!(
        center.contains(&"ctrl + o for verbose output"),
        "Center column should have 'ctrl + o for verbose output'"
    );
    assert!(
        center.contains(&"ctrl + t to show todos"),
        "Center column should have 'ctrl + t to show todos'"
    );
    // The backslash + return line is split across two entries
    assert!(
        center.contains(&"backslash (\\) + return (\u{23ce}) for"),
        "Center column should have backslash + return line"
    );
    assert!(
        center.contains(&"newline"),
        "Center column should have 'newline' continuation"
    );
    assert_eq!(center.len(), 6, "Center column should have 6 entries");
}

#[test]
fn test_shortcuts_by_column_right() {
    let columns = shortcuts_by_column();
    let right = &columns[2];

    assert!(
        right.contains(&"ctrl + _ to undo"),
        "Right column should have 'ctrl + _ to undo'"
    );
    assert!(
        right.contains(&"ctrl + z to suspend"),
        "Right column should have 'ctrl + z to suspend'"
    );
    assert!(
        right.contains(&"cmd + v to paste images"),
        "Right column should have 'cmd + v to paste images'"
    );
    assert!(
        right.contains(&"meta + p to switch model"),
        "Right column should have 'meta + p to switch model'"
    );
    assert!(
        right.contains(&"ctrl + s to stash prompt"),
        "Right column should have 'ctrl + s to stash prompt'"
    );
    assert_eq!(right.len(), 5, "Right column should have 5 shortcuts");
}

#[test]
fn test_total_shortcuts_count() {
    assert_eq!(SHORTCUTS.len(), 15, "Should have 15 total shortcuts");
}
