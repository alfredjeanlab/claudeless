// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Help dialog rendering and key handling, plus Ink simulation helpers.

use iocraft::prelude::*;
use unicode_width::UnicodeWidthStr;

use crate::tui::slash_menu::COMMANDS;
use crate::tui::widgets::help::{HelpDialog, HelpTab};

use crate::tui::app::state::TuiAppState;

// ── Ink simulation helpers ───────────────────────────────────────────────

/// Lay out help dialog content lines simulating real Claude's Ink rendering.
///
/// When the terminal is wide enough for all content, returns clean lines.
/// When content overflows the terminal width, replicates Ink's rendering
/// artifacts where the description wraps and shortcut columns are written
/// at fixed positions, leaving wrapped description text visible in padding gaps.
fn layout_help_general_lines(
    description: &str,
    shortcuts: &[(&str, &str, &str)],
    width: usize,
) -> Vec<String> {
    // Column layout: matches real Claude's 3-column help dialog
    // Col 1 starts at position 2, width 20
    // Col 2 starts at position 24 (2 + 20 + 2), width 30
    // Col 3 starts at position 56 (24 + 30 + 2)
    const COL1_START: usize = 2;
    const COL2_START: usize = 24;
    const COL3_START: usize = 56;

    // Check if everything fits without overflow
    let desc_fits = description.width() <= width;
    let shortcuts_fit = shortcuts
        .iter()
        .all(|(_, _, r)| COL3_START + r.width() <= width);

    if desc_fits && shortcuts_fit {
        let mut lines = vec![description.to_string()];
        for (l, m, r) in shortcuts {
            lines.push(format!("  {:<20}  {:<30}  {}", l, m, r));
        }
        return lines;
    }

    // Content overflows: simulate Ink's column-based rendering.
    //
    // Real Claude uses Ink (React-based terminal renderer) which does differential
    // updates. When the help dialog renders over the initial shortcuts panel:
    // 1. Each column's content is written at its fixed position
    // 2. Gaps between columns are NOT updated — previous screen content shows through
    // 3. Col3 text that exceeds available width is word-wrapped within the column
    // 4. Col3 continuation lines go to subsequent rows at the same column position
    //
    // This produces the garbled appearance in the fixture where description overflow
    // and initial shortcuts content are visible in the gaps between columns.

    let col3_width = width.saturating_sub(COL3_START);
    let total_rows = 2 + shortcuts.len() * 2 + 4;
    let mut screen: Vec<Vec<char>> = vec![vec![' '; width]; total_rows];

    // Step 1: Write description with word-wrapping (base layer)
    write_word_wrapped(&mut screen, 0, description, width);

    // Step 2: Write each shortcut column at its fixed position on the screen.
    // Ink positions shortcuts starting at row 1 (thinks description = 1 row).
    for (i, (left, mid, right)) in shortcuts.iter().enumerate() {
        let row = 1 + i;
        if row >= total_rows {
            break;
        }
        // Write column 1 content at COL1_START
        write_text_at(&mut screen, row, COL1_START, left, width);
        // Write column 2 content at COL2_START
        write_text_at(&mut screen, row, COL2_START, mid, width);
        // Write column 3 content with word-wrapping at available width
        let col3_lines = word_wrap_text(right, col3_width);
        for (j, line) in col3_lines.iter().enumerate() {
            write_text_at(&mut screen, row + j, COL3_START, line, width);
        }
    }

    // Step 3: Fill in bleed-through from initial shortcuts panel.
    //
    // Real Claude uses Ink (differential renderer). When the help dialog
    // renders over the initial shortcuts panel, Ink only writes cells
    // that belong to dialog components. Cells in the gap between a
    // shortcut row's col2 text end and col3 start retain the previous
    // shortcuts panel content. We simulate this by copying the gap
    // content from the initial shortcuts background for each shortcut row.
    {
        let initial_lines = compute_initial_shortcuts_background(width);
        for (i, (_left, mid, _right)) in shortcuts.iter().enumerate() {
            let row = 1 + i;
            if row >= total_rows {
                break;
            }
            // Gap starts after col2 text ends
            let col2_text_end = COL2_START + mid.width();
            if col2_text_end >= COL3_START {
                continue; // No gap
            }
            // Copy gap content from the corresponding initial shortcuts line,
            // but only if multiple non-space characters bleed through.
            // Single boundary characters (e.g., "s" at the exact end of the
            // initial text) are absorbed by Ink's column padding; only longer
            // runs like "utput" from "verbose output" are visible.
            if let Some(initial_line) = initial_lines.get(row) {
                let initial_chars: Vec<char> = initial_line.chars().collect();
                // Count non-space characters in the gap region
                let gap_nonspace: Vec<(usize, char)> = (col2_text_end..COL3_START)
                    .filter_map(|col| {
                        initial_chars
                            .get(col)
                            .filter(|ch| **ch != ' ')
                            .map(|&ch| (col, ch))
                    })
                    .collect();
                if gap_nonspace.len() >= 2 {
                    for (col, ch) in gap_nonspace {
                        if col < width {
                            screen[row][col] = ch;
                        }
                    }
                }
            }
        }
    }

    // Find the last non-empty row
    let last_row = screen
        .iter()
        .rposition(|row| row.iter().any(|&c| c != ' '))
        .unwrap_or(0);

    (0..=last_row)
        .map(|i| screen[i].iter().collect::<String>().trim_end().to_string())
        .collect()
}

