// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::panic)]

//! Unit tests for responsive terminal width rendering.

use super::app::render::format_status_bar;
use super::app::state::{DialogState, DisplayState, InputState};
use super::app::types::{AppMode, RenderState, StatusInfo, DEFAULT_TERMINAL_WIDTH};
use super::separator::{make_compact_separator, make_separator};
use crate::permission::PermissionMode;

/// Test separator rendering at various widths
mod separator_rendering {
    use super::*;

    #[test]
    fn separator_width_matches_terminal() {
        for width in [80, 100, 120, 150, 200] {
            let sep = make_separator(width);
            assert_eq!(
                sep.chars().count(),
                width,
                "Separator should be {} chars wide",
                width
            );
        }
    }

    #[test]
    fn compact_separator_width_matches_terminal() {
        let text = "Conversation compacted · ctrl+o for history";
        for width in [80, 100, 120, 150, 200] {
            let sep = make_compact_separator(text, width);
            assert_eq!(
                sep.chars().count(),
                width,
                "Compact separator should be {} chars wide",
                width
            );
        }
    }
}

/// Test terminal width defaults
mod terminal_width {
    use super::*;

    #[test]
    fn default_terminal_width_is_120() {
        assert_eq!(DEFAULT_TERMINAL_WIDTH, 120);
    }
}

/// Test status bar formatting at various widths
mod status_bar_rendering {
    use super::*;

    fn create_render_state(width: u16) -> RenderState {
        RenderState {
            mode: AppMode::Input,
            input: InputState::default(),
            dialog: DialogState::None,
            display: DisplayState {
                terminal_width: width,
                ..Default::default()
            },
            status: StatusInfo::default(),
            user_name: "test".to_string(),
            thinking_enabled: true,
            permission_mode: PermissionMode::Default,
            claude_version: None,
            is_tty: false,
            is_compacting: false,
            spinner_frame: 0,
            spinner_verb: String::new(),
        }
    }

    #[test]
    fn status_bar_fits_terminal_width() {
        for width in [80, 100, 120, 150] {
            let state = create_render_state(width);
            let bar = format_status_bar(&state, width as usize);
            assert!(
                bar.chars().count() <= width as usize,
                "Status bar should fit within {} chars, got {}",
                width,
                bar.chars().count()
            );
        }
    }

    #[test]
    fn status_bar_thinking_off_aligned_right() {
        let mut state = create_render_state(100);
        state.thinking_enabled = false;
        let bar = format_status_bar(&state, 100);
        // "Thinking off" should be at the right edge
        assert!(bar.trim_end().ends_with("Thinking off"));
    }

    #[test]
    fn status_bar_non_default_mode_has_toggle_hint() {
        let mut state = create_render_state(120);
        state.permission_mode = PermissionMode::Plan;
        let bar = format_status_bar(&state, 120);
        assert!(bar.contains("Use meta+t to toggle thinking"));
    }
}
