# Implementation Plan: Meta+P Shortcut for Model Picker

**Status:** 📋 Planned
**Root Feature:** `cl-0bad`

## Overview

Implement `Meta+P` (Option+P on macOS) keyboard shortcut to open a model picker dialog. The dialog allows users to switch between Claude models (Opus, Sonnet, Haiku) with visual feedback showing the current selection and active model. The shortcut is already registered in `shortcuts.rs` but not yet functional.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs              # Meta+P keybinding + model picker handlers + rendering
│   ├── shortcuts.rs        # "meta + p to switch model" (pre-existing)
│   └── widgets/
│       └── model_picker.rs # ModelChoice enum + ModelPickerDialog struct (NEW)

crates/cli/tests/
├── tui_model.rs            # 6 ignored integration tests → enable
└── fixtures/tui/v2.1.12/
    └── model_picker.txt    # Expected visual output (pre-existing)
```

## Dependencies

No new dependencies. Uses existing:
- `crossterm::event::{KeyCode, KeyModifiers}` for keyboard input
- `iocraft` component system for rendering

## Implementation Phases

### Phase 1: Add AppMode::ModelPicker Variant

Add a new application mode for the model picker dialog.

**File:** `crates/cli/src/tui/app.rs`

```rust
pub enum AppMode {
    Input,
    Responding,
    Permission,
    Thinking,
    Trust,
    ThinkingToggle,
    TasksDialog,
    ModelPicker,  // NEW
}
```

**Verification:** Code compiles.

---

### Phase 2: Create Model Picker Widget Types

**File:** `crates/cli/src/tui/widgets/model_picker.rs` (NEW)

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Model picker dialog widget.
//!
//! Shown when user presses Meta+P to switch between Claude models.

/// Available model choices
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelChoice {
    Default,  // Opus 4.5 (recommended)
    Sonnet,   // Sonnet 4.5
    Haiku,    // Haiku 4.5
}

impl ModelChoice {
    /// Returns the full model ID string
    pub fn model_id(&self) -> &'static str {
        match self {
            ModelChoice::Default => "claude-opus-4-5-20251101",
            ModelChoice::Sonnet => "claude-sonnet-4-20250514",
            ModelChoice::Haiku => "claude-haiku-4-5-20251101",
        }
    }

    /// Returns the display name for the model
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelChoice::Default => "Opus 4.5",
            ModelChoice::Sonnet => "Sonnet 4.5",
            ModelChoice::Haiku => "Haiku 4.5",
        }
    }

    /// Returns the description for the picker
    pub fn description(&self) -> &'static str {
        match self {
            ModelChoice::Default => "Most capable for complex work",
            ModelChoice::Sonnet => "Best for everyday tasks",
            ModelChoice::Haiku => "Fastest for quick answers",
        }
    }

    /// Returns all choices in display order
    pub fn all() -> [ModelChoice; 3] {
        [ModelChoice::Default, ModelChoice::Sonnet, ModelChoice::Haiku]
    }

    /// Convert from model ID string
    pub fn from_model_id(id: &str) -> Self {
        let lower = id.to_lowercase();
        if lower.contains("haiku") {
            ModelChoice::Haiku
        } else if lower.contains("sonnet") {
            ModelChoice::Sonnet
        } else {
            ModelChoice::Default
        }
    }
}

/// Model picker dialog state
#[derive(Clone, Debug)]
pub struct ModelPickerDialog {
    /// Currently highlighted option (cursor position)
    pub selected: ModelChoice,
    /// Currently active model (shows checkmark)
    pub current: ModelChoice,
}

impl ModelPickerDialog {
    pub fn new(current_model: &str) -> Self {
        let current = ModelChoice::from_model_id(current_model);
        Self {
            selected: current.clone(),
            current,
        }
    }

    /// Move selection up (wraps around)
    pub fn move_up(&mut self) {
        self.selected = match self.selected {
            ModelChoice::Default => ModelChoice::Haiku,
            ModelChoice::Sonnet => ModelChoice::Default,
            ModelChoice::Haiku => ModelChoice::Sonnet,
        };
    }

    /// Move selection down (wraps around)
    pub fn move_down(&mut self) {
        self.selected = match self.selected {
            ModelChoice::Default => ModelChoice::Sonnet,
            ModelChoice::Sonnet => ModelChoice::Haiku,
            ModelChoice::Haiku => ModelChoice::Default,
        };
    }
}

#[cfg(test)]
#[path = "model_picker_tests.rs"]
mod tests;
```