/// Word-wrap text at a given width, breaking at word boundaries.
fn word_wrap_text(text: &str, width: usize) -> Vec<&str> {
    if text.width() <= width {
        return vec![text];
    }
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut last_space = None;
    let mut col = 0;
    for (i, ch) in text.char_indices() {
        let ch_w = unicode_width::UnicodeWidthChar::width(ch)
            .unwrap_or(0)
            .max(1);
        if ch == ' ' {
            last_space = Some(i);
        }
        if col + ch_w > width {
            // Need to break
            if let Some(sp) = last_space {
                lines.push(&text[line_start..sp]);
                line_start = sp + 1; // skip the space
                                     // Recompute col from line_start to current position
                col = text[line_start..i].width() + ch_w;
            } else {
                // No space to break at — hard break
                lines.push(&text[line_start..i]);
                line_start = i;
                col = ch_w;
            }
            last_space = None;
        } else {
            col += ch_w;
        }
    }
    if line_start < text.len() {
        lines.push(&text[line_start..]);
    }
    lines
}

/// Compute the initial shortcuts panel lines that would be visible behind
/// the help dialog. Real Claude's Ink renderer does differential updates,
/// so this content shows through in the gaps between dialog columns.
///
/// Uses the help dialog's own column positions (col1=2, col2=24, col3=56)
/// rather than the shortcuts panel's positions, because Ink's layout engine
/// computes positions relative to the dialog's flex container.
fn compute_initial_shortcuts_background(width: usize) -> Vec<String> {
    use crate::tui::shortcuts::shortcuts_by_column;

    // Use the help dialog's column widths so gap content aligns with
    // the positions that the dialog's columns don't overwrite.
    const LEFT_WIDTH: usize = 22;
    const CENTER_WIDTH: usize = 32;

    let columns = shortcuts_by_column();
    let max_rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);

    let mut lines = Vec::new();
    for row_idx in 0..max_rows {
        let left = columns[0].get(row_idx).copied().unwrap_or("");
        let center = columns[1].get(row_idx).copied().unwrap_or("");
        let right = columns[2].get(row_idx).copied().unwrap_or("");

        let line = format!(
            "  {:<left_w$}{:<center_w$}{}",
            left,
            center,
            right,
            left_w = LEFT_WIDTH,
            center_w = CENTER_WIDTH,
        );
        // Truncate to width
        let truncated: String = line.chars().take(width).collect();
        lines.push(truncated);
    }
    lines
}

/// Write text to a specific position in the screen buffer (full overwrite including spaces).
fn write_text_at(screen: &mut [Vec<char>], row: usize, start_col: usize, text: &str, width: usize) {
    if row >= screen.len() {
        return;
    }
    let mut col = start_col;
    for ch in text.chars() {
        if col >= width {
            break;
        }
        screen[row][col] = ch;
        col += unicode_width::UnicodeWidthChar::width(ch)
            .unwrap_or(0)
            .max(1);
    }
}

/// Write text to screen buffer with word-wrapping. Returns number of rows used.
/// Continuation lines are indented to match the leading whitespace of the text.
fn write_word_wrapped(
    screen: &mut [Vec<char>],
    start_row: usize,
    text: &str,
    width: usize,
) -> usize {
    // Determine leading indent (spaces at start of text)
    let indent: usize = text.chars().take_while(|c| *c == ' ').count();

    let mut row = start_row;
    let mut col = 0;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if row >= screen.len() {
            break;
        }

        if chars[i] == ' ' {
            if col < width {
                screen[row][col] = ' ';
                col += 1;
            }
            i += 1;
        } else {
            // Word: collect chars until next space
            let word_start = i;
            let mut word_width = 0;
            while i < chars.len() && chars[i] != ' ' {
                word_width += unicode_width::UnicodeWidthChar::width(chars[i])
                    .unwrap_or(0)
                    .max(1);
                i += 1;
            }
            let word = &chars[word_start..i];

            // Check if word fits on current line
            if col > 0 && col + word_width > width {
                // Word doesn't fit — wrap to next line with indent
                row += 1;
                col = 0;
                if row >= screen.len() {
                    break;
                }
                // Write indent on continuation line
                for _ in 0..indent.min(width) {
                    screen[row][col] = ' ';
                    col += 1;
                }
            }

            // Write word
            for &ch in word {
                if col < width && row < screen.len() {
                    screen[row][col] = ch;
                    col += unicode_width::UnicodeWidthChar::width(ch)
                        .unwrap_or(0)
                        .max(1);
                }
            }
        }
    }

    row - start_row + 1
}

