// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn styled_logo_line1_contains_ansi_codes() {
    let line = styled_logo_line1("v2.1.12");

    // Should contain orange foreground
    assert!(line.contains("\x1b[38;2;215;119;87m"));
    // Should contain black background
    assert!(line.contains("\x1b[48;2;0;0;0m"));
    // Should contain bold
    assert!(line.contains("\x1b[1m"));
    // Should contain gray foreground
    assert!(line.contains("\x1b[38;2;153;153;153m"));
    // Should contain resets
    assert!(line.contains("\x1b[39m"));
    assert!(line.contains("\x1b[49m"));
    assert!(line.contains("\x1b[0m"));
    // Should contain content
    assert!(line.contains("Claude Code"));
    assert!(line.contains("v2.1.12"));
    // Should end with fg_reset after version
    assert!(
        line.ends_with("\x1b[39m"),
        "Line should end with [39m, got: {:?}",
        &line[line.len().saturating_sub(20)..]
    );
}

#[test]
fn styled_logo_line2_contains_ansi_codes() {
    let line = styled_logo_line2("Haiku 4.5 · Claude Max");

    // Should contain orange foreground
    assert!(line.contains("\x1b[38;2;215;119;87m"));
    // Should contain black background
    assert!(line.contains("\x1b[48;2;0;0;0m"));
    // Should contain gray foreground
    assert!(line.contains("\x1b[38;2;153;153;153m"));
    // Should contain content
    assert!(line.contains("Haiku 4.5 · Claude Max"));
}

#[test]
fn styled_logo_line3_contains_ansi_codes() {
    let line = styled_logo_line3("~/Developer/claudeless");

    // Should contain orange foreground
    assert!(line.contains("\x1b[38;2;215;119;87m"));
    // Should contain gray foreground
    assert!(line.contains("\x1b[38;2;153;153;153m"));
    // Should NOT contain black background (line 3 has no bg)
    assert!(!line.contains("\x1b[48;2;0;0;0m"));
    // Should contain content
    assert!(line.contains("~/Developer/claudeless"));
}

#[test]
fn styled_separator_contains_dim_and_gray() {
    let sep = styled_separator(10);

    // Should contain dim
    assert!(sep.contains("\x1b[2m"));
    // Should contain dark gray foreground
    assert!(sep.contains("\x1b[38;2;136;136;136m"));
    // Should NOT contain reset (reset is at start of next line)
    assert!(!sep.contains("\x1b[0m"));
    // Should contain separator characters
    assert!(sep.contains("──────────"));
}

#[test]
fn styled_placeholder_has_inverse_first_char() {
    let placeholder = styled_placeholder("Try \"refactor mod.rs\"");

    // Should contain inverse
    assert!(placeholder.contains("\x1b[7m"));
    // Should contain reset+dim
    assert!(placeholder.contains("\x1b[0;2m"));
    // Should contain reset
    assert!(placeholder.contains("\x1b[0m"));
    // Should contain prompt symbol
    assert!(placeholder.contains("❯"));
}

#[test]
fn styled_status_text_contains_gray() {
    let status = styled_status_text("? for shortcuts");

    // Should contain gray foreground
    assert!(status.contains("\x1b[38;2;153;153;153m"));
    // Should contain reset
    assert!(status.contains("\x1b[39m"));
    // Should contain content
    assert!(status.contains("? for shortcuts"));
}
