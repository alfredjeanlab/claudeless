// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI color definitions and styled text helpers matching real Claude CLI.
//!
//! Colors extracted from fixtures captured from Claude Code v2.1.12.

/// Orange for logo characters: RGB(215, 119, 87)
pub const LOGO_FG: (u8, u8, u8) = (215, 119, 87);

/// Black for logo background: RGB(0, 0, 0)
pub const LOGO_BG: (u8, u8, u8) = (0, 0, 0);

/// Gray for version, model, path, shortcuts: RGB(153, 153, 153)
pub const TEXT_GRAY: (u8, u8, u8) = (153, 153, 153);

/// Dark gray for separator lines: RGB(136, 136, 136)
pub const SEPARATOR_GRAY: (u8, u8, u8) = (136, 136, 136);

/// Teal for plan mode indicator: RGB(72, 150, 140)
pub const PLAN_MODE_COLOR: (u8, u8, u8) = (72, 150, 140);

/// Purple for accept edits mode indicator: RGB(175, 135, 255)
pub const ACCEPT_EDITS_COLOR: (u8, u8, u8) = (175, 135, 255);

/// Red/pink for bypass permissions mode indicator: RGB(255, 107, 128)
pub const BYPASS_PERMISSIONS_COLOR: (u8, u8, u8) = (255, 107, 128);

/// ANSI escape sequence helpers
mod ansi {
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
    let fg_orange = ansi::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let bg_black = ansi::bg(LOGO_BG.0, LOGO_BG.1, LOGO_BG.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{fg_orange} ▐{bg_black}▛███▜{bg_reset}▌{fg_reset}   {bold}{product_name}{reset} {fg_gray}{version}{fg_reset}",
        fg_orange = fg_orange,
        bg_black = bg_black,
        bg_reset = ansi::BG_RESET,
        fg_reset = ansi::FG_RESET,
        bold = ansi::BOLD,
        reset = ansi::RESET,
        fg_gray = fg_gray,
    )
}

/// Format logo line 2 with proper colors.
///
/// Example output:
/// `[orange]▝▜[black bg]█████[/bg]▛▘[/fg]  [gray]Haiku 4.5 · Claude Max[/gray]`
pub fn styled_logo_line2(model_str: &str) -> String {
    let fg_orange = ansi::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let bg_black = ansi::bg(LOGO_BG.0, LOGO_BG.1, LOGO_BG.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{fg_orange}▝▜{bg_black}█████{bg_reset}▛▘{fg_reset}  {fg_gray}{model_str}{fg_reset}",
        fg_orange = fg_orange,
        bg_black = bg_black,
        bg_reset = ansi::BG_RESET,
        fg_reset = ansi::FG_RESET,
        fg_gray = fg_gray,
    )
}

/// Format logo line 3 with proper colors.
///
/// Example output:
/// `[orange]  ▘▘ ▝▝  [/fg]  [gray]~/Developer/claudeless[/gray]`
pub fn styled_logo_line3(path_str: &str) -> String {
    let fg_orange = ansi::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{fg_orange}  ▘▘ ▝▝  {fg_reset}  {fg_gray}{path_str}{fg_reset}",
        fg_orange = fg_orange,
        fg_reset = ansi::FG_RESET,
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
    let fg_gray = ansi::fg(SEPARATOR_GRAY.0, SEPARATOR_GRAY.1, SEPARATOR_GRAY.2);

    format!(
        "{dim}{fg_gray}{line}",
        dim = ansi::DIM,
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
        reset = ansi::RESET,
        inv = ansi::INVERSE,
        first = first_char,
        reset_dim = ansi::RESET_DIM,
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
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_gray}{text}{fg_reset}",
        reset = ansi::RESET,
        fg_reset = ansi::FG_RESET,
    )
}

/// Format plan mode status bar with teal icon and gray cycle hint.
///
/// Example output:
/// `[reset]  [teal]⏸ plan mode on[gray] (shift+tab to cycle)[/fg]`
pub fn styled_plan_mode_status() -> String {
    let fg_teal = ansi::fg(PLAN_MODE_COLOR.0, PLAN_MODE_COLOR.1, PLAN_MODE_COLOR.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_teal}⏸ plan mode on{fg_gray} (shift+tab to cycle){fg_reset}",
        reset = ansi::RESET,
        fg_teal = fg_teal,
        fg_gray = fg_gray,
        fg_reset = ansi::FG_RESET,
    )
}

/// Format accept edits mode status bar with purple icon and gray cycle hint.
///
/// Example output:
/// `[reset]  [purple]⏵⏵ accept edits on[gray] (shift+tab to cycle)[/fg]`
pub fn styled_accept_edits_status() -> String {
    let fg_purple = ansi::fg(
        ACCEPT_EDITS_COLOR.0,
        ACCEPT_EDITS_COLOR.1,
        ACCEPT_EDITS_COLOR.2,
    );
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_purple}⏵⏵ accept edits on{fg_gray} (shift+tab to cycle){fg_reset}",
        reset = ansi::RESET,
        fg_purple = fg_purple,
        fg_gray = fg_gray,
        fg_reset = ansi::FG_RESET,
    )
}

/// Format bypass permissions mode status bar with red/pink icon and gray cycle hint.
///
/// Example output:
/// `[reset]  [red]⏵⏵ bypass permissions on[gray] (shift+tab to cycle)[/fg]`
pub fn styled_bypass_permissions_status() -> String {
    let fg_red = ansi::fg(
        BYPASS_PERMISSIONS_COLOR.0,
        BYPASS_PERMISSIONS_COLOR.1,
        BYPASS_PERMISSIONS_COLOR.2,
    );
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_red}⏵⏵ bypass permissions on{fg_gray} (shift+tab to cycle){fg_reset}",
        reset = ansi::RESET,
        fg_red = fg_red,
        fg_gray = fg_gray,
        fg_reset = ansi::FG_RESET,
    )
}

#[cfg(test)]
#[path = "colors_tests.rs"]
mod tests;
