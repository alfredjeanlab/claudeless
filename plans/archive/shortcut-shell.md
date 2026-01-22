# Implementation Plan: '!' Shell Mode Shortcut

**Test File:** `crates/cli/tests/tui_shell_mode.rs` (6 tests, currently `#[ignore]`)

**Root Feature:** `cl-5869`

## Overview

Implement support for the '!' prefix to enter shell mode. When the user types '!' at the start of empty input, the TUI enters shell mode with a `\!` prefix displayed. The user can then type a shell command, and upon pressing Enter, the command is executed via the Bash tool. The shell command is shown as `\!command` in both the input field and conversation history.

**Key behaviors to implement:**
1. Typing '!' on empty input enters shell mode (displays `\!` prefix)
2. Commands typed in shell mode appear as `\!command` (e.g., `\!ls -la`)
3. The placeholder hint disappears when shell prefix is entered
4. Submitting a shell command executes it via `Bash` tool
5. Conversation history shows the prompt as `❯ \!command`
6. Escape exits shell mode and clears the prefix

## Project Structure

```
crates/cli/src/tui/
├── app.rs                    # Key handling, state, rendering (modify)
├── app_tests.rs              # Unit tests for shell mode state (modify)
└── ...

crates/cli/tests/
├── tui_shell_mode.rs         # Integration tests (enable from #[ignore])
└── fixtures/tui/v2.1.12/
    ├── shell_mode_prefix.txt  # EXISTS: Fixture for `\!` prefix
    └── shell_mode_command.txt # EXISTS: Fixture for `\!ls -la`
```

## Dependencies

No new external dependencies required. Uses existing infrastructure.

## Implementation Phases

### Phase 1: Add Shell Mode State

**Goal:** Track shell mode state in `TuiAppStateInner` and `RenderState`.

**Files:**
- `crates/cli/src/tui/app.rs` (modify)

**Implementation:**

```rust
// Add to TuiAppStateInner struct (around line 154):

/// Whether shell mode is currently active (user typed '!' at empty input)
pub shell_mode: bool,
```

```rust
// Add to RenderState struct (around line 320):

/// Whether shell mode is currently active
pub shell_mode: bool,
```

```rust
// Initialize in TuiAppStateInner (around line 384):

shell_mode: false,
```

```rust
// Copy to render_state() snapshot:

shell_mode: inner.shell_mode,
```

**Verification:**
- [ ] State compiles without errors
- [ ] Default value is `false`

---

### Phase 2: Handle '!' Key Event

**Goal:** Detect '!' key press on empty input and enter shell mode.

**Files:**
- `crates/cli/src/tui/app.rs` (modify `handle_input_key()`)

**Implementation:**

Add before the regular character input handler (around line 644), after the '?' handler:

```rust
// '!' key - enter shell mode on empty input, otherwise type literal
(m, KeyCode::Char('!')) if m.is_empty() || m == KeyModifiers::SHIFT => {
    if inner.input_buffer.is_empty() && !inner.shell_mode {
        // Empty input: enter shell mode
        inner.shell_mode = true;
        // Clear any exit hint
        inner.exit_hint = None;
        inner.exit_hint_shown_at = None;
    } else {
        // Already in shell mode or has input: type literal '!'
        let pos = inner.cursor_pos;
        inner.input_buffer.insert(pos, '!');
        inner.cursor_pos = pos + 1;
        // Reset history browsing on new input
        inner.history_index = None;
        // Clear exit hint on typing
        inner.exit_hint = None;
        inner.exit_hint_shown_at = None;
    }
}
```

**Verification:**
- [ ] '!' on empty input sets `shell_mode = true`
- [ ] '!' when already in shell mode types literal '!'
- [ ] '!' with existing text types literal '!'

---

### Phase 3: Handle Escape to Exit Shell Mode

**Goal:** Modify Escape key handling to exit shell mode before other behaviors.

**Files:**
- `crates/cli/src/tui/app.rs` (modify `handle_input_key()`)

**Implementation:**

Modify the Escape handler (around line 537) to check shell mode first:

```rust
(_, KeyCode::Esc) => {
    if inner.show_shortcuts_panel {
        // First priority: dismiss shortcuts panel
        inner.show_shortcuts_panel = false;
    } else if inner.shell_mode {
        // Second priority: exit shell mode
        inner.shell_mode = false;
        // Also clear any input typed in shell mode
        inner.input_buffer.clear();
        inner.cursor_pos = 0;
    } else if !inner.input_buffer.is_empty() {
        // Existing behavior: show "Esc to clear again" hint
        // or clear input on double-tap
        // ... existing escape logic ...
    }
}
```

