// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Width-aware separator generation for TUI rendering.

/// Default separator character (box drawing horizontal)
pub const SEPARATOR_CHAR: char = '─';

/// Compact separator character (double horizontal)
pub const COMPACT_SEPARATOR_CHAR: char = '═';

/// Light dash character for section dividers
pub const SECTION_DIVIDER_CHAR: char = '╌';

/// Generate a full-width separator line.
pub fn make_separator(width: usize) -> String {
    SEPARATOR_CHAR.to_string().repeat(width)
}

/// Generate a compact separator with centered text.
/// Format: "════...════ {text} ════...════"
pub fn make_compact_separator(text: &str, width: usize) -> String {
    let text_with_spaces = format!(" {} ", text);
    let text_len = text_with_spaces.chars().count();

    if width <= text_len {
        return text_with_spaces;
    }

    let remaining = width - text_len;
    let left_count = remaining / 2;
    let right_count = remaining - left_count;

    format!(
        "{}{}{}",
        COMPACT_SEPARATOR_CHAR.to_string().repeat(left_count),
        text_with_spaces,
        COMPACT_SEPARATOR_CHAR.to_string().repeat(right_count)
    )
}

/// Generate a section divider line.
pub fn make_section_divider(width: usize) -> String {
    SECTION_DIVIDER_CHAR.to_string().repeat(width)
}

#[cfg(test)]
#[path = "separator_tests.rs"]
mod tests;
