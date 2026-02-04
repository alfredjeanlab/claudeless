// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Header, status bar, and model name formatting functions.

use crate::permission::PermissionMode;
use crate::tui::colors::{
    styled_logo_line1, styled_logo_line2, styled_logo_line3, styled_permission_status,
};

use crate::tui::app::types::{AppMode, ExitHint, RenderState};

/// Format header lines with Claude branding (returns 3 lines)
pub(crate) fn format_header_lines(state: &RenderState) -> (String, String, String) {
    let model_name = model_display_name(&state.status.model);

    // Get working directory display (shortened if possible)
    let working_dir = std::env::current_dir()
        .map(|p| {
            // Try to convert to ~ format using HOME env var
            if let Some(home_path) = crate::env::home() {
                if let Ok(stripped) = p.strip_prefix(&home_path) {
                    return format!("~/{}", stripped.display());
                }
            }
            p.display().to_string()
        })
        .unwrap_or_else(|_| "~".to_string());

    // Determine product name and version based on claude_version
    let (product_name, version) = match &state.claude_version {
        Some(v) => ("Claude Code", format!("v{}", v)),
        None => ("Claudeless", env!("CARGO_PKG_VERSION").to_string()),
    };
    let provider = state.provider.as_deref().unwrap_or("Claude Max");
    let model_str = format!("{} · {}", model_name, provider);

    // Use styled ANSI output when connected to a TTY
    if state.is_tty {
        (
            styled_logo_line1(product_name, &version),
            styled_logo_line2(&model_str),
            styled_logo_line3(&working_dir),
        )
    } else {
        let line1 = format!(" ▐▛███▜▌   {} {}", product_name, version);
        let line2 = format!("▝▜█████▛▘  {}", model_str);
        let line3 = format!("  ▘▘ ▝▝    {}", working_dir);
        (line1, line2, line3)
    }
}

/// Format status bar content
pub(crate) fn format_status_bar(state: &RenderState, width: usize) -> String {
    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.display.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
            ExitHint::Escape => {
                let text = "Esc to clear again";
                let pad_width = width.saturating_sub(2);
                format!("{:>width$}", text, width = pad_width)
            }
        };
    }

    // During compacting, show "esc to interrupt" in status bar
    if state.is_compacting {
        return "  esc to interrupt".to_string();
    }

    // During streaming (Responding/Thinking) or when the user has typed input,
    // hide the shortcuts hint to match real Claude behavior
    if matches!(state.mode, AppMode::Responding | AppMode::Thinking)
        || !state.input.buffer.is_empty()
    {
        return String::new();
    }

    // Status bar format matches real Claude CLI
    let mode_text = match &state.permission_mode {
        PermissionMode::Default => "  ? for shortcuts".to_string(),
        PermissionMode::Plan => "  ⏸ plan mode on (shift+tab to cycle)".to_string(),
        PermissionMode::AcceptEdits => "  ⏵⏵ accept edits on (shift+tab to cycle)".to_string(),
        PermissionMode::BypassPermissions => {
            "  ⏵⏵ bypass permissions on (shift+tab to cycle)".to_string()
        }
        PermissionMode::Delegate => "  delegate mode (shift+tab to cycle)".to_string(),
        PermissionMode::DontAsk => "  don't ask mode (shift+tab to cycle)".to_string(),
    };

    // For non-default modes, show "Use meta+t to toggle thinking" on the right
    // For default mode, just show the shortcuts hint (or "Thinking off" if disabled)
    // Plan mode just shows the mode text without right-side text
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                mode_text
            } else {
                // Pad to align "Thinking off" to the right side
                let effective_width = width.saturating_sub(2);
                let padding =
                    effective_width.saturating_sub(mode_text.len() + "Thinking off".len());
                format!("{}{:width$}Thinking off", mode_text, "", width = padding)
            }
        }
        PermissionMode::Plan => mode_text,
        _ => mode_text,
    }
}

