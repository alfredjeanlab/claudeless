# Implementation Plan: /todos Slash Command

**Root Feature:** `cl-c886`

## Overview

Implement the `/todos` slash command and `Ctrl+T` shortcut to display the current todo list in the TUI. When no todos exist, displays "No todos currently tracked". The `TodoState` infrastructure already exists; this plan focuses on wiring it to the TUI layer.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs             # Add /todos handler + Ctrl+T keybinding
│   └── slash_menu.rs      # Add "todos" command to registry
└── state/
    └── todos.rs           # Existing TodoState (no changes needed)

crates/cli/tests/
├── tui_todos.rs           # Enable 5 ignored tests
└── fixtures/tui/v2.1.12/
    └── todos_empty.txt    # Already exists with expected output
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm` for keyboard input handling
- `TodoState` from `crate::state::todos`

## Implementation Phases

### Phase 1: Add TodoState to TuiAppStateInner

Add todo state storage to the TUI application state.

**File:** `crates/cli/src/tui/app.rs`

**Changes:**
1. Add import at top of file:
   ```rust
   use crate::state::todos::TodoState;
   ```

2. Add field to `TuiAppStateInner` struct (around line 282):
   ```rust
   /// Todo list state
   pub todos: TodoState,
   ```

3. Initialize in `TuiAppState::new()` (find the `TuiAppStateInner` initialization):
   ```rust
   todos: TodoState::new(),
   ```

**Verification:** Code compiles without errors.

---

### Phase 2: Register /todos Command

Add the `/todos` command to the slash command registry.

**File:** `crates/cli/src/tui/slash_menu.rs`

**Changes:**
Add entry to `COMMANDS` array in alphabetical position (after `terminal-setup`, before `vim`):
```rust
SlashCommand {
    name: "todos",
    description: "Show the current todo list",
    argument_hint: None,
},
```

**Verification:**
- Run `cargo build`
- Type `/todos` in TUI - should appear in autocomplete menu

---

### Phase 3: Implement /todos Command Handler

Add the command handler to display todo items.

**File:** `crates/cli/src/tui/app.rs`

**Changes:**
Add match arm in `handle_command_inner()` (around line 1017, before the `_ =>` catch-all):

```rust
"/todos" => {
    if inner.todos.is_empty() {
        inner.response_content = "No todos currently tracked".to_string();
    } else {
        let mut lines = Vec::new();
        for item in &inner.todos.items {
            let status = match item.status {
                TodoStatus::Pending => "[ ]",
                TodoStatus::InProgress => "[*]",
                TodoStatus::Completed => "[x]",
            };
            lines.push(format!("{} {}", status, item.content));
        }
        inner.response_content = lines.join("\n");
    }
}
```

Also add the import at the top (if not already present):
```rust
use crate::state::todos::TodoStatus;
```

**Verification:**
- `cargo test --test tui_todos test_tui_todos_command_shows_empty_message` should pass (after removing `#[ignore]`)

---

### Phase 4: Implement Ctrl+T Shortcut

Wire the `Ctrl+T` keyboard shortcut to show todos.

**File:** `crates/cli/src/tui/app.rs`

**Changes:**
Add match arm in `handle_input_key()` (around line 562, with other Ctrl handlers):

```rust
// Ctrl+T - Show todos
(m, KeyCode::Char('t')) if m.contains(KeyModifiers::CONTROL) => {
    // Only show if there are todos to display
    if !inner.todos.is_empty() {
        let mut lines = Vec::new();
        for item in &inner.todos.items {
            let status = match item.status {
                TodoStatus::Pending => "[ ]",
                TodoStatus::InProgress => "[*]",
                TodoStatus::Completed => "[x]",
            };
            lines.push(format!("{} {}", status, item.content));
        }
        inner.response_content = lines.join("\n");
        inner.is_command_output = true;
        inner.conversation_display = "Todo List".to_string();
    }
    // When no todos, do nothing (no visible change)
}
```

