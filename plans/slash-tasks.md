# Implementation Plan: /tasks Slash Command

## Overview

Implement the `/tasks` slash command to display a dialog showing background tasks. When executed, it opens a bordered dialog with:
- Header: "Background tasks"
- Content: List of running tasks, or "No tasks currently running" when empty
- Footer: "↑/↓ to select · Enter to view · Esc to close"

Pressing Escape dismisses the dialog and shows "Background tasks dialog dismissed".

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add AppMode::TasksDialog, /tasks handler, key handling
│   ├── slash_menu.rs          # Add "tasks" command to registry
│   └── widgets/
│       ├── mod.rs             # Export TasksDialog
│       └── tasks.rs           # NEW: TasksDialog struct
└── state/                     # (No changes - tasks are ephemeral, not persisted)

crates/cli/tests/
├── tui_tasks.rs               # Remove #[ignore] from 5 tests
└── fixtures/tui/v2.1.12/
    └── tasks_empty_dialog.txt # Already exists with expected output
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm` for keyboard input handling
- Box-drawing characters for dialog borders (already used elsewhere)

## Implementation Phases

### Phase 1: Create TasksDialog Widget

Create a new widget module for the tasks dialog state.

**File:** `crates/cli/src/tui/widgets/tasks.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

/// Background task info for display
#[derive(Clone, Debug)]
pub struct TaskInfo {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
}

/// State for the /tasks dialog
#[derive(Clone, Debug, Default)]
pub struct TasksDialog {
    /// List of background tasks
    pub tasks: Vec<TaskInfo>,
    /// Currently selected task index
    pub selected_index: usize,
}

impl TasksDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.tasks.is_empty() && self.selected_index < self.tasks.len() - 1 {
            self.selected_index += 1;
        }
    }
}
```

**File:** `crates/cli/src/tui/widgets/mod.rs`

Add export:
```rust
pub mod tasks;
pub use tasks::TasksDialog;
```

**Verification:** `cargo build` succeeds.

---

### Phase 2: Add TasksDialog Mode to TUI State

Extend the application state and mode enum to support the tasks dialog.

**File:** `crates/cli/src/tui/app.rs`

1. Add import at top:
   ```rust
   use crate::tui::widgets::tasks::TasksDialog;
   ```

2. Add variant to `AppMode` enum (around line 130):
   ```rust
   pub enum AppMode {
       // ... existing variants
       TasksDialog,
   }
   ```

3. Add field to `TuiAppStateInner` struct:
   ```rust
   pub tasks_dialog: Option<TasksDialog>,
   ```

4. Initialize in constructor:
   ```rust
   tasks_dialog: None,
   ```

5. Add to `RenderState` struct:
   ```rust
   pub tasks_dialog: Option<TasksDialog>,
   ```

6. Update `render_state()` method to include:
   ```rust
   tasks_dialog: inner.tasks_dialog.clone(),
   ```

**Verification:** `cargo build` succeeds.

---

### Phase 3: Register /tasks Command

Add the `/tasks` command to the slash command registry.

**File:** `crates/cli/src/tui/slash_menu.rs`

Add entry to `COMMANDS` array in alphabetical position (after `terminal-setup`, before `thinking`):
```rust
SlashCommand {
    name: "tasks",
    description: "List and manage background tasks",
    argument_hint: None,
},
```

**Verification:**
- `cargo build`
- Typing `/tasks` in TUI shows it in autocomplete (test `test_tasks_in_autocomplete`)

---

### Phase 4: Implement /tasks Command Handler

Add the command handler to open the tasks dialog.

**File:** `crates/cli/src/tui/app.rs`

Add match arm in `handle_command_inner()` (before the `_ =>` catch-all):
```rust
"/tasks" => {
    inner.mode = AppMode::TasksDialog;
    inner.tasks_dialog = Some(TasksDialog::new());
}
```

**Verification:** Running `/tasks` enters dialog mode (no rendering yet).

---

### Phase 5: Implement Dialog Rendering

Create the rendering function for the tasks dialog that matches the fixture format.

**File:** `crates/cli/src/tui/app.rs`

Add rendering function:
```rust
/// Render tasks dialog with border
fn render_tasks_dialog(dialog: &TasksDialog, width: usize) -> AnyElement<'static> {
    // Inner width accounts for box borders (│ on each side)
    let inner_width = width.saturating_sub(2);

    // Build content string
    let content = if dialog.is_empty() {
        "No tasks currently running".to_string()
    } else {
        // Format task list with selection indicator
        dialog.tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let indicator = if i == dialog.selected_index { "❯ " } else { "  " };
                format!("{}{}", indicator, task.description)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Box drawing chars
    let h_line = "─".repeat(inner_width);
    let top_border = format!("╭{}╮", h_line);
    let bottom_border = format!("╰{}╯", h_line);

    // Pad content lines to fill width
    let pad_line = |s: &str| {
        let visible_len = s.chars().count();
        let padding = inner_width.saturating_sub(visible_len);
        format!("│{}{}│", s, " ".repeat(padding))
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            Text(content: top_border)
            Text(content: pad_line(" Background tasks"))
            Text(content: pad_line(&format!(" {}", content)))
            Text(content: bottom_border)
            Text(content: "  ↑/↓ to select · Enter to view · Esc to close")
        }
    }.into()
}
```

Add rendering branch in `render_main_content()` (before other dialog checks around line 1711):
```rust
// If showing tasks dialog, render just the dialog
if state.mode == AppMode::TasksDialog {
    if let Some(ref dialog) = state.tasks_dialog {
        return render_tasks_dialog(dialog, width);
    }
}
```

**Verification:**
- `test_tasks_empty_shows_no_tasks_message` passes
- `test_tasks_dialog_has_controls` passes
- `test_tasks_empty_matches_fixture` passes

---

### Phase 6: Implement Key Event Handling

Handle keyboard input when in TasksDialog mode.

**File:** `crates/cli/src/tui/app.rs`

1. Add match arm in main key dispatcher (around line 504):
   ```rust
   AppMode::TasksDialog => self.handle_tasks_key(key),
   ```

2. Add handler method:
   ```rust
   fn handle_tasks_key(&self, key: KeyEvent) -> Option<TuiAction> {
       let mut guard = self.inner.lock().unwrap();
       let inner = &mut *guard;

       match key.code {
           KeyCode::Esc => {
               // Close dialog with dismissal message
               inner.mode = AppMode::Input;
               inner.tasks_dialog = None;
               inner.response_content = "Background tasks dialog dismissed".to_string();
               inner.is_command_output = true;
               Some(TuiAction::Redraw)
           }
           KeyCode::Up => {
               if let Some(ref mut dialog) = inner.tasks_dialog {
                   dialog.move_selection_up();
               }
               Some(TuiAction::Redraw)
           }
           KeyCode::Down => {
               if let Some(ref mut dialog) = inner.tasks_dialog {
                   dialog.move_selection_down();
               }
               Some(TuiAction::Redraw)
           }
           KeyCode::Enter => {
               // Future: view selected task details
               // For now, just close the dialog
               inner.mode = AppMode::Input;
               inner.tasks_dialog = None;
               Some(TuiAction::Redraw)
           }
           _ => None,
       }
   }
   ```

**Verification:** `test_tasks_dialog_dismiss_with_escape` passes.

---

### Phase 7: Enable Tests and Final Verification

Remove `#[ignore]` from all tests and verify.

