// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn setup_state_defaults() {
    let state = SetupState::new("2.1.29".to_string());
    assert_eq!(state.step, SetupStep::ThemeSelection);
    assert_eq!(state.selected_theme, 0);
    assert_eq!(state.selected_login, 0);
    assert!(state.syntax_highlighting);
    assert_eq!(state.claude_version, "2.1.29");
}

#[test]
fn theme_navigation_wraps() {
    let mut state = SetupState::new("2.1.29".to_string());
    assert_eq!(state.selected_theme, 0);
    state.theme_up();
    assert_eq!(state.selected_theme, 5);
    state.theme_down();
    assert_eq!(state.selected_theme, 0);
    state.theme_down();
    assert_eq!(state.selected_theme, 1);
}

#[test]
fn login_navigation_wraps() {
    let mut state = SetupState::new("2.1.29".to_string());
    assert_eq!(state.selected_login, 0);
    state.login_up();
    assert_eq!(state.selected_login, 2);
    state.login_down();
    assert_eq!(state.selected_login, 0);
}

#[test]
fn ctrl_t_toggle() {
    let mut state = SetupState::new("2.1.29".to_string());
    assert!(state.syntax_highlighting);
    state.syntax_highlighting = false;
    assert!(!state.syntax_highlighting);
    state.syntax_highlighting = true;
    assert!(state.syntax_highlighting);
}

#[test]
fn step_transition() {
    let mut state = SetupState::new("2.1.29".to_string());
    assert_eq!(state.step, SetupStep::ThemeSelection);
    state.advance_to_login();
    assert_eq!(state.step, SetupStep::LoginMethod);
}

#[test]
fn theme_choice_from_index() {
    assert_eq!(ThemeChoice::from_index(0), ThemeChoice::Dark);
    assert_eq!(ThemeChoice::from_index(1), ThemeChoice::Light);
    assert_eq!(ThemeChoice::from_index(5), ThemeChoice::LightAnsi);
    assert_eq!(ThemeChoice::from_index(99), ThemeChoice::Dark);
}

#[test]
fn theme_syntax_names() {
    assert_eq!(ThemeChoice::Dark.syntax_theme_name(), "Monokai Extended");
    assert_eq!(ThemeChoice::Light.syntax_theme_name(), "Monokai Extended");
    assert_eq!(ThemeChoice::DarkAnsi.syntax_theme_name(), "ansi");
    assert_eq!(ThemeChoice::LightAnsi.syntax_theme_name(), "ansi");
}
