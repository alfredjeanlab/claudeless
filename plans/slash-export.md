# Implementation Plan: /export Slash Command

**Root Feature:** `slash-export`

## Overview

Implement the `/export` slash command to export the current conversation. When executed, it opens a dialog with two export methods:
1. **Copy to clipboard** - Copies conversation text to system clipboard
2. **Save to file** - Prompts for filename and saves to disk

The dialog supports:
- Arrow key navigation between options
- Selection cursor (`❯`) indicating current choice
- Escape to cancel from method selection
- Escape from filename input returns to method selection

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add AppMode::ExportDialog, /export handler, key handling, rendering
│   └── widgets/
│       ├── mod.rs             # Export ExportDialog
│       └── export.rs          # NEW: ExportDialog struct and ExportStep enum
│       └── export_tests.rs    # NEW: Unit tests for ExportDialog

crates/cli/tests/
└── tui_export.rs              # Remove #[ignore] from 8 tests
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm` for keyboard input handling
- `arboard` for clipboard access (already in Cargo.toml)
- `std::fs` for file writing
- Box-drawing characters for dialog borders

## Implementation Phases

### Phase 1: Create ExportDialog Widget

Create a new widget module for the export dialog state.

**File:** `crates/cli/src/tui/widgets/export.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Export dialog widget.
//!
//! Shown when user executes `/export` to export the conversation.

/// Export method options
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExportMethod {
    #[default]
    Clipboard,
    File,
}

/// Current step in the export workflow
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExportStep {
    /// Selecting export method (clipboard or file)
    #[default]
    MethodSelection,
    /// Entering filename for file export
    FilenameInput,
}

/// State for the /export dialog
#[derive(Clone, Debug)]
pub struct ExportDialog {
    /// Current step in the workflow
    pub step: ExportStep,
    /// Selected export method
    pub selected_method: ExportMethod,
    /// Filename input buffer (for file export)
    pub filename: String,
    /// Default filename (generated on dialog open)
    pub default_filename: String,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportDialog {
    pub fn new() -> Self {
        // Generate default filename with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let default_filename = format!("conversation_{}.txt", timestamp);

        Self {
            step: ExportStep::MethodSelection,
            selected_method: ExportMethod::Clipboard,
            filename: default_filename.clone(),
            default_filename,
        }
    }

    /// Toggle between clipboard and file methods
    pub fn toggle_method(&mut self) {
        self.selected_method = match self.selected_method {
            ExportMethod::Clipboard => ExportMethod::File,
            ExportMethod::File => ExportMethod::Clipboard,
        };
    }

    /// Move selection up (wraps to bottom)
    pub fn move_selection_up(&mut self) {
        self.toggle_method();
    }

    /// Move selection down (wraps to top)
    pub fn move_selection_down(&mut self) {
        self.toggle_method();
    }

    /// Confirm current selection and advance workflow
    pub fn confirm_selection(&mut self) -> bool {
        match self.step {
            ExportStep::MethodSelection => {
                if self.selected_method == ExportMethod::File {
                    self.step = ExportStep::FilenameInput;
                    false // Not done yet
                } else {
                    true // Clipboard selected, ready to export
                }
            }
            ExportStep::FilenameInput => true, // Ready to save file
        }
    }

    /// Go back from filename input to method selection
    pub fn go_back(&mut self) -> bool {
        match self.step {
            ExportStep::FilenameInput => {
                self.step = ExportStep::MethodSelection;
                false // Stay in dialog
            }
            ExportStep::MethodSelection => true, // Cancel dialog
        }
    }

    /// Handle character input for filename
    pub fn push_char(&mut self, c: char) {
        if self.step == ExportStep::FilenameInput {
            self.filename.push(c);
        }
    }

    /// Handle backspace for filename
    pub fn pop_char(&mut self) {
        if self.step == ExportStep::FilenameInput {
            self.filename.pop();
        }
    }
}
```

**File:** `crates/cli/src/tui/widgets/mod.rs`

Add export:
```rust
pub mod export;
pub use export::ExportDialog;
```

**Verification:** `cargo build` succeeds.

---

### Phase 2: Add ExportDialog Mode to TUI State

Extend the application state and mode enum to support the export dialog.

**File:** `crates/cli/src/tui/app.rs`

1. Add import at top:
   ```rust
   use crate::tui::widgets::export::{ExportDialog, ExportMethod, ExportStep};
   ```

2. Add variant to `AppMode` enum:
   ```rust
   pub enum AppMode {
       // ... existing variants
       /// Showing export dialog
       ExportDialog,
   }
   ```

3. Add field to `TuiAppStateInner` struct:
   ```rust
   pub export_dialog: Option<ExportDialog>,
   ```

4. Initialize in constructor:
   ```rust
   export_dialog: None,
   ```

5. Add to `RenderState` struct:
   ```rust
   pub export_dialog: Option<ExportDialog>,
   ```

6. Update `render_state()` method to include:
   ```rust
   export_dialog: inner.export_dialog.clone(),
   ```

**Verification:** `cargo build` succeeds.

---

### Phase 3: Implement /export Command Handler

Add the command handler to open the export dialog.

**File:** `crates/cli/src/tui/app.rs`

Add match arm in `handle_command_inner()` (alphabetically, between `/exit` and `/fork`):
```rust
"/export" => {
    inner.mode = AppMode::ExportDialog;
    inner.export_dialog = Some(ExportDialog::new());
}
```

**Verification:** Running `/export` enters dialog mode.

---

### Phase 4: Implement Dialog Rendering

Create the rendering functions for both dialog states.

**File:** `crates/cli/src/tui/app.rs`

Add rendering function:
```rust
/// Render export dialog
fn render_export_dialog(dialog: &ExportDialog, width: usize) -> AnyElement<'static> {
    let inner_width = width.saturating_sub(2);
    let h_line = "─".repeat(inner_width);
    let top_border = format!("╭{}╮", h_line);
    let bottom_border = format!("╰{}╯", h_line);

