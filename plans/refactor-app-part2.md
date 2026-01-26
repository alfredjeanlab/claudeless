# Refactor app.rs Part 2 - Implementation Plan

## Overview

Continue refactoring `crates/cli/src/tui/app.rs` from 2269 lines down to under 750 lines. Part 1 extracted types, state, and input handling. Part 2 extracts dialog handlers, command processing, and rendering into focused modules.

**Current state (after Part 1):**
```
app.rs          (2269 lines) ← Still over limit (750)
app/
  input.rs      (593 lines)  - Input key handling ✓
  state.rs      (562 lines)  - State management ✓
  types.rs      (252 lines)  - Types and config ✓
```

**Target state (after Part 2):**
```
app.rs          (~200 lines) - Module declarations, App component, TuiApp wrapper
app/
  input.rs      (~593 lines) - Input key handling (unchanged)
  state.rs      (~562 lines) - State management (unchanged)
  types.rs      (~252 lines) - Types and config (unchanged)
  dialogs.rs    (~400 lines) - Dialog key handlers
  commands.rs   (~300 lines) - Command processing, streaming, permissions
  render/
    mod.rs      (~150 lines) - Re-exports, render_main_content
    content.rs  (~220 lines) - Conversation, shortcuts, slash menu, hints
    dialogs.rs  (~500 lines) - Dialog render functions
    format.rs   (~180 lines) - Header, status bar, model name formatting
```

## Project Structure

```
crates/cli/src/tui/
├── app.rs                      # Module root, App component, TuiApp wrapper
├── app/
│   ├── input.rs                # (exists) Input mode key handling
│   ├── state.rs                # (exists) State management
│   ├── types.rs                # (exists) Types and configuration
│   ├── dialogs.rs              # NEW: Dialog-specific key handlers
│   ├── commands.rs             # NEW: Slash commands, prompt processing
│   └── render/
│       ├── mod.rs              # NEW: Render module entry point
│       ├── content.rs          # NEW: Content area rendering
│       ├── dialogs.rs          # NEW: Dialog render functions
│       └── format.rs           # NEW: Header, status bar formatting
```

## Dependencies

No new external dependencies. Uses existing:
- `iocraft` - TUI rendering with element! macro
- `parking_lot` - Mutex for state access
- `arboard` - Clipboard operations (for export)

## Implementation Phases

### Phase 1: Extract Render Functions (~1050 lines total)

**Goal:** Move all rendering code to `app/render/` submodule.

**Create `app/render/mod.rs` (~150 lines):**
```rust
//! TUI rendering module.

mod content;
mod dialogs;
mod format;

pub(crate) use content::{
    render_conversation_area, render_shortcuts_panel, render_slash_menu,
    render_stash_indicator, render_argument_hint,
};
pub(crate) use dialogs::{
    render_trust_prompt, render_thinking_dialog, render_tasks_dialog,
    render_export_dialog, render_help_dialog, render_memory_dialog,
    render_hooks_dialog, render_model_picker_dialog, render_permission_dialog,
};
pub(crate) use format::{
    format_header_lines, format_status_bar, format_status_bar_styled,
    model_display_name,
};

use iocraft::prelude::*;
use super::types::{AppMode, RenderState};

/// Render the main content based on current mode
pub(crate) fn render_main_content(state: &RenderState) -> AnyElement<'static> {
    // ... main routing logic, delegates to dialog renderers
}
```

**Create `app/render/content.rs` (~220 lines):**
- `render_conversation_area()` - Conversation history with compact separator
- `render_shortcuts_panel()` - 3-column keyboard shortcuts grid
- `render_slash_menu()` - Command autocomplete dropdown
- `render_stash_indicator()` - "Stashed" notification line
- `render_argument_hint()` - Command argument placeholder

**Create `app/render/dialogs.rs` (~500 lines):**
- `render_trust_prompt()` - Trust folder confirmation
- `render_thinking_dialog()` - Enable/disable thinking mode
- `render_tasks_dialog()` - Background tasks list
- `render_export_dialog()` - Export method selection + filename input
- `render_help_dialog()` - Help tabs (General, Commands, Custom)
- `render_memory_dialog()` - Memory files list
- `render_hooks_dialog()`, `render_hooks_list()`, `render_hooks_matchers()`
- `render_model_picker_dialog()` - Model selection
- `render_permission_dialog()` - Bash/Edit/Write permission request

