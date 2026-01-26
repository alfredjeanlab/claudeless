# Slash Commands Cleanup Plan

**Root Feature:** `cl-79ab`

## Overview

Review, clean up, and enhance the slash commands implementation chain. This covers the command dispatcher (`app.rs`), widget state modules (`widgets/*.rs`), autocomplete menu (`slash_menu.rs`), and associated tests.

## Project Structure

```
crates/cli/src/tui/
├── app.rs                    # Command dispatcher (handle_command_inner)
├── slash_menu.rs             # Command registry and fuzzy matching
├── slash_menu_tests.rs       # Unit tests for autocomplete
└── widgets/
    ├── mod.rs                # Widget exports
    ├── context.rs            # /context token usage display
    ├── export.rs             # /export dialog state
    ├── export_tests.rs       # Export unit tests
    ├── help.rs               # /help dialog state
    ├── help_tests.rs         # Help unit tests
    ├── hooks.rs              # /hooks dialog state
    ├── hooks_tests.rs        # Hooks unit tests
    ├── memory.rs             # /memory dialog state
    ├── memory_tests.rs       # Memory unit tests
    └── tasks.rs              # /tasks dialog state (NO tests)

crates/cli/tests/
├── tui_clear.rs              # /clear integration tests
├── tui_compacting.rs         # /compact integration tests (3 ignored)
├── tui_context.rs            # /context integration tests
├── tui_exit.rs               # /exit integration tests
├── tui_export.rs             # /export integration tests (1 ignored)
├── tui_fork.rs               # /fork integration tests
├── tui_help.rs               # /help integration tests
├── tui_hooks.rs              # /hooks integration tests
├── tui_memory.rs             # /memory integration tests
├── tui_tasks.rs              # /tasks integration tests
└── tui_todos.rs              # /todos integration tests
```

## Dependencies

No new external dependencies required. Uses existing:
- `chrono` (timestamp generation in export dialog)
- Standard library collections

## Implementation Phases

### Phase 1: Code Audit and Dead Code Removal

**Milestone**: Clean codebase with no unused imports or dead code

1. Run `cargo clippy --all-targets --all-features -- -D warnings` and fix any issues
2. Check for unused imports across all slash command files
3. Identify and remove any dead code or unused functions
4. Verify no `#[allow(dead_code)]` attributes hiding unused items

**Files to audit**:
- `app.rs` (lines 1440-1537, handle_command_inner)
- All `widgets/*.rs` files
- `slash_menu.rs`

### Phase 2: Consolidate Navigation Patterns

**Milestone**: Consistent navigation behavior across all dialogs

The dialog widgets have duplicated scroll-aware navigation logic. Extract common pattern:

**Current duplication** (hooks.rs, memory.rs both have):
```rust
pub fn select_prev(&mut self) {
    if self.selected_index == 0 {
        self.selected_index = total - 1;
        // Scroll to bottom
        if total > self.visible_count {
            self.scroll_offset = total - self.visible_count;
        }
    } else {
        self.selected_index -= 1;
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
    }
}
```