    let pad_line = |s: &str| {
        let visible_len = s.chars().count();
        let padding = inner_width.saturating_sub(visible_len);
        format!("│{}{}│", s, " ".repeat(padding))
    };

    match dialog.step {
        ExportStep::MethodSelection => {
            let clipboard_cursor = if dialog.selected_method == ExportMethod::Clipboard {
                "❯"
            } else {
                " "
            };
            let file_cursor = if dialog.selected_method == ExportMethod::File {
                "❯"
            } else {
                " "
            };

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                ) {
                    Text(content: top_border)
                    Text(content: pad_line(" Export Conversation"))
                    Text(content: pad_line(""))
                    Text(content: pad_line(" Select export method:"))
                    Text(content: pad_line(&format!(" {} 1. Copy to clipboard", clipboard_cursor)))
                    Text(content: pad_line(&format!(" {} 2. Save to file", file_cursor)))
                    Text(content: bottom_border)
                    Text(content: "  ↑/↓ to select · Enter to confirm · Esc to cancel")
                }
            }
            .into()
        }
        ExportStep::FilenameInput => {
            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                ) {
                    Text(content: top_border)
                    Text(content: pad_line(" Export Conversation"))
                    Text(content: pad_line(""))
                    Text(content: pad_line(" Enter filename:"))
                    Text(content: pad_line(&format!(" {}", dialog.filename)))
                    Text(content: bottom_border)
                    Text(content: "  Enter to save · esc to go back")
                }
            }
            .into()
        }
    }
}
```

Add rendering branch in the main content render function (similar to tasks dialog):
```rust
if let Some(ref dialog) = state.export_dialog {
    if state.mode == AppMode::ExportDialog {
        return render_export_dialog(dialog, width);
    }
}
```

**Verification:**
- `test_tui_export_shows_method_dialog` passes
- `test_tui_export_file_shows_filename_dialog` passes

---

### Phase 5: Implement Key Event Handling

Handle keyboard input when in ExportDialog mode.

**File:** `crates/cli/src/tui/app.rs`

1. Add match arm in main key dispatcher:
   ```rust
   AppMode::ExportDialog => self.handle_export_dialog_key(key),
   ```

2. Add handler method:
   ```rust
   fn handle_export_dialog_key(&self, key: KeyEvent) {
       let mut inner = self.inner.lock();

       let Some(ref mut dialog) = inner.export_dialog else {
           return;
       };

       match dialog.step {
           ExportStep::MethodSelection => {
               match key.code {
                   KeyCode::Esc => {
                       inner.mode = AppMode::Input;
                       inner.export_dialog = None;
                       inner.response_content = "Export cancelled".to_string();
                       inner.is_command_output = true;
                   }
                   KeyCode::Up => dialog.move_selection_up(),
                   KeyCode::Down => dialog.move_selection_down(),
                   KeyCode::Enter => {
                       if dialog.confirm_selection() {
                           // Clipboard export
                           self.do_clipboard_export(&mut inner);
                       }
                       // else: moved to filename input, dialog updated
                   }
                   _ => {}
               }
           }
           ExportStep::FilenameInput => {
               match key.code {
                   KeyCode::Esc => {
                       dialog.go_back();
                   }
                   KeyCode::Enter => {
                       self.do_file_export(&mut inner);
                   }
                   KeyCode::Backspace => dialog.pop_char(),
                   KeyCode::Char(c) => dialog.push_char(c),
                   _ => {}
               }
           }
       }
   }
   ```

**Verification:**
- `test_tui_export_escape_cancels` passes
- `test_tui_export_filename_escape_returns_to_method` passes
- `test_tui_export_arrow_navigation` passes

---

### Phase 6: Implement Export Functions

Add the actual export functionality for clipboard and file.

**File:** `crates/cli/src/tui/app.rs`

```rust
/// Export conversation to clipboard
fn do_clipboard_export(&self, inner: &mut TuiAppStateInner) {
    // Get conversation content
    let content = self.format_conversation_for_export(inner);

    // Copy to clipboard
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(&content) {
                Ok(()) => {
                    inner.response_content = "Conversation copied to clipboard".to_string();
                }
                Err(e) => {
                    inner.response_content = format!("Failed to copy to clipboard: {}", e);
                }
            }
        }
        Err(e) => {
            inner.response_content = format!("Failed to access clipboard: {}", e);
        }
    }

    inner.mode = AppMode::Input;
    inner.export_dialog = None;
    inner.is_command_output = true;
}

