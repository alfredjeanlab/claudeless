# Implementation Plan: Shortcuts Cleanup

## Overview

Review, cleanup, and enhance the input shortcuts implementation chain. This includes eliminating code duplication across shortcut handlers (Ctrl+Z, Ctrl+_, Ctrl+S), standardizing key event handling patterns, removing dead code and unused imports, and enhancing test coverage for all shortcuts (?, !, Escape, Ctrl+T, Meta+P, Ctrl+_, Ctrl+Z, Ctrl+S).

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs              # Main key event handlers (lines 586-977)
│   ├── app_tests.rs        # 53 unit tests for key handling
│   ├── shortcuts.rs        # Shortcuts data definitions
│   └── shortcuts_tests.rs  # Tests for shortcuts data

crates/cli/tests/
├── tui_shortcuts.rs        # ? shortcut integration tests
├── tui_shell_mode.rs       # ! shortcut tests (2 #[ignore])
├── tui_stash.rs            # Ctrl+S tests
├── tui_suspend.rs          # Ctrl+Z tests
├── tui_interaction.rs      # Escape, Ctrl+_ tests (3 #[ignore])
├── tui_todos.rs            # Ctrl+T tests
└── tui_model.rs            # Meta+P tests
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm 0.28` for key event handling

## Implementation Phases

### Phase 1: Create Key Encoding Helper Macro

Create a macro to consolidate the duplicate terminal encoding patterns. Different terminals send control keys either as raw ASCII or as modifier+character combinations. Currently, this causes ~51 lines of duplicated handler code.

**File:** `crates/cli/src/tui/app.rs`

Add a helper macro near the top of the file (after imports):

```rust
/// Matches a control key that may be encoded as raw ASCII or as modifier+char.
///
/// Terminal encoding varies - some send raw ASCII codes (e.g., Ctrl+S as 0x13),
/// while others send the character with CONTROL modifier. This macro handles both.
macro_rules! ctrl_key {
    // Ctrl+Z: ASCII 0x1A or 'z' with CONTROL
    (z, $modifiers:expr, $code:expr) => {
        matches!(
            ($modifiers, $code),
            (_, KeyCode::Char('\x1a')) |
            (m, KeyCode::Char('z')) if m.contains(KeyModifiers::CONTROL)
        )
    };
    // Ctrl+S: ASCII 0x13 or 's' with CONTROL
    (s, $modifiers:expr, $code:expr) => {
        matches!(
            ($modifiers, $code),
            (_, KeyCode::Char('\x13')) |
            (m, KeyCode::Char('s')) if m.contains(KeyModifiers::CONTROL)
        )
    };
    // Ctrl+_: ASCII 0x1F or '_' with CONTROL or '/' with CONTROL (same ASCII)
    (underscore, $modifiers:expr, $code:expr) => {
        matches!(
            ($modifiers, $code),
            (_, KeyCode::Char('\x1f')) |
            (m, KeyCode::Char('_')) if m.contains(KeyModifiers::CONTROL) |
            (m, KeyCode::Char('/')) if m.contains(KeyModifiers::CONTROL)
        )
    };
}
```

**Verification:** `cargo check -p cli` compiles without errors.

### Phase 2: Consolidate Duplicate Key Handlers

Replace the duplicated match arms with calls to the helper macro, reducing ~51 lines to ~15 lines.

**File:** `crates/cli/src/tui/app.rs`

**2a. Consolidate Ctrl+Z handler** (currently 2 identical blocks at lines 638-645)

Before:
```rust
(_, KeyCode::Char('\x1a')) => {
    should_exit = true;
    exit_reason = Some(ExitReason::Suspended);
}
(m, KeyCode::Char('z')) if m.contains(KeyModifiers::CONTROL) => {
    should_exit = true;
    exit_reason = Some(ExitReason::Suspended);
}
```

After:
```rust
_ if ctrl_key!(z, key.modifiers, key.code) => {
    should_exit = true;
    exit_reason = Some(ExitReason::Suspended);
}
```

**2b. Consolidate Ctrl+_ handler** (currently 3 identical blocks at lines 849-867)

Before:
```rust
(_, KeyCode::Char('\x1f')) => {
    if let Some(snapshot) = inner.undo_stack.pop() {
        inner.input_buffer = snapshot;
        inner.cursor_pos = inner.input_buffer.len();
    }
}
(m, KeyCode::Char('_')) if m.contains(KeyModifiers::CONTROL) => {
    // ... identical code ...
}
(m, KeyCode::Char('/')) if m.contains(KeyModifiers::CONTROL) => {
    // ... identical code ...
}
```

After:
```rust
_ if ctrl_key!(underscore, key.modifiers, key.code) => {
    if let Some(snapshot) = inner.undo_stack.pop() {
        inner.input_buffer = snapshot;
        inner.cursor_pos = inner.input_buffer.len();
    }
}
```

**2c. Consolidate Ctrl+S handler** (currently 2 identical blocks at lines 871-898)

Before:
```rust
(_, KeyCode::Char('\x13')) => {
    if let Some(stashed) = inner.stash_buffer.take() {
        inner.input_buffer = stashed;
        inner.cursor_pos = inner.input_buffer.len();
        inner.show_stash_indicator = false;
    } else if !inner.input_buffer.is_empty() {
        inner.stash_buffer = Some(std::mem::take(&mut inner.input_buffer));
        inner.cursor_pos = 0;
        inner.show_stash_indicator = true;
    }
}
(m, KeyCode::Char('s')) if m.contains(KeyModifiers::CONTROL) => {
    // ... identical 10 lines ...
}
```

After:
```rust
_ if ctrl_key!(s, key.modifiers, key.code) => {
    if let Some(stashed) = inner.stash_buffer.take() {
        inner.input_buffer = stashed;
        inner.cursor_pos = inner.input_buffer.len();
        inner.show_stash_indicator = false;
    } else if !inner.input_buffer.is_empty() {
        inner.stash_buffer = Some(std::mem::take(&mut inner.input_buffer));
        inner.cursor_pos = 0;
        inner.show_stash_indicator = true;
    }
}
```

**Verification:** All 53 unit tests pass, `cargo test -p cli`.

### Phase 3: Standardize Modifier Checking Patterns

Standardize the inconsistent modifier checking patterns across all shortcut handlers.

**File:** `crates/cli/src/tui/app.rs`

Current inconsistencies:
```rust
// Pattern 1: is_empty() check
(m, KeyCode::Char('?')) if m.is_empty() || m == KeyModifiers::SHIFT

