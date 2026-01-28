// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =============================================================================
// Fuzzy Matching Tests
// =============================================================================

#[test]
fn test_fuzzy_matches_prefix() {
    assert!(fuzzy_matches("co", "compact"));
    assert!(fuzzy_matches("co", "config"));
    assert!(fuzzy_matches("co", "context"));
    assert!(fuzzy_matches("hel", "help"));
    assert!(fuzzy_matches("add", "add-dir"));
}

#[test]
fn test_fuzzy_matches_subsequence() {
    assert!(fuzzy_matches("hk", "hooks")); // h_oo_k_s
    assert!(fuzzy_matches("ad", "add-dir")); // _a_d_d-dir
    assert!(fuzzy_matches("adr", "add-dir")); // _a_d_-d_i_r
}

#[test]
fn test_fuzzy_matches_case_insensitive() {
    assert!(fuzzy_matches("CO", "compact"));
    assert!(fuzzy_matches("Co", "config"));
    assert!(fuzzy_matches("HELP", "help"));
}

#[test]
fn test_fuzzy_matches_empty_query() {
    assert!(fuzzy_matches("", "anything"));
    assert!(fuzzy_matches("", ""));
}

#[test]
fn test_fuzzy_no_match() {
    assert!(!fuzzy_matches("xyz", "compact"));
    assert!(!fuzzy_matches("zz", "help"));
    assert!(!fuzzy_matches("abc", "clear"));
}

#[test]
fn test_fuzzy_query_longer_than_text() {
    assert!(!fuzzy_matches("helpme", "help"));
}

// =============================================================================
// Filter Commands Tests
// =============================================================================

#[test]
fn test_filter_commands_empty() {
    let results = filter_commands("");
    assert_eq!(results.len(), COMMANDS.len());
    // First command should be add-dir (alphabetical)
    assert_eq!(results[0].name, "add-dir");
}

#[test]
fn test_filter_commands_co() {
    let results = filter_commands("co");
    let names: Vec<_> = results.iter().map(|c| c.name).collect();
    assert!(names.contains(&"compact"));
    assert!(names.contains(&"config"));
    assert!(names.contains(&"context"));
    assert!(names.contains(&"cost"));
    // Should not include commands without 'co' in sequence
    assert!(!names.contains(&"clear"));
    assert!(!names.contains(&"add-dir"));
}

#[test]
fn test_filter_commands_h() {
    let results = filter_commands("h");
    let names: Vec<_> = results.iter().map(|c| c.name).collect();
    assert!(names.contains(&"help"));
    assert!(names.contains(&"hooks"));
}

#[test]
fn test_filter_commands_no_match() {
    let results = filter_commands("xyz");
    assert!(results.is_empty());
}

#[test]
fn test_commands_alphabetical_order() {
    let names: Vec<_> = COMMANDS.iter().map(|c| c.name).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "COMMANDS should be in alphabetical order");
}

// =============================================================================
// SlashMenuState Tests
// =============================================================================

#[test]
fn test_slash_menu_state_new() {
    let state = SlashMenuState::new();
    assert!(state.filter.is_empty());
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.filtered_commands.len(), COMMANDS.len());
}

#[test]
fn test_slash_menu_set_filter() {
    let mut state = SlashMenuState::new();
    state.set_filter("co".to_string());
    assert_eq!(state.filter, "co");
    // Should have filtered to co-prefixed commands
    let names: Vec<_> = state.filtered_commands.iter().map(|c| c.name).collect();
    assert!(names.contains(&"compact"));
    assert!(names.contains(&"config"));
}

#[test]
fn test_slash_menu_set_filter_resets_selection_if_out_of_bounds() {
    let mut state = SlashMenuState::new();
    state.selected_index = 10; // Beyond what /h would return
    state.set_filter("hel".to_string()); // Only "help" matches
    assert_eq!(state.selected_index, 0);
}

#[test]
fn test_slash_menu_select_next() {
    let mut state = SlashMenuState::new();
    assert_eq!(state.selected_index, 0);
    state.select_next();
    assert_eq!(state.selected_index, 1);
    state.select_next();
    assert_eq!(state.selected_index, 2);
}

#[test]
fn test_slash_menu_select_next_wraps() {
    let mut state = SlashMenuState::new();
    state.set_filter("hel".to_string()); // Only "help" matches
    assert_eq!(state.filtered_commands.len(), 1);
    assert_eq!(state.selected_index, 0);
    state.select_next();
    assert_eq!(state.selected_index, 0); // Wraps back to 0
}

#[test]
fn test_slash_menu_select_prev() {
    let mut state = SlashMenuState::new();
    state.selected_index = 2;
    state.select_prev();
    assert_eq!(state.selected_index, 1);
    state.select_prev();
    assert_eq!(state.selected_index, 0);
}

#[test]
fn test_slash_menu_select_prev_wraps() {
    let mut state = SlashMenuState::new();
    assert_eq!(state.selected_index, 0);
    state.select_prev();
    assert_eq!(state.selected_index, COMMANDS.len() - 1);
}

#[test]
fn test_slash_menu_selected_command() {
    let state = SlashMenuState::new();
    let cmd = state.selected_command().unwrap();
    assert_eq!(cmd.name, "add-dir"); // First alphabetically
}

#[test]
fn test_slash_menu_selected_command_after_navigation() {
    let mut state = SlashMenuState::new();
    state.select_next();
    let cmd = state.selected_command().unwrap();
    assert_eq!(cmd.name, "agents"); // Second alphabetically
}

#[test]
fn test_slash_menu_selected_command_empty_filter() {
    let mut state = SlashMenuState::new();
    state.set_filter("xyz".to_string()); // No matches
    assert!(state.selected_command().is_none());
}

// =============================================================================
// SlashCommand Tests
// =============================================================================

#[test]
fn test_slash_command_full_name() {
    let cmd = &COMMANDS[0]; // add-dir
    assert_eq!(cmd.full_name(), "/add-dir");
}

#[test]
fn test_slash_command_with_argument() {
    let add_dir = COMMANDS.iter().find(|c| c.name == "add-dir").unwrap();
    assert_eq!(add_dir.argument_hint, Some("<path>"));

    let model = COMMANDS.iter().find(|c| c.name == "model").unwrap();
    assert_eq!(model.argument_hint, Some("<model>"));
}

#[test]
fn test_slash_command_without_argument() {
    let clear = COMMANDS.iter().find(|c| c.name == "clear").unwrap();
    assert_eq!(clear.argument_hint, None);

    let help = COMMANDS.iter().find(|c| c.name == "help").unwrap();
    assert_eq!(help.argument_hint, None);
}
