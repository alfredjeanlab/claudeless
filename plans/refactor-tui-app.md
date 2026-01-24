# Refactor TUI App

## Overview

**Root Feature:** `cl-e7cb`

Split the monolithic `crates/cli/src/tui/app.rs` (3596 lines, 32772 tokens) into smaller, focused modules. The file contains mixed concerns: configuration, state management, input handling, command processing, dialog handlers, permission handling, and rendering. This refactor extracts each concern into its own module under `tui/app/` while maintaining the same public API.

## Project Structure

```
crates/cli/src/tui/
├── app.rs              # Re-exports only (slim facade)
├── app_tests.rs        # Existing tests (unchanged)
├── app/
│   ├── mod.rs          # Module declarations + re-exports
│   ├── types.rs        # Data types, enums, RenderState (~300 lines)
│   ├── state.rs        # TuiAppState, TuiAppStateInner (~350 lines)
│   ├── state_tests.rs  # Tests for state module
│   ├── input.rs        # Input mode key handling (~350 lines)
│   ├── input_tests.rs  # Tests for input module
│   ├── commands.rs     # Slash command processing (~200 lines)
│   ├── commands_tests.rs
│   ├── dialogs.rs      # Dialog mode key handlers (~300 lines)
│   ├── dialogs_tests.rs
│   ├── render.rs       # Main render dispatch + helpers (~400 lines)
│   ├── render_tests.rs
│   ├── render_dialogs.rs  # Dialog rendering functions (~500 lines)
│   └── runner.rs       # TuiApp wrapper struct (~150 lines)
└── ...                 # Existing files unchanged
```

**Target**: Each file < 500 lines (well under 750 limit).

## Dependencies

No new external dependencies. Uses existing:
- `iocraft` for UI components
- `parking_lot::Mutex` for thread-safe state
- Existing `crate::tui::widgets::*` for dialog types

## Implementation Phases

### Phase 1: Create app/ directory and types module

**Goal**: Extract data types without breaking existing code.

1. Create `app/mod.rs` with module declarations
2. Create `app/types.rs` containing:
   - `ctrl_key!` macro
   - `TuiConfig` struct and impl
   - `AppMode` enum
   - `StatusInfo` struct
   - `RenderState` struct
   - `PermissionRequest` struct
   - `PermissionChoice` enum and From impl
   - `ExitReason` enum
   - `ExitHint` enum
   - `EXIT_HINT_TIMEOUT_MS` constant
   - `DEFAULT_TERMINAL_WIDTH` constant
   - `TrustPromptState` struct and impl

3. Update `app.rs` to re-export from `app/types.rs`

**Verification**: `cargo build --all && cargo test --all`

### Phase 2: Extract state management

**Goal**: Move TuiAppState and TuiAppStateInner to dedicated module.

1. Create `app/state.rs` containing:
   - `TuiAppState` struct (Arc<Mutex<TuiAppStateInner>>)
   - `TuiAppStateInner` struct (all ~60 fields)
   - `TuiAppState::new()` constructor
   - Accessor methods: `render_state()`, `terminal_width()`, `set_terminal_width()`, `should_exit()`, `exit_reason()`, `exit_message()`, `mode()`, `input_buffer()`, `cursor_pos()`, `history()`, `exit()`, `clear_exit_state()`
   - Timer methods: `check_exit_hint_timeout()`, `check_compacting()`
   - Helper methods: `format_todos()`, `random_farewell()`, `format_context_usage()`

2. Key pattern for cross-module method access:

```rust
// In state.rs - expose inner lock for other modules
impl TuiAppState {
    pub(super) fn with_inner<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TuiAppStateInner) -> R
    {
        let mut inner = self.inner.lock();
        f(&mut inner)
    }
}
```

**Verification**: `cargo build --all && cargo test --all`

### Phase 3: Extract input handling

**Goal**: Move input mode key handling to dedicated module.

1. Create `app/input.rs` containing:
   - `TuiAppState::handle_key_event()` (dispatcher)
   - `TuiAppState::handle_input_key()` (main input handler)
   - `TuiAppState::handle_responding_key()`
   - `TuiAppState::handle_interrupt()`
   - `update_slash_menu_inner()`
   - `navigate_history_inner()`
   - `delete_word_before_cursor_inner()`
   - `push_undo_snapshot()`
   - `clear_undo_stack()`

2. Implementation pattern - use trait extension:

```rust
// In input.rs
pub(super) trait InputHandler {
    fn handle_key_event(&self, key: KeyEvent);
    fn handle_input_key(&self, key: KeyEvent);
    // ... other input methods
}

impl InputHandler for TuiAppState {
    fn handle_key_event(&self, key: KeyEvent) {
        // Implementation moved here
    }
    // ...
}
```

**Verification**: `cargo build --all && cargo test --all`

### Phase 4: Extract command processing and dialogs

**Goal**: Move slash commands and dialog handlers to dedicated modules.

