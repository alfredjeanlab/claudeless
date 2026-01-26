# Implementation Plan: '?' Shortcut Panel Display

**Root Feature:** `cl-95c5`

**Test File:** `crates/cli/tests/tui_shortcuts.rs` (4 tests, currently `#[ignore]`)

## Overview

Implement support for the '?' input shortcut that displays a keyboard shortcuts panel. When the user presses '?' on empty input, a panel shows available keyboard shortcuts (bash mode, commands, file paths, etc.). When input is not empty, '?' types a literal '?' character. Pressing Escape dismisses the panel.

**Key behaviors to implement:**
1. Pressing '?' on empty input shows the shortcuts panel
2. Shortcuts panel displays keyboard shortcuts in a multi-column layout
3. Pressing Escape dismisses the shortcuts panel
4. When input is NOT empty, '?' types a literal '?' character
5. After dismissing the panel, status bar shows "? for shortcuts" hint again

## Project Structure

```
crates/cli/src/tui/
├── app.rs                    # Key handling, state transitions (modify)
├── app_tests.rs              # Unit tests for shortcuts state (modify)
├── mod.rs                    # Module exports (modify)
├── shortcuts.rs              # NEW: Shortcuts data and state
├── shortcuts_tests.rs        # NEW: Unit tests for shortcuts
└── widgets/
    ├── mod.rs                # Export new widget (modify)
    ├── shortcuts.rs          # NEW: Shortcuts panel widget
    └── shortcuts_tests.rs    # NEW: Widget rendering tests

crates/cli/tests/
├── tui_shortcuts.rs          # Integration tests (enable from #[ignore])
└── fixtures/tui/v2.1.12/
    └── shortcuts_display.txt # EXISTS: Fixture for shortcuts panel
```

## Dependencies

No new external dependencies required. Uses existing ratatui for rendering.

## Implementation Phases

### Phase 1: Define Shortcuts Data

**Goal:** Create a static definition of all keyboard shortcuts to display.

**Files:**
- `crates/cli/src/tui/shortcuts.rs` (new)
- `crates/cli/src/tui/shortcuts_tests.rs` (new)
- `crates/cli/src/tui/mod.rs` (modify)

**Implementation:**

```rust
// crates/cli/src/tui/shortcuts.rs

/// A keyboard shortcut definition
#[derive(Clone, Debug)]
pub struct Shortcut {
    /// Key combination (e.g., "! for bash mode")
    pub keys: &'static str,
    /// Column position (0 = left, 1 = center, 2 = right)
    pub column: u8,
}

/// All keyboard shortcuts displayed in the panel
/// Organized in 3 columns, 5 rows (with one cell spanning 2 rows)
pub static SHORTCUTS: &[Shortcut] = &[
    // Left column
    Shortcut { keys: "! for bash mode", column: 0 },
    Shortcut { keys: "/ for commands", column: 0 },
    Shortcut { keys: "@ for file paths", column: 0 },
    Shortcut { keys: "& for background", column: 0 },

    // Center column
    Shortcut { keys: "double tap esc to clear input", column: 1 },
    Shortcut { keys: "shift + tab to auto-accept edits", column: 1 },
    Shortcut { keys: "ctrl + o for verbose output", column: 1 },
    Shortcut { keys: "ctrl + t to show todos", column: 1 },
    Shortcut { keys: "backslash (\\) + return (\u{23ce}) for", column: 1 },
    Shortcut { keys: "newline", column: 1 }, // continuation

    // Right column
    Shortcut { keys: "ctrl + _ to undo", column: 2 },
    Shortcut { keys: "ctrl + z to suspend", column: 2 },
    Shortcut { keys: "cmd + v to paste images", column: 2 },
    Shortcut { keys: "meta + p to switch model", column: 2 },
    Shortcut { keys: "ctrl + s to stash prompt", column: 2 },
];

/// Get shortcuts organized by column
pub fn shortcuts_by_column() -> [Vec<&'static str>; 3] {
    let mut columns = [Vec::new(), Vec::new(), Vec::new()];
    for shortcut in SHORTCUTS {
        columns[shortcut.column as usize].push(shortcut.keys);
    }
    columns
}
```

**Verification:**
- [ ] Shortcuts match the fixture `shortcuts_display.txt`
- [ ] `cargo test -p claudeless -- shortcuts` passes

---

### Phase 2: Add Shortcuts State to TUI

**Goal:** Track shortcuts panel visibility in `TuiAppStateInner`.

