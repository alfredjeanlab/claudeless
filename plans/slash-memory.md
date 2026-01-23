# Implementation Plan: /memory Slash Command

**Root Feature:** `cl-9f45`

## Overview

Implement the `/memory` slash command to display a dialog for viewing and managing conversation memory (CLAUDE.md instruction files). The command shows CLAUDE.md files from various sources: project-level (`.claude/CLAUDE.md`), user-level (`~/.claude/CLAUDE.md`), and enterprise/organization-level files.

Before implementing, we must capture the **real behavior** from Claude Code using tmux as documented in `docs/prompts/tui-test-capture-guide.md`.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add AppMode::MemoryDialog, handler, key handling, rendering
│   └── widgets/
│       ├── mod.rs             # Export MemoryDialog, MemorySource
│       ├── memory.rs          # NEW: MemoryDialog struct, MemorySource enum
│       └── memory_tests.rs    # NEW: Unit tests for MemoryDialog

crates/cli/tests/
├── tui_memory.rs              # NEW: Integration tests for /memory command
└── fixtures/tui/v2.1.12/
    ├── memory_dialog.txt      # NEW: Reference fixture
    └── BEHAVIORS.md           # Update with /memory behavior
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm` for keyboard input handling
- Box-drawing characters for dialog borders (if used)
- Existing state management from `crates/cli/src/state/`

## Implementation Phases

### Phase 1: Capture Real Claude Code Behavior

**Goal:** Document exactly what `/memory` does in real Claude Code before writing any code.

**Steps:**

1. Set up tmux session with real Claude Code:
```bash
# Kill any existing session and create fresh one
tmux kill-session -t claude-test 2>/dev/null
tmux new-session -d -s claude-test -x 120 -y 30

# Start Claude Code
tmux send-keys -t claude-test 'claude --model haiku' Enter
sleep 3
tmux capture-pane -t claude-test -p
```

2. Capture autocomplete behavior:
```bash
# Type /memory to see autocomplete
tmux send-keys -t claude-test '/memory'
sleep 0.5
tmux capture-pane -t claude-test -p > memory_autocomplete.txt
```

3. Execute command and capture dialog:
```bash
# Press Enter to execute
tmux send-keys -t claude-test Enter
sleep 0.5
tmux capture-pane -t claude-test -p > memory_dialog.txt
```

4. Test navigation (Up/Down arrows):
```bash
tmux send-keys -t claude-test Down
sleep 0.3
tmux capture-pane -t claude-test -p > memory_nav_down.txt

tmux send-keys -t claude-test Up
sleep 0.3
tmux capture-pane -t claude-test -p > memory_nav_up.txt
```

5. Test Enter on an item (if applicable):
```bash
tmux send-keys -t claude-test Enter
sleep 0.3
tmux capture-pane -t claude-test -p > memory_select.txt
```

6. Test Escape dismiss:
```bash
# Re-open if needed, then Escape
tmux send-keys -t claude-test Escape
sleep 0.3
tmux capture-pane -t claude-test -p > memory_dismiss.txt
```

7. Clean up:
```bash
tmux kill-session -t claude-test
```

**Deliverables:**
- `crates/cli/tests/fixtures/tui/v2.1.12/memory_dialog.txt`
- Update `crates/cli/tests/fixtures/tui/v2.1.12/BEHAVIORS.md` with observed behavior
- Document: header format, item format, selection cursor, footer hints, dismiss message

### Phase 2: Create MemoryDialog Widget

Based on captured behavior, create the widget module.

**File:** `crates/cli/src/tui/widgets/memory.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Memory dialog widget.
//!
//! Shown when user executes `/memory` to view and manage CLAUDE.md instruction files.

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;

/// Memory source types displayed in the dialog
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemorySource {
    /// Project-level CLAUDE.md (.claude/CLAUDE.md or CLAUDE.md)
    Project,
    /// User-level CLAUDE.md (~/.claude/CLAUDE.md)
    User,
    /// Enterprise/Organization level
    Enterprise,
}

