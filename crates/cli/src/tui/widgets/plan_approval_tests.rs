// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use serde_json::json;

fn sample_input() -> serde_json::Value {
    json!({
        "plan": "# Implementation Plan\n\n## Steps\n\n1. Add feature X\n2. Update tests"
    })
}

#[test]
fn test_from_tool_input_parses_plan_field() {
    let input = sample_input();
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/happy-yellow-dragon.md".to_string(),
    );

    assert!(state.plan_content.contains("# Implementation Plan"));
    assert!(state.plan_content.contains("Add feature X"));
    assert_eq!(state.cursor, 0);
    assert!(state.free_text.is_empty());
}

#[test]
fn test_from_tool_input_parses_plan_content_field() {
    let input = json!({
        "plan_content": "# Plan\n\nDo something."
    });
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    assert!(state.plan_content.contains("Do something."));
}

#[test]
fn test_from_tool_input_default_content() {
    let input = json!({});
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    assert!(state.plan_content.contains("No content provided."));
}

#[test]
fn test_cursor_navigation() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    // 4 options: indices 0..3
    assert_eq!(state.cursor, 0);

    state.cursor_down();
    assert_eq!(state.cursor, 1);

    state.cursor_down();
    assert_eq!(state.cursor, 2);

    state.cursor_down();
    assert_eq!(state.cursor, 3); // free-text

    // Clamps at end
    state.cursor_down();
    assert_eq!(state.cursor, 3);

    state.cursor_up();
    assert_eq!(state.cursor, 2);

    state.cursor_up();
    assert_eq!(state.cursor, 1);

    state.cursor_up();
    assert_eq!(state.cursor, 0);

    // Wraps from 0 to free-text (3)
    state.cursor_up();
    assert_eq!(state.cursor, 3);
    assert!(state.is_on_free_text());
}

#[test]
fn test_select_by_number() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.select_by_number(2);
    assert_eq!(state.cursor, 1);

    state.select_by_number(3);
    assert_eq!(state.cursor, 2);

    state.select_by_number(4);
    assert_eq!(state.cursor, 3);
    assert!(state.is_on_free_text());
}

#[test]
fn test_out_of_range_number_ignored() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.select_by_number(5);
    assert_eq!(state.cursor, 0); // Unchanged
}

#[test]
fn test_collect_result_clear_context() {
    let input = sample_input();
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    match state.collect_result() {
        PlanApprovalResult::Approved(mode) => {
            assert_eq!(mode, ApprovalMode::ClearContext);
        }
        other => panic!("Expected Approved(ClearContext), got {:?}", other),
    }
}

#[test]
fn test_collect_result_auto_accept() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.cursor_down();
    match state.collect_result() {
        PlanApprovalResult::Approved(mode) => {
            assert_eq!(mode, ApprovalMode::AutoAccept);
        }
        other => panic!("Expected Approved(AutoAccept), got {:?}", other),
    }
}

#[test]
fn test_collect_result_manual_approve() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.select_by_number(3);
    match state.collect_result() {
        PlanApprovalResult::Approved(mode) => {
            assert_eq!(mode, ApprovalMode::ManualApprove);
        }
        other => panic!("Expected Approved(ManualApprove), got {:?}", other),
    }
}

#[test]
fn test_collect_result_revised() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    // Navigate to free-text and type feedback
    state.select_by_number(4);
    state.insert_char('A');
    state.insert_char('d');
    state.insert_char('d');
    state.insert_char(' ');
    state.insert_char('X');

    match state.collect_result() {
        PlanApprovalResult::Revised(text) => {
            assert_eq!(text, "Add X");
        }
        other => panic!("Expected Revised, got {:?}", other),
    }
}

#[test]
fn test_collect_result_empty_free_text_cancels() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.select_by_number(4);
    assert!(state.free_text.is_empty());

    match state.collect_result() {
        PlanApprovalResult::Cancelled => {}
        other => panic!("Expected Cancelled, got {:?}", other),
    }
}

#[test]
fn test_free_text_typing() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    // Navigate to free-text
    state.select_by_number(4);
    assert!(state.is_on_free_text());

    state.insert_char('H');
    state.insert_char('i');
    assert_eq!(state.free_text, "Hi");

    state.backspace_char();
    assert_eq!(state.free_text, "H");

    state.backspace_char();
    assert_eq!(state.free_text, "");
}

#[test]
fn test_insert_char_ignored_when_not_on_free_text() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    // Cursor on first option (0), not free-text
    state.insert_char('X');
    assert!(state.free_text.is_empty());
}

// =========================================================================
// Render tests
// =========================================================================

#[test]
fn test_render_shows_header() {
    let input = sample_input();
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    let rendered = state.render(60);
    assert!(rendered.contains("Ready to code?"));
    assert!(rendered.contains("Here is Claude's plan:"));
}

#[test]
fn test_render_shows_plan_content() {
    let input = sample_input();
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    let rendered = state.render(60);
    assert!(rendered.contains("# Implementation Plan"));
    assert!(rendered.contains("Add feature X"));
    // Plan bordered with ╌
    assert!(rendered.contains("╌"));
}

#[test]
fn test_render_shows_options() {
    let input = sample_input();
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    let rendered = state.render(60);
    assert!(rendered.contains("❯ 1. Yes, clear context and auto-accept edits (shift+tab)"));
    assert!(rendered.contains("  2. Yes, auto-accept edits"));
    assert!(rendered.contains("  3. Yes, manually approve edits"));
    assert!(rendered.contains("  4. Type here to tell Claude what to change"));
}

#[test]
fn test_render_shows_footer() {
    let input = sample_input();
    let state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    let rendered = state.render(60);
    assert!(rendered.contains("ctrl-g to edit in VS Code"));
    assert!(rendered.contains("~/.claude/plans/test.md"));
    assert!(rendered.contains("Enter to select"));
    assert!(rendered.contains("Esc to cancel"));
}

#[test]
fn test_render_cursor_moves() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.cursor_down();
    let rendered = state.render(60);
    assert!(rendered.contains("  1. Yes, clear context"));
    assert!(rendered.contains("❯ 2. Yes, auto-accept edits"));
}

#[test]
fn test_render_free_text_shows_typed_text() {
    let input = sample_input();
    let mut state = PlanApprovalState::from_tool_input(
        &input,
        "toolu_001".to_string(),
        "~/.claude/plans/test.md".to_string(),
    );

    state.select_by_number(4);
    for c in "Add tests".chars() {
        state.insert_char(c);
    }

    let rendered = state.render(60);
    assert!(rendered.contains("Add tests"));
    assert!(!rendered.contains("Type here to tell Claude what to change"));
}