// ── Help dialog ──────────────────────────────────────────────────────────

/// Render help dialog
pub(crate) fn render_help_dialog(dialog: &HelpDialog, width: usize) -> AnyElement<'static> {
    let inner_width = width.saturating_sub(2);

    // Build tab header line
    // Format: ──Claude Code v{ver} {tab1}  {tab2} ─ {tab3} (hint)─
    // Active tab has no ─ separator; inactive tabs separated by ─.
    let version_part = format!("Claude Code v{}", dialog.version);
    let tabs = [HelpTab::General, HelpTab::Commands, HelpTab::CustomCommands];
    let mut tabs_part = String::new();
    let mut prev_was_active = false;
    for (i, tab) in tabs.iter().enumerate() {
        if *tab == dialog.active_tab {
            tabs_part.push_str(&format!("  {}  ", tab.name()));
            prev_was_active = true;
        } else {
            if i > 0 && !prev_was_active {
                tabs_part.push_str(" ─");
            }
            tabs_part.push_str(&format!(" {}", tab.name()));
            prev_was_active = false;
        }
    }
    let hint = " (←/→ or tab to cycle)";
    let prefix = format!("──{}{}{}", version_part, tabs_part, hint);
    let remaining = inner_width.saturating_sub(prefix.len());
    let tab_header = format!("{}{}", prefix, "─".repeat(remaining));

    let footer = " For more help: https://code.claude.com/docs/en/overview";

    match dialog.active_tab {
        HelpTab::General => {
            // 3-column shortcuts table matching real Claude layout
            // Col 1: positions 2-23 (22 chars), Col 2: 24-55 (32 chars), Col 3: 56+
            let shortcuts: &[(&str, &str, &str)] = &[
                (
                    "! for bash mode",
                    "double tap esc to clear input",
                    "ctrl + z to suspend",
                ),
                (
                    "/ for commands",
                    "shift + tab to auto-accept",
                    "meta + p to switch models",
                ),
                (
                    "& for background",
                    "ctrl + t to show todos",
                    "ctrl + g to edit in $EDITOR",
                ),
                ("", "shift + ⏎ for newline", "/keybindings to customize"),
            ];
            let description = "  Claude understands your codebase, makes edits with your permission, and executes commands — right from your terminal.";

            // Simulate real Claude's Ink rendering: word-wrap description,
            // then overlay shortcuts (spaces transparent) to match the fixture.
            let content_lines = layout_help_general_lines(description, shortcuts, width);

            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header, wrap: TextWrap::NoWrap)
                    Text(content: "")
                    Text(content: "")
                    #(content_lines.into_iter().map(|line| {
                        element! { Text(content: line, wrap: TextWrap::NoWrap) }
                    }).collect::<Vec<_>>())
                    Text(content: "")
                    Text(content: footer, wrap: TextWrap::NoWrap)
                    Text(content: " Esc to cancel")
                }
            }
            .into()
        }
        HelpTab::Commands => {
            let selected = dialog.commands_selected;
            let cmd = COMMANDS.get(selected);
            let next_cmd = COMMANDS.get(selected + 1);

            let selected_line = format!("  ❯ /{}", cmd.map(|c| c.name).unwrap_or(""));
            let description_line = format!("    {}", cmd.map(|c| c.description).unwrap_or(""));
            let next_line = if let Some(next) = next_cmd {
                format!("    /{}", next.name)
            } else {
                String::new()
            };
            let next_description = if let Some(next) = next_cmd {
                format!("    {}", next.description)
            } else {
                String::new()
            };

            element! {
                View(flex_direction: FlexDirection::Column, width: 100pct) {
                    Text(content: tab_header)
                    Text(content: "")
                    Text(content: "")
                    Text(content: "  Browse default commands:")
                    Text(content: selected_line)
                    Text(content: description_line)
                    Text(content: next_line)
                    Text(content: next_description)
                    Text(content: footer)
                }
            }
            .into()
        }
        HelpTab::CustomCommands => element! {
            View(flex_direction: FlexDirection::Column, width: 100pct) {
                Text(content: tab_header)
                Text(content: "")
                Text(content: "  Browse custom commands:")
                Text(content: "  (no custom commands configured)")
                Text(content: "")
                Text(content: footer)
            }
        }
        .into(),
    }
}

impl TuiAppState {
    /// Handle key events in help dialog mode
    pub(in crate::tui::app) fn handle_help_dialog_key(&self, key: KeyEvent) {
        let mut inner = self.inner.lock();

        let Some(dialog) = inner.dialog.as_help_mut() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                inner.dismiss_dialog("Help dialog");
            }
            KeyCode::Tab | KeyCode::Right => dialog.next_tab(),
            KeyCode::Left | KeyCode::BackTab => dialog.prev_tab(),
            KeyCode::Up => dialog.select_prev(COMMANDS.len()),
            KeyCode::Down => dialog.select_next(COMMANDS.len()),
            _ => {}
        }
    }
}