// Pattern 2: contains() check
(m, KeyCode::Char('p')) if m.contains(KeyModifiers::META) || m.contains(KeyModifiers::ALT)
```

**3a. Document the chosen pattern:**
- Use `is_empty()` for checking no modifiers required
- Use `contains()` for checking specific modifiers are present
- Use `== KeyModifiers::SHIFT` only when SHIFT is the *only* acceptable modifier

**3b. Audit and fix any inconsistent patterns:**

Review each handler and ensure consistent pattern:
- `?` shortcut: `m.is_empty() || m == KeyModifiers::SHIFT` (correct - only shift or nothing)
- `!` shortcut: `m.is_empty() || m == KeyModifiers::SHIFT` (correct - only shift or nothing)
- Meta+P: `m.contains(KeyModifiers::META) || m.contains(KeyModifiers::ALT)` (correct - either meta or alt)
- Meta+T: Verify same pattern as Meta+P

**Verification:** Manual review confirms consistent patterns.

### Phase 4: Remove Dead Code and Unused Imports

Identify and remove any unused imports or dead code in the shortcut handling modules.

**File:** `crates/cli/src/tui/app.rs`

**4a. Audit imports:**
Run `cargo clippy --all-targets --all-features -- -D warnings` and check for:
- `unused_imports` warnings
- `dead_code` warnings

**4b. Remove any identified dead code:**
- Check for unreachable match arms
- Remove commented-out code blocks
- Remove unused helper functions

**4c. Clean up comments:**
- Remove outdated TODO comments
- Ensure documentation comments are accurate

**Verification:** `cargo clippy` reports no warnings related to imports or dead code.

### Phase 5: Enhance Unit Test Coverage

Add missing unit tests for edge cases in shortcut handling.

**File:** `crates/cli/src/tui/app_tests.rs`

**5a. Add edge case tests for existing shortcuts:**

```rust
// ========================
// Additional Shortcut Edge Cases
// ========================

#[test]
fn ctrl_z_works_with_input_present() {
    // Verify Ctrl+Z suspends even when input buffer has text
    let state = create_test_app();
    for c in "some text".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }
    state.handle_key_event(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(state.should_exit());
}

#[test]
fn ctrl_z_raw_char_suspends() {
    // Verify raw ASCII 0x1A encoding works
    let state = create_test_app();
    state.handle_key_event(key_event(KeyCode::Char('\x1a'), KeyModifiers::NONE));
    assert!(state.should_exit());
}

#[test]
fn meta_t_opens_thinking_dialog() {
    let state = create_test_app();
    state.handle_key_event(key_event(KeyCode::Char('t'), KeyModifiers::META));
    assert_eq!(state.mode(), AppMode::ThinkingToggle);
}

#[test]
fn alt_t_opens_thinking_dialog() {
    // ALT should work same as META
    let state = create_test_app();
    state.handle_key_event(key_event(KeyCode::Char('t'), KeyModifiers::ALT));
    assert_eq!(state.mode(), AppMode::ThinkingToggle);
}

#[test]
fn meta_p_opens_model_picker() {
    let state = create_test_app();
    state.handle_key_event(key_event(KeyCode::Char('p'), KeyModifiers::META));
    assert_eq!(state.mode(), AppMode::ModelPicker);
}

