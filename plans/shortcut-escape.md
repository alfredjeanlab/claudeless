# Plan: Double-Tap Escape to Clear Input

## Overview

Implement support for double-tapping the Escape key to clear input text. When the user has text in the input field and presses Escape once, a hint message "Esc to clear again" is displayed. Pressing Escape again within a 2-second timeout clears the input. If Escape is pressed on an empty input, nothing happens.

## Project Structure

```
crates/cli/
├── src/tui/
│   ├── app.rs           # Main TUI state - modify Escape handling (lines 596-614)
│   └── app_tests.rs     # Unit tests - add new tests for escape behavior
└── tests/
    └── tui_interaction.rs  # Integration tests - enable 4 ignored tests
```

## Dependencies

No new dependencies required. The feature uses existing infrastructure:
- `ExitHint::Escape` variant (already defined at `app.rs:201`)
- `EXIT_HINT_TIMEOUT_MS` constant (2000ms, at `app.rs:207`)
- `exit_hint` / `exit_hint_shown_at` state fields (at `app.rs:314-317`)
- `check_exit_hint_timeout()` method (at `app.rs:1412-1421`)
- `FakeClock` for deterministic time in tests (`time.rs`)

## Implementation Phases

### Phase 1: Modify Escape Key Handler

**File:** `crates/cli/src/tui/app.rs` (lines 596-614)

Current behavior clears input immediately on Escape. Change to:

1. If input is empty: do nothing (no hint, no action)
2. If input has text:
   - Check if `exit_hint == Some(ExitHint::Escape)` and within timeout
   - If within timeout: clear input, clear hint
   - If not within timeout: show hint, record timestamp

```rust
// Escape - Dismiss shortcuts panel first, then exit shell mode, then check for clear
(_, KeyCode::Esc) => {
    if inner.show_shortcuts_panel {
        // First priority: dismiss shortcuts panel
        inner.show_shortcuts_panel = false;
    } else if inner.shell_mode {
        // Second priority: exit shell mode
        inner.shell_mode = false;
        inner.input_buffer.clear();
        inner.cursor_pos = 0;
    } else if inner.slash_menu.is_some() {
        // Third priority: close slash menu (keep text, show hint)
        // Note: handled above at lines 520-528
    } else if !inner.input_buffer.is_empty() {
        // Input has text - check for double-tap
        let now = inner.clock.now_millis();
        let within_timeout = inner.exit_hint == Some(ExitHint::Escape)
            && inner
                .exit_hint_shown_at
                .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                .unwrap_or(false);

        if within_timeout {
            // Second Escape within timeout - clear input
            inner.input_buffer.clear();
            inner.cursor_pos = 0;
            inner.exit_hint = None;
            inner.exit_hint_shown_at = None;
        } else {
            // First Escape - show hint
            inner.exit_hint = Some(ExitHint::Escape);
            inner.exit_hint_shown_at = Some(now);
        }
    }
    // Empty input: do nothing (no else branch)
}
```

**Milestone:** First Escape shows hint, second Escape clears input, empty input does nothing.

### Phase 2: Add Unit Tests

**File:** `crates/cli/src/tui/app_tests.rs`

Add tests following the existing Ctrl+C/Ctrl+D test patterns:

```rust
// ============================================================================
// Escape to clear input
// ============================================================================

#[test]
fn escape_with_text_shows_clear_hint() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));

    // Press Escape
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::Escape));
    assert_eq!(render.input_buffer, "x"); // Input still present
}

#[test]
fn double_escape_clears_input() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));

    // Double-tap Escape
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert!(render.input_buffer.is_empty());
    assert_eq!(render.exit_hint, None);
}

#[test]
fn escape_on_empty_input_does_nothing() {
    let state = create_test_app();

    // Escape on empty input
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.exit_hint, None);
    assert!(render.input_buffer.is_empty());
}

#[test]
fn escape_clear_hint_times_out() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // Type text and press Escape
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    assert_eq!(state.render_state().exit_hint, Some(ExitHint::Escape));

    // Advance time past timeout
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    assert_eq!(state.render_state().exit_hint, None);
    assert_eq!(state.render_state().input_buffer, "x"); // Input not cleared
}

#[test]
fn escape_after_timeout_shows_hint_again() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // Type text, press Escape, wait for timeout
    state.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::empty()));
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    // Press Escape again - should show hint (not clear)
    state.handle_key_event(key_event(KeyCode::Esc, KeyModifiers::empty()));

    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::Escape));
    assert_eq!(render.input_buffer, "x"); // Still present
}
```

