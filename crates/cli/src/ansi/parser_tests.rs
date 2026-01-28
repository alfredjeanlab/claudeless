// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =============================================================================
// AnsiSequence parsing tests
// =============================================================================

#[test]
fn test_parse_reset() {
    assert_eq!(AnsiSequence::from_params("0"), AnsiSequence::Reset);
    assert_eq!(AnsiSequence::from_params(""), AnsiSequence::Reset);
}

#[test]
fn test_parse_bold() {
    assert_eq!(AnsiSequence::from_params("1"), AnsiSequence::Bold);
}

#[test]
fn test_parse_dim() {
    assert_eq!(AnsiSequence::from_params("2"), AnsiSequence::Dim);
}

#[test]
fn test_parse_inverse() {
    assert_eq!(AnsiSequence::from_params("7"), AnsiSequence::Inverse);
}

#[test]
fn test_parse_fg_reset() {
    assert_eq!(AnsiSequence::from_params("39"), AnsiSequence::FgReset);
}

#[test]
fn test_parse_bg_reset() {
    assert_eq!(AnsiSequence::from_params("49"), AnsiSequence::BgReset);
}

#[test]
fn test_parse_reset_dim() {
    assert_eq!(AnsiSequence::from_params("0;2"), AnsiSequence::ResetDim);
}

#[test]
fn test_parse_fg_rgb() {
    assert_eq!(
        AnsiSequence::from_params("38;2;215;119;87"),
        AnsiSequence::FgRgb {
            r: 215,
            g: 119,
            b: 87
        }
    );
}

#[test]
fn test_parse_bg_rgb() {
    assert_eq!(
        AnsiSequence::from_params("48;2;0;0;0"),
        AnsiSequence::BgRgb { r: 0, g: 0, b: 0 }
    );
}

#[test]
fn test_parse_unknown() {
    assert_eq!(
        AnsiSequence::from_params("42"),
        AnsiSequence::Other("42".to_string())
    );
}

// =============================================================================
// to_escape_code round-trip tests
// =============================================================================

#[test]
fn test_escape_code_round_trip_reset() {
    assert_eq!(AnsiSequence::Reset.to_escape_code(), "\x1b[0m");
}

#[test]
fn test_escape_code_round_trip_bold() {
    assert_eq!(AnsiSequence::Bold.to_escape_code(), "\x1b[1m");
}

#[test]
fn test_escape_code_round_trip_dim() {
    assert_eq!(AnsiSequence::Dim.to_escape_code(), "\x1b[2m");
}

#[test]
fn test_escape_code_round_trip_inverse() {
    assert_eq!(AnsiSequence::Inverse.to_escape_code(), "\x1b[7m");
}

#[test]
fn test_escape_code_round_trip_fg_reset() {
    assert_eq!(AnsiSequence::FgReset.to_escape_code(), "\x1b[39m");
}

#[test]
fn test_escape_code_round_trip_bg_reset() {
    assert_eq!(AnsiSequence::BgReset.to_escape_code(), "\x1b[49m");
}

#[test]
fn test_escape_code_round_trip_reset_dim() {
    assert_eq!(AnsiSequence::ResetDim.to_escape_code(), "\x1b[0;2m");
}

#[test]
fn test_escape_code_round_trip_fg_rgb() {
    assert_eq!(
        AnsiSequence::FgRgb {
            r: 215,
            g: 119,
            b: 87
        }
        .to_escape_code(),
        "\x1b[38;2;215;119;87m"
    );
}

#[test]
fn test_escape_code_round_trip_bg_rgb() {
    assert_eq!(
        AnsiSequence::BgRgb { r: 0, g: 0, b: 0 }.to_escape_code(),
        "\x1b[48;2;0;0;0m"
    );
}

// =============================================================================
// parse_ansi tests
// =============================================================================

#[test]
fn test_parse_ansi_plain_text() {
    let spans = parse_ansi("Hello, world!");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].text, "Hello, world!");
    assert!(spans[0].sequences.is_empty());
}

#[test]
fn test_parse_ansi_single_sequence() {
    let spans = parse_ansi("\x1b[1mBold text");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].text, "Bold text");
    assert_eq!(spans[0].sequences, vec![AnsiSequence::Bold]);
}

#[test]
fn test_parse_ansi_sequence_at_end() {
    let spans = parse_ansi("Normal text\x1b[0m");
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "Normal text");
    assert!(spans[0].sequences.is_empty());
    assert_eq!(spans[1].text, "");
    assert_eq!(spans[1].sequences, vec![AnsiSequence::Reset]);
}

#[test]
fn test_parse_ansi_multiple_sequences() {
    let spans = parse_ansi("\x1b[1m\x1b[38;2;255;0;0mRed bold\x1b[0m");
    assert_eq!(spans.len(), 2);

    assert_eq!(spans[0].text, "Red bold");
    assert_eq!(
        spans[0].sequences,
        vec![
            AnsiSequence::Bold,
            AnsiSequence::FgRgb { r: 255, g: 0, b: 0 }
        ]
    );

    assert_eq!(spans[1].text, "");
    assert_eq!(spans[1].sequences, vec![AnsiSequence::Reset]);
}

