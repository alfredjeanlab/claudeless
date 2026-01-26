# Refactor app.rs Implementation Plan

## Overview

Refactor `crates/cli/src/tui/app.rs` (2269 lines) into smaller, testable modules. The file currently mixes rendering, dialog key handling, command processing, and the iocraft component. The goal is to reduce the main file to ~250 lines while extracting functionality into focused sibling modules under `app/`.

**Current structure:**
```
app.rs          (2269 lines) ← Problem file
app/
  input.rs      (593 lines)  - Input key handling
  state.rs      (562 lines)  - State management
  types.rs      (252 lines)  - Types and config
```

**Target structure:**
```
app.rs          (~250 lines) - Module declarations, iocraft component, public API
app/
  input.rs      (~600 lines) - Input key handling (exists)
  state.rs      (~560 lines) - State management (exists)
  types.rs      (~250 lines) - Types and config (exists)
  dialogs.rs    (~400 lines) - Dialog key handlers
  commands.rs   (~280 lines) - Command processing, streaming
  render/
    mod.rs      (~100 lines) - Re-exports, render_main_content
    content.rs  (~250 lines) - Conversation, shortcuts, slash menu rendering
    dialogs.rs  (~450 lines) - Dialog render functions
    format.rs   (~200 lines) - Header, status bar, model name formatting
```

## Project Structure

```
crates/cli/src/tui/
├── app.rs                      # Main module, iocraft component, TuiApp wrapper
├── app/
│   ├── input.rs                # (existing) Input mode key handling
│   ├── state.rs                # (existing) State management
│   ├── types.rs                # (existing) Types and configuration
│   ├── dialogs.rs              # NEW: Dialog-specific key handlers
│   ├── commands.rs             # NEW: Slash commands, prompt processing
│   └── render/
│       ├── mod.rs              # NEW: Render module, main content router
│       ├── content.rs          # NEW: Conversation, shortcuts, menu rendering
│       ├── dialogs.rs          # NEW: Dialog render functions
│       └── format.rs           # NEW: Header, status bar formatting
```

## Dependencies

No new external dependencies required. Uses existing:
- `iocraft` - TUI rendering
- `parking_lot` - Mutex
- `crossterm` - Terminal events

## Implementation Phases

### Phase 1: Extract Render Functions

**Goal:** Move all rendering code to `app/render/` submodule (~900 lines total)

**Files to create:**

1. **`app/render/mod.rs`** (~100 lines)
   - Module declarations and re-exports
   - `render_main_content()` - Main routing function that delegates to dialog renderers

2. **`app/render/format.rs`** (~200 lines)
   - `format_header_lines()` - Claude branding header
   - `format_status_bar()` - Plain status bar
   - `format_status_bar_styled()` - ANSI-styled status bar
   - `model_display_name()` - Model ID to display name mapping
   - `extract_model_version()` - Version extraction from model ID

3. **`app/render/content.rs`** (~250 lines)
   - `render_conversation_area()` - Conversation history display
   - `render_shortcuts_panel()` - Keyboard shortcuts grid
   - `render_slash_menu()` - Autocomplete menu
   - `render_stash_indicator()` - Stash status indicator
   - `render_argument_hint()` - Command argument hints

4. **`app/render/dialogs.rs`** (~450 lines)
   - `render_trust_prompt()` - Trust confirmation dialog
   - `render_thinking_dialog()` - Thinking mode toggle
   - `render_tasks_dialog()` - Background tasks list
   - `render_export_dialog()` - Export method selection
   - `render_help_dialog()` - Help/commands browser
   - `render_memory_dialog()` - Memory files list
   - `render_hooks_dialog()` - Hooks configuration
   - `render_hooks_list()` - Hook type list view
   - `render_hooks_matchers()` - Matcher configuration view
   - `render_model_picker_dialog()` - Model selection
   - `render_permission_dialog()` - Permission request dialog

**Verification:**
- `cargo check --all`
- `cargo test tui::app_tests` - All existing tests pass
- `quench check --fix` - No new lints

---

### Phase 2: Extract Dialog Key Handlers

**Goal:** Move dialog-specific key handlers from `app.rs` to `app/dialogs.rs` (~400 lines)

**Functions to move:**
- `handle_permission_key()` - Permission dialog navigation/confirmation
- `handle_trust_key()` - Trust prompt Yes/No selection
- `handle_thinking_key()` - Thinking mode enable/disable
- `handle_tasks_key()` - Task list navigation
- `handle_model_picker_key()` - Model selection
- `handle_export_dialog_key()` - Export method/filename input
- `handle_help_dialog_key()` - Help tab navigation
- `handle_hooks_dialog_key()` - Hooks view navigation
- `handle_memory_dialog_key()` - Memory file selection

**Pattern:**
```rust
// app/dialogs.rs
impl TuiAppState {
    pub(super) fn handle_permission_key(&self, key: KeyEvent) {
        // ... existing implementation
    }
    // ... other dialog handlers
}
```

**Verification:**
- `cargo test tui::app_tests::test_session_grant*` - Permission tests pass
- `cargo test tui::app_tests::meta_*` - Dialog opening tests pass

---

### Phase 3: Extract Command Processing

**Goal:** Move command/prompt processing to `app/commands.rs` (~280 lines)

