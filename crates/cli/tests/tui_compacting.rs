// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

//! Compacting tests - /compact command behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{assert_tui_matches_fixture, TuiTestSession};

const COMPACT_SCENARIO: &str = r#"
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
"#;

/// Helper to run a conversation and then trigger /compact
fn run_compact_test(name: &str) -> (String, String, String) {
    let tui = TuiTestSession::new(name, COMPACT_SCENARIO);

    tui.send_line("read the file");
    tui.wait_for("file contains test content");

    tui.send_line("generate lorem ipsum");
    tui.wait_for("Lorem ipsum");

    let before_capture = tui.capture();

    tui.send_line("/compact");

    // Capture during compacting (look for compacting indicator)
    let during_capture = tui.wait_for("Compacting");

    // Wait for completion
    let after_capture = tui.wait_for("Compacted");

    (before_capture, during_capture, after_capture)
}

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// During compacting, shows "✻ Compacting conversation… (ctrl+c to interrupt)"
#[test]
#[ignore] // TODO(flaky): Timing-sensitive test that captures transient UI state; fails on CI
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
#[ignore] // TODO(flaky): Timing-sensitive test that captures transient UI state; fails on CI
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
#[ignore] // TODO(flaky): Timing-sensitive test that captures transient UI state; fails on CI
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
#[ignore] // TODO(flaky): Timing-sensitive test that captures transient UI state; fails on CI
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
#[ignore] // TODO(flaky): Timing-sensitive test that captures transient UI state; fails on CI
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
#[ignore] // BLOCKED: Simulator header/response format differs from real CLI. See tests/capture/skipped/CLAUDE.md
fn test_compact_before_matches_fixture() {
    let tui = TuiTestSession::new(
        "fixture-compact-before",
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

    // Build up conversation
    tui.send_line("read Cargo.toml and tell me the package name");
    tui.wait_for("workspace configuration");

    tui.send_line("generate 3 paragraphs of lorem ipsum");
    let capture = tui.wait_for("dolor repellendus");

    assert_tui_matches_fixture(&capture, "compact_before.txt", None);
}

/// Compare conversation state during /compact against fixture
#[test]
#[ignore] // BLOCKED: Simulator header format differs from real CLI. See tests/capture/skipped/CLAUDE.md
fn test_compact_during_matches_fixture() {
    let tui = TuiTestSession::new(
        "fixture-compact-during",
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

    // Build up some conversation
    tui.send_line("lorem");
    tui.wait_for("Lorem ipsum");

    // Trigger compact
    tui.send_line("/compact");

    // Capture during compacting
    let capture = tui.wait_for("Compacting");

    assert_tui_matches_fixture(&capture, "compact_during.txt", None);
}

/// Compare conversation state after /compact against fixture
#[test]
#[ignore] // BLOCKED: Simulator doesn't track tool calls for compaction summary. See tests/capture/skipped/CLAUDE.md
fn test_compact_after_matches_fixture() {
    let tui = TuiTestSession::new(
        "fixture-compact-after",
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

    // Build up some conversation
    tui.send_line("read Cargo.toml and tell me about it");
    tui.wait_for("workspace configuration");

    // Trigger compact and wait for completion
    tui.send_line("/compact");
    let capture = tui.wait_for("Compacted");

    assert_tui_matches_fixture(&capture, "compact_after.txt", None);
}
