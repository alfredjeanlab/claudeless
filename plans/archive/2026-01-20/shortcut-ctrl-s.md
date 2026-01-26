# Implementation Plan: Ctrl+S Stash Prompt Shortcut

**Root Feature:** `cl-b9d9`

## Overview

Implement Ctrl+S keyboard shortcut to stash and restore the current input prompt. When pressed with text in the input, the text is saved to a stash buffer and the input is cleared. When pressed again (with a stash existing), the stashed text is restored. After submitting a prompt and receiving a response, any stashed text is automatically restored.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs              # Add stash state fields and Ctrl+S handler
│   ├── app_tests.rs        # Add unit tests for stash behavior
│   └── shortcuts.rs        # Already includes "ctrl + s to stash prompt" (line 81)

crates/cli/tests/
└── tui_stash.rs            # Remove #[ignore] from 5 existing tests
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm 0.28` for key event handling

## Implementation Phases

### Phase 1: Add Stash State Fields

Add state fields to track the stashed prompt and indicator visibility.

**File:** `crates/cli/src/tui/app.rs`

Add to `TuiAppStateInner` struct (after `undo_stack` at line 378):
```rust
/// Stashed input text (Ctrl+S to stash/restore)
pub stash_buffer: Option<String>,

/// Whether to show the stash indicator message
pub show_stash_indicator: bool,
```

Add to `RenderState` struct (after `export_dialog` at line 184):
```rust
/// Stashed input text (for checking in tests)
pub stash_buffer: Option<String>,

/// Whether to show "Stashed (auto-restores after submit)" message
pub show_stash_indicator: bool,
```

Update `render_state()` method (after line 483):
```rust
stash_buffer: inner.stash_buffer.clone(),
show_stash_indicator: inner.show_stash_indicator,
```

Initialize in `new()` method (after `undo_stack: Vec::new()` around line 455):
```rust
stash_buffer: None,
show_stash_indicator: false,
```

**Verification:** `cargo check -p cli` compiles without errors.

### Phase 2: Implement Ctrl+S Key Handler

Handle Ctrl+S keypress to stash/restore the input buffer.

**File:** `crates/cli/src/tui/app.rs`

Add match arm in `handle_input_key()` (after Ctrl+/ handler around line 843):
```rust
// Ctrl+S - Stash/restore prompt
// Note: Ctrl+S is encoded as ASCII 0x13 (device control 3) in terminals
(_, KeyCode::Char('\x13')) => {
    if let Some(stashed) = inner.stash_buffer.take() {
        // Restore: stash exists, restore it to input
        inner.input_buffer = stashed;
        inner.cursor_pos = inner.input_buffer.len();
        inner.show_stash_indicator = false;
    } else if !inner.input_buffer.is_empty() {
        // Stash: input is not empty, save it
        inner.stash_buffer = Some(std::mem::take(&mut inner.input_buffer));
        inner.cursor_pos = 0;
        inner.show_stash_indicator = true;
    }
    // If input is empty and no stash exists, do nothing
}
(m, KeyCode::Char('s')) if m.contains(KeyModifiers::CONTROL) => {
    if let Some(stashed) = inner.stash_buffer.take() {
        // Restore: stash exists, restore it to input
        inner.input_buffer = stashed;
        inner.cursor_pos = inner.input_buffer.len();
        inner.show_stash_indicator = false;
    } else if !inner.input_buffer.is_empty() {
        // Stash: input is not empty, save it
        inner.stash_buffer = Some(std::mem::take(&mut inner.input_buffer));
        inner.cursor_pos = 0;
        inner.show_stash_indicator = true;
    }
    // If input is empty and no stash exists, do nothing
}
```

**Verification:** Unit tests for Ctrl+S key recognition pass.

### Phase 3: Display Stash Indicator Message

Show "Stashed (auto-restores after submit)" message when a stash exists.

**File:** `crates/cli/src/tui/app.rs`

Locate the input area rendering in `render_main_content()` (around line 2200). Add the stash indicator display before the input line:

```rust
// Format stash indicator if present
let stash_indicator = if state.show_stash_indicator {
    format!(
        "{}  {} Stashed (auto-restores after submit)\n",
        " ".repeat(prompt_len),
        styled("›", styles.accent)
    )
} else {
    String::new()
};
```

Insert this before the input display in the element construction.

**Verification:** Manual test shows stash message appears above input when stashed.

### Phase 4: Auto-Restore Stash After Response

When a prompt is submitted and response completes, automatically restore any stashed text.

**File:** `crates/cli/src/tui/app.rs`

In `stream_response()` (around line 1579, after `inner.mode = AppMode::Input;`):
```rust
// Auto-restore stashed text after response completes
if let Some(stashed) = inner.stash_buffer.take() {
    inner.input_buffer = stashed;
    inner.cursor_pos = inner.input_buffer.len();
    // Keep show_stash_indicator true briefly to indicate restore happened
    // Actually, per test expectations, the text should just appear in input
    inner.show_stash_indicator = false;
}
```

Similarly update shell command completion in `execute_shell_command()` where mode returns to Input.

**Verification:** Integration test `test_tui_stash_auto_restores_after_submit` passes.

### Phase 5: Add Unit Tests

Add unit tests following the sibling file convention.

**File:** `crates/cli/src/tui/app_tests.rs`

```rust
// ========================
// Ctrl+S Stash Tests
// ========================