/// Format styled status bar content (with ANSI colors)
pub(crate) fn format_status_bar_styled(state: &RenderState, width: usize) -> String {
    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.display.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
            ExitHint::Escape => {
                let text = "Esc to clear again";
                let pad_width = width.saturating_sub(2);
                format!("{:>width$}", text, width = pad_width)
            }
        };
    }

    // During compacting, show "esc to interrupt" in status bar
    if state.is_compacting {
        return "  esc to interrupt".to_string();
    }

    // During streaming (Responding/Thinking) or when the user has typed input,
    // hide the shortcuts hint to match real Claude behavior
    if matches!(state.mode, AppMode::Responding | AppMode::Thinking)
        || !state.input.buffer.is_empty()
    {
        return String::new();
    }

    // Get styled permission status
    let status = styled_permission_status(&state.permission_mode);

    // Calculate visual width of the status text (excluding ANSI sequences)
    let mode_visual_width = match &state.permission_mode {
        PermissionMode::Default => "  ? for shortcuts".chars().count(),
        PermissionMode::Plan => "  ⏸ plan mode on (shift+tab to cycle)".chars().count(),
        PermissionMode::AcceptEdits => "  ⏵⏵ accept edits on (shift+tab to cycle)".chars().count(),
        PermissionMode::BypassPermissions => "  ⏵⏵ bypass permissions on (shift+tab to cycle)"
            .chars()
            .count(),
        PermissionMode::Delegate => "  delegate mode (shift+tab to cycle)".chars().count(),
        PermissionMode::DontAsk => "  don't ask mode (shift+tab to cycle)".chars().count(),
    };

    // Add right-aligned text based on mode
    // Plan mode just shows the mode text without right-side text
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                status
            } else {
                // Show "Thinking off" aligned to the right
                let right = "Thinking off";
                let effective_width = width.saturating_sub(2);
                let padding = effective_width.saturating_sub(mode_visual_width + right.len());
                format!("{}{:width$}{}", status, "", right, width = padding)
            }
        }
        PermissionMode::Plan => status,
        _ => status,
    }
}

/// Map model ID to display name
pub(crate) fn model_display_name(model: &str) -> String {
    let model_lower = model.to_lowercase();

    // Short aliases default to current version (4.5)
    match model_lower.as_str() {
        "haiku" | "claude-haiku" => return "Haiku 4.5".to_string(),
        "sonnet" | "claude-sonnet" => return "Sonnet 4.5".to_string(),
        "opus" | "claude-opus" => return "Opus 4.5".to_string(),
        _ => {}
    }

    // Parse full model ID like "claude-sonnet-4-20250514"
    let base_name = if model_lower.contains("haiku") {
        "Haiku"
    } else if model_lower.contains("opus") {
        "Opus"
    } else if model_lower.contains("sonnet") {
        "Sonnet"
    } else {
        // Unknown model, show as-is
        return model.to_string();
    };

    // Extract version if present (e.g., "4.5" from "claude-opus-4-5-...")
    let version = extract_model_version(model);

    match version {
        Some(v) => format!("{} {}", base_name, v),
        None => base_name.to_string(),
    }
}

