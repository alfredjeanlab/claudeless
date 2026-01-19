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

use common::{assert_tui_matches_fixture, start_tui, tmux, write_scenario};

/// Compare initial state against real Claude fixture
#[test]
fn test_initial_state_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true
        }
        "#,
    );

    let session = "claudeless-fixture-initial";
    let capture = start_tui(session, &scenario);

    tmux::kill_session(session);

    assert_tui_matches_fixture(&capture, "initial_state.txt", None);
}
