// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::ansi::strip_ansi;
use crate::permission::PermissionMode;
use crate::tui::app::state::{DialogState, DisplayState, InputState};
use crate::tui::app::types::{AppMode, RenderState, StatusInfo};

fn make_render_state(model: &str, claude_version: Option<&str>, is_tty: bool) -> RenderState {
    RenderState {
        mode: AppMode::Input,
        input: InputState::default(),
        dialog: DialogState::None,
        display: DisplayState::new(),
        status: StatusInfo {
            model: model.to_string(),
            ..Default::default()
        },
        permission_mode: PermissionMode::Default,
        thinking_enabled: true,
        user_name: "TestUser".to_string(),
        claude_version: claude_version.map(|s| s.to_string()),
        is_tty,
        is_compacting: false,
        spinner_frame: 0,
        spinner_verb: String::new(),
        placeholder: Some("Try \"fix lint errors\"".to_string()),
        provider: Some("Claude API".to_string()),
        show_welcome_back: true,
        welcome_back_right_panel: None,
    }
}

#[test]
fn welcome_box_correct_number_of_lines() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    // 1 top border + 9 content rows + 1 bottom border = 11
    assert_eq!(lines.len(), 11);
}

#[test]
fn welcome_box_width_matches_terminal() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    for (i, line) in lines.iter().enumerate() {
        let char_count = line.chars().count();
        assert_eq!(
            char_count, 80,
            "Line {} has {} chars, expected 80: {:?}",
            i, char_count, line
        );
    }
}

#[test]
fn welcome_box_top_border() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    assert!(lines[0].starts_with("╭─── Claude Code v2.1.29 "));
    assert!(lines[0].ends_with("╮"));
}

#[test]
fn welcome_box_bottom_border() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    let last = lines.last().unwrap();
    assert!(last.starts_with("╰"));
    assert!(last.ends_with("╯"));
}

#[test]
fn welcome_box_welcome_back_centered() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    // Row 1 (index 2 in lines: top border + row 0 + row 1)
    let row1 = &lines[2]; // index 0=top, 1=row0, 2=row1
    assert!(
        row1.contains("Welcome back!"),
        "Row 1 should contain 'Welcome back!'"
    );
    // Check centering: left panel is 52 chars, "Welcome back!" is 13.
    // Left padding = 20, right padding = 19
    assert!(row1.contains("                    Welcome back!                   "));
}

#[test]
fn welcome_box_logo_characters() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    // Row 3 (index 4): logo line 1
    assert!(
        lines[4].contains("▗ ▗   ▖ ▖"),
        "Row 3 should contain logo top"
    );
    // Row 5 (index 6): logo line 3
    assert!(
        lines[6].contains("▘▘ ▝▝"),
        "Row 5 should contain logo bottom"
    );
}

#[test]
fn welcome_box_model_provider_centered() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    // Row 7 (index 8): model/provider
    assert!(
        lines[8].contains("Haiku 4.5 · Claude API"),
        "Row 7 should contain model/provider"
    );
}

#[test]
fn welcome_box_right_panel() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let lines = format_welcome_back_box(&state, 80);
    // Check right panel content
    assert!(lines[1].contains("Tips for getting"), "Row 0 right: Tips");
    assert!(lines[2].contains("started"), "Row 1 right: started");
    assert!(
        lines[3].contains("Ask Claude to create a"),
        "Row 2 right: tip text"
    );
    assert!(lines[5].contains("Recent activity"), "Row 4 right: Recent");
    assert!(
        lines[6].contains("No recent activity"),
        "Row 5 right: No recent"
    );
}

#[test]
fn truncate_path_short_path_unchanged() {
    assert_eq!(
        truncate_path("/home/user/project", 50),
        "/home/user/project"
    );
}

#[test]
fn truncate_path_long_path_truncated() {
    let long_path = "/var/folders/t5/6tq8cxtj20z035rv8hsnzwvh0000gn/T/capture-iIucdS";
    let truncated = truncate_path(long_path, 50);
    assert!(truncated.starts_with("/\u{2026}/"));
    assert!(truncated.chars().count() <= 50);
}

#[test]
fn status_bar_hidden_during_responding() {
    let mut state = make_render_state("haiku", Some("2.1.29"), false);
    state.mode = AppMode::Responding;
    let bar = format_status_bar(&state, 80);
    assert!(
        !bar.contains("? for shortcuts"),
        "Status bar should not show '? for shortcuts' during Responding mode"
    );
}

#[test]
fn status_bar_hidden_during_thinking() {
    let mut state = make_render_state("haiku", Some("2.1.29"), true);
    state.mode = AppMode::Thinking;
    let bar = format_status_bar_styled(&state, 80);
    assert!(
        !bar.contains("? for shortcuts"),
        "Status bar should not show '? for shortcuts' during Thinking mode"
    );
}

#[test]
fn status_bar_shown_during_input() {
    let state = make_render_state("haiku", Some("2.1.29"), false);
    let bar = format_status_bar(&state, 80);
    assert!(
        bar.contains("? for shortcuts"),
        "Status bar should show '? for shortcuts' during Input mode with empty buffer"
    );
}

#[test]
fn status_bar_hidden_when_input_has_text() {
    let mut state = make_render_state("haiku", Some("2.1.29"), false);
    state.input.buffer = "hello".to_string();
    let bar = format_status_bar(&state, 80);
    assert!(
        !bar.contains("? for shortcuts"),
        "Status bar should hide '? for shortcuts' when user has typed input"
    );
}

#[test]
fn status_bar_styled_hidden_when_input_has_text() {
    let mut state = make_render_state("haiku", Some("2.1.29"), true);
    state.input.buffer = "hello".to_string();
    let bar = format_status_bar_styled(&state, 80);
    assert!(
        !bar.contains("? for shortcuts"),
        "Styled status bar should hide '? for shortcuts' when user has typed input"
    );
}

#[test]
fn status_bar_thinking_off_fits_in_terminal_width() {
    let mut state = make_render_state("haiku", Some("2.1.29"), false);
    state.thinking_enabled = false;
    let bar = format_status_bar(&state, 80);
    assert!(
        bar.contains("Thinking off"),
        "Status bar should show 'Thinking off' when thinking is disabled"
    );
    assert!(
        bar.chars().count() <= 78,
        "Status bar should be at most 78 chars in 80-col terminal, got {}",
        bar.chars().count()
    );
}

#[test]
fn status_bar_styled_thinking_off_fits_in_terminal_width() {
    let mut state = make_render_state("haiku", Some("2.1.29"), true);
    state.thinking_enabled = false;
    let bar = format_status_bar_styled(&state, 80);
    assert!(
        bar.contains("Thinking off"),
        "Styled status bar should show 'Thinking off' when thinking is disabled"
    );
    // Strip ANSI escape sequences for visual width check
    let stripped = strip_ansi(&bar);
    let visual_width = stripped.chars().count();
    assert!(
        visual_width <= 78,
        "Styled status bar visual width should be at most 78 chars in 80-col terminal, got {}",
        visual_width
    );
}

#[test]
fn welcome_box_claudeless_native() {
    let state = make_render_state("haiku", None, false);
    let lines = format_welcome_back_box(&state, 80);
    assert!(lines[0].contains("Claudeless"));
}