impl MemorySource {
    /// All memory sources in display order
    pub fn all() -> &'static [MemorySource] {
        &[
            MemorySource::Project,
            MemorySource::User,
            MemorySource::Enterprise,
        ]
    }

    /// Display name for the source type
    pub fn name(self) -> &'static str {
        match self {
            MemorySource::Project => "Project",
            MemorySource::User => "User",
            MemorySource::Enterprise => "Enterprise",
        }
    }

    /// Description for the source type
    pub fn description(self) -> &'static str {
        match self {
            MemorySource::Project => "Project-specific instructions (.claude/CLAUDE.md)",
            MemorySource::User => "User-level instructions (~/.claude/CLAUDE.md)",
            MemorySource::Enterprise => "Organization-level instructions",
        }
    }
}

/// A loaded memory entry
#[derive(Clone, Debug)]
pub struct MemoryEntry {
    /// Source type
    pub source: MemorySource,
    /// File path (if available)
    pub path: Option<String>,
    /// Whether this entry exists/is active
    pub is_active: bool,
    /// Preview of content (first N chars)
    pub preview: Option<String>,
}

/// State for the /memory dialog
#[derive(Clone, Debug)]
pub struct MemoryDialog {
    /// Currently selected entry index (0-based)
    pub selected_index: usize,
    /// Memory entries (loaded from filesystem)
    pub entries: Vec<MemoryEntry>,
    /// Scroll offset for the list
    pub scroll_offset: usize,
    /// Visible item count (based on terminal height)
    pub visible_count: usize,
}

impl Default for MemoryDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryDialog {
    pub fn new() -> Self {
        // In production, this would scan for actual CLAUDE.md files
        // For now, create placeholder entries
        let entries = vec![
            MemoryEntry {
                source: MemorySource::Project,
                path: Some(".claude/CLAUDE.md".to_string()),
                is_active: true,
                preview: Some("Project-specific instructions...".to_string()),
            },
            MemoryEntry {
                source: MemorySource::User,
                path: Some("~/.claude/CLAUDE.md".to_string()),
                is_active: false,
                preview: None,
            },
        ];

        Self {
            selected_index: 0,
            entries,
            scroll_offset: 0,
            visible_count: 5,
        }
    }

