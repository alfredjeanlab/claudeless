# Implementation Plan: Ctrl+T Shortcut for Todos

**Status:** ✅ Complete (commit cdece22)

## Overview

Implement `Ctrl+T` keyboard shortcut to display the current todo list in the TUI. When no todos exist, the shortcut does nothing (no visible change). When todos exist, displays them with status indicators. Also includes the `/todos` slash command.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs             # Ctrl+T keybinding + /todos handler + format_todos()
│   ├── shortcuts.rs       # "ctrl + t to show todos" shortcut entry (pre-existing)
│   └── slash_menu.rs      # "todos" command registration
└── state/
    └── todos.rs           # TodoState (no changes needed)

crates/cli/tests/
└── tui_todos.rs           # 5 TUI integration tests
```

## Dependencies

No new dependencies. Uses existing:
- `crossterm::event::{KeyCode, KeyModifiers}` for keyboard input
- `crate::state::todos::{TodoState, TodoStatus}` for todo data

## Implementation Phases

### Phase 1: Add TodoState to TuiAppStateInner

Add todo state storage to the TUI application state.

**File:** `crates/cli/src/tui/app.rs`

```rust
use crate::state::todos::TodoState;

// In TuiAppStateInner:
pub todos: TodoState,

// In TuiAppState::new():
todos: TodoState::new(),
```

**Verification:** Code compiles.

---

### Phase 2: Register /todos Command

**File:** `crates/cli/src/tui/slash_menu.rs`

```rust
SlashCommand {
    name: "todos",
    description: "Show the current todo list",
    argument_hint: None,
},
```

**Verification:** `/todos` appears in autocomplete menu.

---

### Phase 3: Implement Format Helper

**File:** `crates/cli/src/tui/app.rs`

```rust
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

---

### Phase 4: Implement /todos Command Handler

**File:** `crates/cli/src/tui/app.rs`

In `handle_command_inner()`:
```rust
"/todos" => {
    inner.response_content = Self::format_todos(&inner.todos);
}
```

**Verification:** `/todos` shows "No todos currently tracked".

---

### Phase 5: Implement Ctrl+T Shortcut

**File:** `crates/cli/src/tui/app.rs`

In `handle_input_key()`:
```rust
// Ctrl+T - Show todos (only when todos exist)
(m, KeyCode::Char('t')) if m.contains(KeyModifiers::CONTROL) => {
    if !inner.todos.is_empty() {
        inner.response_content = Self::format_todos(&inner.todos);
        inner.is_command_output = true;
        inner.conversation_display = "Todo List".to_string();
    }
    // When no todos, do nothing (no visible change)
}
```

**Verification:** `Ctrl+T` with no todos causes no visible change.

---

### Phase 6: Enable Tests

**File:** `crates/cli/tests/tui_todos.rs`

Remove `#[ignore]` from:
- `test_tui_ctrl_t_no_change_when_no_todos`
- `test_tui_shortcuts_shows_ctrl_t_for_todos`
- `test_tui_todos_command_shows_empty_message`
- `test_tui_ctrl_t_shows_active_todos`
- `test_tui_todos_command_shows_active_items`

**Verification:** `cargo test --test tui_todos` passes.

## Key Implementation Details

### Ctrl+T vs /todos Behavior

| Action | No Todos | Has Todos |
|--------|----------|-----------|
| `Ctrl+T` | No visible change | Shows todo list |
| `/todos` | "No todos currently tracked" | Shows todo list |

### Status Indicators

| Status | Indicator |
|--------|-----------|
| Pending | `[ ]` |
| InProgress | `[*]` |
| Completed | `[x]` |

### Shortcut Panel Entry

Pre-existing in `shortcuts.rs`:
```rust
Shortcut {
    keys: "ctrl + t to show todos",
    column: 1,  // Center column
},
```

## Verification Plan

1. **Unit Tests:**
   ```bash
   cargo test --test tui_todos
   ```
   All 5 tests pass (26 total with common module tests).

2. **Full Check:**
   ```bash
   make check
   ```

3. **Manual Testing:**
   - Press `?` to show shortcuts - verify "ctrl + t to show todos" appears
   - Press `Ctrl+T` with no todos - no change
   - Type `/todos` - shows "No todos currently tracked"

## Files Modified

| File | Changes |
|------|---------|
| `crates/cli/src/tui/app.rs` | +TodoState field, +Ctrl+T handler, +/todos handler, +format_todos() |
| `crates/cli/src/tui/slash_menu.rs` | +todos command entry |
| `crates/cli/tests/tui_todos.rs` | Remove `#[ignore]` from 5 tests |
