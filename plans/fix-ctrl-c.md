# Plan: Fix Ctrl+C and Ctrl+D Exit Confirmation

**Root Feature:** `cl-07a3`

## Overview

Implement double-press exit confirmation with timeout for Ctrl+C and Ctrl+D in the TUI, matching real Claude CLI v2.1.12 behavior. The first press shows an exit hint in the status bar; a second press within ~2 seconds exits. The hint times out and returns to the normal status bar after the timeout period.

## Project Structure

Key files to modify:

```
crates/cli/src/tui/
├── app.rs              # Main TUI state and key handling (primary changes)
└── test_helpers.rs     # Test harness updates for exit confirmation

crates/cli/src/time.rs  # Already has FakeClock for deterministic testing
```

Test files:
```
crates/cli/tests/
└── tui_exit.rs         # Exit behavior tests (enable 5 ignored tests)

crates/cli/src/tui/
└── app_tests.rs        # Unit tests for exit confirmation logic (new file)
```

## Dependencies

No new external dependencies required. Uses existing:
- `iocraft` for TUI rendering
- `time.rs` clock abstraction for timeout handling

## Implementation Phases

### Phase 1: Add Exit Hint State Tracking

**Goal:** Add state to track exit hint display and timeout.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Add new enum for exit hint type:

```rust
/// Type of exit hint being shown
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitHint {
    /// "Press Ctrl-C again to exit"
    CtrlC,
    /// "Press Ctrl-D again to exit"
    CtrlD,
}
```

2. Add fields to `TuiAppStateInner`:

```rust
/// Active exit hint (if any)
pub exit_hint: Option<ExitHint>,

/// When exit hint was shown (milliseconds from clock)
pub exit_hint_shown_at: Option<u64>,
```

3. Add constant for timeout duration:

```rust
/// Exit hint timeout in milliseconds (2 seconds)
const EXIT_HINT_TIMEOUT_MS: u64 = 2000;
```

4. Add fields to `RenderState`:

```rust
pub exit_hint: Option<ExitHint>,
```

5. Update `render_state()` to include exit hint.

### Phase 2: Implement Exit Hint Timeout Check

**Goal:** Clear exit hint after timeout period.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Add `check_exit_hint_timeout()` method to `TuiAppState`:

```rust
/// Check if exit hint has timed out and clear it
pub fn check_exit_hint_timeout(&self) {
    let mut inner = self.inner.lock();
    if let (Some(_hint), Some(shown_at)) = (&inner.exit_hint, inner.exit_hint_shown_at) {
        let now = inner.clock.now_millis();
        if now.saturating_sub(shown_at) >= EXIT_HINT_TIMEOUT_MS {
            inner.exit_hint = None;
            inner.exit_hint_shown_at = None;
        }
    }
}
```

2. Call `check_exit_hint_timeout()` in the timer loop (alongside `check_compacting()`):

```rust
// In App component's use_future hook
hooks.use_future({
    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let current = *timer_counter.read();
            timer_counter.set(current.wrapping_add(1));
        }
    }
});

// Check for timeouts (both compacting and exit hint)
state_clone.check_compacting();
state_clone.check_exit_hint_timeout();
```

### Phase 3: Update Ctrl+C Handling

**Goal:** Implement double-press confirmation for Ctrl+C.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Update `handle_interrupt()` for input mode:

```rust
fn handle_interrupt(&self) {
    let mut inner = self.inner.lock();
    match inner.mode {
        AppMode::Input => {
            // Check if within exit hint timeout
            let now = inner.clock.now_millis();
            let within_timeout = inner.exit_hint == Some(ExitHint::CtrlC)
                && inner.exit_hint_shown_at
                    .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                    .unwrap_or(false);

            if within_timeout {
                // Second Ctrl+C within timeout - exit
                inner.should_exit = true;
                inner.exit_reason = Some(ExitReason::Interrupted);
            } else {
                // First Ctrl+C - clear input (if any) and show hint
                inner.input_buffer.clear();
                inner.cursor_pos = 0;
                inner.exit_hint = Some(ExitHint::CtrlC);
                inner.exit_hint_shown_at = Some(now);
            }
        }
        // ... rest of match arms unchanged
    }
}
```

