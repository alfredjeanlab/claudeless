// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Keyboard shortcuts data for the shortcuts panel.
//!
//! When the user presses '?' on empty input, this panel displays
//! all available keyboard shortcuts in a 3-column layout.

/// A keyboard shortcut definition
#[derive(Clone, Debug)]
pub struct Shortcut {
    /// Key combination (e.g., "! for bash mode")
    pub keys: &'static str,
    /// Column position (0 = left, 1 = center, 2 = right)
    pub column: u8,
}

/// All keyboard shortcuts displayed in the panel
/// Organized in 3 columns matching the fixture layout
pub static SHORTCUTS: &[Shortcut] = &[
    // Left column
    Shortcut {
        keys: "! for bash mode",
        column: 0,
    },
    Shortcut {
        keys: "/ for commands",
        column: 0,
    },
    Shortcut {
        keys: "@ for file paths",
        column: 0,
    },
    Shortcut {
        keys: "& for background",
        column: 0,
    },
    // Center column
    Shortcut {
        keys: "double tap esc to clear input",
        column: 1,
    },
    Shortcut {
        keys: "shift + tab to auto-accept edits",
        column: 1,
    },
    Shortcut {
        keys: "ctrl + o for verbose output",
        column: 1,
    },
    Shortcut {
        keys: "ctrl + t to show todos",
        column: 1,
    },
    Shortcut {
        keys: "backslash (\\) + return (\u{23ce}) for",
        column: 1,
    },
    Shortcut {
        keys: "newline",
        column: 1,
    }, // continuation line
    // Right column
    Shortcut {
        keys: "ctrl + _ to undo",
        column: 2,
    },
    Shortcut {
        keys: "ctrl + z to suspend",
        column: 2,
    },
    Shortcut {
        keys: "cmd + v to paste images",
        column: 2,
    },
    Shortcut {
        keys: "meta + p to switch model",
        column: 2,
    },
    Shortcut {
        keys: "ctrl + s to stash prompt",
        column: 2,
    },
];

/// Get shortcuts organized by column
pub fn shortcuts_by_column() -> [Vec<&'static str>; 3] {
    let mut columns: [Vec<&'static str>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for shortcut in SHORTCUTS {
        columns[shortcut.column as usize].push(shortcut.keys);
    }
    columns
}

#[cfg(test)]
#[path = "shortcuts_tests.rs"]
mod tests;
