// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
        center.contains(&"shift + tab to auto-accept"),
        "Center column should have 'shift + tab to auto-accept'"
    );
    assert!(
        center.contains(&"edits"),
        "Center column should have 'edits' continuation"
    );
    assert!(
        center.contains(&"ctrl + o for verbose output"),
        "Center column should have 'ctrl + o for verbose output'"
    );
    assert!(
        center.contains(&"ctrl + t to show todos"),
        "Center column should have 'ctrl + t to show todos'"
    );
    assert!(
        center.contains(&"shift + \u{23ce} for newline"),
        "Center column should have 'shift + ⏎ for newline'"
    );
    assert_eq!(center.len(), 6, "Center column should have 6 entries");
}

#[test]
fn test_shortcuts_by_column_right() {
    let columns = shortcuts_by_column();
    let right = &columns[2];

    assert!(
        right.contains(&"ctrl + shift + - to"),
        "Right column should have 'ctrl + shift + - to'"
    );
    assert!(
        right.contains(&"undo"),
        "Right column should have 'undo' continuation"
    );
    assert!(
        right.contains(&"ctrl + z to suspend"),
        "Right column should have 'ctrl + z to suspend'"
    );
    assert!(
        right.contains(&"ctrl + v to paste"),
        "Right column should have 'ctrl + v to paste'"
    );
    assert!(
        right.contains(&"images"),
        "Right column should have 'images' continuation"
    );
    assert!(
        right.contains(&"meta + p to switch"),
        "Right column should have 'meta + p to switch'"
    );
    assert!(
        right.contains(&"model"),
        "Right column should have 'model' continuation"
    );
    assert!(
        right.contains(&"ctrl + s to stash"),
        "Right column should have 'ctrl + s to stash'"
    );
    assert!(
        right.contains(&"prompt"),
        "Right column should have 'prompt' continuation"
    );
    assert!(
        right.contains(&"ctrl + g to edit in"),
        "Right column should have 'ctrl + g to edit in'"
    );
    assert!(
        right.contains(&"$EDITOR"),
        "Right column should have '$EDITOR' continuation"
    );
    assert!(
        right.contains(&"/keybindings to"),
        "Right column should have '/keybindings to'"
    );
    assert!(
        right.contains(&"customize"),
        "Right column should have 'customize' continuation"
    );
    assert_eq!(right.len(), 13, "Right column should have 13 entries");
}

#[test]
fn test_total_shortcuts_count() {
    assert_eq!(SHORTCUTS.len(), 23, "Should have 23 total shortcuts");
}
