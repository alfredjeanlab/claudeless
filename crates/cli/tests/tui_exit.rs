// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI exit tests - Ctrl+C behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)

mod common;

use common::{start_tui, tmux, write_scenario};

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Ctrl+C on empty input should exit
#[test]
fn test_tui_ctrl_c_exits_on_empty_input() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-exit-test";
    let previous = start_tui(session, &scenario);

    // First C-c cancels, wait for effect, second C-c exits
    tmux::send_keys(session, "C-c");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "C-c");

    // Wait for shell prompt to appear (indicating exit)
    // Note: ❯ is starship/zsh prompt, $ is bash, % is zsh default
    let capture = tmux::wait_for_any(session, &["$", "%", "❯"]);

    tmux::kill_session(session);

    assert!(
        capture.contains("$") || capture.contains("%") || capture.contains("❯"),
        "Ctrl+C should exit TUI and show shell prompt.\nCapture:\n{}",
        capture
    );
}