### Phase 4: Update Ctrl+D Handling

**Goal:** Implement double-press confirmation for Ctrl+D.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Update the Ctrl+D handler in `handle_input_key()`:

```rust
// Ctrl+D - Exit (only on empty input)
(m, KeyCode::Char('d')) if m.contains(KeyModifiers::CONTROL) => {
    if inner.input_buffer.is_empty() {
        let now = inner.clock.now_millis();
        let within_timeout = inner.exit_hint == Some(ExitHint::CtrlD)
            && inner.exit_hint_shown_at
                .map(|t| now.saturating_sub(t) < EXIT_HINT_TIMEOUT_MS)
                .unwrap_or(false);

        if within_timeout {
            // Second Ctrl+D within timeout - exit
            inner.should_exit = true;
            inner.exit_reason = Some(ExitReason::UserQuit);
        } else {
            // First Ctrl+D - show hint
            inner.exit_hint = Some(ExitHint::CtrlD);
            inner.exit_hint_shown_at = Some(now);
        }
    }
    // With text in input: ignored (do nothing)
}
```

### Phase 5: Update Status Bar Rendering

**Goal:** Show exit hint messages in status bar.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Update `format_status_bar()` to show exit hints:

```rust
fn format_status_bar(state: &RenderState) -> String {
    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
        };
    }

    // Normal status bar (existing logic)
    let mode_text = match &state.permission_mode {
        // ... existing match arms
    };
    // ... rest of function
}
```

### Phase 6: Unit Tests

**Goal:** Add unit tests for exit confirmation logic.

**Create `crates/cli/src/tui/app_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::scenario::Scenario;
use crate::state::session::SessionManager;
use crate::time::ClockHandle;
use iocraft::prelude::*;

fn create_test_app() -> TuiAppState {
    let scenario = Scenario::default();
    let sessions = SessionManager::new();
    let clock = ClockHandle::fake_at_epoch();
    let config = TuiConfig::default();
    TuiAppState::new(scenario, sessions, clock, config)
}

#[test]
fn ctrl_c_on_empty_input_shows_exit_hint() {
    let state = create_test_app();

    // First Ctrl+C
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::CtrlC));
    assert!(!state.should_exit());
}

#[test]
fn ctrl_c_with_text_clears_and_shows_hint() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('h'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    assert_eq!(state.input_buffer(), "h");

    // First Ctrl+C
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    assert_eq!(state.input_buffer(), "");
    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::CtrlC));
    assert!(!state.should_exit());
}

#[test]
fn double_ctrl_c_exits() {
    let state = create_test_app();

    // First Ctrl+C
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });
    assert!(!state.should_exit());

    // Second Ctrl+C (within timeout)
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::Interrupted));
}

#[test]
fn ctrl_c_hint_times_out() {
    let state = create_test_app();
    let clock = state.inner.lock().clock.as_fake().unwrap().clone();

    // First Ctrl+C
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    assert_eq!(state.render_state().exit_hint, Some(ExitHint::CtrlC));

    // Advance time past timeout
    clock.advance_ms(2100);
    state.check_exit_hint_timeout();

    assert_eq!(state.render_state().exit_hint, None);
}

#[test]
fn ctrl_d_on_empty_shows_exit_hint() {
    let state = create_test_app();

    // Ctrl+D on empty input
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    let render = state.render_state();
    assert_eq!(render.exit_hint, Some(ExitHint::CtrlD));
    assert!(!state.should_exit());
}

#[test]
fn ctrl_d_with_text_is_ignored() {
    let state = create_test_app();

    // Type some text
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    // Ctrl+D with text
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    // Should be ignored - no hint, no exit
    assert_eq!(state.input_buffer(), "x");
    assert_eq!(state.render_state().exit_hint, None);
    assert!(!state.should_exit());
}

#[test]
fn double_ctrl_d_exits() {
    let state = create_test_app();

    // First Ctrl+D
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    // Second Ctrl+D
    state.handle_key_event(KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    assert!(state.should_exit());
    assert_eq!(state.exit_reason(), Some(ExitReason::UserQuit));
}
```