#[test]
fn test_parse_ansi_mixed_content() {
    let spans = parse_ansi("Normal \x1b[1mBold\x1b[0m Normal again");
    assert_eq!(spans.len(), 3);

    assert_eq!(spans[0].text, "Normal ");
    assert!(spans[0].sequences.is_empty());

    assert_eq!(spans[1].text, "Bold");
    assert_eq!(spans[1].sequences, vec![AnsiSequence::Bold]);

    assert_eq!(spans[2].text, " Normal again");
    assert_eq!(spans[2].sequences, vec![AnsiSequence::Reset]);
}

#[test]
fn test_parse_ansi_from_fixture() {
    // Example from initial_state_ansi.txt: orange logo
    let input = "\x1b[38;2;215;119;87m ▐\x1b[48;2;0;0;0m▛███▜\x1b[49m▌\x1b[39m";
    let spans = parse_ansi(input);

    assert_eq!(spans.len(), 4);

    // Orange foreground: " ▐"
    assert_eq!(spans[0].text, " ▐");
    assert_eq!(
        spans[0].sequences,
        vec![AnsiSequence::FgRgb {
            r: 215,
            g: 119,
            b: 87
        }]
    );

    // Black background: "▛███▜"
    assert_eq!(spans[1].text, "▛███▜");
    assert_eq!(
        spans[1].sequences,
        vec![AnsiSequence::BgRgb { r: 0, g: 0, b: 0 }]
    );

    // Background reset: "▌"
    assert_eq!(spans[2].text, "▌");
    assert_eq!(spans[2].sequences, vec![AnsiSequence::BgReset]);

    // Foreground reset
    assert_eq!(spans[3].text, "");
    assert_eq!(spans[3].sequences, vec![AnsiSequence::FgReset]);
}

// =============================================================================
// strip_ansi tests
// =============================================================================

#[test]
fn test_strip_ansi_plain_text() {
    assert_eq!(strip_ansi("Hello, world!"), "Hello, world!");
}

#[test]
fn test_strip_ansi_removes_sequences() {
    assert_eq!(strip_ansi("\x1b[1mBold\x1b[0m"), "Bold");
}

#[test]
fn test_strip_ansi_complex() {
    let input = "\x1b[38;2;215;119;87m ▐\x1b[48;2;0;0;0m▛███▜\x1b[49m▌\x1b[39m";
    assert_eq!(strip_ansi(input), " ▐▛███▜▌");
}

#[test]
fn test_strip_ansi_multiline() {
    let input = "\x1b[1mLine 1\x1b[0m\n\x1b[2mLine 2\x1b[0m";
    assert_eq!(strip_ansi(input), "Line 1\nLine 2");
}

// =============================================================================
// extract_sequences tests
// =============================================================================

#[test]
fn test_extract_sequences_empty() {
    let sequences = extract_sequences("Plain text");
    assert!(sequences.is_empty());
}

#[test]
fn test_extract_sequences_single() {
    let sequences = extract_sequences("\x1b[1mBold");
    assert_eq!(sequences.len(), 1);
    assert_eq!(sequences[0].0, 0);
    assert_eq!(sequences[0].1, AnsiSequence::Bold);
}

#[test]
fn test_extract_sequences_multiple() {
    let sequences = extract_sequences("\x1b[1mBold\x1b[0m Normal");
    assert_eq!(sequences.len(), 2);
    assert_eq!(sequences[0].1, AnsiSequence::Bold);
    assert_eq!(sequences[1].1, AnsiSequence::Reset);
}

#[test]
fn test_extract_sequences_positions() {
    let input = "AB\x1b[1mCD\x1b[0mEF";
    let sequences = extract_sequences(input);
    assert_eq!(sequences.len(), 2);
    // "\x1b[1m" starts at byte 2
    assert_eq!(sequences[0].0, 2);
    // "\x1b[0m" starts after "AB" (2) + "\x1b[1m" (4) + "CD" (2) = 8
    assert_eq!(sequences[1].0, 8);
}

// =============================================================================
// Real fixture content tests
// =============================================================================

#[test]
fn test_parse_fixture_line_logo() {
    // Full logo line from fixture
    let input = "\x1b[38;2;215;119;87m ▐\x1b[48;2;0;0;0m▛███▜\x1b[49m▌\x1b[39m   \x1b[1mClaude Code\x1b[0m \x1b[38;2;153;153;153mv2.1.12\x1b[39m";
    let stripped = strip_ansi(input);
    assert_eq!(stripped, " ▐▛███▜▌   Claude Code v2.1.12");

    let spans = parse_ansi(input);
    // Should have spans for: orange logo, black bg, bg reset, fg reset, spaces, bold, reset, gray version, fg reset
    assert!(spans.len() >= 6);
}

#[test]
fn test_parse_fixture_separator() {
    // Separator line from fixture
    let input = "\x1b[2m\x1b[38;2;136;136;136m────────────────────────────────────────\n\x1b[0m";
    let stripped = strip_ansi(input);
    assert!(stripped.starts_with("─"));

    let sequences = extract_sequences(input);
    assert!(sequences.iter().any(|(_, s)| *s == AnsiSequence::Dim));
}

#[test]
fn test_parse_fixture_prompt_line() {
    // Prompt line with inverse for cursor
    let input = "\x1b[0m❯ \x1b[7mT\x1b[0;2mry \"write a test for scenario.rs\"\x1b[0m";
    let stripped = strip_ansi(input);
    assert!(stripped.contains("❯"));
    assert!(stripped.contains("Try"));

    let sequences = extract_sequences(input);
    assert!(sequences.iter().any(|(_, s)| *s == AnsiSequence::Inverse));
}
