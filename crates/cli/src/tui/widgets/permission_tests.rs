// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]
use super::*;

// =========================================================================
// Bash Command Categorization Tests
// =========================================================================

#[test]
fn test_categorize_etc_reading() {
    assert_eq!(
        categorize_bash_command("cat /etc/passwd"),
        BashCommandCategory::ReadingEtc
    );
    assert_eq!(
        categorize_bash_command("cat /etc/passwd | head -5"),
        BashCommandCategory::ReadingEtc
    );
    assert_eq!(
        categorize_bash_command("ls /etc/"),
        BashCommandCategory::ReadingEtc
    );
}

#[test]
fn test_categorize_named_commands() {
    assert_eq!(
        categorize_bash_command("npm test"),
        BashCommandCategory::NamedCommand("npm".to_string())
    );
    assert_eq!(
        categorize_bash_command("rm -rf /tmp/foo"),
        BashCommandCategory::NamedCommand("rm".to_string())
    );
    assert_eq!(
        categorize_bash_command("cargo build --release"),
        BashCommandCategory::NamedCommand("cargo".to_string())
    );
    assert_eq!(
        categorize_bash_command("git status"),
        BashCommandCategory::NamedCommand("git".to_string())
    );
}

#[test]
fn test_categorize_commands_with_paths() {
    assert_eq!(
        categorize_bash_command("/usr/bin/npm test"),
        BashCommandCategory::NamedCommand("npm".to_string())
    );
    assert_eq!(
        categorize_bash_command("./scripts/build.sh"),
        BashCommandCategory::NamedCommand("build.sh".to_string())
    );
}

#[test]
fn test_categorize_empty_or_whitespace() {
    assert_eq!(categorize_bash_command(""), BashCommandCategory::Generic);
    assert_eq!(categorize_bash_command("   "), BashCommandCategory::Generic);
}

// =========================================================================
// Option 2 Text Integration Tests
// =========================================================================

#[test]
fn test_option2_text_etc_reading() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "cat /etc/passwd".to_string(),
        description: None,
    });
    let output = dialog.render(120);
    assert!(output.contains("Yes, allow reading from etc/ from this project"));
}

#[test]
fn test_option2_text_npm_command() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "npm test".to_string(),
        description: None,
    });
    let output = dialog.render(120);
    assert!(output.contains("Yes, allow npm commands from this project"));
}

#[test]
fn test_option2_text_rm_command() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "rm -rf /tmp/test".to_string(),
        description: None,
    });
    let output = dialog.render(120);
    assert!(output.contains("Yes, allow rm commands from this project"));
}

// =========================================================================
// PermissionSelection Tests
// =========================================================================

#[test]
fn test_selection_next_cycles() {
    assert_eq!(
        PermissionSelection::Yes.next(),
        PermissionSelection::YesSession
    );
    assert_eq!(
        PermissionSelection::YesSession.next(),
        PermissionSelection::No
    );
    assert_eq!(PermissionSelection::No.next(), PermissionSelection::Yes);
}

#[test]
fn test_selection_prev_cycles() {
    assert_eq!(PermissionSelection::Yes.prev(), PermissionSelection::No);
    assert_eq!(
        PermissionSelection::YesSession.prev(),
        PermissionSelection::Yes
    );
    assert_eq!(
        PermissionSelection::No.prev(),
        PermissionSelection::YesSession
    );
}

#[test]
fn test_selection_default() {
    let selection = PermissionSelection::default();
    assert_eq!(selection, PermissionSelection::Yes);
}

// =========================================================================
// RichPermissionDialog Bash Tests
// =========================================================================

#[test]
fn test_bash_dialog_render() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "cat /etc/passwd | head -5".to_string(),
        description: Some("Display first 5 lines of /etc/passwd".to_string()),
    });

    let output = dialog.render(120);

    // Check key elements
    assert!(output.contains("Bash command"));
    assert!(output.contains("cat /etc/passwd | head -5"));
    assert!(output.contains("Display first 5 lines of /etc/passwd"));
    assert!(output.contains("Do you want to proceed?"));
    assert!(output.contains("❯ 1. Yes"));
    assert!(output.contains("2. Yes, allow reading from etc/ from this project"));
    assert!(output.contains("3. No"));
    assert!(output.contains("Esc to cancel"));
}

#[test]
fn test_bash_dialog_selection_indicator() {
    let mut dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "ls".to_string(),
        description: None,
    });

    // Default selection is Yes
    let output = dialog.render(120);
    assert!(output.contains(" ❯ 1. Yes"));
    assert!(output.contains("   2. Yes"));
    assert!(output.contains("   3. No"));

    // Select YesSession
    dialog.selected = PermissionSelection::YesSession;
    let output = dialog.render(120);
    assert!(output.contains("   1. Yes"));
    assert!(output.contains(" ❯ 2. Yes"));
    assert!(output.contains("   3. No"));

    // Select No
    dialog.selected = PermissionSelection::No;
    let output = dialog.render(120);
    assert!(output.contains("   1. Yes"));
    assert!(output.contains("   2. Yes"));
    assert!(output.contains(" ❯ 3. No"));
}

// =========================================================================
// RichPermissionDialog Edit Tests
// =========================================================================

