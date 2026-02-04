// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use serde_json::json;

fn sample_input() -> serde_json::Value {
    json!({
        "questions": [
            {
                "question": "What language?",
                "header": "Language",
                "options": [
                    { "label": "Rust", "description": "Systems programming" },
                    { "label": "Python", "description": "Scripting" }
                ],
                "multiSelect": false
            }
        ]
    })
}

fn multi_select_input() -> serde_json::Value {
    json!({
        "questions": [
            {
                "question": "Which features?",
                "header": "Features",
                "options": [
                    { "label": "Auth", "description": "Authentication" },
                    { "label": "API", "description": "REST endpoints" },
                    { "label": "UI", "description": "Frontend" }
                ],
                "multiSelect": true
            }
        ]
    })
}

#[test]
fn test_from_tool_input_parses_questions() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    assert_eq!(state.questions.len(), 1);
    assert_eq!(state.questions[0].question, "What language?");
    assert_eq!(state.questions[0].header, "Language");
    assert_eq!(state.questions[0].options.len(), 2);
    assert_eq!(state.questions[0].options[0].label, "Rust");
    assert_eq!(state.questions[0].options[1].label, "Python");
    assert!(!state.questions[0].multi_select);
}

#[test]
fn test_cursor_navigation() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    assert_eq!(state.questions[0].cursor, 0);

    state.cursor_down();
    assert_eq!(state.questions[0].cursor, 1);

    // Can't go past end
    state.cursor_down();
    assert_eq!(state.questions[0].cursor, 1);

    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 0);

    // Can't go before start
    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 0);
}

#[test]
fn test_single_select() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Select first option
    state.toggle_or_select();
    assert_eq!(state.questions[0].selected, vec![0]);

    // Move down and select second - replaces first
    state.cursor_down();
    state.toggle_or_select();
    assert_eq!(state.questions[0].selected, vec![1]);
}

#[test]
fn test_multi_select_toggle() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Toggle first
    state.toggle_or_select();
    assert_eq!(state.questions[0].selected, vec![0]);

    // Toggle second
    state.cursor_down();
    state.toggle_or_select();
    assert_eq!(state.questions[0].selected, vec![0, 1]);

    // Untoggle first
    state.cursor_up();
    state.toggle_or_select();
    assert_eq!(state.questions[0].selected, vec![1]);
}

#[test]
fn test_select_by_number() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    state.select_by_number(2);
    assert_eq!(state.questions[0].cursor, 1);
    assert_eq!(state.questions[0].selected, vec![1]);

    state.select_by_number(3);
    assert_eq!(state.questions[0].cursor, 2);
    assert_eq!(state.questions[0].selected, vec![1, 2]);
}

#[test]
fn test_collect_answers_single_select() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    state.cursor_down();
    state.toggle_or_select();

    let result = state.collect_answers();
    match result {
        ElicitationResult::Answered(answers) => {
            assert_eq!(answers.get("What language?").unwrap(), "Python");
        }
        ElicitationResult::Cancelled => panic!("Expected answered"),
    }
}

#[test]
fn test_collect_answers_multi_select() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    state.select_by_number(1); // Auth
    state.select_by_number(3); // UI

    let result = state.collect_answers();
    match result {
        ElicitationResult::Answered(answers) => {
            assert_eq!(answers.get("Which features?").unwrap(), "Auth, UI");
        }
        ElicitationResult::Cancelled => panic!("Expected answered"),
    }
}

#[test]
fn test_collect_answers_default_first_when_none_selected() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    let result = state.collect_answers();
    match result {
        ElicitationResult::Answered(answers) => {
            assert_eq!(answers.get("What language?").unwrap(), "Rust");
        }
        ElicitationResult::Cancelled => panic!("Expected answered"),
    }
}

#[test]
fn test_render_single_select() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    let rendered = state.render(60);

    assert!(rendered.contains("Clarifying question"));
    assert!(rendered.contains("Language: What language?"));
    assert!(rendered.contains("Rust"));
    assert!(rendered.contains("Python"));
    assert!(rendered.contains("Enter to confirm"));
}

#[test]
fn test_render_multi_select() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    state.toggle_or_select(); // Select first

    let rendered = state.render(60);
    assert!(rendered.contains("☑")); // Selected checkbox
    assert!(rendered.contains("☐")); // Unselected checkbox
    assert!(rendered.contains("Space to toggle"));
}

#[test]
fn test_question_navigation() {
    let input = json!({
        "questions": [
            {
                "question": "Q1?",
                "header": "H1",
                "options": [{ "label": "A", "description": "" }],
                "multiSelect": false
            },
            {
                "question": "Q2?",
                "header": "H2",
                "options": [{ "label": "B", "description": "" }],
                "multiSelect": false
            }
        ]
    });
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    assert_eq!(state.current_question, 0);

    state.next_question();
    assert_eq!(state.current_question, 1);

    // Can't go past last
    state.next_question();
    assert_eq!(state.current_question, 1);

    state.prev_question();
    assert_eq!(state.current_question, 0);

    // Can't go before first
    state.prev_question();
    assert_eq!(state.current_question, 0);
}
