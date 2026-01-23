// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI color definitions and styled text helpers matching real Claude CLI.
//!
//! Colors extracted from fixtures captured from Claude Code v2.1.12.

use crate::permission::PermissionMode;

/// Orange for logo characters: RGB(215, 119, 87)
pub const LOGO_FG: (u8, u8, u8) = (215, 119, 87);

/// Black for logo background: RGB(0, 0, 0)
pub const LOGO_BG: (u8, u8, u8) = (0, 0, 0);

/// Gray for version, model, path, shortcuts: RGB(153, 153, 153)
pub const TEXT_GRAY: (u8, u8, u8) = (153, 153, 153);

/// Dark gray for separator lines: RGB(136, 136, 136)
pub const SEPARATOR_GRAY: (u8, u8, u8) = (136, 136, 136);

// Permission mode colors (from v2.1.15 fixtures)
/// Teal for plan mode: RGB(72, 150, 140)
pub const PLAN_MODE: (u8, u8, u8) = (72, 150, 140);
/// Purple for accept edits mode: RGB(175, 135, 255)
pub const ACCEPT_EDITS_MODE: (u8, u8, u8) = (175, 135, 255);
/// Red/Pink for bypass permissions mode: RGB(255, 107, 128)
pub const BYPASS_MODE: (u8, u8, u8) = (255, 107, 128);

/// ANSI escape sequence helpers (public for reuse)
pub mod escape {
    /// 24-bit foreground color
    pub fn fg(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }

    /// 24-bit background color
    pub fn bg(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    }

    /// Reset foreground color
    pub const FG_RESET: &str = "\x1b[39m";

    /// Reset background color
    pub const BG_RESET: &str = "\x1b[49m";

    /// Reset all attributes
    pub const RESET: &str = "\x1b[0m";

    /// Bold
    pub const BOLD: &str = "\x1b[1m";

    /// Dim
    pub const DIM: &str = "\x1b[2m";

    /// Inverse/reverse video
    pub const INVERSE: &str = "\x1b[7m";

    /// Reset + dim combined
    pub const RESET_DIM: &str = "\x1b[0;2m";
}

/// Format logo line 1 with proper colors.
///
/// Example output:
/// `[orange] ▐[black bg]▛███▜[/bg]▌[/fg]   [bold]Claude Code[/bold] [gray]v2.1.12[/gray]`
pub fn styled_logo_line1(product_name: &str, version: &str) -> String {
    let fg_orange = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let bg_black = escape::bg(LOGO_BG.0, LOGO_BG.1, LOGO_BG.2);
    let fg_gray = escape::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{fg_orange} ▐{bg_black}▛███▜{bg_reset}▌{fg_reset}   {bold}{product_name}{reset} {fg_gray}{version}{fg_reset}",
        fg_orange = fg_orange,
        bg_black = bg_black,
        bg_reset = escape::BG_RESET,
        fg_reset = escape::FG_RESET,
        bold = escape::BOLD,
        reset = escape::RESET,
        fg_gray = fg_gray,
    )
}

/// Format logo line 2 with proper colors.
///
/// Example output:
/// `[orange]▝▜[black bg]█████[/bg]▛▘[/fg]  [gray]Haiku 4.5 · Claude Max[/gray]`
pub fn styled_logo_line2(model_str: &str) -> String {
    let fg_orange = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let bg_black = escape::bg(LOGO_BG.0, LOGO_BG.1, LOGO_BG.2);
    let fg_gray = escape::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{fg_orange}▝▜{bg_black}█████{bg_reset}▛▘{fg_reset}  {fg_gray}{model_str}{fg_reset}",
        fg_orange = fg_orange,
        bg_black = bg_black,
        bg_reset = escape::BG_RESET,
        fg_reset = escape::FG_RESET,
        fg_gray = fg_gray,
    )
}

/// Format logo line 3 with proper colors.
///
/// Example output:
/// `[orange]  ▘▘ ▝▝  [/fg]  [gray]~/Developer/claudeless[/gray]`
pub fn styled_logo_line3(path_str: &str) -> String {
    let fg_orange = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let fg_gray = escape::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{fg_orange}  ▘▘ ▝▝  {fg_reset}  {fg_gray}{path_str}{fg_reset}",
        fg_orange = fg_orange,
        fg_reset = escape::FG_RESET,
        fg_gray = fg_gray,
    )
}