**Files:**
- `crates/cli/src/tui/app.rs` (modify)

**Implementation:**

```rust
// Add to TuiAppStateInner struct (around line 224):

/// Whether the shortcuts panel is currently visible
pub show_shortcuts_panel: bool,
```

```rust
// Initialize in TuiAppStateInner::new() or wherever state is created:

show_shortcuts_panel: false,
```

**Verification:**
- [ ] State compiles without errors
- [ ] Default value is `false`

---

### Phase 3: Handle '?' Key Event

**Goal:** Detect '?' key press and conditionally show shortcuts or type literal.

**Files:**
- `crates/cli/src/tui/app.rs` (modify `handle_input_key()`)

**Key Handling Logic:**

```rust
// In handle_input_key(), find the character input handling section
// (around line 613 based on exploration results)

// Handle '?' key specially
(m, KeyCode::Char('?')) if m.is_empty() || m == KeyModifiers::SHIFT => {
    if inner.input_buffer.is_empty() && !inner.show_shortcuts_panel {
        // Empty input: show shortcuts panel
        inner.show_shortcuts_panel = true;
    } else {
        // Non-empty input: type literal '?'
        let pos = inner.cursor_pos;
        inner.input_buffer.insert(pos, '?');
        inner.cursor_pos = pos + 1;
        // Update any autocomplete state if needed
    }
}
```

**Verification:**
- [ ] '?' on empty input sets `show_shortcuts_panel = true`
- [ ] '?' with text types literal '?'
- [ ] Existing character handling still works

---

### Phase 4: Handle Escape to Dismiss Panel

**Goal:** Modify Escape key handling to dismiss shortcuts panel before clearing input.

**Files:**
- `crates/cli/src/tui/app.rs` (modify `handle_input_key()`)

**Implementation:**

```rust
// In handle_input_key(), find Escape handling (around line 528-532):

(_, KeyCode::Esc) => {
    if inner.show_shortcuts_panel {
        // First priority: dismiss shortcuts panel
        inner.show_shortcuts_panel = false;
    } else if !inner.input_buffer.is_empty() {
        // Existing behavior: show "Esc to clear again" hint
        // or clear input on double-tap
        // ... existing escape logic ...
    }
}
```

**Verification:**
- [ ] Escape dismisses shortcuts panel when visible
- [ ] Escape still clears input when panel is not visible
- [ ] Panel dismissed state shows "? for shortcuts" hint

---

### Phase 5: Implement Shortcuts Panel Widget

**Goal:** Create a widget to render the shortcuts panel in multi-column layout.

**Files:**
- `crates/cli/src/tui/widgets/shortcuts.rs` (new)
- `crates/cli/src/tui/widgets/shortcuts_tests.rs` (new)
- `crates/cli/src/tui/widgets/mod.rs` (modify)

**Implementation:**

```rust
// crates/cli/src/tui/widgets/shortcuts.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::tui::shortcuts::shortcuts_by_column;

/// Widget for rendering the shortcuts panel
pub struct ShortcutsPanel;

impl ShortcutsPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Widget for ShortcutsPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let columns = shortcuts_by_column();
        let col_width = area.width / 3;
        let style = Style::default().fg(Color::Rgb(153, 153, 153)); // Gray

        for (col_idx, shortcuts) in columns.iter().enumerate() {
            let x = area.x + (col_idx as u16) * col_width;

            for (row_idx, shortcut) in shortcuts.iter().enumerate() {
                if row_idx as u16 >= area.height {
                    break;
                }
                let y = area.y + row_idx as u16;

                // Add 2-space indent for left column alignment
                let text = format!("  {}", shortcut);
                buf.set_string(x, y, &text, style);
            }
        }
    }
}
```

**Verification:**
- [ ] Widget renders 3 columns
- [ ] Text is gray colored (rgb 153,153,153)
- [ ] Layout matches fixture

---

### Phase 6: Integrate Widget into Render Loop

**Goal:** Render shortcuts panel when `show_shortcuts_panel` is true.

**Files:**
- `crates/cli/src/tui/app.rs` (modify render/draw method)

**Implementation:**

The shortcuts panel appears below the input line, replacing the normal conversation area. Based on the fixture, it renders after the separator line:

