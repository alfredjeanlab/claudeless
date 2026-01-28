// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]

use super::*;

#[test]
fn make_separator_generates_correct_width() {
    assert_eq!(make_separator(10).chars().count(), 10);
    assert_eq!(make_separator(80).chars().count(), 80);
    assert_eq!(make_separator(120).chars().count(), 120);
    assert_eq!(make_separator(200).chars().count(), 200);
}

#[test]
fn make_separator_uses_correct_char() {
    let sep = make_separator(5);
    assert!(sep.chars().all(|c| c == SEPARATOR_CHAR));
}

#[test]
fn make_compact_separator_centers_text() {
    let sep = make_compact_separator("Test", 20);
    assert_eq!(sep.chars().count(), 20);
    assert!(sep.contains(" Test "));
}

#[test]
fn make_compact_separator_handles_odd_widths() {
    // Width 21 with " Test " (6 chars) = 15 remaining
    // Left: 7, Right: 8
    let sep = make_compact_separator("Test", 21);
    assert_eq!(sep.chars().count(), 21);
}

#[test]
fn make_compact_separator_handles_narrow_width() {
    // When width is smaller than text, just return the text
    let sep = make_compact_separator("Very Long Text Here", 10);
    assert!(sep.contains("Very Long Text Here"));
}

#[test]
fn make_section_divider_generates_correct_width() {
    assert_eq!(make_section_divider(50).chars().count(), 50);
    assert!(make_section_divider(50)
        .chars()
        .all(|c| c == SECTION_DIVIDER_CHAR));
}
