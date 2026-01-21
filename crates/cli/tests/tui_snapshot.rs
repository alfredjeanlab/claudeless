// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI snapshot tests comparing claudeless with real Claude CLI.
//!
//! These tests capture claudeless's TUI output via tmux and compare it
//! against snapshots captured from real Claude CLI v2.1.12.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! Test files:
//! - `common/mod.rs`: Shared helpers
//! - `common/mod.rs`: Tmux helpers
//! - `tui_interaction.rs`: TUI interaction tests (input, response display)
//! - `tui_exit.rs`: TUI exit tests (Ctrl+C behavior)
//! - `tui_model.rs`: Model name display tests
//! - `tui_permission.rs`: Permission mode display and shift+tab cycling
//! - `tui_thinking.rs`: Thinking toggle dialog tests (Meta+t)
//! - `tui_compacting.rs`: /compact command tests
//! - `tui_trust.rs`: Trust prompt tests

mod common;

use common::ansi::assert_ansi_matches_fixture;
use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario, TUI_READY_PATTERN};

/// Compare initial state against real Claude fixture
#[test]
fn test_initial_state_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = "claudeless-fixture-initial";
    let capture = start_tui(session, &scenario);

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "initial_state.txt", None);
}

/// Compare ANSI-colored initial state against real Claude fixture.
///
/// NOTE: This test is ignored until TUI color rendering is implemented.
/// The test infrastructure (ANSI parser, comparison utilities) is complete.
/// Once the TUI outputs ANSI colors, remove the #[ignore] attribute.
#[test]
#[ignore = "TUI color rendering not yet implemented"]
fn test_initial_state_ansi_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = "claudeless-fixture-initial-ansi";

    // Start TUI and wait for ready (using plain text pattern)
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);
    let cmd = format!(
        "{} --scenario {} --tui",
        common::claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);
    tmux::wait_for_content(session, TUI_READY_PATTERN);

    // Capture with ANSI sequences
    let capture = tmux::capture_pane_ansi(session);
    tmux::kill_session(session);

    // Compare against ANSI fixture
    assert_ansi_matches_fixture(&capture, "initial_state_ansi.txt", None);
}