/// Format the "Welcome back!" box (returns one line per row of the box)
pub(crate) fn format_welcome_back_box(state: &RenderState, width: usize) -> Vec<String> {
    use crate::tui::colors::{escape, LOGO_BG, LOGO_FG, SEPARATOR_GRAY, TEXT_GRAY};

    let is_tty = state.is_tty;
    let total_inner = width.saturating_sub(2);
    let right_panel_width: usize = 25;
    let divider_width: usize = 1;
    let left_panel_width = total_inner.saturating_sub(divider_width + right_panel_width);

    let model_name = model_display_name(&state.status.model);
    let provider = state.provider.as_deref().unwrap_or("Claude Max");
    let model_str = format!("{} · {}", model_name, provider);

    let (product_name, version) = match &state.claude_version {
        Some(v) => ("Claude Code", format!("v{}", v)),
        None => ("Claudeless", env!("CARGO_PKG_VERSION").to_string()),
    };

    // Working directory
    let working_dir = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~".to_string());

    // Max path length: left panel minus 2 padding chars
    let max_path_len = left_panel_width.saturating_sub(2);
    let path_display = truncate_path(&working_dir, max_path_len);

    // Helper: center text in width
    let center = |text: &str, w: usize| -> String {
        let text_len = text.chars().count();
        if text_len >= w {
            return text.chars().take(w).collect();
        }
        let total_pad = w - text_len;
        let pad_right = total_pad / 2;
        let pad_left = total_pad - pad_right;
        format!("{}{}{}", " ".repeat(pad_left), text, " ".repeat(pad_right))
    };

    // Helper: left-align text with 1-char left padding, fill to width
    let left_align = |text: &str, w: usize| -> String {
        let content = format!(" {}", text);
        let text_len = content.chars().count();
        if text_len >= w {
            return content.chars().take(w).collect();
        }
        format!("{}{}", content, " ".repeat(w - text_len))
    };

    // Helper: right panel content with 1-char left padding, fill to width
    let right_content = |text: &str, w: usize| -> String {
        let content = format!(" {}", text);
        let text_len = content.chars().count();
        if text_len >= w {
            return content.chars().take(w).collect();
        }
        format!("{}{}", content, " ".repeat(w - text_len))
    };

    // Build right panel entries (typed for styling in TTY mode)
    #[derive(Clone)]
    enum RightPanelEntry {
        Header(String),
        Content(String),
        Separator,
        Empty,
    }

    let panel_source: Vec<String> = if let Some(ref panel) = state.welcome_back_right_panel {
        let mut v: Vec<String> = panel.iter().take(9).cloned().collect();
        while v.len() < 9 {
            v.push(String::new());
        }
        v
    } else {
        vec![
            "Tips for getting".to_string(),
            "started".to_string(),
            "Ask Claude to create a\u{2026}".to_string(),
            "---".to_string(),
            "Recent activity".to_string(),
            "No recent activity".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ]
    };

    // Classify entries: first item and first item after separator are headers
    let mut right_entries: Vec<RightPanelEntry> = Vec::new();
    let mut next_is_header = true;
    for s in &panel_source {
        if s == "---" {
            right_entries.push(RightPanelEntry::Separator);
            next_is_header = true;
        } else if s.is_empty() {
            right_entries.push(RightPanelEntry::Empty);
        } else if next_is_header {
            right_entries.push(RightPanelEntry::Header(s.clone()));
            next_is_header = false;
        } else {
            right_entries.push(RightPanelEntry::Content(s.clone()));
        }
    }

    // Build plain-text right panel rows from entries
    let right_rows: Vec<String> = right_entries
        .iter()
        .map(|e| match e {
            RightPanelEntry::Header(s) | RightPanelEntry::Content(s) => {
                right_content(s, right_panel_width)
            }
            RightPanelEntry::Separator => {
                right_content(&"─".repeat(right_panel_width - 2), right_panel_width)
            }
            RightPanelEntry::Empty => format!("{:w$}", "", w = right_panel_width),
        })
        .collect();

    // Build left panel content (9 rows)
    let logo_line1 = "\u{2597} \u{2597}   \u{2596} \u{2596}"; // ▗ ▗   ▖ ▖
    let logo_line3 = "\u{2598}\u{2598} \u{259D}\u{259D}"; // ▘▘ ▝▝

    let left_rows: Vec<String> = vec![
        format!("{:w$}", "", w = left_panel_width), // Row 0: empty
        center("Welcome back!", left_panel_width),  // Row 1
        format!("{:w$}", "", w = left_panel_width), // Row 2: empty
        center(logo_line1, left_panel_width),       // Row 3: logo top
        format!("{:w$}", "", w = left_panel_width), // Row 4: logo mid (invisible plain)
        center(logo_line3, left_panel_width),       // Row 5: logo bottom
        format!("{:w$}", "", w = left_panel_width), // Row 6: empty
        center(&model_str, left_panel_width),       // Row 7: model
        left_align(&path_display, left_panel_width), // Row 8: path
    ];

    let mut lines = Vec::new();

    if is_tty {
        let fg_orange = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
        let fg_gray = escape::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);
        let fg_sep_gray = escape::fg(SEPARATOR_GRAY.0, SEPARATOR_GRAY.1, SEPARATOR_GRAY.2);
        let _fg_black = escape::fg(LOGO_BG.0, LOGO_BG.1, LOGO_BG.2);
        let bg_orange = escape::bg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
        // Combined fg black + bg orange in single sequence (avoids extra escape codes)
        let fg_black_bg_orange = format!(
            "\x1b[38;2;{};{};{};48;2;{};{};{}m",
            LOGO_BG.0, LOGO_BG.1, LOGO_BG.2, LOGO_FG.0, LOGO_FG.1, LOGO_FG.2
        );
        // Combined dim + fg orange in single sequence
        let dim_fg_orange = format!("\x1b[2;38;2;{};{};{}m", LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
        let reset = escape::RESET;
        let bold = escape::BOLD;
        let _dim = escape::DIM;

        // Top border
        let title_part = format!("─── {} ", product_name);
        let version_fill =
            total_inner.saturating_sub(title_part.chars().count() + version.len() + 1);
        lines.push(format!(
            "{reset}{fg_orange}╭{title_part}{reset}{fg_gray}{version}{reset}{fg_orange} {fill}╮",
            fill = "─".repeat(version_fill),
        ));

        // Content rows
        for (row, right_entry) in right_entries.iter().enumerate() {
            // Left border
            let mut line = format!("{reset}{fg_orange}│{reset}");

            // Left panel content (with styling based on row)
            match row {
                1 => {
                    // "Welcome back!" - bold white centered
                    let text = "Welcome back!";
                    let total_pad = left_panel_width.saturating_sub(text.len());
                    let pad_right = total_pad / 2;
                    let pad_left = total_pad - pad_right;
                    line.push_str(&format!(
                        "{:pl$}{reset}{bold}{text}{reset}{:pr$}",
                        "",
                        "",
                        pl = pad_left,
                        pr = pad_right,
                    ));
                }
                3 => {
                    // Logo line 1: ▗ [bg] ▗   ▖ [/bg] ▖
                    let logo_visual_width = 9; // "▗ ▗   ▖ ▖" = 9 chars visual
                    let total_pad = left_panel_width.saturating_sub(logo_visual_width);
                    let pad_right = total_pad / 2;
                    let pad_left = total_pad - pad_right;
                    line.push_str(&format!(
                        "{:pl$}{reset}{fg_orange}\u{2597}{reset}{fg_black_bg_orange} \u{2597}   \u{2596} {reset}{fg_orange}\u{2596}{reset}{:pr$}",
                        "", "",
                        pl = pad_left,
                        pr = pad_right,
                    ));
                }
                4 => {
                    // Logo line 2: solid fill (orange bg, 7 spaces) centered
                    let logo_visual_width = 7;
                    let total_pad = left_panel_width.saturating_sub(logo_visual_width);
                    let pad_right = total_pad / 2;
                    let pad_left = total_pad - pad_right;
                    line.push_str(&format!(
                        "{:pl$}{reset}{bg_orange}       {reset}{:pr$}",
                        "",
                        "",
                        pl = pad_left,
                        pr = pad_right,
                    ));
                }
                5 => {
                    // Logo line 3: ▘▘ ▝▝ in orange
                    let text = "\u{2598}\u{2598} \u{259D}\u{259D}";
                    let text_len = 5;
                    let total_pad = left_panel_width.saturating_sub(text_len);
                    let pad_right = total_pad / 2;
                    let pad_left = total_pad - pad_right;
                    line.push_str(&format!(
                        "{:pl$}{reset}{fg_orange}{text}{reset}{:pr$}",
                        "",
                        "",
                        pl = pad_left,
                        pr = pad_right,
                    ));
                }
                7 => {
                    // Model/provider - gray centered
                    let text_len = model_str.chars().count();
                    let total_pad = left_panel_width.saturating_sub(text_len);
                    let pad_right = total_pad / 2;
                    let pad_left = total_pad - pad_right;
                    line.push_str(&format!(
                        "{:pl$}{reset}{fg_gray}{model_str}{reset}{:pr$}",
                        "",
                        "",
                        pl = pad_left,
                        pr = pad_right,
                    ));
                }
                8 => {
                    // Path - gray, left-aligned with 1-char padding
                    let path_len = path_display.chars().count();
                    let pad = left_panel_width.saturating_sub(path_len + 2);
                    line.push_str(&format!(
                        " {reset}{fg_gray}{path_display}{reset} {:p$}",
                        "",
                        p = pad,
                    ));
                }
                _ => {
                    // Empty rows
                    line.push_str(&format!("{:w$}", "", w = left_panel_width));
                }
            }

            // Divider (dim orange)
            line.push_str(&format!("{reset}{dim_fg_orange}│{reset}"));

            // Right panel content (styled based on entry type)
            match right_entry {
                RightPanelEntry::Header(text) => {
                    let pad = right_panel_width.saturating_sub(text.chars().count() + 1);
                    line.push_str(&format!(
                        " {reset}\x1b[1;38;2;215;119;87m{text}{reset}{:p$}",
                        "",
                        p = pad,
                    ));
                }
                RightPanelEntry::Content(text) => {
                    let pad = right_panel_width.saturating_sub(text.chars().count() + 1);
                    line.push_str(&format!(" {reset}{fg_gray}{text}{reset}{:p$}", "", p = pad));
                }
                RightPanelEntry::Separator => {
                    let dashes = right_panel_width - 2;
                    line.push_str(&format!(
                        " {reset}{dim_fg_orange}{}{reset} ",
                        "─".repeat(dashes),
                    ));
                }
                RightPanelEntry::Empty => {
                    line.push_str(&format!("{:w$}", "", w = right_panel_width));
                }
            }

            // Right border (orange) — no trailing reset; iocraft inserts \x1b[K
            // (erase-to-EOL) after the content and a trailing \x1b[0m would get
            // split by that insertion, leaking a stray 'm' character.
            line.push_str(&format!("{reset}{fg_orange}│"));
            lines.push(line);
        }

        // Bottom border
        lines.push(format!("{reset}{fg_orange}╰{}╯", "─".repeat(total_inner),));

        // Suppress unused variable warnings
        let _ = fg_sep_gray;
    } else {
        // Plain text mode

        // Top border
        let title_part = format!("─── {} ", product_name);
        let version_fill =
            total_inner.saturating_sub(title_part.chars().count() + version.len() + 1);
        lines.push(format!(
            "╭{title_part}{version} {}╮",
            "─".repeat(version_fill),
        ));

        // Content rows
        for row in 0..9 {
            lines.push(format!("│{}│{}│", left_rows[row], right_rows[row]));
        }

        // Bottom border
        lines.push(format!("╰{}╯", "─".repeat(total_inner)));
    }

    lines
}

