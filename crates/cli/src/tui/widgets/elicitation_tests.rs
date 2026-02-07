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

fn multi_question_input() -> serde_json::Value {
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
            },
            {
                "question": "What type of project?",
                "header": "Project",
                "options": [
                    { "label": "CLI", "description": "Command-line tool" },
                    { "label": "Web", "description": "Web application" }
                ],
                "multiSelect": false
            },
            {
                "question": "Which CI/CD?",
                "header": "CI/CD",
                "options": [
                    { "label": "GitHub Actions", "description": "GitHub native" },
                    { "label": "None", "description": "No CI/CD" }
                ],
                "multiSelect": false
            },
            {
                "question": "Which license?",
                "header": "License",
                "options": [
                    { "label": "MIT", "description": "Permissive" },
                    { "label": "Apache", "description": "Patent grant" }
                ],
                "multiSelect": false
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

    // 2 defined options: Rust(0), Python(1), Type something.(2), Chat about this(3)
    assert_eq!(state.questions[0].cursor, 0);

    state.cursor_down();
    assert_eq!(state.questions[0].cursor, 1);

    // Can go past defined options into "Type something." (index 2) and "Chat about this" (index 3)
    state.cursor_down();
    assert_eq!(state.questions[0].cursor, 2); // Type something.
    state.cursor_down();
    assert_eq!(state.questions[0].cursor, 3); // Chat about this

    // Down clamps at end
    state.cursor_down();
    assert_eq!(state.questions[0].cursor, 3);

    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 2);

    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 1);

    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 0);

    // Up from 0 wraps to "Type something." (skips "Chat about this")
    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 2);
}

#[test]
fn test_cursor_up_wraps_to_type_something() {
    // Real Claude Code: Up from position 0 wraps to "Type something."
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    assert_eq!(state.questions[0].cursor, 0);

    // Up from 0 → wraps to "Type something." (index 2, not "Chat about this" at 3)
    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 2);
    assert!(state.is_on_free_text());

    // Up again → Blue (index 1)
    state.cursor_up();
    assert_eq!(state.questions[0].cursor, 1);
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
        other => panic!("Expected answered, got {:?}", other),
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
        other => panic!("Expected answered, got {:?}", other),
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
        other => panic!("Expected answered, got {:?}", other),
    }
}

#[test]
fn test_render_single_select() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    let rendered = state.render(60);

    // Header matches real Claude Code: "☐ Header"
    assert!(rendered.contains("☐ Language"));
    assert!(rendered.contains("What language?"));
    assert!(rendered.contains("❯ 1. Rust"));
    assert!(rendered.contains("  2. Python"));
    assert!(rendered.contains("Systems programming"));
    // "Type something." and "Chat about this" appended
    assert!(rendered.contains("3. Type something."));
    assert!(rendered.contains("4. Chat about this"));
    assert!(rendered.contains("Enter to select"));
    assert!(rendered.contains("Esc to cancel"));
    // Single question: no tab bar
    assert!(!rendered.contains("←"));
    assert!(!rendered.contains("Submit"));
}

#[test]
fn test_render_single_select_footer() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    let rendered = state.render(60);

    // Updated footer format (no Question N/M)
    assert!(rendered.contains("Enter to select · Tab/Arrow keys to navigate · Esc to cancel"));
    assert!(!rendered.contains("Question 1/1"));
}

#[test]
fn test_render_multi_select() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    state.toggle_or_select(); // Select first

    let rendered = state.render(60);
    assert!(rendered.contains("☒ Features")); // Partially checked header
    assert!(rendered.contains("Space to toggle"));
}