**File:** `crates/cli/src/tui/widgets/model_picker_tests.rs` (NEW)

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn model_choice_from_opus_model_id() {
    assert_eq!(
        ModelChoice::from_model_id("claude-opus-4-5-20251101"),
        ModelChoice::Default
    );
}

#[test]
fn model_choice_from_sonnet_model_id() {
    assert_eq!(
        ModelChoice::from_model_id("claude-sonnet-4-20250514"),
        ModelChoice::Sonnet
    );
}

#[test]
fn model_choice_from_haiku_model_id() {
    assert_eq!(
        ModelChoice::from_model_id("claude-haiku-4-5-20251101"),
        ModelChoice::Haiku
    );
}

#[test]
fn model_picker_navigation() {
    let mut dialog = ModelPickerDialog::new("claude-opus-4-5-20251101");
    assert_eq!(dialog.selected, ModelChoice::Default);

    dialog.move_down();
    assert_eq!(dialog.selected, ModelChoice::Sonnet);

    dialog.move_down();
    assert_eq!(dialog.selected, ModelChoice::Haiku);

    dialog.move_down(); // Wraps
    assert_eq!(dialog.selected, ModelChoice::Default);

    dialog.move_up(); // Wraps back
    assert_eq!(dialog.selected, ModelChoice::Haiku);
}
```

**File:** `crates/cli/src/tui/widgets/mod.rs`

Add module declaration:
```rust
pub mod model_picker;
```

**Verification:** `cargo test model_picker` passes.

---

### Phase 3: Add ModelPickerDialog State to TuiAppStateInner

**File:** `crates/cli/src/tui/app.rs`

```rust
use crate::tui::widgets::model_picker::ModelPickerDialog;

// In TuiAppStateInner struct:
pub model_picker_dialog: Option<ModelPickerDialog>,

// In TuiAppState::new():
model_picker_dialog: None,
```

**Verification:** Code compiles.

---

### Phase 4: Implement Meta+P Key Handler

**File:** `crates/cli/src/tui/app.rs`

In `handle_input_key()`, add after existing shortcuts:

```rust
// Meta+P - Open model picker
(m, KeyCode::Char('p')) if m.contains(KeyModifiers::META) || m.contains(KeyModifiers::ALT) => {
    inner.model_picker_dialog = Some(ModelPickerDialog::new(&inner.status.model));
    inner.mode = AppMode::ModelPicker;
}
```

In `handle_key_event()`, add dispatch case:

```rust
AppMode::ModelPicker => self.handle_model_picker_key(key),
```

**Verification:** Meta+P changes mode (not yet rendered).

---

### Phase 5: Implement Model Picker Key Handler

**File:** `crates/cli/src/tui/app.rs`

```rust
fn handle_model_picker_key(&self, key: KeyEvent) {
    let mut inner = self.inner.lock();

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut dialog) = inner.model_picker_dialog {
                dialog.move_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
            if let Some(ref mut dialog) = inner.model_picker_dialog {
                dialog.move_down();
            }
        }
        KeyCode::Enter => {
            if let Some(ref dialog) = inner.model_picker_dialog {
                // Apply selection
                inner.status.model = dialog.selected.model_id().to_string();
            }
            inner.model_picker_dialog = None;
            inner.mode = AppMode::Input;
        }
        KeyCode::Esc => {
            // Cancel without changes
            inner.model_picker_dialog = None;
            inner.mode = AppMode::Input;
        }
        _ => {}
    }
}
```

**Verification:** Navigation changes selection, Enter/Esc close dialog.

---

### Phase 6: Implement Model Picker Rendering

**File:** `crates/cli/src/tui/app.rs`

Add rendering function (following `render_thinking_dialog` pattern):

```rust
fn render_model_picker_dialog(dialog: &ModelPickerDialog, width: usize) -> AnyElement<'static> {
    use crate::tui::widgets::model_picker::ModelChoice;

    let choices = ModelChoice::all();
    let content_width = width.saturating_sub(2);

    element! {
        View(flex_direction: FlexDirection::Column) {
            // Title
            View {
                Text(content: " Select model", color: Color::White, weight: Weight::Bold)
            }
            // Description (wrapped)
            View {
                Text(
                    content: " Switch between Claude models. Applies to this session and future Claude Code sessions. For other/previous model names,",
                    color: Color::DarkGrey
                )
            }
            View {
                Text(content: "  specify with --model.", color: Color::DarkGrey)
            }
            // Empty line
            View { Text(content: "") }
            // Options
            #(choices.iter().enumerate().map(|(i, choice)| {
                let is_selected = *choice == dialog.selected;
                let is_current = *choice == dialog.current;

                let cursor = if is_selected { "❯" } else { " " };
                let checkmark = if is_current { " ✔" } else { "" };
                let number = i + 1;

                let label = match choice {
                    ModelChoice::Default => "Default (recommended)",
                    ModelChoice::Sonnet => "Sonnet",
                    ModelChoice::Haiku => "Haiku",
                };

                let description = format!(
                    "{} · {}",
                    choice.display_name(),
                    choice.description()
                );

                // Format: ❯ 1. Label checkmark    Description
                let content = format!(
                    " {} {}. {}{:<22} {}",
                    cursor,
                    number,
                    label,
                    checkmark,
                    description
                );

                element! {
                    View {
                        Text(
                            content: content,
                            color: if is_selected { Color::Cyan } else { Color::White }
                        )
                    }
                }
            }))
            // Empty line
            View { Text(content: "") }
            // Footer
            View {
                Text(content: " Enter to confirm · esc to exit", color: Color::DarkGrey)
            }
        }
    }
    .into_any()
}
```

In main render function, add case for `AppMode::ModelPicker`:

```rust
if state.mode == AppMode::ModelPicker {
    if let Some(ref dialog) = state.model_picker_dialog {
        return Self::render_model_picker_dialog(dialog, width);
    }
}
```

**Verification:** Visual output matches `fixtures/tui/v2.1.12/model_picker.txt`.

---

### Phase 7: Enable Integration Tests

**File:** `crates/cli/tests/tui_model.rs`

Remove `#[ignore]` from:
- `test_tui_meta_p_opens_model_picker` (line 98)
- `test_tui_model_picker_shows_available_models` (line 130)
- `test_tui_model_picker_shows_active_model_checkmark` (line 161)
- `test_tui_model_picker_arrow_navigation` (line 192)
- `test_tui_model_picker_escape_closes` (line 230)
- `test_tui_model_picker_shows_footer_hints` (line 266)