**Functions to move:**
- `submit_input()` - Input submission routing
- `execute_shell_command()` - Shell command execution
- `handle_command_inner()` - Slash command dispatch
- `process_prompt()` - Prompt processing and response generation
- `handle_test_permission_triggers()` - Test permission triggers
- `start_streaming_inner()` - Response streaming initialization
- `confirm_permission()` - Permission confirmation handling

**Export helpers:**
- `do_clipboard_export()` - Clipboard export implementation
- `do_file_export()` - File export implementation
- `format_conversation_for_export()` - Export content formatting

**Verification:**
- `cargo test tui::app_tests::typing_clears_exit_hint` - Input processing works
- `cargo test tui::app_tests::test_clear_command_clears_session_grants` - Commands work

---

### Phase 4: Finalize Module Structure

**Goal:** Clean up `app.rs` to ~250 lines containing only:

1. Module declarations (imports from submodules)
2. `AppProps` struct
3. `App` iocraft component function
4. `TuiApp` wrapper struct with public API methods
5. Public permission API methods (`show_permission_request`, etc.)

**app.rs structure after refactoring:**
```rust
// Module declarations
mod commands;
mod dialogs;
mod input;
mod render;
mod state;
mod types;

// Re-exports
pub use state::TuiAppState;
pub use types::{AppMode, ExitReason, ...};

// Import helpers
use commands::{confirm_permission, ...};
use render::{render_main_content, format_status_bar};

// iocraft component (~80 lines)
#[derive(Default, Props)]
pub struct AppProps { ... }

#[component]
pub fn App(...) -> impl Into<AnyElement<'static>> { ... }

// TuiApp wrapper (~140 lines)
pub struct TuiApp { ... }
impl TuiApp { ... }

// Permission public API (~50 lines)
impl TuiAppState {
    pub fn show_permission_request(&self, ...) { ... }
    pub fn show_bash_permission(&self, ...) { ... }
    pub fn show_edit_permission(&self, ...) { ... }
    pub fn show_write_permission(&self, ...) { ... }
}
```

**Verification:**
- `make check` - All checks pass
- `cargo test --all` - All tests pass
- `quench check` - No line count violations

---

### Phase 5: Add Unit Tests

**Goal:** Add focused unit tests in sibling `_tests.rs` files

**Test files to create:**
- `app/render/format_tests.rs` - Header/status formatting tests
- `app/render/content_tests.rs` - Content rendering tests (optional, may rely on integration tests)
- `app/commands_tests.rs` - Command dispatch tests
- `app/dialogs_tests.rs` - Dialog key handler tests

**Example test pattern:**
```rust
// app/render/format_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn model_display_name_short_aliases() {
    assert_eq!(model_display_name("opus"), "Opus 4.5");
    assert_eq!(model_display_name("sonnet"), "Sonnet 4.5");
    assert_eq!(model_display_name("haiku"), "Haiku 4.5");
}

#[test]
fn model_display_name_full_id() {
    assert_eq!(
        model_display_name("claude-opus-4-5-20251101"),
        "Opus 4.5"
    );
}
```

**Verification:**
- `cargo test app::render::format_tests` - New tests pass
- `cargo test app::commands_tests` - New tests pass

## Key Implementation Details

### Module Visibility Pattern

Use `pub(super)` for cross-module access within `app/`:
```rust
// app/dialogs.rs
impl TuiAppState {
    /// Handle key events in permission mode
    pub(super) fn handle_permission_key(&self, key: KeyEvent) {
        // Can access inner via self.inner.lock()
    }
}
```

### Render Function Signatures

All render functions are pure functions taking `RenderState` or specific dialog state:
```rust
// app/render/dialogs.rs
pub(crate) fn render_trust_prompt(prompt: &TrustPromptState, width: usize) -> AnyElement<'static> {
    // ...
}
```

### Format Function Exports

Export format functions for use in tests (like existing `format_status_bar`):
```rust
// app/render/mod.rs
pub(crate) use format::{format_header_lines, format_status_bar, format_status_bar_styled};
```

### State Access Pattern

Methods that need mutable state access follow the existing pattern:
```rust
fn handle_permission_key(&self, key: KeyEvent) {
    let mut inner = self.inner.lock();
    // ... modify inner
    drop(inner);  // Explicit drop before calling other methods
    self.confirm_permission();
}
```

## Verification Plan

### After Each Phase

1. **Compile check:** `cargo check --all`
2. **Unit tests:** `cargo test tui::app_tests`
3. **Lint check:** `cargo clippy --all-targets --all-features -- -D warnings`
4. **Format check:** `cargo fmt --all -- --check`

### Final Verification

```bash
make check
```

This runs:
- `make lint` (shellcheck)
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `quench check --fix`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

### Line Count Targets

After refactoring:
- `app.rs`: ~250 lines (was 2269)
- `app/render/mod.rs`: ~100 lines
- `app/render/format.rs`: ~200 lines
- `app/render/content.rs`: ~250 lines
- `app/render/dialogs.rs`: ~450 lines
- `app/dialogs.rs`: ~400 lines
- `app/commands.rs`: ~280 lines

All files should be under 750 lines, satisfying `quench check`.

## Risk Mitigation

1. **Test coverage:** Existing tests in `app_tests.rs` provide regression coverage
2. **Incremental approach:** Each phase is independently verifiable
3. **Public API preserved:** `TuiApp` wrapper maintains backward compatibility
4. **No behavior changes:** Pure code reorganization, no logic modifications
