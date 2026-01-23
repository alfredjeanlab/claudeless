# Implementation Plan: /hooks Slash Command

## Overview

Implement the `/hooks` slash command to display a dialog for managing hook configurations. The dialog shows a scrollable list of hook types (PreToolUse, PostToolUse, etc.) with their descriptions. Selecting a hook type opens a nested "Tool Matchers" dialog showing configured matchers and exit code documentation.

Key features:
- Scrollable list of 14 hook types with active hook count header
- Up/Down arrow navigation with selection cursor (`❯`) and scroll indicator (`↓`)
- Enter to select a hook type and view its matchers
- Nested matchers dialog with exit code help text
- Escape to dismiss (returns to hooks list from matchers, exits dialog from hooks list)

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add AppMode::HooksDialog, handler, key handling, rendering
│   └── widgets/
│       ├── mod.rs             # Export HooksDialog, HookType
│       ├── hooks.rs           # NEW: HooksDialog struct, HookType enum
│       └── hooks_tests.rs     # NEW: Unit tests for HooksDialog

crates/cli/tests/
├── tui_hooks.rs               # Remove #[ignore] from 11 tests
└── fixtures/tui/v2.1.12/
    ├── hooks_autocomplete.txt # Reference fixture
    ├── hooks_dialog.txt       # Reference fixture
    └── hooks_matcher_dialog.txt # Reference fixture
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm` for keyboard input handling
- `hooks::HookEvent` for hook type metadata (optional integration)
- Box-drawing characters for dialog borders

## Implementation Phases

### Phase 1: Create HooksDialog Widget

Create the widget module with `HookType` enum and `HooksDialog` state struct.

**File:** `crates/cli/src/tui/widgets/hooks.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Hooks dialog widget.
//!
//! Shown when user executes `/hooks` to manage hook configurations.

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;

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
            HookType::SubagentStop => "Right before a subagent (Task tool call) concludes its response",
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
            HookType::PreToolUse
                | HookType::PostToolUse
                | HookType::PostToolUseFailure
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
    /// Currently selected hook type index (0-based)
    pub selected_index: usize,
    /// Current view (list or matchers)
    pub view: HooksView,
    /// Selected hook type when viewing matchers
    pub selected_hook: Option<HookType>,
    /// Selected matcher index in matchers view (0-based)
    pub matcher_selected: usize,
    /// Number of active hooks (for display)
    pub active_hook_count: usize,
    /// Scroll offset for the hook list
    pub scroll_offset: usize,
    /// Visible item count (based on terminal height)
    pub visible_count: usize,
}

impl Default for HooksDialog {
    fn default() -> Self {
        Self::new(4) // Default to showing 4 active hooks
    }
}

impl HooksDialog {
    pub fn new(active_hook_count: usize) -> Self {
        Self {
            selected_index: 0,
            view: HooksView::HookList,
            selected_hook: None,
            matcher_selected: 0,
            active_hook_count,
            scroll_offset: 0,
            visible_count: 5, // Default visible items
        }
    }