2. Add module reference in `app.rs`:

```rust
#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
```

### Phase 7: Update Test Harness

**Goal:** Update test harness to support exit hint testing.

**Changes to `crates/cli/src/tui/test_helpers.rs`:**

1. Add exit hint fields to `TuiAppState`:

```rust
pub struct TuiAppState {
    // ... existing fields ...
    pub exit_hint: Option<super::app::ExitHint>,
    pub exit_hint_shown_at: Option<u64>,
}
```

2. Update `press_ctrl_c()` and `press_ctrl_d()` to implement double-press logic.

### Phase 8: Enable Integration Tests

**Goal:** Remove `#[ignore]` from passing tests.

**Changes to `crates/cli/tests/tui_exit.rs`:**

Remove `#[ignore]` attribute from:
- `test_tui_ctrl_c_shows_exit_hint_on_empty_input` (line 34)
- `test_tui_ctrl_c_clears_input_and_shows_exit_hint` (line 66)
- `test_tui_ctrl_c_exit_hint_times_out` (line 112)
- `test_tui_ctrl_d_shows_exit_hint_on_empty_input` (line 203)
- `test_tui_ctrl_d_exit_hint_times_out` (line 235)

## Key Implementation Details

### Exit Hint Messages

Exact messages to match real Claude CLI:
- Ctrl+C: `"Press Ctrl-C again to exit"`
- Ctrl+D: `"Press Ctrl-D again to exit"`

Note: The message uses `-` (hyphen) not `+` between Ctrl and the key.

### Timeout Duration

The timeout is 2 seconds (2000ms), matching observed behavior in Claude CLI v2.1.12.

### Clock Usage

The implementation uses `inner.clock.now_millis()` for timestamps to support:
- Real-time behavior with `SystemClock`
- Deterministic testing with `FakeClock`

### State Precedence

Exit hint display takes precedence over normal status bar content. When exit hint is active:
- Hide permission mode indicator
- Hide "? for shortcuts"
- Hide thinking status
- Show only the exit hint message

### Clearing Exit Hint

Exit hint is cleared when:
1. Timeout expires (2 seconds)
2. User types any character
3. User presses Enter
4. User takes other significant actions (navigation should NOT clear it)

This matches real Claude CLI behavior where typing "resets" the exit confirmation state.

## Verification Plan

1. Run `cargo fmt --all -- --check`
2. Run `cargo clippy --all-targets --all-features -- -D warnings`
3. Run unit tests:
   ```bash
   cargo test app_tests -- --nocapture
   ```
4. Run integration tests:
   ```bash
   cargo test tui_exit -- --nocapture
   ```
5. Run all tests: `cargo test --all`
6. Run full check: `make check`

**Manual verification:**
```bash
# Start TUI
claudeless scenarios/full-featured.toml

# Test Ctrl+C:
# 1. Press Ctrl+C on empty input - verify "Press Ctrl-C again to exit" appears
# 2. Wait ~2 seconds - verify hint disappears, "? for shortcuts" returns
# 3. Type text, press Ctrl+C - verify text clears AND hint appears
# 4. Press Ctrl+C twice quickly - verify exit

# Test Ctrl+D:
# 1. Type text, press Ctrl+D - verify nothing happens (ignored)
# 2. Clear input, press Ctrl+D - verify "Press Ctrl-D again to exit" appears
# 3. Wait ~2 seconds - verify hint disappears
# 4. Press Ctrl+D twice quickly - verify exit
```

## Files Changed

| File | Action |
|------|--------|
| `crates/cli/src/tui/app.rs` | Edit (add ExitHint enum, state fields, key handlers, status bar) |
| `crates/cli/src/tui/app_tests.rs` | Create (unit tests) |
| `crates/cli/src/tui/test_helpers.rs` | Edit (add exit hint fields) |
| `crates/cli/tests/tui_exit.rs` | Edit (remove #[ignore]) |
