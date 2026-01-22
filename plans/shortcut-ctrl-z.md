# Implementation Plan: Ctrl+Z Suspend Shortcut

**Root Feature:** `cl-4c61`

## Overview

Implement Ctrl+Z keyboard shortcut to suspend Claude Code, returning control to the shell. When suspended, the application prints informational messages, then sends SIGTSTP to actually suspend the process. On resume via `fg`, the TUI redraws with all state preserved.

## Project Structure

```
crates/cli/src/
├── main.rs                 # Add SIGTSTP/SIGCONT signal handlers
├── tui/
│   ├── app.rs              # Add Ctrl+Z key handler with suspend logic
│   └── app_tests.rs        # Add unit tests for Ctrl+Z key recognition

crates/cli/tests/
├── tui_suspend.rs          # Remove #[ignore] from 5 existing tests
└── fixtures/tui/v2.1.12/
    └── ctrl_z_suspend.txt  # Already exists with expected output
```

## Dependencies

No new dependencies required. Uses existing:
- `nix 0.29` for signal handling (SIGTSTP, SIGCONT)
- `crossterm 0.28` for key event handling and terminal restoration

## Implementation Phases

### Phase 1: Add SIGCONT Handler for Resume Detection

Set up signal handling infrastructure to detect when the process resumes after being suspended.

**File:** `crates/cli/src/main.rs`

Add alongside existing SIGINT handler:
```rust
#[cfg(unix)]
{
    use std::sync::atomic::{AtomicBool, Ordering};
    use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};

    // Flag to detect resume from suspend
    static RESUMED: AtomicBool = AtomicBool::new(false);

    extern "C" fn handle_sigcont(_: libc::c_int) {
        RESUMED.store(true, Ordering::SeqCst);
    }

    // Install SIGCONT handler
    let sa = SigAction::new(
        SigHandler::Handler(handle_sigcont),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );
    unsafe {
        if let Err(e) = sigaction(Signal::SIGCONT, &sa) {
            eprintln!("Warning: Failed to install SIGCONT handler: {}", e);
        }
    }
}
```

Pass the `RESUMED` flag to the TUI so it can poll for resume events.

**Verification:** Code compiles with `cargo check`.

### Phase 2: Add Ctrl+Z Key Handler with Suspend Logic

Handle Ctrl+Z keypress to print messages and trigger process suspension.

**File:** `crates/cli/src/tui/app.rs`

Add to `handle_input_key()` match block (near other Ctrl+key handlers):
```rust
// Ctrl+Z - Suspend process
// Note: Ctrl+Z is encoded as ASCII 0x1A (substitute) in terminals
(_, KeyCode::Char('\x1a')) => {
    return Some(AppAction::Suspend);
}
(m, KeyCode::Char('z')) if m.contains(KeyModifiers::CONTROL) => {
    return Some(AppAction::Suspend);
}
```

Add `Suspend` variant to `AppAction` enum:
```rust
pub enum AppAction {
    // ... existing variants ...
    Suspend,
}
```

Handle the suspend action in the main event loop:
```rust
AppAction::Suspend => {
    // Exit raw mode and restore terminal before printing
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::Show,
        crossterm::terminal::LeaveAlternateScreen,
    )?;

    // Print suspend messages
    println!("Claude Code has been suspended. Run `fg` to bring Claude Code back.");
    println!("Note: ctrl + z now suspends Claude Code, ctrl + _ undoes input.");

    // Raise SIGTSTP to actually suspend the process
    #[cfg(unix)]
    {
        use nix::sys::signal::{raise, Signal};
        let _ = raise(Signal::SIGTSTP);
    }

    // After resume (SIGCONT), re-enter raw mode and redraw
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide,
    )?;

    // Force full redraw
    self.force_redraw();
}
```

**Verification:** Unit tests pass for Ctrl+Z key recognition.

### Phase 3: Add Unit Tests for Ctrl+Z

Add unit tests in sibling test file following project conventions.

**File:** `crates/cli/src/tui/app_tests.rs`

```rust
// ========================
// Ctrl+Z Suspend Tests
// ========================

#[test]
fn ctrl_z_returns_suspend_action() {
    let state = create_test_app();

    // Type some input first
    for c in "hello".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Ctrl+Z should return Suspend action
    let action = state.handle_key_event(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(AppAction::Suspend)));
}

#[test]
fn ctrl_z_raw_char_returns_suspend_action() {
    let state = create_test_app();

    // Ctrl+Z may come as raw ASCII 0x1A
    let action = state.handle_key_event(key_event(KeyCode::Char('\x1a'), KeyModifiers::NONE));
    assert!(matches!(action, Some(AppAction::Suspend)));
}

#[test]
fn state_preserved_after_suspend_resume_cycle() {
    let state = create_test_app();

    // Type some input
    for c in "hello world".chars() {
        state.handle_key_event(key_event(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Trigger suspend (action returned, but state unchanged)
    let action = state.handle_key_event(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(AppAction::Suspend)));

    // State should be unchanged (preserved for resume)
    assert_eq!(state.render_state().input_buffer, "hello world");
}
```