/// Format a styled separator line (dim + dark gray).
///
/// The separator does NOT include a reset - the next content line should start with [0m].
///
/// Example output:
/// `[dim][dark gray]────────...`
pub fn styled_separator(width: usize) -> String {
    let fg_gray = escape::fg(SEPARATOR_GRAY.0, SEPARATOR_GRAY.1, SEPARATOR_GRAY.2);

    format!(
        "{dim}{fg_gray}{line}",
        dim = escape::DIM,
        fg_gray = fg_gray,
        line = "─".repeat(width),
    )
}

/// Format the placeholder prompt with proper styling.
///
/// Starts with [0m] to reset from the separator's dim/gray.
/// The "T" in "Try" gets inverse video, the rest is dim.
///
/// Example output:
/// `[reset]❯ [inverse]T[reset+dim]ry "write a test for scenario.rs"[reset]`
pub fn styled_placeholder(text: &str) -> String {
    // Text should be something like: Try "refactor mod.rs"
    // We need to make the first character inverse and the rest dim
    let first_char = text.chars().next().unwrap_or('T');
    let rest = &text[first_char.len_utf8()..];

    format!(
        "{reset}❯ {inv}{first}{reset_dim}{rest}{reset}",
        reset = escape::RESET,
        inv = escape::INVERSE,
        first = first_char,
        reset_dim = escape::RESET_DIM,
        rest = rest,
    )
}

/// Format styled gray text (for status bar shortcuts).
///
/// Starts with [0m] to reset from the separator's dim/gray.
///
/// Example output:
/// `[reset]  [gray]? for shortcuts[/gray]`
pub fn styled_status_text(text: &str) -> String {
    let fg_gray = escape::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_gray}{text}{fg_reset}",
        reset = escape::RESET,
        fg_reset = escape::FG_RESET,
    )
}

/// Generate styled permission status text with ANSI colors.
///
/// Format for default mode:
/// `[reset]  [gray]? for shortcuts[fg_reset]`
///
/// Format for non-default modes:
/// `[reset]  [mode_color][icon] [mode_text][gray] (shift+tab to cycle)[fg_reset]`
pub fn styled_permission_status(mode: &PermissionMode) -> String {
    let fg_gray = escape::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    match mode {
        PermissionMode::Default => styled_status_text("? for shortcuts"),
        PermissionMode::Plan => {
            let (r, g, b) = PLAN_MODE;
            format!(
                "{}  {}⏸ plan mode on{} (shift+tab to cycle){}",
                escape::RESET,
                escape::fg(r, g, b),
                fg_gray,
                escape::FG_RESET
            )
        }
        PermissionMode::AcceptEdits => {
            let (r, g, b) = ACCEPT_EDITS_MODE;
            format!(
                "{}  {}⏵⏵ accept edits on{} (shift+tab to cycle){}",
                escape::RESET,
                escape::fg(r, g, b),
                fg_gray,
                escape::FG_RESET
            )
        }
        PermissionMode::BypassPermissions => {
            let (r, g, b) = BYPASS_MODE;
            format!(
                "{}  {}⏵⏵ bypass permissions on{} (shift+tab to cycle){}",
                escape::RESET,
                escape::fg(r, g, b),
                fg_gray,
                escape::FG_RESET
            )
        }
        // Delegate and DontAsk modes use gray (same as default cycle hint)
        PermissionMode::Delegate => {
            format!(
                "{}  {}delegate mode (shift+tab to cycle){}",
                escape::RESET,
                fg_gray,
                escape::FG_RESET
            )
        }
        PermissionMode::DontAsk => {
            format!(
                "{}  {}don't ask mode (shift+tab to cycle){}",
                escape::RESET,
                fg_gray,
                escape::FG_RESET
            )
        }
    }
}

#[cfg(test)]
#[path = "colors_tests.rs"]
mod tests;