**Create `app/render/format.rs` (~180 lines):**
- `format_header_lines()` - Claude logo + model + working directory
- `format_status_bar()` - Plain text status bar
- `format_status_bar_styled()` - ANSI-colored status bar
- `model_display_name()` - Model ID to human-readable name
- `extract_model_version()` - Parse version from model ID

**Milestone:** `cargo check && cargo test tui::app_tests`

---

### Phase 2: Extract Dialog Key Handlers (~400 lines)

**Goal:** Move dialog-specific key handlers to `app/dialogs.rs`.

**Create `app/dialogs.rs`:**
```rust
//! Dialog key handlers for the TUI application.

use iocraft::prelude::*;
use super::state::{TuiAppState, TuiAppStateInner};
use super::types::{AppMode, ExitReason};

impl TuiAppState {
    pub(super) fn handle_permission_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_trust_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_thinking_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_tasks_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_model_picker_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_export_dialog_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_help_dialog_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_hooks_dialog_key(&self, key: KeyEvent) { ... }
    pub(super) fn handle_memory_dialog_key(&self, key: KeyEvent) { ... }
}
```

**Functions to move:**
| Function | Lines | Purpose |
|----------|-------|---------|
| `handle_permission_key` | ~80 | Navigate/confirm permission dialog |
| `handle_trust_key` | ~50 | Trust prompt Yes/No selection |
| `handle_thinking_key` | ~30 | Toggle thinking enable/disable |
| `handle_tasks_key` | ~30 | Task list up/down/enter/esc |
| `handle_model_picker_key` | ~30 | Model selection navigation |
| `handle_export_dialog_key` | ~40 | Export step navigation/input |
| `handle_help_dialog_key` | ~25 | Help tab cycling |
| `handle_hooks_dialog_key` | ~40 | Hooks list/matchers navigation |
| `handle_memory_dialog_key` | ~35 | Memory file selection |

**Milestone:** `cargo test tui::app_tests::test_session_grant*`

---

### Phase 3: Extract Command Processing (~300 lines)

**Goal:** Move command/prompt processing to `app/commands.rs`.

**Create `app/commands.rs`:**
```rust
//! Command processing and prompt handling.

use super::state::{TuiAppState, TuiAppStateInner};
use super::types::AppMode;
use super::input::clear_undo_stack;

impl TuiAppState {
    pub(super) fn submit_input(&self) { ... }
    pub(super) fn execute_shell_command(&self, command: String) { ... }
    pub(super) fn process_prompt(&self, prompt: String) { ... }
    pub(super) fn confirm_permission(&self) { ... }

    // Permission display API (public)
    pub fn show_permission_request(&self, permission_type: PermissionType) { ... }
    pub fn show_bash_permission(&self, command: String, description: Option<String>) { ... }
    pub fn show_edit_permission(&self, file_path: String, diff_lines: Vec<DiffLine>) { ... }
    pub fn show_write_permission(&self, file_path: String, content_lines: Vec<String>) { ... }
}

pub(super) fn handle_command_inner(inner: &mut TuiAppStateInner, input: &str) { ... }
pub(super) fn start_streaming_inner(inner: &mut TuiAppStateInner, text: String) { ... }
fn handle_test_permission_triggers(state: &TuiAppState, prompt: &str) -> bool { ... }
fn simulate_permission_accept(inner: &mut TuiAppStateInner, perm: &PermissionType, name: &str) { ... }

// Export helpers
pub(super) fn do_clipboard_export(inner: &mut TuiAppStateInner) { ... }
pub(super) fn do_file_export(inner: &mut TuiAppStateInner) { ... }
fn format_conversation_for_export(inner: &TuiAppStateInner) -> String { ... }
```

**Milestone:** `cargo test tui::app_tests::typing_clears_exit_hint`

---

### Phase 4: Finalize Module Structure (~200 lines in app.rs)

**Goal:** Reduce `app.rs` to module declarations, App component, and TuiApp wrapper.