#[test]
fn test_render_multi_select_all_checked() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    state.select_by_number(1);
    state.select_by_number(2);
    state.select_by_number(3);

    let rendered = state.render(60);
    assert!(rendered.contains("☑ Features")); // Fully checked header
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
    assert!(!state.on_submit_tab);

    state.next_question();
    assert_eq!(state.current_question, 1);
    assert!(!state.on_submit_tab);

    // Next from last question → submit tab
    state.next_question();
    assert!(state.on_submit_tab);

    // Next from submit tab → stays on submit tab
    state.next_question();
    assert!(state.on_submit_tab);

    // Prev from submit tab → back to last question
    state.prev_question();
    assert!(!state.on_submit_tab);
    assert_eq!(state.current_question, 1);

    state.prev_question();
    assert_eq!(state.current_question, 0);

    // Can't go before first
    state.prev_question();
    assert_eq!(state.current_question, 0);
}

#[test]
fn test_render_option_descriptions_on_separate_lines() {
    // Real Claude Code renders descriptions indented below the label
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    let rendered = state.render(60);

    let lines: Vec<&str> = rendered.lines().collect();
    // Find the Rust option line, description should be on the next line
    let rust_idx = lines.iter().position(|l| l.contains("1. Rust")).unwrap();
    assert!(
        lines[rust_idx + 1].contains("Systems programming"),
        "Description should be on line after label"
    );
}

#[test]
fn test_number_key_selects_option() {
    // In real Claude Code, pressing a number key selects that option.
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Initially cursor at 0, nothing selected
    assert_eq!(state.questions[0].cursor, 0);
    assert!(state.questions[0].selected.is_empty());

    // select_by_number(2) should move cursor to index 1 and select it
    state.select_by_number(2);
    assert_eq!(state.questions[0].cursor, 1);
    assert_eq!(state.questions[0].selected, vec![1]);

    // Collecting answers should give Python
    let result = state.collect_answers();
    match result {
        ElicitationResult::Answered(answers) => {
            assert_eq!(answers.get("What language?").unwrap(), "Python");
        }
        other => panic!("Expected answered, got {:?}", other),
    }
}

#[test]
fn test_out_of_range_number_ignored() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Only 2 options; pressing 5 should do nothing
    state.select_by_number(5);
    assert_eq!(state.questions[0].cursor, 0);
    assert!(state.questions[0].selected.is_empty());
}

// =========================================================================
// Free-text "Type something." option
// =========================================================================

#[test]
fn test_free_text_typing() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something." (index 2 for 2 defined options)
    state.cursor_down();
    state.cursor_down();
    assert!(state.is_on_free_text());

    // Type "Parrot"
    state.insert_char('P');
    state.insert_char('a');
    state.insert_char('r');
    state.insert_char('r');
    state.insert_char('o');
    state.insert_char('t');

    assert_eq!(state.questions[0].free_text, "Parrot");
}

#[test]
fn test_free_text_backspace() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something."
    state.cursor_down();
    state.cursor_down();

    state.insert_char('A');
    state.insert_char('B');
    assert_eq!(state.questions[0].free_text, "AB");

    state.backspace_char();
    assert_eq!(state.questions[0].free_text, "A");

    state.backspace_char();
    assert_eq!(state.questions[0].free_text, "");
}

#[test]
fn test_free_text_preserved_on_navigate_away() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something." and type
    state.cursor_down();
    state.cursor_down();
    state.insert_char('X');
    state.insert_char('Y');

    // Navigate away and back
    state.cursor_up();
    assert!(!state.is_on_free_text());
    state.cursor_down();
    assert!(state.is_on_free_text());

    // Text preserved
    assert_eq!(state.questions[0].free_text, "XY");
}

#[test]
fn test_free_text_submit_returns_answer() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something." and type "Parrot"
    state.cursor_down();
    state.cursor_down();
    for c in "Parrot".chars() {
        state.insert_char(c);
    }

    let result = state.collect_answers();
    match result {
        ElicitationResult::Answered(answers) => {
            assert_eq!(answers.get("What language?").unwrap(), "Parrot");
        }
        _ => panic!("Expected answered with free text"),
    }
}

#[test]
fn test_free_text_empty_submit_cancels() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something." with empty text
    state.cursor_down();
    state.cursor_down();
    assert!(state.questions[0].free_text.is_empty());

    let result = state.collect_answers();
    assert!(matches!(result, ElicitationResult::Cancelled));
}

