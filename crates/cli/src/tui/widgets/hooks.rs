// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hooks dialog widget.
//!
//! Shown when user executes `/hooks` to manage hook configurations.

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;

use super::scrollable::ScrollState;

/// Hook types displayed in the dialog
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookType {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Notification,
    UserPromptSubmit,
    SessionStart,
    Stop,
    SubagentStart,
    SubagentStop,
    PreCompact,
    SessionEnd,
    PermissionRequest,
    Setup,
    DisableAllHooks,
}

impl HookType {
    /// All hook types in display order
    pub fn all() -> &'static [HookType] {
        &[
            HookType::PreToolUse,
            HookType::PostToolUse,
            HookType::PostToolUseFailure,
            HookType::Notification,
            HookType::UserPromptSubmit,
            HookType::SessionStart,
            HookType::Stop,
            HookType::SubagentStart,
            HookType::SubagentStop,
            HookType::PreCompact,
            HookType::SessionEnd,
            HookType::PermissionRequest,
            HookType::Setup,
            HookType::DisableAllHooks,
        ]
    }

    /// Display name for the hook type
    pub fn name(self) -> &'static str {
        match self {
            HookType::PreToolUse => "PreToolUse",
            HookType::PostToolUse => "PostToolUse",
            HookType::PostToolUseFailure => "PostToolUseFailure",
            HookType::Notification => "Notification",
            HookType::UserPromptSubmit => "UserPromptSubmit",
            HookType::SessionStart => "SessionStart",
            HookType::Stop => "Stop",
            HookType::SubagentStart => "SubagentStart",
            HookType::SubagentStop => "SubagentStop",
            HookType::PreCompact => "PreCompact",
            HookType::SessionEnd => "SessionEnd",
            HookType::PermissionRequest => "PermissionRequest",
            HookType::Setup => "Setup",
            HookType::DisableAllHooks => "Disable all hooks",
        }
    }

    /// Description for the hook type
    pub fn description(self) -> &'static str {
        match self {
            HookType::PreToolUse => "Before tool execution",
            HookType::PostToolUse => "After tool execution",
            HookType::PostToolUseFailure => "After tool execution fails",
            HookType::Notification => "When notifications are sent",
            HookType::UserPromptSubmit => "When the user submits a prompt",
            HookType::SessionStart => "When a new session is started",
            HookType::Stop => "Right before Claude concludes its response",
            HookType::SubagentStart => "When a subagent (Task tool call) is started",
            HookType::SubagentStop => {
                "Right before a subagent (Task tool call) concludes its response"
            }
            HookType::PreCompact => "Before conversation compaction",
            HookType::SessionEnd => "When a session is ending",
            HookType::PermissionRequest => "When a permission dialog is displayed",
            HookType::Setup => "Repo setup hooks for init and maintenance",
            HookType::DisableAllHooks => "Temporarily disable all hooks",
        }
    }

    /// Whether this hook type shows tool matchers dialog
    pub fn has_matchers(self) -> bool {
        matches!(
            self,
            HookType::PreToolUse | HookType::PostToolUse | HookType::PostToolUseFailure
        )
    }
}

/// View state for the hooks dialog
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HooksView {
    /// Main hook type list
    #[default]
    HookList,
    /// Matchers dialog for a specific hook type
    Matchers,
}

/// State for the /hooks dialog
#[derive(Clone, Debug)]
pub struct HooksDialog {
    /// Scroll-aware navigation state for hook list
    scroll: ScrollState,
    /// Current view (list or matchers)
    pub view: HooksView,
    /// Selected hook type when viewing matchers
    pub selected_hook: Option<HookType>,
    /// Selected matcher index in matchers view (0-based)
    pub matcher_selected: usize,
    /// Number of active hooks (for display)
    pub active_hook_count: usize,
}

impl Default for HooksDialog {
    fn default() -> Self {
        Self::new(5) // Default to showing 5 active hooks
    }
}

impl HooksDialog {
    pub fn new(active_hook_count: usize) -> Self {
        let mut scroll = ScrollState::new(5); // Default visible items
        scroll.set_total(HookType::all().len());
        Self {
            scroll,
            view: HooksView::HookList,
            selected_hook: None,
            matcher_selected: 0,
            active_hook_count,
        }
    }

    /// Get the currently selected index
    pub fn selected_index(&self) -> usize {
        self.scroll.selected_index
    }

    /// Get the scroll offset for rendering
    pub fn scroll_offset(&self) -> usize {
        self.scroll.scroll_offset
    }

    /// Get the visible item count
    pub fn visible_count(&self) -> usize {
        self.scroll.visible_count
    }

    /// Set the visible item count (call when terminal resizes)
    pub fn set_visible_count(&mut self, count: usize) {
        self.scroll.visible_count = count;
    }

    /// Move selection up (wraps at boundaries)
    pub fn select_prev(&mut self) {
        self.scroll.select_prev();
    }

    /// Move selection down (wraps at boundaries)
    pub fn select_next(&mut self) {
        self.scroll.select_next();
    }

    /// Get currently selected hook type
    pub fn selected_hook_type(&self) -> HookType {
        HookType::all()[self.scroll.selected_index]
    }

    /// Open matchers dialog for current selection
    pub fn open_matchers(&mut self) {
        let hook = self.selected_hook_type();
        self.selected_hook = Some(hook);
        self.view = HooksView::Matchers;
        self.matcher_selected = 0;
    }

    /// Return to hook list from matchers
    pub fn close_matchers(&mut self) {
        self.view = HooksView::HookList;
        self.selected_hook = None;
    }

    /// Check if we should show scroll indicator above
    pub fn has_more_above(&self) -> bool {
        self.scroll.has_more_above()
    }

    /// Check if we should show scroll indicator below
    pub fn has_more_below(&self) -> bool {
        self.scroll.has_more_below()
    }
}