    /// Move selection up (wraps at boundaries)
    pub fn select_prev(&mut self) {
        let total = HookType::all().len();
        if self.selected_index == 0 {
            self.selected_index = total - 1;
            // Scroll to bottom
            if total > self.visible_count {
                self.scroll_offset = total - self.visible_count;
            }
        } else {
            self.selected_index -= 1;
            // Scroll up if needed
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    /// Move selection down (wraps at boundaries)
    pub fn select_next(&mut self) {
        let total = HookType::all().len();
        self.selected_index = (self.selected_index + 1) % total;
        // Handle wrap to top
        if self.selected_index == 0 {
            self.scroll_offset = 0;
        }
        // Scroll down if needed
        else if self.selected_index >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected_index - self.visible_count + 1;
        }
    }

    /// Get currently selected hook type
    pub fn selected_hook_type(&self) -> HookType {
        HookType::all()[self.selected_index]
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

    /// Check if we should show scroll indicator below
    pub fn has_more_below(&self) -> bool {
        let total = HookType::all().len();
        self.scroll_offset + self.visible_count < total
    }
}
```

**Tasks:**
1. Create `crates/cli/src/tui/widgets/hooks.rs` with `HookType` enum and `HooksDialog` struct
2. Add unit tests in `crates/cli/src/tui/widgets/hooks_tests.rs`:
   - Test `HookType::all()` returns all 14 types
   - Test `HooksDialog::select_next()` and `select_prev()` with wrapping
   - Test scroll offset updates when navigating
   - Test `open_matchers()` and `close_matchers()` state transitions

### Phase 2: Wire Up AppMode and State

Integrate the HooksDialog into the TUI application state.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `HooksDialog` variant to `AppMode` enum:
   ```rust
   /// Showing hooks management dialog
   HooksDialog,
   ```

2. Add `hooks_dialog` field to `RenderState` struct:
   ```rust
   /// Hooks dialog state (None if not showing)
   pub hooks_dialog: Option<HooksDialog>,
   ```

3. Add `hooks_dialog` field to `TuiAppStateInner` struct

4. Update `render_state()` to include `hooks_dialog`

5. Export from `widgets/mod.rs`:
   ```rust
   pub mod hooks;
   pub use hooks::{HooksDialog, HooksView, HookType};
   ```

### Phase 3: Implement Command Handler

Add handler for the `/hooks` command to open the dialog.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `/hooks` match arm in `handle_command_inner()`:
   ```rust
   "/hooks" => {
       inner.mode = AppMode::HooksDialog;
       let active_count = inner.get_active_hook_count(); // Or hard-code 4 initially
       inner.hooks_dialog = Some(HooksDialog::new(active_count));
   }
   ```

### Phase 4: Implement Key Handler

Add keyboard handling for the hooks dialog with nested navigation.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `AppMode::HooksDialog` to the match in `handle_key_event()`:
   ```rust
   AppMode::HooksDialog => self.handle_hooks_dialog_key(key),
   ```

2. Add `AppMode::HooksDialog` to escape handling

3. Implement `handle_hooks_dialog_key()`:
   ```rust
   fn handle_hooks_dialog_key(&self, key: KeyEvent) {
       let mut inner = self.inner.lock();

       let Some(ref mut dialog) = inner.hooks_dialog else {
           return;
       };

       match dialog.view {
           HooksView::HookList => match key.code {
               KeyCode::Esc => {
                   inner.mode = AppMode::Input;
                   inner.hooks_dialog = None;
                   inner.response_content = "Hooks dialog dismissed".to_string();
                   inner.is_command_output = true;
               }
               KeyCode::Up => dialog.select_prev(),
               KeyCode::Down => dialog.select_next(),
               KeyCode::Enter => dialog.open_matchers(),
               _ => {}
           },
           HooksView::Matchers => match key.code {
               KeyCode::Esc => dialog.close_matchers(),
               KeyCode::Up => {
                   // Navigate matchers (when implemented)
               }
               KeyCode::Down => {
                   // Navigate matchers (when implemented)
               }
               KeyCode::Enter => {
                   // Add new matcher (when implemented)
               }
               _ => {}
           },
       }
   }
   ```

### Phase 5: Implement Render Functions

Add dialog rendering for both the hook list and matchers views.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add conditional rendering in main render function:
   ```rust
   if state.mode == AppMode::HooksDialog {
       if let Some(ref dialog) = state.hooks_dialog {
           return render_hooks_dialog(dialog, width);
       }
   }
   ```

2. Implement `render_hooks_dialog()`:
   ```rust
   fn render_hooks_dialog(dialog: &HooksDialog, _width: usize) -> AnyElement<'static> {
       match dialog.view {
           HooksView::HookList => render_hooks_list(dialog),
           HooksView::Matchers => render_hooks_matchers(dialog),
       }
   }
   ```

3. Implement `render_hooks_list()`:
   ```rust
   fn render_hooks_list(dialog: &HooksDialog) -> AnyElement<'static> {
       // Header: " Hooks\n 4 hooks"
       // Then numbered list with selection cursor:
       // " ❯ 1.  PreToolUse - Before tool execution"
       // "   2.  PostToolUse - After tool execution"
       // " ↓ 5.  UserPromptSubmit - When the user submits a prompt"
       // Footer: " Enter to confirm · esc to cancel"

       let hooks = HookType::all();
       let visible_start = dialog.scroll_offset;
       let visible_end = (visible_start + dialog.visible_count).min(hooks.len());

       // Build visible items
       let mut items = Vec::new();
       for i in visible_start..visible_end {
           let hook = hooks[i];
           let is_selected = i == dialog.selected_index;
           let is_last_visible = i == visible_end - 1 && dialog.has_more_below();

           let prefix = if is_selected {
               "❯"
           } else if is_last_visible {
               "↓"
           } else {
               " "
           };

           items.push(format!(
               " {} {}.  {} - {}",
               prefix,
               i + 1,
               hook.name(),
               hook.description()
           ));
       }

