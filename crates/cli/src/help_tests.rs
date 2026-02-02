// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn test_calc_desc_col_basic() {
    let sections = vec![HelpSection {
        title: "Options",
        items: vec![HelpItem::Entry {
            flags: "-h, --help",
            description: "Display help for command",
        }],
    }];
    // 2 (indent) + 10 (flags) + 2 (padding) = 14
    assert_eq!(calc_desc_col(&sections), 14);
}

#[test]
fn test_calc_desc_col_across_sections() {
    let sections = vec![
        HelpSection {
            title: "Options",
            items: vec![HelpItem::Entry {
                flags: "-h, --help",
                description: "Display help for command",
            }],
        },
        HelpSection {
            title: "Commands",
            items: vec![HelpItem::Entry {
                flags: "uninstall|remove [options] <plugin>",
                description: "Uninstall a plugin",
            }],
        },
    ];
    // longest is 35 chars: 2 + 35 + 2 = 39
    assert_eq!(calc_desc_col(&sections), 39);
}

#[test]
fn test_render_entry_alignment() {
    let mut out = String::new();
    render_entry(&mut out, "-h, --help", "Display help for command", 14, None);
    assert_eq!(out, "  -h, --help  Display help for command\n");
}

#[test]
fn test_render_entry_wrapping() {
    let mut out = String::new();
    render_entry(
        &mut out,
        "-t, --transport <transport>",
        "Transport type (stdio, sse, http). Defaults to stdio if not specified.",
        31,
        Some(80),
    );
    let expected = concat!(
        "  -t, --transport <transport>  Transport type (stdio, sse, http). Defaults to\n",
        "                               stdio if not specified.\n",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_wrap_text_no_wrap_needed() {
    let result = wrap_text("short text", 80);
    assert_eq!(result, vec!["short text"]);
}

#[test]
fn test_wrap_text_wraps_at_boundary() {
    let result = wrap_text("hello world foo bar", 12);
    assert_eq!(result, vec!["hello world", "foo bar"]);
}

#[test]
fn test_doctor_help_matches_fixture() {
    let output = render_doctor_help();
    let expected = concat!(
        "Usage: claude doctor [options]\n",
        "\n",
        "Check the health of your Claude Code auto-updater\n",
        "\n",
        "Options:\n",
        "  -h, --help  Display help for command\n",
    );
    assert_eq!(output, expected);
}

#[test]
fn test_update_help_matches_fixture() {
    let output = render_update_help();
    let expected = concat!(
        "Usage: claude update [options]\n",
        "\n",
        "Check for updates and install if available\n",
        "\n",
        "Options:\n",
        "  -h, --help  Display help for command\n",
    );
    assert_eq!(output, expected);
}

#[test]
fn test_install_help_matches_fixture() {
    let output = render_install_help();
    let expected = concat!(
        "Usage: claude install [options] [target]\n",
        "\n",
        "Install Claude Code native build. Use [target] to specify version (stable,\n",
        "latest, or specific version)\n",
        "\n",
        "Options:\n",
        "  --force     Force installation even if already installed\n",
        "  -h, --help  Display help for command\n",
    );
    assert_eq!(output, expected);
}

#[test]
fn test_setup_token_help_matches_fixture() {
    let output = render_setup_token_help();
    let expected = concat!(
        "Usage: claude setup-token [options]\n",
        "\n",
        "Set up a long-lived authentication token (requires Claude subscription)\n",
        "\n",
        "Options:\n",
        "  -h, --help  Display help for command\n",
    );
    assert_eq!(output, expected);
}

#[test]
fn test_main_help_starts_with_expected() {
    let output = render_main_help();
    assert!(output.starts_with("Usage: claude [options] [command] [prompt]\n"));
    assert!(output.contains("  --add-dir <directories...>"));
    assert!(output.contains("  -v, --version"));
    assert!(output.contains("Commands:"));
    assert!(output.contains("  doctor"));
}