#[test]
fn test_insert_char_ignored_when_not_on_free_text() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Cursor on first defined option (Rust)
    state.insert_char('X');
    assert!(state.questions[0].free_text.is_empty());
}

// =========================================================================
// "Chat about this" option
// =========================================================================

#[test]
fn test_chat_about_this_returns_result() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Chat about this" (index 3 for 2 defined options)
    state.cursor_down(); // Python
    state.cursor_down(); // Type something.
    state.cursor_down(); // Chat about this
    assert_eq!(state.questions[0].cursor, 3);

    let result = state.collect_answers();
    assert!(matches!(result, ElicitationResult::ChatAboutThis));
}

// =========================================================================
// Render with free-text and chat
// =========================================================================

#[test]
fn test_render_type_something_with_text() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something." and type
    state.cursor_down();
    state.cursor_down();
    for c in "Parrot".chars() {
        state.insert_char(c);
    }

    let rendered = state.render(60);
    // "Type something." placeholder replaced by typed text
    assert!(rendered.contains("Parrot"));
    assert!(!rendered.contains("Type something."));
}

#[test]
fn test_render_chat_about_this_cursor() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Chat about this"
    state.cursor_down();
    state.cursor_down();
    state.cursor_down();

    let rendered = state.render(60);
    assert!(rendered.contains("❯ 4. Chat about this"));
}

// =========================================================================
// is_question_answered helper
// =========================================================================

#[test]
fn test_is_question_answered_empty() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    assert!(!state.is_question_answered(0));
}

#[test]
fn test_is_question_answered_with_selection() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    state.toggle_or_select(); // Select Rust
    assert!(state.is_question_answered(0));
}

#[test]
fn test_is_question_answered_with_free_text() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    // Navigate to "Type something." and type
    state.cursor_down();
    state.cursor_down();
    state.insert_char('X');
    assert!(state.is_question_answered(0));
}

#[test]
fn test_is_question_answered_out_of_range() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    assert!(!state.is_question_answered(99));
}

// =========================================================================
// Auto-advance on select (select_and_advance)
// =========================================================================

#[test]
fn test_select_and_advance_single_select() {
    let input = multi_question_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    assert_eq!(state.current_question, 0);
    assert!(!state.on_submit_tab);

    // Select at cursor (Rust) and advance
    state.select_and_advance();
    assert_eq!(state.questions[0].selected, vec![0]);
    assert_eq!(state.current_question, 1);
    assert!(!state.on_submit_tab);
}

#[test]
fn test_select_and_advance_last_question_goes_to_submit() {
    let input = json!({
        "questions": [{
            "question": "Q1?", "header": "H1",
            "options": [{ "label": "A", "description": "" }],
            "multiSelect": false
        }]
    });
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    state.select_and_advance();
    assert!(state.on_submit_tab);
    assert_eq!(state.questions[0].selected, vec![0]);
}

#[test]
fn test_select_and_advance_multi_select_is_noop() {
    let input = multi_select_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    state.select_and_advance(); // Should not advance (multi-select)
    assert_eq!(state.current_question, 0);
    // Should not change selected either
    assert!(state.questions[0].selected.is_empty());
}

// =========================================================================
// Submit tab
// =========================================================================

#[test]
fn test_submit_tab_cursor_navigation() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    state.on_submit_tab = true;

    assert_eq!(state.submit_cursor, 0);

    state.submit_cursor_down();
    assert_eq!(state.submit_cursor, 1);

    // Clamp at 1
    state.submit_cursor_down();
    assert_eq!(state.submit_cursor, 1);

    state.submit_cursor_up();
    assert_eq!(state.submit_cursor, 0);

    // Clamp at 0
    state.submit_cursor_up();
    assert_eq!(state.submit_cursor, 0);
}