**Milestone:** All unit tests pass (`cargo test --all`).

### Phase 3: Enable Integration Tests

**File:** `crates/cli/tests/tui_interaction.rs`

Remove `#[ignore]` from the four escape-related tests (lines 147, 188, 234, 268):

1. `test_tui_escape_shows_clear_hint_with_input` (line 147)
2. `test_tui_double_escape_clears_input` (line 188)
3. `test_tui_escape_on_empty_input_does_nothing` (line 234)
4. `test_tui_escape_clear_hint_timeout` (line 268)

**Milestone:** All integration tests pass.

### Phase 4: Run Full Verification

Execute the full verification suite:

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

**Milestone:** All checks pass, ready for commit.

## Key Implementation Details

### Interaction with Existing Escape Handling

The Escape key currently has a priority system:

1. **Shortcuts panel** (highest): If open, dismiss it
2. **Shell mode**: If active, exit shell mode and clear input
3. **Slash menu** (handled separately at lines 520-528): Close menu, show "Esc to clear again" hint
4. **Normal input** (new): Double-tap to clear

The new double-tap behavior only applies to priority 4 (normal input with text).

### ExitHint::Escape Dual Use

The `ExitHint::Escape` variant is already used when closing the slash menu (line 525). This is intentional - both scenarios use the same hint message "Esc to clear again" and the same timeout. The implementation unifies these behaviors:

- Escape while slash menu open: Close menu, show hint
- Escape with input text (no slash menu): Show hint
- Second Escape within timeout: Clear input

### Hint Timeout Behavior

The existing `check_exit_hint_timeout()` method (line 1412) already handles timeout for all hint types, including `ExitHint::Escape`. No modification needed - the hint will automatically clear after 2 seconds.

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| Empty input | No action, no hint |
| Text, first Escape | Show "Esc to clear again" hint |
| Text, second Escape within 2s | Clear input and hint |
| Text, second Escape after 2s | Show hint again (not clear) |
| Typing clears hint | Handled by existing hint system |
| Shortcuts panel open | Dismiss panel (priority 1) |
| Shell mode active | Exit shell mode (priority 2) |
| Slash menu open | Close menu, show hint (priority 3) |

## Verification Plan

### Unit Tests (Phase 2)

Test each behavior in isolation with `FakeClock` for deterministic timing:

| Test | Behavior |
|------|----------|
| `escape_with_text_shows_clear_hint` | First Escape shows hint, keeps text |
| `double_escape_clears_input` | Second Escape within timeout clears |
| `escape_on_empty_input_does_nothing` | Empty input = no action |
| `escape_clear_hint_times_out` | Hint clears after 2s, text remains |
| `escape_after_timeout_shows_hint_again` | After timeout, need double-tap again |

### Integration Tests (Phase 3)

Test full TUI behavior with tmux:

| Test | Behavior |
|------|----------|
| `test_tui_escape_shows_clear_hint_with_input` | UI shows hint message |
| `test_tui_double_escape_clears_input` | Input field clears visually |
| `test_tui_escape_on_empty_input_does_nothing` | No visual change |
| `test_tui_escape_clear_hint_timeout` | Hint disappears after ~2.5s |

### Manual Verification

1. Run `cargo run` in `crates/cli`
2. Type some text
3. Press Escape - verify "Esc to clear again" appears in status bar
4. Press Escape again - verify input clears
5. Type text, press Escape, wait 3 seconds - verify hint disappears but text remains
6. Verify Escape on empty input does nothing