    /// Move selection up (wraps at boundaries)
    pub fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.entries.len() - 1;
            // Scroll to bottom
            if self.entries.len() > self.visible_count {
                self.scroll_offset = self.entries.len() - self.visible_count;
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
        if self.entries.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.entries.len();
        // Handle wrap to top
        if self.selected_index == 0 {
            self.scroll_offset = 0;
        }
        // Scroll down if needed
        else if self.selected_index >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected_index - self.visible_count + 1;
        }
    }

    /// Get currently selected entry
    pub fn selected_entry(&self) -> Option<&MemoryEntry> {
        self.entries.get(self.selected_index)
    }

    /// Check if we should show scroll indicator below
    pub fn has_more_below(&self) -> bool {
        self.scroll_offset + self.visible_count < self.entries.len()
    }
}
```

**Tasks:**
1. Create `crates/cli/src/tui/widgets/memory.rs` based on captured behavior
2. Add unit tests in `crates/cli/src/tui/widgets/memory_tests.rs`:
   - Test `MemorySource::all()` returns expected sources
   - Test `MemoryDialog::select_next()` and `select_prev()` with wrapping
   - Test scroll offset updates when navigating
   - Test `selected_entry()` returns correct entry

### Phase 3: Wire Up AppMode and State

Integrate the MemoryDialog into the TUI application state.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `MemoryDialog` variant to `AppMode` enum:
   ```rust
   /// Showing memory management dialog
   MemoryDialog,
   ```

2. Add `memory_dialog` field to `RenderState` struct:
   ```rust
   /// Memory dialog state (None if not showing)
   pub memory_dialog: Option<MemoryDialog>,
   ```

3. Add `memory_dialog` field to `TuiAppStateInner` struct

4. Update `render_state()` to include `memory_dialog`

5. Export from `widgets/mod.rs`:
   ```rust
   pub mod memory;
   pub use memory::{MemoryDialog, MemoryEntry, MemorySource};
   ```

### Phase 4: Implement Command Handler and Key Handler

Add handler for the `/memory` command to open the dialog, and keyboard navigation.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `/memory` match arm in `handle_command_inner()`:
   ```rust
   "/memory" => {
       inner.mode = AppMode::MemoryDialog;
       inner.memory_dialog = Some(super::widgets::MemoryDialog::new());
   }
   ```

2. Add `AppMode::MemoryDialog` to the match in `handle_key_event()`

3. Implement `handle_memory_dialog_key()`:
   ```rust
   fn handle_memory_dialog_key(&self, key: KeyEvent) {
       let mut inner = self.inner.lock();

       let Some(ref mut dialog) = inner.memory_dialog else {
           return;
       };

       match key.code {
           KeyCode::Esc => {
               inner.mode = AppMode::Input;
               inner.memory_dialog = None;
               inner.response_content = "Memory dialog dismissed".to_string();
               inner.is_command_output = true;
           }
           KeyCode::Up => dialog.select_prev(),
           KeyCode::Down => dialog.select_next(),
           KeyCode::Enter => {
               // Open selected memory file for viewing/editing
               // Implementation depends on captured behavior
           }
           _ => {}
       }
   }
   ```

### Phase 5: Implement Render Function

Add dialog rendering based on captured behavior from real Claude Code.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add conditional rendering in main render function:
   ```rust
   if state.mode == AppMode::MemoryDialog {
       if let Some(ref dialog) = state.memory_dialog {
           return render_memory_dialog(dialog, width);
       }
   }
   ```

2. Implement `render_memory_dialog()` - format will be based on Phase 1 captures:
   ```rust
   fn render_memory_dialog(dialog: &MemoryDialog, _width: usize) -> AnyElement<'static> {
       // Header: " Memory" or similar
       // List of memory sources with selection cursor
       // Footer: navigation hints

       let mut items = Vec::new();
       for (i, entry) in dialog.entries.iter().enumerate() {
           let is_selected = i == dialog.selected_index;
           let prefix = if is_selected { "❯" } else { " " };
           let status = if entry.is_active { "✓" } else { " " };

           items.push(format!(
               " {} {} {}  {} - {}",
               prefix,
               status,
               i + 1,
               entry.source.name(),
               entry.path.as_deref().unwrap_or("(not configured)")
           ));
       }

       element! {
           View(flex_direction: FlexDirection::Column, width: 100pct) {
               Text(content: " Memory")
               Text(content: "")
               #(items.into_iter().map(|item| {
                   element! { Text(content: item) }
               }))
               Text(content: "")
               Text(content: " Enter to view · esc to cancel")
           }
       }.into()
   }
   ```

### Phase 6: Write Integration Tests

Create TUI integration tests following the patterns established in `tui_hooks.rs`.

**File:** `crates/cli/tests/tui_memory.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI /memory command tests - memory management dialog behavior.
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## /memory Command Behavior
//! - [Document observed behaviors from Phase 1]

mod common;

use common::{start_tui, tmux, write_scenario};

// =============================================================================
// /memory Autocomplete Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Typing /memory shows autocomplete dropdown with memory description
#[test]
fn test_tui_memory_command_shows_autocomplete() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-autocomplete";
    let previous = start_tui(session, &scenario);

    // Type /memory
    tmux::send_keys(session, "/memory");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert!(
        capture.contains("/memory")
            && capture.contains("View or manage conversation memory"),
        "/memory should show autocomplete with description.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /memory Dialog Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// /memory command shows a dialog with memory sources
#[test]
fn test_tui_memory_shows_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-dialog";
    let previous = start_tui(session, &scenario);

    // Type /memory and press Enter
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let capture = tmux::wait_for_content(session, "Memory");

    tmux::kill_session(session);

    // Should show the memory dialog
    assert!(
        capture.contains("Memory"),
        "Should show 'Memory' header.\nCapture:\n{}",
        capture
    );
}