#[test]
fn escape_priority_shortcuts_then_shell_then_input() {
    // Verify escape dismisses shortcuts panel first
    let state = create_test_app();

    // Show shortcuts panel
    state.handle_key_event(key_event(KeyCode::Char('?'), KeyModifiers::NONE));
    assert!(state.render_state().show_shortcuts);

    // Enter shell mode (should be hidden by shortcuts panel)
    // Actually, ? only works on empty input, so:
    // ... test the priority logic
}

#[test]
fn double_escape_only_clears_non_empty_input() {
    let state = create_test_app();

    // Type some text
    for c in "test".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // First escape shows hint
    state.handle_key_event(key_event(KeyCode::Escape, KeyModifiers::NONE));
    assert!(!state.input_buffer().is_empty()); // Still has text

    // Second escape clears
    state.handle_key_event(key_event(KeyCode::Escape, KeyModifiers::NONE));
    assert!(state.input_buffer().is_empty()); // Now cleared
}
```

**Verification:** `cargo test -p cli` with new tests passing.

### Phase 6: Review #[ignore] Tests and Run Full Check

Review all `#[ignore]` tests, determine if they can now pass, and run the full verification suite.

**Files:**
- `crates/cli/tests/tui_shell_mode.rs` (2 ignored)
- `crates/cli/tests/tui_interaction.rs` (3 ignored)

**6a. Review ignored tests:**

| Test | File | Reason | Action |
|------|------|--------|--------|
| `test_tui_shell_prefix_ansi_matches_fixture_v2115` | tui_shell_mode.rs:401 | Requires shell mode status bar hiding | Keep ignored with clear comment |
| `test_tui_shell_command_ansi_matches_fixture_v2115` | tui_shell_mode.rs:434 | Requires cursor block rendering | Keep ignored with clear comment |
| `test_input_display_matches_fixture` | tui_interaction.rs:113 | Simulator shows status bar while fixture does not | Keep ignored with clear comment |
| `test_tui_ctrl_underscore_undoes_last_word` | tui_interaction.rs:318 | tmux cannot reliably send Ctrl+_ | Keep ignored, unit tests verify |
| `test_tui_ctrl_underscore_clears_all_input` | tui_interaction.rs:366 | tmux cannot reliably send Ctrl+_ | Keep ignored, unit tests verify |

**6b. Update ignored test comments:**

Ensure each `#[ignore]` has a clear `// Reason:` comment explaining:
1. Why it's ignored
2. What would need to change for it to pass
3. Alternative coverage (e.g., "unit tests verify this behavior")

**6c. Run full verification:**

```bash
make check
```

This runs:
- `make lint` (shellcheck)
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

**Verification:** All checks pass with zero warnings.

## Key Implementation Details

### Terminal Key Encoding

Control keys may be encoded two ways by different terminals:

| Shortcut | Raw ASCII | Modifier Pattern |
|----------|-----------|------------------|
| Ctrl+Z | `\x1a` (0x1A) | `Char('z')` + CONTROL |
| Ctrl+S | `\x13` (0x13) | `Char('s')` + CONTROL |
| Ctrl+_ | `\x1f` (0x1F) | `Char('_')` + CONTROL |
| Ctrl+/ | `\x1f` (0x1F) | `Char('/')` + CONTROL |

The `ctrl_key!` macro abstracts this complexity.

### Escape Priority Chain

Escape key has complex priority-based behavior:
1. **Priority 1:** Dismiss shortcuts panel if shown
2. **Priority 2:** Exit shell mode if in shell mode
3. **Priority 3:** Double-tap to clear input (with timeout)
4. **Priority 4:** Do nothing if input is empty

### Modifier Checking Conventions

- `m.is_empty()` - No modifiers pressed
- `m == KeyModifiers::SHIFT` - Only SHIFT is pressed
- `m.contains(KeyModifiers::CONTROL)` - CONTROL is pressed (possibly with others)
- `m.contains(KeyModifiers::META) || m.contains(KeyModifiers::ALT)` - Either META or ALT

### Test Organization

- Unit tests in `app_tests.rs` test key handling logic directly
- Integration tests in `tests/tui_*.rs` test full TUI rendering
- Some shortcuts (Ctrl+_, Ctrl+/) cannot be reliably sent via tmux, so rely on unit tests

## Verification Plan

1. **Phase 1:** `cargo check -p cli` compiles
2. **Phase 2:** `cargo test -p cli` - all 53+ unit tests pass
3. **Phase 3:** Manual review of patterns
4. **Phase 4:** `cargo clippy` - no dead code warnings
5. **Phase 5:** `cargo test -p cli` - new tests pass
6. **Phase 6:** `make check` - full suite passes

Final verification:
```bash
make check
```

Expected output: All lints, tests, and builds pass with zero warnings.