**File:** `crates/cli/tests/tui_tasks.rs`

Remove `#[ignore]` attribute from these tests:
- `test_tasks_empty_shows_no_tasks_message` (line 26)
- `test_tasks_dialog_has_controls` (line 63)
- `test_tasks_empty_matches_fixture` (line 108)
- `test_tasks_dialog_dismiss_with_escape` (line 141)
- `test_tasks_in_autocomplete` (line 184)

**Verification:**
```bash
cargo test --test tui_tasks
make check
```

## Key Implementation Details

### Dialog Box Format

The fixture uses box-drawing characters for a bordered dialog:
```
╭────────────────────────────────────────────────────────────────────────────────────╮
│ Background tasks                                                                   │
│ No tasks currently running                                                         │
╰────────────────────────────────────────────────────────────────────────────────────╯
  ↑/↓ to select · Enter to view · Esc to close
```

Key formatting requirements:
- Box width matches terminal width (120 chars in fixture)
- Header: "Background tasks" with leading space
- Content padded with trailing spaces to fill width
- Footer is indented 2 spaces, outside the box

### State Machine

```
AppMode::Input
    │
    ├── /tasks command
    │       ↓
    AppMode::TasksDialog
        │
        ├── Escape key → AppMode::Input + dismiss message
        ├── Up/Down keys → Update selection (redraw)
        └── Enter key → AppMode::Input (view task, future feature)
```

### Dismiss Message

When Escape is pressed, the TUI shows:
```
❯ /tasks
  ⎿  Background tasks dialog dismissed
```

This matches the pattern used by other commands - setting `is_command_output = true` and `response_content`.

## Verification Plan

1. **Unit Tests:**
   - All 5 tests in `tui_tasks.rs` pass

2. **Integration:**
   - `make check` passes (includes lint, format, clippy, tests, build, audit)

3. **Manual Testing:**
   - Launch TUI: `cargo run -- --scenario test`
   - Type `/tasks` - should show bordered dialog with "No tasks currently running"
   - Press `Escape` - dialog closes with "Background tasks dialog dismissed"
   - Type `/tasks` partially - should appear in autocomplete with description

## Files Modified Summary

| File | Changes |
|------|---------|
| `crates/cli/src/tui/widgets/tasks.rs` | NEW: TasksDialog struct and methods |
| `crates/cli/src/tui/widgets/mod.rs` | Export TasksDialog |
| `crates/cli/src/tui/app.rs` | Add TasksDialog mode, handler, key handling, rendering |
| `crates/cli/src/tui/slash_menu.rs` | Add "tasks" to COMMANDS array |
| `crates/cli/tests/tui_tasks.rs` | Remove `#[ignore]` from 5 tests |
