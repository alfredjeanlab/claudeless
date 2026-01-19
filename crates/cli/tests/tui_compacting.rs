// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Compacting tests - /compact command behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

/// Helper to run a conversation and then trigger /compact
fn run_compact_test(session: &str) -> (String, String, String) {
    let scenario = write_scenario(
        r#"
        name = "compact-test"
        [[responses]]
        pattern = { type = "contains", text = "read" }
        response = "The file contains test content."
        [[responses]]
        pattern = { type = "contains", text = "lorem" }
        response = "Lorem ipsum dolor sit amet, consectetur adipiscing elit."
        [[responses]]
        pattern = { type = "any" }
        response = "ok"
        "#,
    );

    start_tui(session, &scenario);

    tmux::send_line(session, "read the file");
    tmux::wait_for_content(session, "file contains test content");

    tmux::send_line(session, "generate lorem ipsum");
    tmux::wait_for_content(session, "Lorem ipsum");

    let before_capture = tmux::capture_pane(session);

    tmux::send_line(session, "/compact");

    // Capture during compacting (look for compacting indicator)
    let during_capture = tmux::wait_for_content(session, "Compacting");

    // Wait for completion
    let after_capture = tmux::wait_for_content(session, "Compacted");

    tmux::send_keys(session, "C-c");
    tmux::send_keys(session, "C-c");
    tmux::kill_session(session);

    (before_capture, during_capture, after_capture)
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// During compacting, shows "✻ Compacting conversation… (ctrl+c to interrupt)"
#[test]
fn test_compact_shows_in_progress_message() {
    let (_, during, _) = run_compact_test("claudeless-compact-during");

    assert!(
        during.contains("Compacting") || during.to_lowercase().contains("compact"),
        "Should show compacting message during operation.\nCapture:\n{}",
        during
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// During compacting, shows interrupt hint "(ctrl+c to interrupt)"
#[test]
fn test_compact_shows_interrupt_hint() {
    let (_, during, _) = run_compact_test("claudeless-compact-hint");

    assert!(
        during.contains("ctrl+c") || during.contains("interrupt") || during.contains("Ctrl+C"),
        "Should show interrupt hint during compacting.\nCapture:\n{}",
        during
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// After compacting, shows "Compacted (ctrl+o to see full summary)"
#[test]
fn test_compact_shows_completion_message() {
    let (_, _, after) = run_compact_test("claudeless-compact-after");

    assert!(
        after.contains("Compacted"),
        "After compact, should show 'Compacted' message.\nCapture:\n{}",
        after
    );

    assert!(
        after.contains("ctrl+o") || after.contains("summary"),
        "After compact, should show ctrl+o hint for full summary.\nCapture:\n{}",
        after
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// After compacting, shows separator "════ Conversation compacted · ctrl+o for history ════"
#[test]
fn test_compact_shows_separator() {
    let (_, _, after) = run_compact_test("claudeless-compact-sep");

    assert!(
        after.contains("═") || after.contains("="),
        "After compact, should show separator line.\nCapture:\n{}",
        after
    );

    assert!(
        after.to_lowercase().contains("conversation compacted")
            || after.to_lowercase().contains("history"),
        "Separator should mention 'Conversation compacted' or 'history'.\nCapture:\n{}",
        after
    );
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// After compacting, conversation history is collapsed/hidden
#[test]
fn test_compact_collapses_history() {
    let (before, _, after) = run_compact_test("claudeless-compact-collapse");

    let before_exchanges = before.matches("❯").count() + before.matches("⏺").count();
    let after_exchanges = after.matches("❯").count() + after.matches("⏺").count();

    assert!(
        before.len() > after.len() || before_exchanges > after_exchanges,
        "Compact should collapse conversation history.\nBefore length: {}, After length: {}\nBefore:\n{}\nAfter:\n{}",
        before.len(),
        after.len(),
        before,
        after
    );
}

/// Compare conversation state before /compact against fixture
#[test]
#[ignore] // TODO(implement): Simulator rendering differs from real Claude CLI fixture
fn test_compact_before_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "responses": [
                { "pattern": { "type": "contains", "text": "read Cargo.toml" }, "response": "Read(Cargo.toml)\n  ⎿  Read 14 lines\n\nThis is a workspace configuration, not a single package. The workspace contains two members:\n\n  1. crates/cli\n\nThe project is called claudeless." },
                { "pattern": { "type": "contains", "text": "lorem ipsum" }, "response": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.\n\n  Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem.\n\n  At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus." },
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-fixture-compact-before";
    start_tui(session, &scenario);

    // Build up conversation
    tmux::send_line(session, "read Cargo.toml and tell me the package name");
    tmux::wait_for_content(session, "workspace configuration");

    tmux::send_line(session, "generate 3 paragraphs of lorem ipsum");
    let capture = tmux::wait_for_content(session, "dolor repellendus");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "compact_before.txt", None);
}

/// Compare conversation state during /compact against fixture
#[test]
#[ignore] // TODO(fixture): Header inclusion differs from tmux capture method
fn test_compact_during_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "responses": [
                { "pattern": { "type": "contains", "text": "lorem" }, "response": "Lorem ipsum dolor sit amet." },
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-fixture-compact-during";
    start_tui(session, &scenario);

    // Build up some conversation
    tmux::send_line(session, "lorem");
    tmux::wait_for_content(session, "Lorem ipsum");

    // Trigger compact
    tmux::send_line(session, "/compact");

    // Capture during compacting
    let capture = tmux::wait_for_content(session, "Compacting");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "compact_during.txt", None);
}

/// Compare conversation state after /compact against fixture
#[test]
#[ignore] // TODO(fixture): Tool summary requires tool_calls to be recorded in session
fn test_compact_after_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "trusted": true,
            "responses": [
                { "pattern": { "type": "contains", "text": "read Cargo.toml" }, "response": "Read(Cargo.toml)\n  ⎿  Read 14 lines\n\nThis is a workspace configuration." },
                { "pattern": { "type": "any" }, "response": "ok" }
            ]
        }
        "#,
    );

    let session = "claudeless-fixture-compact-after";
    start_tui(session, &scenario);

    // Build up some conversation
    tmux::send_line(session, "read Cargo.toml and tell me about it");
    tmux::wait_for_content(session, "workspace configuration");

    // Trigger compact and wait for completion
    tmux::send_line(session, "/compact");
    let capture = tmux::wait_for_content(session, "Compacted");

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "compact_after.txt", None);
}