// =============================================================================
// /memory Navigation Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Down arrow navigates through memory entries
#[test]
fn test_tui_memory_arrow_navigation() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-nav";
    let previous = start_tui(session, &scenario);

    // Open memory dialog
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let initial = tmux::wait_for_content(session, "Memory");

    // Press Down to move to next entry
    tmux::send_keys(session, "Down");
    let after_down = tmux::wait_for_change(session, &initial);

    tmux::kill_session(session);

    // Should show selection moved
    assert!(
        after_down.contains("❯"),
        "Should show selection cursor.\nCapture:\n{}",
        after_down
    );
}

// =============================================================================
// /memory Dismiss Tests
// =============================================================================

/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// Pressing Escape dismisses the memory dialog
#[test]
fn test_tui_memory_escape_dismisses_dialog() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-memory-dismiss";
    let previous = start_tui(session, &scenario);

    // Open memory dialog
    tmux::send_keys(session, "/memory");
    let _ = tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "Enter");
    let dialog = tmux::wait_for_content(session, "Memory");

    // Press Escape to dismiss
    tmux::send_keys(session, "Escape");
    let capture = tmux::wait_for_change(session, &dialog);

    tmux::kill_session(session);

    assert!(
        capture.contains("Memory dialog dismissed"),
        "Escape should dismiss memory dialog and show message.\nCapture:\n{}",
        capture
    );
}
```

**Tasks:**
1. Create test file with initial tests marked `#[ignore]` if behavior differs
2. Update tests based on actual captured behavior from Phase 1
3. Add fixture files to `crates/cli/tests/fixtures/tui/v2.1.12/`

## Key Implementation Details

### Memory Sources

Based on Claude Code documentation, memory files can come from:
1. **Project-level:** `.claude/CLAUDE.md` or `CLAUDE.md` in project root
2. **User-level:** `~/.claude/CLAUDE.md`
3. **Enterprise:** Organization-configured locations

### Dialog Format (To Be Updated After Phase 1)

Expected format similar to `/hooks`:
```
 Memory

 ❯ 1.  Project - .claude/CLAUDE.md (active)
   2.  User - ~/.claude/CLAUDE.md (not found)

 Enter to view · esc to cancel
```

### Navigation Behavior

- **List:** Up/Down navigate, Enter views selected file, Escape dismisses
- Selection wraps at boundaries (up from first goes to last, down from last goes to first)
- Scroll offset adjusts to keep selection visible

### Dismiss Behavior

- Escape dismisses and shows "Memory dialog dismissed"
- Returns to Input mode with clean state

## Verification Plan

### Phase 1 Verification

Confirm captured fixtures match real Claude Code:
- Screenshot comparison with captured files
- Document any animations or transitions

### Unit Tests (Phase 2)

```bash
cargo test --lib -- tui::widgets::memory
```

Verify:
- `MemorySource::all()` returns expected sources
- `MemoryDialog::select_next()` increments and wraps
- `MemoryDialog::select_prev()` decrements and wraps
- Scroll offset updates correctly on navigation
- `selected_entry()` returns correct entry

### Integration Tests (Phase 6)

```bash
cargo test --test tui_memory
```

All tests should pass:
1. `test_tui_memory_command_shows_autocomplete` - `/memory` appears in autocomplete
2. `test_tui_memory_shows_dialog` - Dialog shows memory sources
3. `test_tui_memory_arrow_navigation` - Up/Down arrows work
4. `test_tui_memory_escape_dismisses_dialog` - Escape dismisses with message

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
