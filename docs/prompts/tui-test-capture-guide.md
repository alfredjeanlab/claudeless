# TUI Test Capture Guide

Guide for capturing new TUI snapshot and behavioral tests from real Claude Code.

## Overview

When adding new TUI features to claudeless, we capture the **real behavior** from Claude Code first, then write tests that document that behavior. Tests may be marked `#[ignore]` until the feature is implemented.

## Step 1: Capture Real Claude Code Behavior

Use tmux to interact with real Claude Code and observe its behavior.

### Setup tmux session

```bash
# Kill any existing session and create fresh one
tmux kill-session -t claude-test 2>/dev/null
tmux new-session -d -s claude-test -x 120 -y 20
```

### Start Claude Code

```bash
tmux send-keys -t claude-test 'claude --model haiku' Enter
sleep 3
tmux capture-pane -t claude-test -p
```

### Send keys and capture states

For example, if you were testing Ctrl+C and Ctrl+D handling:

```bash
# Send a key combination (e.g., Ctrl+C)
tmux send-keys -t claude-test C-c
sleep 0.3
tmux capture-pane -t claude-test -p

# Wait longer to observe timeouts
sleep 2
tmux capture-pane -t claude-test -p

# Type text
tmux send-keys -t claude-test 'hello world'
sleep 0.3
tmux capture-pane -t claude-test -p

# Exit Claude (double Ctrl+C)
tmux send-keys -t claude-test C-c && sleep 0.2 && tmux send-keys -t claude-test C-c

# Cleanup
tmux kill-session -t claude-test
```

## Step 2: Create Fixtures

Save captured TUI states as fixtures in `crates/cli/tests/fixtures/tui/v2.1.12/`.

### Fixture conventions

- Use descriptive names: `ctrl_c_exit_hint.txt`, `permission_bash_command.txt`
- Include only the TUI portion (not shell prompts before/after)
- Document in `crates/cli/tests/fixtures/tui/CLAUDE.md`

### Example fixture

```
 ▐▛███▜▌   Claude Code v2.1.12
▝▜█████▛▘  Haiku 4.5 · Claude Max
  ▘▘ ▝▝    ~/Developer/claudeless

────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
❯ Try "edit <filepath> to..."
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  Press Ctrl-C again to exit
```

## Step 3: Write Tests

Tests go in `crates/cli/tests/tui_*.rs` files.

### Test file structure

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! TUI [feature] tests - [description].
//!
//! Behavior observed with: claude --version 2.1.12 (Claude Code)
//!
//! ## [Feature] Behavior
//! - Document observed behaviors here
//! - One bullet per distinct behavior

mod common;

use common::{start_tui, tmux, write_scenario};
```

### Test conventions

1. **Document the Claude version** in module docs and test docs
2. **Use descriptive test names**: `test_tui_ctrl_c_shows_exit_hint_on_empty_input`
3. **Group related tests** with section comments
4. **Each test gets a unique tmux session name**: `"claudeless-ctrl-c-hint-empty"`

### Basic test pattern

```rust
/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// [Description of what this test verifies]
#[test]
fn test_tui_feature_behavior() {
    let scenario = write_scenario(
        r#"
        name = "test"
        [[responses]]
        pattern = { type = "any" }
        response = "Hello!"
        "#,
    );

    let session = "claudeless-unique-session-name";
    let previous = start_tui(session, &scenario);

    // Perform actions
    tmux::send_keys(session, "C-c");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    // Assert expected behavior
    assert!(
        capture.contains("expected text"),
        "Description of failure.\nCapture:\n{}",
        capture
    );
}
```

## Step 4: Mark Unimplemented Tests

If claudeless doesn't yet implement the behavior, mark tests with `#[ignore]` and a TODO comment:

```rust
/// Behavior observed with: claude --version 2.1.12 (Claude Code)
///
/// [Description]
// TODO(implement): requires [feature description]
#[test]
#[ignore]
fn test_tui_unimplemented_feature() {
    // ...
}
```

## Important Conventions

### Use tmux helpers, not thread::sleep

**Bad (❌)**
```rust
std::thread::sleep(std::time::Duration::from_millis(200));
let capture = tmux::capture_pane(session);
```

**Good (✅)**
```rust
// For waiting for content to appear
let capture = tmux::wait_for_content(session, "expected text");

// For waiting for any change
let capture = tmux::wait_for_change(session, &previous);

// For waiting for one of several patterns
let capture = tmux::wait_for_any(session, &["$", "%", "❯"]);

// For asserting nothing changes (e.g., key should be ignored)
let capture = tmux::assert_unchanged_ms(session, &previous, 200);
```

### Test types

1. **Rendering tests**: Verify the TUI displays correct content
   - Check for specific text in status bar, dialogs, etc.
   - Use fixtures for complex UI comparisons

2. **Behavioral tests**: Verify actions produce correct results
   - Key presses cause expected state changes
   - Exit codes are correct
   - Session cleanup happens properly

### Session cleanup

Always kill the tmux session, even if assertions fail:

```rust
// Pattern 1: Kill before assertions (preferred for simple tests)
tmux::kill_session(session);
assert!(capture.contains("expected"));

// Pattern 2: Assertions then kill (if you need capture after kill)
assert!(capture.contains("expected"));
tmux::kill_session(session);
```

### Timeouts

The default tmux timeout is 1000ms. For tests that wait for longer behaviors (like 2-second exit hint timeout), either:

1. Set environment variable: `TMUX_TEST_TIMEOUT_MS=5000 cargo test`
2. Or design the test to not require waiting for the timeout

## Checklist

Before committing new TUI tests:

- [ ] Captured real Claude Code behavior with tmux
- [ ] Created fixtures for new UI states
- [ ] Documented Claude version in test file and individual tests
- [ ] Used unique session names for each test
- [ ] Used tmux helpers instead of thread::sleep
- [ ] Marked unimplemented features with `#[ignore]` and `// TODO(implement):`
- [ ] Ran `make check` to verify all checks pass