#[test]
fn test_submit_tab_render_all_answered() {
    let input = multi_question_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Answer all questions
    state.select_and_advance(); // Q1: Rust → Q2
    state.select_and_advance(); // Q2: CLI → Q3
    state.select_and_advance(); // Q3: GitHub Actions → Q4
    state.select_and_advance(); // Q4: MIT → Submit tab
    assert!(state.on_submit_tab);

    let rendered = state.render(60);
    assert!(rendered.contains("Review your answers"));
    assert!(rendered.contains("→ Rust"));
    assert!(rendered.contains("→ CLI"));
    assert!(rendered.contains("→ GitHub Actions"));
    assert!(rendered.contains("→ MIT"));
    assert!(rendered.contains("❯ 1. Submit answers"));
    assert!(rendered.contains("  2. Cancel"));
    // No warning when all answered
    assert!(!rendered.contains("⚠"));
    // Tab bar present
    assert!(rendered.contains("[✔ Submit]"));
}

#[test]
fn test_submit_tab_render_partially_answered() {
    let input = multi_question_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Answer only first question
    state.select_and_advance(); // Q1: Rust → Q2
                                // Skip to submit tab
    state.on_submit_tab = true;

    let rendered = state.render(60);
    assert!(rendered.contains("Review your answers"));
    assert!(rendered.contains("→ Rust"));
    assert!(rendered.contains("(no answer)"));
    assert!(rendered.contains("⚠ You have not answered all questions"));
}

#[test]
fn test_submit_tab_render_with_free_text_answer() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to "Type something." and type
    state.cursor_down();
    state.cursor_down();
    for c in "Go".chars() {
        state.insert_char(c);
    }
    state.on_submit_tab = true;

    let rendered = state.render(60);
    assert!(rendered.contains("→ Go"));
}

// =========================================================================
// Tab bar rendering
// =========================================================================

#[test]
fn test_tab_bar_not_shown_for_single_question() {
    let input = sample_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    let rendered = state.render(60);

    assert!(!rendered.contains("←"));
    assert!(!rendered.contains("Submit"));
}

#[test]
fn test_tab_bar_shown_for_multi_question() {
    let input = multi_question_input();
    let state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());
    let rendered = state.render(60);

    assert!(rendered.contains("←"));
    assert!(rendered.contains("→"));
    // Current question is bracketed
    assert!(rendered.contains("[☐ Language]"));
    // Other questions not bracketed
    assert!(rendered.contains("☐ Project"));
    assert!(rendered.contains("☐ CI/CD"));
    assert!(rendered.contains("☐ License"));
    assert!(rendered.contains("✔ Submit"));
}

#[test]
fn test_tab_bar_shows_answered_state() {
    let input = multi_question_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Answer first question and advance
    state.select_and_advance(); // Q1 answered, now on Q2
    let rendered = state.render(60);

    // First question should show ☒ (answered)
    assert!(rendered.contains("☒ Language"));
    // Current question (Q2) bracketed
    assert!(rendered.contains("[☐ Project]"));
}

// =========================================================================
// Selected option indicators (✔)
// =========================================================================

#[test]
fn test_render_selected_option_check() {
    let input = multi_question_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Answer first question and go back to see the ✔
    state.select_and_advance(); // Select Rust, advance to Q2
    state.prev_question(); // Back to Q1

    let rendered = state.render(60);
    assert!(rendered.contains("1. Rust ✔"));
    assert!(!rendered.contains("2. Python ✔"));
}

#[test]
fn test_render_single_select_header_checked_when_answered() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    state.toggle_or_select(); // Select Rust
    let rendered = state.render(60);

    // Single-select with selection shows ☒ header
    assert!(rendered.contains("☒ Language"));
}

// =========================================================================
// is_on_free_text returns false on submit tab
// =========================================================================

#[test]
fn test_is_on_free_text_false_on_submit_tab() {
    let input = sample_input();
    let mut state = ElicitationState::from_tool_input(&input, "toolu_001".to_string());

    // Navigate to free text row
    state.cursor_down();
    state.cursor_down();
    assert!(state.is_on_free_text());

    // Switch to submit tab
    state.on_submit_tab = true;
    assert!(!state.is_on_free_text());
}