**Verification:**
- [ ] Escape exits shell mode when active
- [ ] Escape still dismisses shortcuts panel first
- [ ] Escape still clears input when not in shell mode

---

### Phase 4: Modify Input Display for Shell Mode

**Goal:** Show `\!` prefix when in shell mode, with command appended.

**Files:**
- `crates/cli/src/tui/app.rs` (modify `render_main_content()`)

**Implementation:**

Modify the input_display logic (around line 1416):

```rust
// Format input line
let input_display = if state.shell_mode {
    // Shell mode: show \! prefix with any typed command
    if state.input_buffer.is_empty() {
        "❯ \\!".to_string()
    } else {
        format!("❯ \\!{}", state.input_buffer)
    }
} else if state.input_buffer.is_empty() {
    if state.conversation_display.is_empty() && state.response_content.is_empty() {
        // Show placeholder only on initial state
        "❯ Try \"refactor mod.rs\"".to_string()
    } else {
        // After conversation started, show just the cursor
        "❯".to_string()
    }
} else {
    format!("❯ {}", state.input_buffer)
};
```

**Verification:**
- [ ] Shell mode displays `❯ \!` when input is empty
- [ ] Shell mode displays `❯ \!ls -la` when command is typed
- [ ] Matches fixture `shell_mode_prefix.txt`
- [ ] Matches fixture `shell_mode_command.txt`

---

### Phase 5: Execute Shell Commands on Submit

**Goal:** When Enter is pressed in shell mode, execute the command via Bash.

**Files:**
- `crates/cli/src/tui/app.rs` (modify `submit_input()`)

**Implementation:**

Modify `submit_input()` (around line 861) to handle shell mode:

```rust
fn submit_input(&self) {
    let mut inner = self.inner.lock();
    let input = std::mem::take(&mut inner.input_buffer);
    let was_shell_mode = inner.shell_mode;
    inner.shell_mode = false;  // Reset shell mode after submit
    inner.cursor_pos = 0;

    // Add to history (with shell prefix if applicable)
    let history_entry = if was_shell_mode {
        format!("\\!{}", input)
    } else {
        input.clone()
    };
    if !history_entry.is_empty() {
        inner.history.push(history_entry);
    }
    inner.history_index = None;

    // Check for slash commands (not applicable in shell mode)
    if !was_shell_mode && input.starts_with('/') {
        Self::handle_command_inner(&mut inner, &input);
    } else if was_shell_mode {
        // Shell mode: execute command via Bash
        let command = input;
        drop(inner);
        self.execute_shell_command(command);
    } else {
        // Process the input as a prompt
        drop(inner);
        self.process_prompt(input);
    }
}
```

**Verification:**
- [ ] Shell commands trigger Bash execution
- [ ] Shell mode is reset after submit
- [ ] History shows `\!command` format

---

### Phase 6: Implement Shell Command Execution

**Goal:** Create method to execute shell commands and show in conversation history.

**Files:**
- `crates/cli/src/tui/app.rs` (add new method)

**Implementation:**

Add a new method to `TuiAppState`:

```rust
/// Execute a shell command via Bash tool
fn execute_shell_command(&self, command: String) {
    let mut inner = self.inner.lock();

    // Add previous response to conversation display if any
    if !inner.response_content.is_empty() && !inner.is_command_output {
        let response = inner.response_content.clone();
        if !inner.conversation_display.is_empty() {
            inner.conversation_display.push_str("\n\n");
        }
        inner
            .conversation_display
            .push_str(&format!("⏺ {}", response));
    }

    // Add the shell command to conversation display with \! prefix
    if !inner.conversation_display.is_empty() {
        inner.conversation_display.push_str("\n\n");
    }
    inner
        .conversation_display
        .push_str(&format!("❯ \\!{}", command));

    // Show bash permission dialog or execute directly based on permission mode
    inner.mode = AppMode::Thinking;
    inner.response_content.clear();
    inner.is_command_output = false;

    drop(inner);

    // Use existing bash permission flow
    self.show_bash_permission(command.clone(), Some(format!("Execute: {}", command)));
}
```

Alternatively, for trusted/bypass modes, execute directly via the scenario's simulated bash response.

**Verification:**
- [ ] Shell command shows `❯ \!pwd` in conversation history
- [ ] Bash permission dialog appears (or bypasses based on mode)
- [ ] Command executes and response is shown

---

### Phase 7: Handle Backspace in Shell Mode

**Goal:** When backspace is pressed on empty input in shell mode, exit shell mode.

**Files:**
- `crates/cli/src/tui/app.rs` (modify backspace handler)

**Implementation:**

Modify the backspace handler to check shell mode:

