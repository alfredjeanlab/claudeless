// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;

#[test]
fn new_scroll_state_is_zeroed() {
    let state = ScrollState::new(5);
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.scroll_offset, 0);
    assert_eq!(state.visible_count, 5);
    assert_eq!(state.total_items, 0);
}

#[test]
fn set_total_updates_total_items() {
    let mut state = ScrollState::new(5);
    state.set_total(10);
    assert_eq!(state.total_items, 10);
}

#[test]
fn set_total_clamps_selection_when_list_shrinks() {
    let mut state = ScrollState::new(5);
    state.set_total(10);
    state.selected_index = 8;
    state.set_total(5);
    assert_eq!(state.selected_index, 4);
}

#[test]
fn select_next_wraps_to_top() {
    let mut state = ScrollState::new(5);
    state.set_total(3);
    state.selected_index = 2;
    state.select_next();
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.scroll_offset, 0);
}

#[test]
fn select_prev_wraps_to_bottom() {
    let mut state = ScrollState::new(5);
    state.set_total(3);
    state.selected_index = 0;
    state.select_prev();
    assert_eq!(state.selected_index, 2);
}

#[test]
fn select_next_scrolls_down_when_needed() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.selected_index = 2;
    state.scroll_offset = 0;
    state.select_next();
    assert_eq!(state.selected_index, 3);
    assert_eq!(state.scroll_offset, 1);
}

#[test]
fn select_prev_scrolls_up_when_needed() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.selected_index = 3;
    state.scroll_offset = 3;
    state.select_prev();
    assert_eq!(state.selected_index, 2);
    assert_eq!(state.scroll_offset, 2);
}

#[test]
fn has_more_above_when_scrolled() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.scroll_offset = 2;
    assert!(state.has_more_above());
}

#[test]
fn has_more_above_false_at_top() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.scroll_offset = 0;
    assert!(!state.has_more_above());
}

#[test]
fn has_more_below_when_not_at_end() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.scroll_offset = 0;
    assert!(state.has_more_below());
}

#[test]
fn has_more_below_false_at_bottom() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.scroll_offset = 7;
    assert!(!state.has_more_below());
}

#[test]
fn empty_list_navigation_is_noop() {
    let mut state = ScrollState::new(5);
    state.set_total(0);
    state.select_next();
    assert_eq!(state.selected_index, 0);
    state.select_prev();
    assert_eq!(state.selected_index, 0);
}

#[test]
fn select_prev_scrolls_to_bottom_on_wrap() {
    let mut state = ScrollState::new(3);
    state.set_total(10);
    state.selected_index = 0;
    state.scroll_offset = 0;
    state.select_prev();
    assert_eq!(state.selected_index, 9);
    assert_eq!(state.scroll_offset, 7); // 10 - 3 = 7
}