```rust
// In the render method, after rendering the input area:

if inner.show_shortcuts_panel {
    // Calculate area for shortcuts (below separator, above input)
    let shortcuts_area = Rect {
        x: content_area.x,
        y: separator_y + 1, // Below separator
        width: content_area.width,
        height: 6, // 6 rows of shortcuts
    };

    ShortcutsPanel::new().render(shortcuts_area, buf);
}
```

**Verification:**
- [ ] Panel appears when `show_shortcuts_panel` is true
- [ ] Panel disappears when false
- [ ] Layout matches fixture exactly

---

### Phase 7: Enable Integration Tests

**Goal:** Remove `#[ignore]` from tests and verify they pass.

**Files:**
- `crates/cli/tests/tui_shortcuts.rs` (modify)

**Steps:**
1. Remove `#[ignore]` from `test_tui_question_mark_shows_shortcuts_on_empty_input`
2. Run test, fix any issues
3. Remove `#[ignore]` from `test_tui_shortcuts_display_matches_fixture`
4. Run test, adjust rendering to match fixture
5. Remove `#[ignore]` from `test_tui_escape_dismisses_shortcuts_panel`
6. Run test, fix any issues
7. Remove `#[ignore]` from `test_tui_question_mark_types_literal_when_input_present`
8. Run test, verify conditional behavior

**Verification:**
- [ ] All 4 tests pass without `#[ignore]`
- [ ] `cargo test tui_shortcuts` passes
- [ ] `make check` passes

---

## Key Implementation Details

### Shortcuts Layout

The shortcuts are displayed in a 3-column layout matching the fixture:

```
  ! for bash mode         double tap esc to clear input      ctrl + _ to undo
  / for commands          shift + tab to auto-accept edits   ctrl + z to suspend
  @ for file paths        ctrl + o for verbose output        cmd + v to paste images
  & for background        ctrl + t to show todos             meta + p to switch model
                          backslash (\) + return (⏎) for     ctrl + s to stash prompt
                          newline
```

Column widths should be approximately equal thirds of the available width.

### State Machine

```
┌──────────────────────────────────────────────────────────┐
│                    Input Mode                             │
│              (show_shortcuts_panel: false)                │
│              Shows "? for shortcuts" hint                 │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Press '?' (input empty)
                           ▼
┌──────────────────────────────────────────────────────────┐
│                 Shortcuts Panel Visible                   │
│              (show_shortcuts_panel: true)                 │
│                                                           │
│  - Shows keyboard shortcuts in 3 columns                  │
│  - Escape → close panel, return to Input Mode             │
│  - Any other key → close panel + process key              │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Press Escape
                           ▼
┌──────────────────────────────────────────────────────────┐
│                    Input Mode                             │
│              (show_shortcuts_panel: false)                │
│              Shows "? for shortcuts" hint                 │
└──────────────────────────────────────────────────────────┘
```

### Conditional '?' Behavior

```
User presses '?'
    ↓
Check: is input_buffer empty?
    ├─ YES: Set show_shortcuts_panel = true (show panel)
    └─ NO: Insert '?' into input_buffer at cursor_pos (literal char)
```

### Color Scheme

Based on the fixture, use gray color for shortcut text:
- RGB: `(153, 153, 153)` or similar gray tone

---

## Verification Plan

### Unit Tests

**Shortcuts Module (`shortcuts_tests.rs`):**
- [ ] `test_shortcuts_by_column_left` - left column has correct shortcuts
- [ ] `test_shortcuts_by_column_center` - center column has correct shortcuts
- [ ] `test_shortcuts_by_column_right` - right column has correct shortcuts

**App State (`app_tests.rs`):**
- [ ] `test_shortcuts_panel_initially_hidden` - default state is false
- [ ] `test_question_mark_shows_panel_on_empty` - '?' on empty shows panel
- [ ] `test_question_mark_types_literal_with_text` - '?' with text types char
- [ ] `test_escape_dismisses_panel` - Escape sets show_shortcuts_panel = false

### Integration Tests

All 4 tests in `tui_shortcuts.rs`:
- [ ] `test_tui_question_mark_shows_shortcuts_on_empty_input`
- [ ] `test_tui_shortcuts_display_matches_fixture`
- [ ] `test_tui_escape_dismisses_shortcuts_panel`
- [ ] `test_tui_question_mark_types_literal_when_input_present`

### Final Checklist

- [ ] `make check` passes
- [ ] All unit tests pass
- [ ] All integration tests pass (no `#[ignore]`)
- [ ] No new clippy warnings
- [ ] Output matches `shortcuts_display.txt` fixture
