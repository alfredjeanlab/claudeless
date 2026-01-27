// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Header, status bar, and model name formatting functions.

use crate::permission::PermissionMode;
use crate::tui::colors::{
    styled_logo_line1, styled_logo_line2, styled_logo_line3, styled_permission_status,
};

use super::super::types::{ExitHint, RenderState};

/// Format header lines with Claude branding (returns 3 lines)
pub(crate) fn format_header_lines(state: &RenderState) -> (String, String, String) {
    let model_name = model_display_name(&state.status.model);

    // Get working directory display (shortened if possible)
    let working_dir = std::env::current_dir()
        .map(|p| {
            // Try to convert to ~ format using HOME env var
            if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::PathBuf::from(&home);
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
    let model_str = format!("{} · Claude Max", model_name);

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
            ExitHint::Escape => "  Esc to clear again".to_string(),
        };
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
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                mode_text
            } else {
                // Pad to align "Thinking off" to the right side
                let padding = width.saturating_sub(mode_text.len() + "Thinking off".len());
                format!("{}{:width$}Thinking off", mode_text, "", width = padding)
            }
        }
        _ => {
            // Non-default modes show "Use meta+t to toggle thinking" on the right
            let right_text = "Use meta+t to toggle thinking";
            // Calculate visual width of mode_text (accounting for multi-byte chars)
            let mode_visual_width = mode_text.chars().count();
            let right_width = right_text.len();
            let padding = width.saturating_sub(mode_visual_width + right_width);
            format!("{}{:width$}{}", mode_text, "", right_text, width = padding)
        }
    }
}

/// Format styled status bar content (with ANSI colors)
pub(crate) fn format_status_bar_styled(state: &RenderState, width: usize) -> String {
    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.display.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
            ExitHint::Escape => "  Esc to clear again".to_string(),
        };
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
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                status
            } else {
                // Show "Thinking off" aligned to the right
                let right = "Thinking off";
                let padding = width.saturating_sub(mode_visual_width + right.len());
                format!("{}{:width$}{}", status, "", right, width = padding)
            }
        }
        _ => {
            // Non-default modes show "Use meta+t to toggle thinking" on the right
            let right_text = "Use meta+t to toggle thinking";
            let padding = width.saturating_sub(mode_visual_width + right_text.len());
            format!("{}{:width$}{}", status, "", right_text, width = padding)
        }
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