#[test]
fn ctrl_s_stashes_non_empty_input() {
    let state = create_test_app();

    // Type some text
    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    assert_eq!(state.input_buffer(), "hello world");

    // Ctrl+S to stash
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    // Input should be cleared
    assert_eq!(state.input_buffer(), "");
    let render = state.render_state();
    assert_eq!(render.stash_buffer, Some("hello world".to_string()));
    assert!(render.show_stash_indicator);
}

#[test]
fn ctrl_s_empty_input_does_nothing() {
    let state = create_test_app();

    // Ctrl+S with empty input
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    // Nothing should change
    assert_eq!(state.input_buffer(), "");
    let render = state.render_state();
    assert_eq!(render.stash_buffer, None);
    assert!(!render.show_stash_indicator);
}

#[test]
fn ctrl_s_restores_stashed_text() {
    let state = create_test_app();

    // Type and stash
    for c in "stashed text".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    assert_eq!(state.input_buffer(), "");

    // Ctrl+S again to restore
    state.handle_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL));

    // Stashed text should be restored
    assert_eq!(state.input_buffer(), "stashed text");
    let render = state.render_state();
    assert_eq!(render.stash_buffer, None);
    assert!(!render.show_stash_indicator);
}

#[test]
fn ctrl_s_raw_char_works() {
    let state = create_test_app();

    // Type some text
    for c in "test".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Ctrl+S may come as raw ASCII 0x13
    state.handle_key_event(key_event(KeyCode::Char('\x13'), KeyModifiers::NONE));

    assert_eq!(state.input_buffer(), "");
    assert!(state.render_state().show_stash_indicator);
}
```

**Verification:** `cargo test -p cli ctrl_s` passes.

### Phase 6: Enable Integration Tests

Remove `#[ignore]` from the five existing integration tests.

**File:** `crates/cli/tests/tui_stash.rs`

Remove `#[ignore]` and `// TODO(implement):` comments from:
1. `test_tui_ctrl_s_stashes_prompt_with_message` (line 32)
2. `test_tui_ctrl_s_restores_stashed_prompt` (line 77)
3. `test_tui_ctrl_s_empty_input_does_nothing` (line 125)
4. `test_tui_stash_message_persists` (line 161)
5. `test_tui_stash_auto_restores_after_submit` (line 213)

**Verification:** All five integration tests pass with `cargo test tui_stash`.

## Key Implementation Details

### Terminal Key Encoding

Ctrl+S is encoded as ASCII 0x13 (device control 3 / XOFF). Handle both representations:
- Raw character `\x13` (some terminals)
- `KeyCode::Char('s')` with `KeyModifiers::CONTROL` (crossterm normalization)

### Stash vs. Restore Logic

The toggle behavior:
1. **If stash exists**: Restore it to input, clear stash, hide indicator
2. **If input is non-empty**: Save to stash, clear input, show indicator
3. **If input is empty and no stash**: Do nothing

### Message Format

Display format per test expectations:
```
  › Stashed (auto-restores after submit)
```
- Positioned above the input line
- Uses accent color for the `›` character
- Persists until stash is restored or auto-restored

### Auto-Restore Timing

Auto-restore happens:
- After a response completes (streaming finishes)
- After shell command output completes
- The stashed text appears in the input buffer
- The indicator disappears

### Cursor Position

- **On stash**: Cursor moves to position 0 (input cleared)
- **On restore**: Cursor moves to end of restored text (`len()`)

## Verification Plan

1. **Unit Tests:** Run `cargo test -p cli ctrl_s` for app_tests.rs
2. **Integration Tests:** Run `cargo test tui_stash --no-fail-fast`
3. **Full Check:** Run `make check`:
   - `make lint` (shellcheck)
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   - `cargo build --all`
   - `cargo audit`
   - `cargo deny check`
4. **Manual Test:**
   - Type text, press Ctrl+S - text disappears, message appears
   - Press Ctrl+S again - text reappears, message disappears
   - Stash text, type new prompt, submit - after response, stashed text auto-restores
   - Press Ctrl+S on empty input - no change