**Verification:** `cargo test -p cli ctrl_z` passes.

### Phase 4: Handle Terminal Restoration and Redraw

Ensure terminal state is properly managed across suspend/resume cycles.

**File:** `crates/cli/src/tui/app.rs`

The suspend sequence must:
1. Exit alternate screen (so suspend messages are visible in main buffer)
2. Disable raw mode (so shell job control works)
3. Print messages
4. Raise SIGTSTP
5. On resume: re-enable raw mode
6. Re-enter alternate screen
7. Force full TUI redraw

The key insight: SIGTSTP pauses execution, so code after `raise(SIGTSTP)` runs after the process resumes.

Add method for forcing redraw:
```rust
impl TuiAppState {
    /// Force a complete redraw of the TUI (e.g., after resume from suspend)
    pub fn force_redraw(&self) {
        // Clear any cached render state
        // Trigger full repaint on next render cycle
        let mut inner = self.inner.borrow_mut();
        inner.needs_full_redraw = true;
    }
}
```

Add field to track redraw need:
```rust
struct TuiAppStateInner {
    // ... existing fields ...
    needs_full_redraw: bool,
}
```

**Verification:** Manual test - suspend with Ctrl+Z, resume with `fg`, TUI redraws correctly.

### Phase 5: Enable Integration Tests

Remove `#[ignore]` from the five existing integration tests.

**File:** `crates/cli/tests/tui_suspend.rs`

Tests to enable:
1. `test_tui_ctrl_z_suspends_with_message` (line 37)
2. `test_tui_ctrl_z_shows_keybinding_note` (line 78)
3. `test_tui_ctrl_z_returns_to_shell` (line 113)
4. `test_tui_ctrl_z_resume_redraws_tui` (line 154)
5. `test_tui_ctrl_z_resume_preserves_input_state` (line 195)

Also remove the `// TODO(implement):` comments from each test.

**Verification:** All five integration tests pass.

## Key Implementation Details

### Terminal Key Encoding

Ctrl+Z is encoded as ASCII 0x1A (substitute character). Handle both representations:
- Raw character `\x1a` (some terminals)
- `KeyCode::Char('z')` with `KeyModifiers::CONTROL` (crossterm normalization)

### Terminal State Management

The suspend sequence is critical:
1. **Exit alternate screen first** - Messages must appear in the main terminal buffer, not the alternate screen that will be invisible while suspended
2. **Disable raw mode** - Required for shell job control to work properly
3. **Raise SIGTSTP** (not SIGSTOP) - SIGTSTP is the user-initiated suspend signal that can be caught; SIGSTOP cannot be caught

### Resume Detection

After `raise(Signal::SIGTSTP)`, execution pauses until SIGCONT is received. Code following the raise call executes after resume, so the flow is:
```
Ctrl+Z pressed
    → Exit alternate screen
    → Disable raw mode
    → Print messages
    → raise(SIGTSTP)  ← Process suspends here
    ← Process resumes on SIGCONT
    → Re-enable raw mode
    → Enter alternate screen
    → Force redraw
```

### State Preservation

No special handling needed - the TuiAppState struct remains in memory while suspended. On resume, simply redrawing with the existing state restores the exact previous view.

### iocraft Integration

The TUI uses iocraft's fullscreen mode. The suspend handling may need to coordinate with iocraft's terminal management. If iocraft provides hooks for suspend/resume, prefer those over manual terminal manipulation.

Check if iocraft's `Element::fullscreen()` has built-in suspend support:
```rust
// May need to use iocraft's suspend mechanism if available
element.fullscreen().suspend()  // hypothetical
```

If not, manual terminal management as described above is required.

## Verification Plan

1. **Unit Tests:** Run `cargo test -p cli ctrl_z` for app_tests.rs
2. **Integration Tests:** Run the five tui_suspend tests:
   ```bash
   cargo test tui_suspend --no-fail-fast
   ```
3. **Full Check:** Run `make check` for:
   - `make lint` (shellcheck)
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   - `cargo build --all`
   - `cargo audit`
   - `cargo deny check`
4. **Manual Test:** Build and run the CLI:
   - Press Ctrl+Z - should see suspend messages and return to shell prompt
   - Run `fg` - TUI should redraw with state intact
   - Any typed input should be preserved