Also remove the `// TODO(implement):` comments.

**Verification:** `cargo test --test tui_model` - all 10 tests pass.

## Key Implementation Details

### Model Choices

| Choice | Model ID | Display | Description |
|--------|----------|---------|-------------|
| Default | `claude-opus-4-5-20251101` | Opus 4.5 | Most capable for complex work |
| Sonnet | `claude-sonnet-4-20250514` | Sonnet 4.5 | Best for everyday tasks |
| Haiku | `claude-haiku-4-5-20251101` | Haiku 4.5 | Fastest for quick answers |

### Visual Indicators

| Indicator | Meaning |
|-----------|---------|
| `❯` | Cursor position (currently highlighted) |
| `✔` | Active model (currently in use) |

### Keyboard Controls

| Key | Action |
|-----|--------|
| `Meta+P` / `Alt+P` | Open model picker |
| `↑` / `k` | Move selection up |
| `↓` / `j` / `Tab` | Move selection down |
| `Enter` | Confirm selection |
| `Esc` | Cancel (no change) |

### Pattern Reference

This implementation follows the established `ThinkingDialog` pattern:
- Widget types in `widgets/model_picker.rs`
- State stored as `Option<ModelPickerDialog>` in `TuiAppStateInner`
- Mode-based key handling in `handle_model_picker_key()`
- Rendering via `render_model_picker_dialog()`

## Verification Plan

1. **Unit Tests:**
   ```bash
   cargo test model_picker
   ```
   Tests navigation and model ID conversion.

2. **Integration Tests:**
   ```bash
   cargo test --test tui_model
   ```
   All 10 tests pass (4 model display + 6 model picker).

3. **Full Check:**
   ```bash
   make check
   ```

4. **Manual Testing:**
   - Press `?` to show shortcuts - verify "meta + p to switch model" appears
   - Press `Meta+P` - model picker dialog opens
   - Use arrow keys to navigate - cursor moves
   - Current model shows checkmark
   - Press `Enter` - model changes, status bar updates
   - Press `Esc` - dialog closes without change

## Files Modified

| File | Changes |
|------|---------|
| `crates/cli/src/tui/app.rs` | +AppMode::ModelPicker, +state field, +Meta+P handler, +model_picker_key handler, +render function |
| `crates/cli/src/tui/widgets/mod.rs` | +model_picker module |
| `crates/cli/src/tui/widgets/model_picker.rs` | NEW: ModelChoice, ModelPickerDialog |
| `crates/cli/src/tui/widgets/model_picker_tests.rs` | NEW: unit tests |
| `crates/cli/tests/tui_model.rs` | Remove `#[ignore]` from 6 tests |