       element! {
           View(flex_direction: FlexDirection::Column, width: 100pct) {
               Text(content: " Hooks")
               Text(content: format!(" {} hooks", dialog.active_hook_count))
               Text(content: "")
               #(items.into_iter().map(|item| {
                   element! { Text(content: item) }
               }))
               Text(content: "")
               Text(content: " Enter to confirm · esc to cancel")
           }
       }.into()
   }
   ```

4. Implement `render_hooks_matchers()`:
   ```rust
   fn render_hooks_matchers(dialog: &HooksDialog) -> AnyElement<'static> {
       let hook = dialog.selected_hook.unwrap_or(HookType::PreToolUse);

       element! {
           View(flex_direction: FlexDirection::Column, width: 100pct) {
               Text(content: format!(" {} - Tool Matchers", hook.name()))
               Text(content: " Input to command is JSON of tool call arguments.")
               Text(content: " Exit code 0 - stdout/stderr not shown")
               Text(content: " Exit code 2 - show stderr to model and block tool call")
               Text(content: " Other exit codes - show stderr to user only but continue with tool call")
               Text(content: "")
               Text(content: " ❯ 1. + Add new matcher…")
               Text(content: "   No matchers configured yet")
               Text(content: "")
               Text(content: " Enter to confirm · esc to cancel")
           }
       }.into()
   }
   ```

### Phase 6: Enable Tests

Remove `#[ignore]` attributes and verify all tests pass.

**File:** `crates/cli/tests/tui_hooks.rs`

**Tasks:**
1. Remove `#[ignore]` from all 11 test functions
2. Update TODO comments to reflect implementation status
3. Run tests: `cargo test --test tui_hooks`
4. Adjust rendering to match fixture expectations if needed

## Key Implementation Details

### Hook List Format

From the fixture `hooks_dialog.txt`:
```
 Hooks
 4 hooks

 ❯ 1.  PreToolUse - Before tool execution
   2.  PostToolUse - After tool execution
   3.  PostToolUseFailure - After tool execution fails
   4.  Notification - When notifications are sent
 ↓ 5.  UserPromptSubmit - When the user submits a prompt

 Enter to confirm · esc to cancel
```

- Leading space for all lines
- Selection cursor `❯` at position 1
- Scroll indicator `↓` on last visible item when more below
- Numbered items with double space after period

### Matchers Dialog Format

From the fixture `hooks_matcher_dialog.txt`:
```
 PreToolUse - Tool Matchers
 Input to command is JSON of tool call arguments.
 Exit code 0 - stdout/stderr not shown
 Exit code 2 - show stderr to model and block tool call
 Other exit codes - show stderr to user only but continue with tool call

 ❯ 1. + Add new matcher…
   No matchers configured yet

 Enter to confirm · esc to cancel
```

### Navigation Behavior

- **Hook List:** Up/Down navigate, Enter opens matchers for selected hook, Escape dismisses dialog
- **Matchers:** Escape returns to hook list (not dismisses entire dialog)
- Selection wraps at boundaries (up from first goes to last, down from last goes to first)
- Scroll offset adjusts to keep selection visible

### Dismiss Behavior

- From hook list: Escape dismisses, shows "Hooks dialog dismissed"
- From matchers: Escape returns to hook list (does NOT dismiss dialog)
- Returns to Input mode with clean state

## Verification Plan

### Unit Tests (Phase 1)

```bash
cargo test --lib -- tui::widgets::hooks
```

Verify:
- `HookType::all()` returns 14 hook types in correct order
- `HookType::name()` and `description()` return expected strings
- `HooksDialog::select_next()` increments and wraps
- `HooksDialog::select_prev()` decrements and wraps
- Scroll offset updates correctly on navigation
- `open_matchers()` sets view and selected_hook
- `close_matchers()` resets to HookList view

### Integration Tests (Phase 6)

```bash
cargo test --test tui_hooks
```

All 11 tests should pass:
1. `test_tui_hooks_command_shows_autocomplete` - `/hooks` appears in autocomplete
2. `test_tui_hooks_shows_dialog_with_hook_types` - Dialog shows hook types
3. `test_tui_hooks_shows_active_hooks_count` - Shows "N hooks" header
4. `test_tui_hooks_arrow_navigation` - Up/Down arrows work
5. `test_tui_hooks_list_scrolls` - List scrolls when navigating
6. `test_tui_hooks_select_shows_matchers` - Enter opens matchers dialog
7. `test_tui_hooks_matchers_shows_exit_code_help` - Matchers shows exit codes
8. `test_tui_hooks_escape_dismisses_dialog` - Escape dismisses with message
9. `test_tui_hooks_escape_from_matchers_returns_to_hooks` - Escape from matchers returns to list

### Final Verification

```bash
make check
```

Ensures:
- `cargo fmt` passes
- `cargo clippy` passes
- All tests pass
- Build succeeds
- `cargo audit` passes
- `cargo deny check` passes