```rust
// In handle_input_key(), find Backspace handling:

(_, KeyCode::Backspace) => {
    if inner.cursor_pos > 0 {
        inner.cursor_pos -= 1;
        inner.input_buffer.remove(inner.cursor_pos);
        inner.history_index = None;
    } else if inner.shell_mode && inner.input_buffer.is_empty() {
        // Backspace on empty input in shell mode: exit shell mode
        inner.shell_mode = false;
    }
}
```

**Verification:**
- [ ] Backspace deletes characters normally
- [ ] Backspace on empty shell mode input exits shell mode

---

### Phase 8: Enable Integration Tests

**Goal:** Remove `#[ignore]` from tests and verify they pass.

**Files:**
- `crates/cli/tests/tui_shell_mode.rs` (modify)

**Steps:**
1. Remove `#[ignore]` from `test_tui_exclamation_shows_shell_prefix`
2. Run test, fix any issues
3. Remove `#[ignore]` from `test_tui_shell_prefix_matches_fixture`
4. Run test, adjust rendering to match fixture
5. Remove `#[ignore]` from `test_tui_shell_mode_shows_command`
6. Remove `#[ignore]` from `test_tui_shell_command_matches_fixture`
7. Remove `#[ignore]` from `test_tui_shell_mode_executes_command`
8. Remove `#[ignore]` from `test_tui_shell_mode_shows_prefixed_prompt_in_history`

**Verification:**
- [ ] All 6 tests pass without `#[ignore]`
- [ ] `cargo test tui_shell_mode` passes
- [ ] `make check` passes

---

## Key Implementation Details

### Shell Mode State Machine

```
┌──────────────────────────────────────────────────────────┐
│                    Input Mode                            │
│                   (shell_mode: false)                    │
│              Shows "❯" or "❯ Try..." hint                │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Press '!' (input empty)
                           ▼
┌──────────────────────────────────────────────────────────┐
│                    Shell Mode                            │
│                   (shell_mode: true)                     │
│                                                          │
│  Input display: "❯ \!" or "❯ \!<command>"               │
│  - Type command (appended after \!)                      │
│  - Escape → exit shell mode, return to Input Mode        │
│  - Backspace on empty → exit shell mode                  │
│  - Enter → execute command via Bash, reset to Input Mode │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Press Enter
                           ▼
┌──────────────────────────────────────────────────────────┐
│               Command Execution                          │
│                                                          │
│  - Add "❯ \!command" to conversation history             │
│  - Trigger Bash tool execution                           │
│  - Show response with "⏺" prefix                        │
│  - Return to Input Mode                                  │
└──────────────────────────────────────────────────────────┘
```

### Display Format

Based on the fixtures:

**shell_mode_prefix.txt:**
```
❯ \!
```

**shell_mode_command.txt:**
```
❯ \!ls -la
```

**Conversation history after execution:**
```
❯ \!pwd

⏺ Bash(pwd)

/home/user/project
```

### Escape Handling Priority

The Escape key has multiple behaviors with this priority:
1. Dismiss shortcuts panel (if visible)
2. Exit shell mode (if active)
3. Clear input (double-tap behavior)

### History Integration

When a shell command is submitted:
- Add `\!command` to history
- User can navigate history with up/down arrows
- If a history entry starts with `\!`, restore shell mode when selected

---

## Verification Plan

### Unit Tests

**App State (`app_tests.rs`):**
- [ ] `test_shell_mode_initially_disabled` - default state is false
- [ ] `test_exclamation_enters_shell_mode_on_empty` - '!' on empty sets shell_mode
- [ ] `test_exclamation_types_literal_with_text` - '!' with text types char
- [ ] `test_escape_exits_shell_mode` - Escape sets shell_mode = false
- [ ] `test_backspace_exits_shell_mode_on_empty` - Backspace on empty exits shell mode
- [ ] `test_submit_resets_shell_mode` - Enter resets shell_mode

### Integration Tests

All 6 tests in `tui_shell_mode.rs`:
- [ ] `test_tui_exclamation_shows_shell_prefix`
- [ ] `test_tui_shell_prefix_matches_fixture`
- [ ] `test_tui_shell_mode_shows_command`
- [ ] `test_tui_shell_command_matches_fixture`
- [ ] `test_tui_shell_mode_executes_command`
- [ ] `test_tui_shell_mode_shows_prefixed_prompt_in_history`

### Final Checklist

- [ ] `make check` passes
- [ ] All unit tests pass
- [ ] All integration tests pass (no `#[ignore]`)
- [ ] No new clippy warnings
- [ ] Output matches `shell_mode_prefix.txt` fixture
- [ ] Output matches `shell_mode_command.txt` fixture