**Verification:**
- `cargo test --test tui_todos test_tui_ctrl_t_no_change_when_no_todos` should pass

---

### Phase 5: Extract Formatting Helper

Refactor to avoid code duplication between `/todos` and `Ctrl+T`.

**File:** `crates/cli/src/tui/app.rs`

**Changes:**
Add helper function (near other helper methods):

```rust
/// Format todo items for display.
fn format_todos(todos: &TodoState) -> String {
    if todos.is_empty() {
        "No todos currently tracked".to_string()
    } else {
        todos.items
            .iter()
            .map(|item| {
                let status = match item.status {
                    TodoStatus::Pending => "[ ]",
                    TodoStatus::InProgress => "[*]",
                    TodoStatus::Completed => "[x]",
                };
                format!("{} {}", status, item.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

Update `/todos` handler:
```rust
"/todos" => {
    inner.response_content = Self::format_todos(&inner.todos);
}
```

Update `Ctrl+T` handler:
```rust
(m, KeyCode::Char('t')) if m.contains(KeyModifiers::CONTROL) => {
    if !inner.todos.is_empty() {
        inner.response_content = Self::format_todos(&inner.todos);
        inner.is_command_output = true;
        inner.conversation_display = "Todo List".to_string();
    }
}
```

**Verification:** All tests still pass.

---

### Phase 6: Enable Tests and Final Verification

Remove `#[ignore]` from all tests and verify.

**File:** `crates/cli/tests/tui_todos.rs`

**Changes:**
Remove `#[ignore]` from these tests:
- `test_tui_ctrl_t_no_change_when_no_todos` (line 30)
- `test_tui_shortcuts_shows_ctrl_t_for_todos` (line 64)
- `test_tui_todos_command_shows_empty_message` (line 101)
- `test_tui_ctrl_t_shows_active_todos` (line 138)
- `test_tui_todos_command_shows_active_items` (line 166)

**Note:** The last two tests (`test_tui_ctrl_t_shows_active_todos` and `test_tui_todos_command_shows_active_items`) are placeholder tests that currently just start a session and kill it. They will pass but don't actually verify todo display with active items. Consider either:
1. Leaving them as placeholders for future TodoWrite integration
2. Updating them to programmatically add todos before testing

**Verification:**
```bash
cargo test --test tui_todos
make check
```

## Key Implementation Details

### Command Output Format

The `/todos` command uses the elbow connector format:
```
❯ /todos
  ⎿  No todos currently tracked
```

This is handled automatically by setting `inner.is_command_output = true` and `inner.response_content`.

### Status Indicators

| Status | Indicator |
|--------|-----------|
| Pending | `[ ]` |
| InProgress | `[*]` |
| Completed | `[x]` |

### Ctrl+T Behavior

Per observed Claude Code behavior:
- When no todos exist, pressing `Ctrl+T` does nothing (no visible change)
- When todos exist, displays the todo list similar to `/todos` output
- The shortcut is listed in the shortcuts panel (`?`) as "ctrl + t to show todos"

### Shortcut Panel

The shortcut text "ctrl + t to show todos" already exists in `crates/cli/src/tui/shortcuts.rs` (line 52). No changes needed there.

## Verification Plan

1. **Unit Tests:**
   - All 5 tests in `tui_todos.rs` pass

2. **Integration:**
   - `make check` passes (includes lint, format, clippy, tests, build, audit)

3. **Manual Testing:**
   - Launch TUI: `cargo run -- --scenario test`
   - Type `/todos` - should show "No todos currently tracked"
   - Press `?` - should show "ctrl + t to show todos" in shortcuts
   - Press `Ctrl+T` with no todos - no visible change

## Files Modified Summary

| File | Changes |
|------|---------|
| `crates/cli/src/tui/app.rs` | Add TodoState field, /todos handler, Ctrl+T handler, format helper |
| `crates/cli/src/tui/slash_menu.rs` | Add "todos" to COMMANDS array |
| `crates/cli/tests/tui_todos.rs` | Remove `#[ignore]` from 5 tests |
