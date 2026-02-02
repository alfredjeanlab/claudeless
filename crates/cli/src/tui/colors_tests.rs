// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn styled_logo_line1_contains_ansi_codes() {
    let line = styled_logo_line1("Claude Code", "v2.1.12");

    // Should contain orange foreground
    assert!(line.contains("\x1b[38;2;215;119;87m"));
    // Should contain black background
    assert!(line.contains("\x1b[48;2;0;0;0m"));
    // Should contain bold
    assert!(line.contains("\x1b[1m"));
    // Should contain gray foreground
    assert!(line.contains("\x1b[38;2;153;153;153m"));
    // Should contain resets
    assert!(line.contains("\x1b[49m"));
    assert!(line.contains("\x1b[0m"));
    // Should contain content
    assert!(line.contains("Claude Code"));
    assert!(line.contains("v2.1.12"));
    // Should NOT end with reset — iocraft inserts \x1b[K after Text content
    // and a trailing \x1b[0m would get split, leaking a stray 'm' character.
    assert!(
        line.ends_with("v2.1.12"),
        "Line should end with version (no trailing reset), got: {:?}",
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
    assert!(status.contains("\x1b[0m"));
    // Should contain content
    assert!(status.contains("? for shortcuts"));
}

// Permission mode tests

#[test]
fn styled_permission_status_default_mode() {
    let status = styled_permission_status(&PermissionMode::Default);
    // Default mode shows "? for shortcuts" in gray
    assert!(status.contains("\x1b[38;2;153;153;153m")); // Gray color
    assert!(status.contains("? for shortcuts"));
}

#[test]
fn styled_permission_status_plan_mode() {
    let status = styled_permission_status(&PermissionMode::Plan);
    // Plan mode uses teal color
    assert!(status.contains("\x1b[38;2;72;150;140m")); // Teal color
    assert!(status.contains("⏸ plan mode on"));
    assert!(status.contains("(shift+tab to cycle)"));
}

#[test]
fn styled_permission_status_accept_edits_mode() {
    let status = styled_permission_status(&PermissionMode::AcceptEdits);
    // Accept edits mode uses purple color
    assert!(status.contains("\x1b[38;2;175;135;255m")); // Purple color
    assert!(status.contains("⏵⏵ accept edits on"));
    assert!(status.contains("(shift+tab to cycle)"));
}

#[test]
fn styled_permission_status_bypass_mode() {
    let status = styled_permission_status(&PermissionMode::BypassPermissions);
    // Bypass mode uses red/pink color
    assert!(status.contains("\x1b[38;2;255;107;128m")); // Red/Pink color
    assert!(status.contains("⏵⏵ bypass permissions on"));
    assert!(status.contains("(shift+tab to cycle)"));
}

#[test]
fn styled_permission_status_delegate_mode() {
    let status = styled_permission_status(&PermissionMode::Delegate);
    // Delegate mode uses gray
    assert!(status.contains("\x1b[38;2;153;153;153m")); // Gray color
    assert!(status.contains("delegate mode"));
    assert!(status.contains("(shift+tab to cycle)"));
}

#[test]
fn styled_permission_status_dontask_mode() {
    let status = styled_permission_status(&PermissionMode::DontAsk);
    // DontAsk mode uses gray
    assert!(status.contains("\x1b[38;2;153;153;153m")); // Gray color
    assert!(status.contains("don't ask mode"));
    assert!(status.contains("(shift+tab to cycle)"));
}

// Edge case tests

#[test]
fn styled_placeholder_handles_unicode() {
    let placeholder = styled_placeholder("Tëst");
    // Should have inverse on first char 'T'
    assert!(placeholder.contains("\x1b[7mT"));
    // Should contain the unicode rest
    assert!(placeholder.contains("ëst"));
}

#[test]
fn styled_logo_line3_handles_long_paths() {
    let long_path = "~/".to_string() + &"a".repeat(200);
    let line = styled_logo_line3(&long_path);
    // Should contain the full path
    assert!(line.contains(&long_path));
    // Should still have proper ANSI codes
    assert!(line.contains("\x1b[38;2;215;119;87m")); // Orange
    assert!(line.contains("\x1b[38;2;153;153;153m")); // Gray
}

#[test]
fn styled_separator_handles_large_widths() {
    let sep = styled_separator(500);
    // Should contain 500 separator characters
    assert_eq!(sep.matches('─').count(), 500);
    // Should still have proper ANSI codes
    assert!(sep.contains("\x1b[2m")); // Dim
    assert!(sep.contains("\x1b[38;2;136;136;136m")); // Dark gray
}

#[test]
fn styled_separator_handles_zero_width() {
    let sep = styled_separator(0);
    // Should have ANSI codes but no separator chars
    assert!(sep.contains("\x1b[2m"));
    assert!(!sep.contains('─'));
}

#[test]
fn styled_placeholder_handles_single_char() {
    let placeholder = styled_placeholder("X");
    // Should have inverse on the single char
    assert!(placeholder.contains("\x1b[7mX"));
    // Should have proper structure
    assert!(placeholder.contains("❯"));
    assert!(placeholder.starts_with("\x1b[0m")); // Reset at start
}