#[test]
fn test_edit_dialog_render() {
    let dialog = RichPermissionDialog::new(PermissionType::Edit {
        file_path: "hello.txt".to_string(),
        diff_lines: vec![
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::Removed,
                content: "Hello World".to_string(),
            },
            DiffLine {
                line_num: Some(1),
                kind: DiffKind::NoNewline,
                content: "  No newline at end of file".to_string(),
            },
            DiffLine {
                line_num: Some(2),
                kind: DiffKind::Added,
                content: "Hello Universe".to_string(),
            },
            DiffLine {
                line_num: Some(3),
                kind: DiffKind::NoNewline,
                content: "  No newline at end of file".to_string(),
            },
        ],
    });

    let output = dialog.render(120);

    // Check key elements
    assert!(output.contains("Edit file hello.txt"));
    assert!(output.contains("╌")); // Dashed separator
    assert!(output.contains(" 1 -Hello World"));
    assert!(output.contains(" 2 +Hello Universe"));
    assert!(output.contains("Do you want to make this edit to hello.txt?"));
    assert!(output.contains("Yes, allow all edits during this session"));
}

#[test]
fn test_diff_line_rendering() {
    // Test removed line
    let removed = DiffLine {
        line_num: Some(1),
        kind: DiffKind::Removed,
        content: "old line".to_string(),
    };
    assert_eq!(render_diff_line(&removed), " 1 -old line");

    // Test added line
    let added = DiffLine {
        line_num: Some(2),
        kind: DiffKind::Added,
        content: "new line".to_string(),
    };
    assert_eq!(render_diff_line(&added), " 2 +new line");

    // Test context line
    let context = DiffLine {
        line_num: Some(3),
        kind: DiffKind::Context,
        content: "unchanged".to_string(),
    };
    assert_eq!(render_diff_line(&context), " 3  unchanged");

    // Test line without number (uses space padding for alignment)
    let no_num = DiffLine {
        line_num: None,
        kind: DiffKind::NoNewline,
        content: " No newline".to_string(),
    };
    // Format: "    " (4 spaces) + " " (prefix for NoNewline) + " No newline" (content)
    assert_eq!(render_diff_line(&no_num), "      No newline");
}

// =========================================================================
// RichPermissionDialog Write Tests
// =========================================================================

#[test]
fn test_write_dialog_render() {
    let dialog = RichPermissionDialog::new(PermissionType::Write {
        file_path: "hello.txt".to_string(),
        content_lines: vec!["Hello World".to_string()],
    });

    let output = dialog.render(120);

    // Check key elements
    assert!(output.contains("Create file hello.txt"));
    assert!(output.contains("╌")); // Dashed separator
    assert!(output.contains("  1 Hello World"));
    assert!(output.contains("Do you want to create hello.txt?"));
    assert!(output.contains("Yes, allow all edits during this session"));
}

#[test]
fn test_write_dialog_multiple_lines() {
    let dialog = RichPermissionDialog::new(PermissionType::Write {
        file_path: "test.txt".to_string(),
        content_lines: vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
        ],
    });

    let output = dialog.render(120);

    assert!(output.contains("  1 Line 1"));
    assert!(output.contains("  2 Line 2"));
    assert!(output.contains("  3 Line 3"));
}

// =========================================================================
// Separator Tests
// =========================================================================

#[test]
fn test_make_separator() {
    assert_eq!(make_separator('─', 5), "─────");
    assert_eq!(make_separator('╌', 3), "╌╌╌");
}

// =========================================================================
// Session Permission Key Tests
// =========================================================================

#[test]
fn test_bash_prefix_extraction_with_path() {
    let result = extract_bash_prefix("cat /etc/passwd | head -5");
    assert_eq!(result, "cat /etc/");
}

#[test]
fn test_bash_prefix_extraction_simple() {
    let result = extract_bash_prefix("npm test");
    assert_eq!(result, "npm");
}

#[test]
fn test_bash_prefix_extraction_flags_before_path() {
    // When the first argument is a flag, not a path
    let result = extract_bash_prefix("rm -rf /tmp/foo");
    assert_eq!(result, "rm");
}

#[test]
fn test_bash_prefix_extraction_empty() {
    let result = extract_bash_prefix("");
    assert_eq!(result, "");
}

#[test]
fn test_bash_prefix_extraction_single_command() {
    let result = extract_bash_prefix("ls");
    assert_eq!(result, "ls");
}

#[test]
fn test_session_key_bash() {
    let dialog = RichPermissionDialog::new(PermissionType::Bash {
        command: "cat /etc/passwd".to_string(),
        description: None,
    });
    assert!(matches!(
        dialog.session_key(),
        SessionPermissionKey::BashPrefix(_)
    ));
    if let SessionPermissionKey::BashPrefix(prefix) = dialog.session_key() {
        assert_eq!(prefix, "cat /etc/");
    }
}

#[test]
fn test_session_key_edit() {
    let dialog = RichPermissionDialog::new(PermissionType::Edit {
        file_path: "foo.txt".to_string(),
        diff_lines: vec![],
    });
    assert_eq!(dialog.session_key(), SessionPermissionKey::EditAll);
}

#[test]
fn test_session_key_write() {
    let dialog = RichPermissionDialog::new(PermissionType::Write {
        file_path: "foo.txt".to_string(),
        content_lines: vec![],
    });
    assert_eq!(dialog.session_key(), SessionPermissionKey::WriteAll);
}