1. Create `app/commands.rs` containing:
   - `TuiAppState::submit_input()`
   - `TuiAppState::execute_shell_command()`
   - `TuiAppState::handle_command_inner()`
   - `TuiAppState::process_prompt()`
   - `TuiAppState::handle_test_permission_triggers()`
   - `TuiAppState::start_streaming_inner()`

2. Create `app/dialogs.rs` containing:
   - `TuiAppState::handle_permission_key()`
   - `TuiAppState::handle_trust_key()`
   - `TuiAppState::handle_thinking_key()`
   - `TuiAppState::handle_tasks_key()`
   - `TuiAppState::handle_model_picker_key()`
   - `TuiAppState::handle_export_dialog_key()`
   - `TuiAppState::handle_help_dialog_key()`
   - `TuiAppState::handle_hooks_dialog_key()`
   - `TuiAppState::handle_memory_dialog_key()`
   - `TuiAppState::confirm_permission()`
   - Permission methods: `show_permission_request()`, `show_bash_permission()`, etc.
   - Export helpers: `do_clipboard_export()`, `do_file_export()`

**Verification**: `cargo build --all && cargo test --all`

### Phase 5: Extract rendering

**Goal**: Move all rendering functions to dedicated modules.

1. Create `app/render.rs` containing:
   - `AppProps` struct
   - `App` component function
   - `render_main_content()`
   - `format_header_lines()`
   - `format_status_bar()` (pub(crate))
   - `format_status_bar_styled()`
   - `model_display_name()`
   - `extract_model_version()`
   - Content rendering: conversation, shortcuts panel, slash menu, stash indicator

2. Create `app/render_dialogs.rs` containing:
   - `render_trust_prompt()`
   - `render_thinking_dialog()`
   - `render_tasks_dialog()`
   - `render_export_dialog()`
   - `render_help_dialog()`
   - `render_hooks_dialog()`
   - `render_memory_dialog()`
   - `render_model_picker_dialog()`
   - `render_permission_dialog()`
   - `format_tool_summary()`
   - `format_compacted_summary()`

3. Create `app/runner.rs` containing:
   - `TuiApp` struct
   - `TuiApp::new()`
   - `TuiApp::run()`
   - Compatibility delegation methods

**Verification**: `cargo build --all && cargo test --all`

### Phase 6: Final cleanup and consolidation

**Goal**: Slim down `app.rs` to pure re-exports, run lints.

1. Update `app.rs` to just re-export:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TUI application state and main iocraft component.

mod app;

pub use app::{
    AppMode, AppProps, ExitHint, ExitReason, PermissionChoice,
    PermissionRequest, RenderState, StatusInfo, TrustPromptState,
    TuiApp, TuiConfig,
};

// Re-export state for test harness
pub(crate) use app::TuiAppState;
pub(crate) use app::format_status_bar;

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
```

2. Update `tui/mod.rs` imports as needed

3. Run full verification:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   - `quench check --fix`

**Verification**: `make check` passes

## Key Implementation Details

### Cross-module method access pattern

The main challenge is that `TuiAppStateInner` has many methods that need to call each other across module boundaries. Use a combination of:

1. **Direct `with_inner()` access** for simple operations:
```rust
impl TuiAppState {
    pub(super) fn with_inner<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TuiAppStateInner) -> R
    {
        f(&mut self.inner.lock())
    }
}
```

2. **Trait extensions** for logically grouped methods:
```rust
// In dialogs.rs
pub(super) trait DialogHandler {
    fn handle_trust_key(&self, key: KeyEvent);
    fn handle_thinking_key(&self, key: KeyEvent);
    // ...
}
impl DialogHandler for TuiAppState { /* ... */ }

// In input.rs - import and call
use super::dialogs::DialogHandler;
self.handle_trust_key(key);
```

### Preserving public API

All existing public exports must remain unchanged:
- `TuiApp`, `TuiConfig`, `TuiAppState`
- `AppMode`, `ExitReason`, `ExitHint`
- `RenderState`, `StatusInfo`, `PermissionRequest`, `PermissionChoice`
- `TrustPromptState`

The `app.rs` file becomes a thin facade that re-exports from `app/mod.rs`.

### Module visibility

- Types needed by other TUI modules: `pub` or `pub(crate)`
- Types only for internal use: `pub(super)` within `app/`
- `TuiAppStateInner` stays private (accessed via `with_inner()`)

### Test organization

Existing `app_tests.rs` stays as-is. Each new module gets its own `_tests.rs` file following project convention. Tests that span multiple modules remain in `app_tests.rs`.

## Verification Plan

After each phase:
1. `cargo build --all` - Compiles without errors
2. `cargo test --all` - All existing tests pass
3. `cargo clippy --all-targets --all-features -- -D warnings` - No new warnings

After Phase 6:
1. `make check` - Full lint suite passes
2. `quench check` - File size limits satisfied (all < 750 lines)
3. Manual review: Count lines per file, verify < 500 target

**Success criteria**:
- `app.rs` < 50 lines (just re-exports)
- Each `app/*.rs` file < 500 lines
- All tests pass
- No public API changes