**Final `app.rs` structure:**
```rust
// SPDX-License-Identifier: MIT
// Module declarations
mod commands;
mod dialogs;
mod input;
mod render;
mod state;
mod types;

// Public re-exports
pub use state::TuiAppState;
pub use types::{
    AppMode, ExitHint, ExitReason, PermissionChoice, PermissionRequest,
    RenderState, StatusInfo, TrustPromptState, TuiConfig, DEFAULT_TERMINAL_WIDTH,
};

use state::TuiAppStateInner;
use render::{render_main_content, format_status_bar};

// Imports for App component (~20 lines)
use iocraft::prelude::*;
// ... other imports

// AppProps struct (~5 lines)
#[derive(Default, Props)]
pub struct AppProps {
    pub state: Option<TuiAppState>,
}

// App component (~80 lines)
#[component]
pub fn App(mut hooks: Hooks, props: &AppProps) -> impl Into<AnyElement<'static>> {
    // ... event handling, render loop
}

// TuiApp wrapper (~100 lines)
pub struct TuiApp { state: TuiAppState }
impl TuiApp {
    pub fn new(...) -> std::io::Result<Self> { ... }
    pub fn run(&mut self) -> std::io::Result<ExitReason> { ... }
    // ... delegation methods
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
```

**Milestone:** `quench check` passes (app.rs under 750 lines)

---

### Phase 5: Add Unit Tests

**Goal:** Add focused tests for extracted modules.

**Create `app/render/format_tests.rs`:**
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn model_display_name_short_aliases() {
    assert_eq!(model_display_name("opus"), "Opus 4.5");
    assert_eq!(model_display_name("sonnet"), "Sonnet 4.5");
    assert_eq!(model_display_name("haiku"), "Haiku 4.5");
}

#[test]
fn model_display_name_full_id_with_minor_version() {
    assert_eq!(model_display_name("claude-opus-4-5-20251101"), "Opus 4.5");
}

#[test]
fn model_display_name_full_id_major_only() {
    assert_eq!(model_display_name("claude-sonnet-4-20250514"), "Sonnet 4");
}
```

**Create `app/commands_tests.rs`:**
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn handle_command_clear_resets_tokens() {
    // Test /clear resets input_tokens and output_tokens
}

#[test]
fn handle_command_unknown_shows_error() {
    // Test unknown command shows "Unknown command: ..."
}
```

**Milestone:** `cargo test app::render::format_tests && cargo test app::commands_tests`

---

## Key Implementation Details

### Module Visibility

Use `pub(super)` for cross-module access within `app/`:
```rust
// app/dialogs.rs
impl TuiAppState {
    pub(super) fn handle_permission_key(&self, key: KeyEvent) { ... }
}

// app/input.rs can call via self.handle_permission_key(key)
```

### Render Function Pattern

All render functions are pure, taking state references:
```rust
pub(crate) fn render_trust_prompt(prompt: &TrustPromptState, width: usize) -> AnyElement<'static>
```

### State Access Pattern

Methods needing mutable state explicitly drop the lock before calling other methods:
```rust
fn handle_permission_key(&self, key: KeyEvent) {
    let mut inner = self.inner.lock();
    // ... modify inner
    drop(inner);
    self.confirm_permission();  // Safe to call now
}
```

### Import Strategy

Use explicit imports rather than glob imports for clarity:
```rust
// In app/render/mod.rs
use super::types::{AppMode, ExitHint, RenderState};
use crate::tui::widgets::{HelpDialog, HelpTab};
```

## Verification Plan

### After Each Phase

```bash
cargo check --all
cargo test tui::app_tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

### Final Verification

```bash
make check
```

Runs: lint, fmt, clippy, quench check, tests, build, audit, deny check.

### Line Count Targets

| File | Target Lines |
|------|-------------|
| `app.rs` | ~200 |
| `app/render/mod.rs` | ~150 |
| `app/render/content.rs` | ~220 |
| `app/render/dialogs.rs` | ~500 |
| `app/render/format.rs` | ~180 |
| `app/dialogs.rs` | ~400 |
| `app/commands.rs` | ~300 |

All under 750 line limit.

## Risk Mitigation

1. **Test regression:** Existing `app_tests.rs` provides coverage; run after each phase
2. **Incremental phases:** Each phase is independently verifiable with cargo check/test
3. **API preservation:** Public `TuiApp` and `TuiAppState` APIs remain unchanged
4. **No behavior changes:** Pure code reorganization, no logic modifications
