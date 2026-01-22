# Implementation Plan: Ctrl+_ Undo Shortcut

**Root Feature:** `cl-576f`

## Overview

Implement Ctrl+_ keyboard shortcut to undo input text progressively. Each press removes the last typed word/segment, allowing users to quickly revert their input. The shortcut documentation already exists in the shortcuts widget; this plan covers the actual functionality.

## Project Structure

```
crates/cli/src/tui/
├── app.rs              # Add undo handler and history tracking
├── app_tests.rs        # Add unit tests for undo behavior
└── shortcuts.rs        # Already documents "ctrl + _ to undo"

crates/cli/tests/
└── tui_interaction.rs  # Remove #[ignore] from 3 existing tests
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm 0.28` for key event handling

## Implementation Phases

### Phase 1: Add Undo History Tracking to State

Add undo history field to `TuiAppStateInner` in `app.rs`.

**File:** `crates/cli/src/tui/app.rs`

```rust
struct TuiAppStateInner {
    // ... existing fields ...

    /// Stack of previous input states for undo (Ctrl+_)
    /// Each entry is a snapshot of input_buffer before a change
    pub undo_stack: Vec<String>,
}
```

Initialize in `TuiAppStateInner::new()`:
```rust
undo_stack: Vec::new(),
```

**Verification:** Code compiles with `cargo check`.

### Phase 2: Track Input Changes for Undo

Modify input-modifying operations to push state to undo stack before making changes. The key insight from tests: undo should work at word/segment granularity, not character-by-character.

**Strategy:** Push to undo stack on:
1. First character typed after a space or at start (new word boundary)
2. Paste operations
3. Before delete/clear operations (so Ctrl+_ can restore)

**File:** `crates/cli/src/tui/app.rs`

Add helper method:
```rust
impl TuiAppStateInner {
    /// Push current input state to undo stack if appropriate
    fn push_undo_snapshot(&mut self) {
        // Push if stack is empty or last snapshot differs from current
        if self.undo_stack.last() != Some(&self.input_buffer) {
            self.undo_stack.push(self.input_buffer.clone());
        }
    }

    /// Clear undo stack (e.g., when submitting input)
    fn clear_undo_stack(&mut self) {
        self.undo_stack.clear();
    }
}
```

Track word boundaries in character insertion:
```rust
// In handle_input_key() where Char(c) is handled:
(_, KeyCode::Char(c)) => {
    // Push snapshot at word boundaries (space typed or first char of new word)
    let should_snapshot = c == ' ' ||
        inner.input_buffer.is_empty() ||
        inner.input_buffer.ends_with(' ');

    if should_snapshot {
        inner.push_undo_snapshot();
    }

    // ... existing insert logic ...
}
```

**Verification:** Unit test that verifies undo_stack grows at word boundaries.

### Phase 3: Implement Ctrl+_ Key Handler

Add the key handler in `handle_input_key()` match block.

**File:** `crates/cli/src/tui/app.rs` (around line 756, near Ctrl+U and Ctrl+W)

```rust
// Ctrl+_ - Undo last input segment
(m, KeyCode::Char('_')) if m.contains(KeyModifiers::CONTROL) => {
    if let Some(previous) = inner.undo_stack.pop() {
        inner.input_buffer = previous;
        // Clamp cursor position to new buffer length
        inner.cursor_pos = inner.cursor_pos.min(inner.input_buffer.len());
    }
    // If undo_stack is empty, do nothing (per test requirements)
}
```

**Verification:** Unit test that Ctrl+_ restores previous state.

### Phase 4: Add Unit Tests

Add unit tests in sibling test file following project conventions.

**File:** `crates/cli/src/tui/app_tests.rs`

```rust
// ========================
// Ctrl+_ Undo Tests
// ========================

#[test]
fn ctrl_underscore_undoes_to_previous_word_boundary() {
    let state = create_test_app();

    // Type "hello world"
    for c in "hello ".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    for c in "world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    assert_eq!(state.render_state().input_buffer, "hello world");

    // Ctrl+_ should undo back to "hello "
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    assert_eq!(state.render_state().input_buffer, "hello ");
}

#[test]
fn ctrl_underscore_on_empty_does_nothing() {
    let state = create_test_app();

    // Press Ctrl+_ on empty input
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));

    assert_eq!(state.render_state().input_buffer, "");
}

#[test]
fn ctrl_underscore_clears_all_with_multiple_presses() {
    let state = create_test_app();

    // Type "one two three"
    for word in ["one ", "two ", "three"] {
        for c in word.chars() {
            state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    // Undo all words
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));

    assert_eq!(state.render_state().input_buffer, "");
}

#[test]
fn undo_stack_clears_on_submit() {
    let state = create_test_app();

    for c in "test".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Submit clears undo stack
    state.handle_key_event(key_event(KeyCode::Enter, KeyModifiers::NONE));

    // Ctrl+_ should do nothing now
    state.handle_key_event(key_event(KeyCode::Char('_'), KeyModifiers::CONTROL));
    assert_eq!(state.render_state().input_buffer, "");
}
```

**Verification:** `cargo test --all` passes unit tests.

### Phase 5: Enable Integration Tests

Remove `#[ignore]` from the three existing integration tests.

**File:** `crates/cli/tests/tui_interaction.rs`

Tests to enable (lines 306-435):
1. `test_tui_ctrl_underscore_undoes_last_word` (line 317)
2. `test_tui_ctrl_underscore_clears_all_input` (line 363)
3. `test_tui_ctrl_underscore_on_empty_input_does_nothing` (line 408)

Also remove the `// TODO(implement):` comments.

**Verification:** All three integration tests pass.

### Phase 6: Clear Undo Stack on Submit/Reset

Ensure undo stack is cleared when appropriate to prevent stale state.

**File:** `crates/cli/src/tui/app.rs`

Clear undo stack in:
1. `handle_submit()` - after user presses Enter
2. When Escape clears input (if applicable)
3. When loading new prompt from history (Up/Down arrows)

```rust
// In handle_submit() or where Enter is processed:
inner.clear_undo_stack();

// In history navigation (Up/Down arrows):
inner.clear_undo_stack();
```

**Verification:** Full test suite passes.

## Key Implementation Details

### Word Boundary Detection

The tests expect undo to work at word boundaries:
- "first second third" → Ctrl+_ → "first second" (not "first second thir")

Push undo snapshots when:
- Input is empty and first character typed
- Character typed after a space (starting new word)
- Space is typed (completing a word)

### Cursor Position Handling

After undo, cursor position must be clamped:
```rust
inner.cursor_pos = inner.cursor_pos.min(inner.input_buffer.len());
```

This prevents cursor from pointing past end of restored shorter buffer.

### Terminal Key Encoding

Ctrl+_ is often encoded as Ctrl+Shift+- in terminals. Crossterm handles this transparently - the KeyCode will be `Char('_')` with `KeyModifiers::CONTROL`.

In tmux tests, the key is sent as `C-_`:
```rust
tmux::send_keys(session, "C-_");
```

### Empty Input Behavior

Per test requirements, pressing Ctrl+_ on empty input should do nothing (no error, no visual feedback). The handler simply returns early if undo_stack is empty.

## Verification Plan

1. **Unit Tests:** Run `cargo test -p cli` for app_tests.rs
2. **Integration Tests:** Run the three tui_interaction tests:
   ```bash
   cargo test test_tui_ctrl_underscore --no-fail-fast
   ```
3. **Full Check:** Run `make check` for:
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   - `cargo build --all`
   - `cargo audit`
   - `cargo deny check`
4. **Manual Test:** Build and run the CLI, verify Ctrl+_ works interactively