/// Truncate a path from the left using `/…/` prefix when it exceeds max_len.
fn truncate_path(path: &str, max_len: usize) -> String {
    let char_count = path.chars().count();
    if char_count <= max_len {
        return path.to_string();
    }
    // Find a good cut point: remove from start, find next `/`
    let chars: Vec<char> = path.chars().collect();
    // We need: "/…" (2 chars) + remaining. So remaining = max_len - 2.
    let target_remaining = max_len.saturating_sub(2);
    let start = chars.len().saturating_sub(target_remaining);
    // Find the next '/' at or after start to make a clean path break
    let mut cut = start;
    while cut < chars.len() && chars[cut] != '/' {
        cut += 1;
    }
    let suffix: String = chars[cut..].iter().collect();
    format!("/\u{2026}{}", suffix)
}

fn extract_model_version(model: &str) -> Option<String> {
    // Pattern: claude-{name}-{major}-{minor?}-{date}
    // e.g., "claude-opus-4-5-20251101" -> "4.5"
    // e.g., "claude-sonnet-4-20250514" -> "4"
    let parts: Vec<&str> = model.split('-').collect();
    if parts.len() >= 4 && parts[0] == "claude" {
        let major = parts[2];
        if major.chars().all(|c| c.is_ascii_digit()) {
            let minor = parts.get(3);
            if let Some(m) = minor {
                if m.chars().all(|c| c.is_ascii_digit()) && m.len() <= 2 {
                    return Some(format!("{}.{}", major, m));
                }
            }
            return Some(major.to_string());
        }
    }
    None
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