**Tasks** (in priority order):
1. Make `tasks.rs` navigation wrap like other dialogs (currently doesn't wrap)
2. Extract scroll-aware navigation into a reusable trait or helper:
   ```rust
   // widgets/scrollable.rs
   pub struct ScrollState {
       pub selected_index: usize,
       pub scroll_offset: usize,
       pub visible_count: usize,
       pub total_items: usize,
   }

   impl ScrollState {
       pub fn select_prev(&mut self) { /* common logic */ }
       pub fn select_next(&mut self) { /* common logic */ }
   }
   ```
3. Update `HooksDialog`, `MemoryDialog`, and `TasksDialog` to use shared helper

### Phase 3: Standardize Error Handling

**Milestone**: Consistent error messages and handling patterns

**Current state**: `/fork` has proper error handling, other commands vary

**Tasks**:
1. Define error message format: `"Failed to <action>: <reason>"`
2. Update `handle_command_inner` with consistent error handling:
   ```rust
   match cmd.as_str() {
       "/clear" => {
           // Clear could fail if sessions lock is poisoned
           // Currently ignores potential errors
       }
       "/fork" => {
           // Good: has proper error message
       }
       _ => {
           // Good: "Unknown command: {}"
       }
   }
   ```
3. Add error handling for edge cases:
   - `/export` when no conversation exists
   - `/compact` when already compacting
   - `/context` when context calculation fails

### Phase 4: Add Missing Unit Tests

**Milestone**: All widget modules have corresponding `_tests.rs` files

**Currently missing**:
- `tasks_tests.rs` - No unit tests for TasksDialog

**Tasks**:
1. Create `widgets/tasks_tests.rs`:
   ```rust
   #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
   use super::*;

   #[test]
   fn new_dialog_is_empty() {
       let dialog = TasksDialog::new();
       assert!(dialog.is_empty());
       assert_eq!(dialog.selected_index, 0);
   }

   #[test]
   fn move_selection_wraps_down() {
       let mut dialog = TasksDialog::new();
       dialog.tasks = vec![/* test tasks */];
       // Test wrapping behavior
   }

   #[test]
   fn move_selection_wraps_up() {
       // Test wrapping when at index 0
   }
   ```

2. Add test coverage for edge cases in existing tests:
   - `export_tests.rs`: Test filename with special characters
   - `help_tests.rs`: Test with zero total_commands
   - `memory_tests.rs`: Test empty entries list
   - `hooks_tests.rs`: Test selecting DisableAllHooks (no matchers)

### Phase 5: Resolve or Document Ignored Tests

**Milestone**: All ignored tests either pass or have documented rationale

**25 ignored tests across files**:

| File | Count | Reason |
|------|-------|--------|
| `tui_thinking.rs` | 4 | Simulator rendering differs from fixture |
| `tui_setup.rs` | 8 | Setup flow not fully implemented |
| `smoke_test.rs` | 6 | CLI flags not implemented |
| `tui_compacting.rs` | 3 | Fixture capture method differs |
| `tui_shell_mode.rs` | 2 | Version-specific rendering |
| `tui_export.rs` | 1 | Autocomplete is separate feature |
| `tui_interaction.rs` | 1 | Status bar display differs |

**Tasks**:
1. Categorize ignored tests:
   - **Fixable**: Tests that can pass with code changes
   - **Fixture update needed**: Tests that need updated reference fixtures
   - **Deferred**: Tests for unimplemented features

2. For each fixable test, create tracking comment:
   ```rust
   #[ignore] // TODO(slash-cleanup): Fix simulator rendering to match fixture
   ```

3. Update tests that can pass now:
   - Review `tui_export.rs:test_tui_export_command_shows_autocomplete` - if autocomplete now works, un-ignore

4. Document deferred tests with clear rationale:
   ```rust
   #[ignore] // DEFERRED: Requires setup flow implementation (see plans/setup-flow.md)
   ```

### Phase 6: Integration Test Edge Cases

**Milestone**: Comprehensive edge case coverage for slash commands

**Current gaps identified**:
1. No test for `/clear` when session is already empty
2. No test for `/fork` success case (only error case)
3. No test for `/help` tab switching with arrow keys
4. No test for `/hooks` matchers view
5. No test for `/memory` with no CLAUDE.md files

**Tasks**:
1. Add edge case tests to existing files:
   ```rust
   // tui_clear.rs
   #[test]
   fn test_clear_empty_session_succeeds() { /* ... */ }

   // tui_fork.rs
   #[test]
   fn test_fork_success_with_conversation() { /* ... */ }

   // tui_help.rs
   #[test]
   fn test_help_tab_navigation_with_arrows() { /* ... */ }
   ```

2. Add tests for dialog keyboard interactions:
   - Tab/Shift+Tab for tab switching
   - Enter for confirmation
   - Escape for cancellation
   - Arrow keys for navigation within dialogs

## Key Implementation Details

### Dialog State Pattern

All dialog widgets follow this pattern:
```rust
pub struct XyzDialog {
    pub selected_index: usize,     // Current selection
    pub scroll_offset: usize,      // For viewports
    pub visible_count: usize,      // Items visible at once
    // ... dialog-specific state
}

impl XyzDialog {
    pub fn new() -> Self { /* initialize */ }
    pub fn select_next(&mut self) { /* with wrap & scroll */ }
    pub fn select_prev(&mut self) { /* with wrap & scroll */ }
}
```

### Command Dispatch Pattern

Commands are handled in `app.rs:handle_command_inner`:
```rust
fn handle_command_inner(inner: &mut TuiAppStateInner, input: &str) {
    let cmd = input.trim().to_lowercase();
    inner.is_command_output = true;
    inner.conversation_display = format!("❯ {}", input.trim());

    match cmd.as_str() {
        "/xyz" => {
            // Dialog: set mode + create dialog state
            inner.mode = AppMode::XyzDialog;
            inner.xyz_dialog = Some(XyzDialog::new());
        }
        "/abc" => {
            // Content: set response_content
            inner.response_content = Self::format_abc();
        }
        _ => {
            inner.response_content = format!("Unknown command: {}", input);
        }
    }
}
```

### Test Patterns

**Unit tests** (sibling `_tests.rs` files):
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn descriptive_test_name() {
    // Arrange
    let mut dialog = XyzDialog::new();

    // Act
    dialog.select_next();

    // Assert
    assert_eq!(dialog.selected_index, 1);
}
```

**Integration tests** (tmux-based scenarios):
```rust
#[test]
fn test_slash_xyz_shows_dialog() {
    let scenario = write_scenario(r#"
        name = "test"
        [[responses]]
        prompt = ".*"
        response = "OK"
    "#);

    let output = run_scenario(&scenario, &["send-keys", "/xyz", "Enter"]);
    assert!(output.contains("expected content"));
}
```

## Verification Plan

### Phase Gate: Each Phase

1. `cargo fmt --all -- --check` passes
2. `cargo clippy --all-targets --all-features -- -D warnings` passes
3. `cargo test --all` passes
4. No regressions in existing tests

### Final Verification

1. Run full `make check`:
   - `make lint`
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   - `cargo build --all`
   - `cargo audit`
   - `cargo deny check`

2. Review test coverage:
   - All widget modules have `_tests.rs` files
   - All slash commands have integration tests
   - All previously-ignored tests either pass or have documented rationale

3. Code review checklist:
   - [ ] No dead code or unused imports
   - [ ] Consistent navigation patterns across dialogs
   - [ ] Consistent error handling and messages
   - [ ] No duplicate logic that should be shared
   - [ ] All tests pass without `#[ignore]` or with documented reason
