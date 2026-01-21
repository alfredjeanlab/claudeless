// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! ANSI escape sequence parser.
//!
//! Parses terminal output containing ANSI escape sequences into structured data.

use regex::Regex;
use std::sync::LazyLock;

/// Regex for matching ANSI SGR (Select Graphic Rendition) escape sequences.
/// Matches ESC [ followed by semicolon-separated numbers, ending with 'm'.
///
/// This is a compile-time constant regex pattern that is guaranteed to be valid,
/// so the unwrap is safe. We use unwrap_or_else to avoid clippy's expect_used warning.
static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // SAFETY: This regex pattern is a compile-time constant and is guaranteed to be valid
    #[allow(clippy::expect_used)]
    Regex::new(r"\x1b\[([0-9;]*)m").expect("ANSI regex pattern is invalid")
});

/// Represents a parsed ANSI escape sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnsiSequence {
    /// 24-bit RGB foreground color: ESC[38;2;R;G;Bm
    FgRgb { r: u8, g: u8, b: u8 },
    /// 24-bit RGB background color: ESC[48;2;R;G;Bm
    BgRgb { r: u8, g: u8, b: u8 },
    /// Reset foreground color: ESC[39m
    FgReset,
    /// Reset background color: ESC[49m
    BgReset,
    /// Reset all attributes: ESC[0m
    Reset,
    /// Bold: ESC[1m
    Bold,
    /// Dim: ESC[2m
    Dim,
    /// Inverse/reverse video: ESC[7m
    Inverse,
    /// Combined reset and dim: ESC[0;2m
    ResetDim,
    /// Other/unknown sequence (preserved as-is for round-tripping)
    Other(String),
}

impl AnsiSequence {
    /// Parse a sequence from its parameter string (the part between `[` and `m`).
    fn from_params(params: &str) -> Self {
        let parts: Vec<&str> = params.split(';').collect();

        match parts.as_slice() {
            // Reset
            ["0"] | [""] => AnsiSequence::Reset,
            // Bold
            ["1"] => AnsiSequence::Bold,
            // Dim
            ["2"] => AnsiSequence::Dim,
            // Inverse
            ["7"] => AnsiSequence::Inverse,
            // Foreground reset
            ["39"] => AnsiSequence::FgReset,
            // Background reset
            ["49"] => AnsiSequence::BgReset,
            // Reset + Dim
            ["0", "2"] => AnsiSequence::ResetDim,
            // 24-bit foreground: 38;2;R;G;B
            ["38", "2", r, g, b] => {
                if let (Ok(r), Ok(g), Ok(b)) = (r.parse(), g.parse(), b.parse()) {
                    AnsiSequence::FgRgb { r, g, b }
                } else {
                    AnsiSequence::Other(params.to_string())
                }
            }
            // 24-bit background: 48;2;R;G;B
            ["48", "2", r, g, b] => {
                if let (Ok(r), Ok(g), Ok(b)) = (r.parse(), g.parse(), b.parse()) {
                    AnsiSequence::BgRgb { r, g, b }
                } else {
                    AnsiSequence::Other(params.to_string())
                }
            }
            // Unknown sequence
            _ => AnsiSequence::Other(params.to_string()),
        }
    }

    /// Convert the sequence back to its escape code representation.
    pub fn to_escape_code(&self) -> String {
        match self {
            AnsiSequence::FgRgb { r, g, b } => format!("\x1b[38;2;{};{};{}m", r, g, b),
            AnsiSequence::BgRgb { r, g, b } => format!("\x1b[48;2;{};{};{}m", r, g, b),
            AnsiSequence::FgReset => "\x1b[39m".to_string(),
            AnsiSequence::BgReset => "\x1b[49m".to_string(),
            AnsiSequence::Reset => "\x1b[0m".to_string(),
            AnsiSequence::Bold => "\x1b[1m".to_string(),
            AnsiSequence::Dim => "\x1b[2m".to_string(),
            AnsiSequence::Inverse => "\x1b[7m".to_string(),
            AnsiSequence::ResetDim => "\x1b[0;2m".to_string(),
            AnsiSequence::Other(params) => format!("\x1b[{}m", params),
        }
    }
}

/// A segment of text with its preceding ANSI sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiSpan {
    /// The text content of this span (may be empty if only sequences).
    pub text: String,
    /// ANSI sequences that precede this text.
    pub sequences: Vec<AnsiSequence>,
}

impl AnsiSpan {
    /// Create a new span with no sequences.
    pub fn plain(text: impl Into<String>) -> Self {
        AnsiSpan {
            text: text.into(),
            sequences: Vec::new(),
        }
    }

    /// Create a new span with sequences.
    pub fn styled(text: impl Into<String>, sequences: Vec<AnsiSequence>) -> Self {
        AnsiSpan {
            text: text.into(),
            sequences,
        }
    }
}

/// Parse a string containing ANSI escape sequences into spans.
///
/// Each span contains the text content and the ANSI sequences that preceded it.
/// This preserves the exact sequence of ANSI codes and text for comparison.
pub fn parse_ansi(input: &str) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let mut current_sequences = Vec::new();
    let mut last_end = 0;

    for cap in ANSI_REGEX.captures_iter(input) {
        // cap.get(0) is the full match, which is always present when the regex matches
        let Some(full_match) = cap.get(0) else {
            continue;
        };
        let params = cap.get(1).map(|m| m.as_str()).unwrap_or("");

        // Text before this escape sequence
        let text_before = &input[last_end..full_match.start()];
        if !text_before.is_empty() {
            spans.push(AnsiSpan {
                text: text_before.to_string(),
                sequences: std::mem::take(&mut current_sequences),
            });
        }

        // Parse the ANSI sequence
        current_sequences.push(AnsiSequence::from_params(params));
        last_end = full_match.end();
    }

    // Remaining text after the last escape sequence
    let remaining = &input[last_end..];
    if !remaining.is_empty() || !current_sequences.is_empty() {
        spans.push(AnsiSpan {
            text: remaining.to_string(),
            sequences: current_sequences,
        });
    }

    // Ensure we return at least one span for empty or plain input
    if spans.is_empty() && !input.is_empty() {
        spans.push(AnsiSpan::plain(input));
    }

    spans
}

/// Strip all ANSI escape sequences, returning plain text.
pub fn strip_ansi(input: &str) -> String {
    ANSI_REGEX.replace_all(input, "").to_string()
}

/// Extract only the ANSI sequences with their byte positions.
///
/// Returns pairs of (position, sequence) where position is the byte offset
/// in the original string where the sequence started.
pub fn extract_sequences(input: &str) -> Vec<(usize, AnsiSequence)> {
    ANSI_REGEX
        .captures_iter(input)
        .filter_map(|cap| {
            // cap.get(0) is the full match, which is always present when the regex matches
            let full_match = cap.get(0)?;
            let params = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            Some((full_match.start(), AnsiSequence::from_params(params)))
        })
        .collect()
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
