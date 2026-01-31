// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

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
use common::{assert_tui_matches_fixture, tmux, TuiTestSession, TUI_READY_PATTERN};

const JSON_SCENARIO: &str = r#"
    {
        "default_response": "Hello!",
        "trusted": true,
        "claude_version": "2.1.12"
    }
"#;

/// Compare initial state against real Claude fixture
#[test]
fn test_initial_state_matches_fixture() {
    let tui = TuiTestSession::new("fixture-initial", JSON_SCENARIO);
    let capture = tui.capture();

    assert_tui_matches_fixture(&capture, "initial_state.txt", None);
}

/// Compare ANSI-colored initial state against real Claude fixture.
#[test]
fn test_initial_state_ansi_matches_fixture() {
    let tui = TuiTestSession::new("fixture-initial-ansi", JSON_SCENARIO);

    // Wait for ready state first
    let _ = tui.wait_for(TUI_READY_PATTERN);

    // Capture with ANSI sequences
    let capture = tmux::capture_pane_ansi(tui.name());

    // Compare against ANSI fixture
    assert_ansi_matches_fixture(&capture, "initial_state_ansi.txt", None);
}