/// Export conversation to file
fn do_file_export(&self, inner: &mut TuiAppStateInner) {
    let filename = inner
        .export_dialog
        .as_ref()
        .map(|d| d.filename.clone())
        .unwrap_or_else(|| "conversation.txt".to_string());

    let content = self.format_conversation_for_export(inner);

    match std::fs::write(&filename, &content) {
        Ok(()) => {
            inner.response_content = format!("Conversation exported to: {}", filename);
        }
        Err(e) => {
            inner.response_content = format!("Failed to write file: {}", e);
        }
    }

    inner.mode = AppMode::Input;
    inner.export_dialog = None;
    inner.is_command_output = true;
}

/// Format conversation for export
fn format_conversation_for_export(&self, inner: &TuiAppStateInner) -> String {
    // Export the conversation display content
    // This includes the visible conversation history
    inner.conversation_display.clone()
}
```

**Verification:**
- `test_tui_export_clipboard_shows_confirmation` passes
- `test_tui_export_file_shows_save_confirmation` passes

---

### Phase 7: Enable Tests and Final Verification

Remove `#[ignore]` from all tests and verify.

**File:** `crates/cli/tests/tui_export.rs`

Remove `#[ignore]` attribute from these tests:
- `test_tui_export_command_shows_autocomplete` (line 31)
- `test_tui_export_shows_method_dialog` (line 68)
- `test_tui_export_clipboard_shows_confirmation` (line 118)
- `test_tui_export_file_shows_filename_dialog` (line 154)
- `test_tui_export_file_shows_save_confirmation` (line 207)
- `test_tui_export_escape_cancels` (line 251)
- `test_tui_export_filename_escape_returns_to_method` (line 287)
- `test_tui_export_arrow_navigation` (line 337)

**Verification:**
```bash
cargo test --test tui_export
make check
```

## Key Implementation Details

### Dialog Box Format

**Method Selection State:**
```
╭────────────────────────────────────────────────────────────────────────────────────╮
│ Export Conversation                                                                 │
│                                                                                     │
│ Select export method:                                                               │
│ ❯ 1. Copy to clipboard                                                              │
│   2. Save to file                                                                   │
╰────────────────────────────────────────────────────────────────────────────────────╯
  ↑/↓ to select · Enter to confirm · Esc to cancel
```

**Filename Input State:**
```
╭────────────────────────────────────────────────────────────────────────────────────╮
│ Export Conversation                                                                 │
│                                                                                     │
│ Enter filename:                                                                     │
│ conversation_20260122_143000.txt                                                    │
╰────────────────────────────────────────────────────────────────────────────────────╯
  Enter to save · esc to go back
```

### State Machine

```
AppMode::Input
    │
    └── /export command
            ↓
    AppMode::ExportDialog (ExportStep::MethodSelection)
        │
        ├── Escape → AppMode::Input + "Export cancelled"
        ├── Up/Down → Toggle selection
        └── Enter
            ├── If Clipboard → do_clipboard_export() → AppMode::Input
            └── If File → ExportStep::FilenameInput
                    │
                    ├── Escape → ExportStep::MethodSelection
                    ├── Char(c) → Append to filename
                    ├── Backspace → Remove from filename
                    └── Enter → do_file_export() → AppMode::Input
```

### Export Content

The export captures `inner.conversation_display` which contains the visible conversation history including:
- User prompts (prefixed with `❯`)
- Assistant responses
- Tool calls and their outputs

### Clipboard Crate

Uses `arboard` crate (already a dependency) for cross-platform clipboard access:
```rust
let mut clipboard = arboard::Clipboard::new()?;
clipboard.set_text(&content)?;
```

## Verification Plan

1. **Unit Tests:**
   - All 8 tests in `tui_export.rs` pass

2. **Integration:**
   - `make check` passes (includes lint, format, clippy, tests, build, audit)

3. **Manual Testing:**
   - Launch TUI: `cargo run -- --scenario test`
   - Type `/export` and press Enter - should show method dialog
   - Arrow keys toggle between options
   - Enter on "Copy to clipboard" - shows "Conversation copied to clipboard"
   - Enter on "Save to file" - shows filename prompt
   - Enter on filename - shows "Conversation exported to: filename.txt"
   - Escape from method selection - shows "Export cancelled"
   - Escape from filename - returns to method selection

## Files Modified Summary

| File | Changes |
|------|---------|
| `crates/cli/src/tui/widgets/export.rs` | NEW: ExportDialog struct, ExportMethod, ExportStep |
| `crates/cli/src/tui/widgets/export_tests.rs` | NEW: Unit tests for ExportDialog |
| `crates/cli/src/tui/widgets/mod.rs` | Export ExportDialog |
| `crates/cli/src/tui/app.rs` | Add ExportDialog mode, handler, key handling, rendering, export functions |
| `crates/cli/tests/tui_export.rs` | Remove `#[ignore]` from 8 tests |
