# Implementation Plan: /fork Command

**Test File:** `crates/cli/tests/tui_fork.rs` (3 tests, currently `#[ignore]`)

## Overview

Implement the `/fork` command which creates a fork of the current conversation at the current point. This is a simpler command implementation since the slash menu infrastructure already exists. The main work involves:

1. Adding `/fork` to the command registry
2. Implementing the fork behavior in the command handler
3. Handling the "no conversation to fork" error case

**Key behaviors to implement:**
- `/fork` creates a fork of the current conversation at the current point
- When executed with no conversation, shows error "Failed to fork conversation: No conversation to fork"
- The command appears in autocomplete with description "Create a fork of the current conversation at this point"

## Project Structure

```
crates/cli/src/tui/
├── app.rs               # Add /fork handler to handle_command_inner()
├── slash_menu.rs        # Add /fork to COMMANDS registry

crates/cli/tests/
├── tui_fork.rs                        # Integration tests (enable from #[ignore])
└── fixtures/tui/v2.1.12/
    └── fork_no_conversation.txt       # Existing fixture
```

## Dependencies

No new external dependencies required. Uses existing session management infrastructure.

## Implementation Phases

### Phase 1: Add /fork to Command Registry

**Goal:** Register the `/fork` command so it appears in autocomplete.

**Files:**
- `crates/cli/src/tui/slash_menu.rs`

**Implementation:**

Add the `/fork` command to the `COMMANDS` array in alphabetical order (between `cost` and `help`):

```rust
// In slash_menu.rs, add to COMMANDS array after "cost" and before "help":

SlashCommand {
    name: "fork",
    description: "Create a fork of the current conversation at this point",
    argument_hint: None,
},
```

**Verification:**
- [ ] `/fork` appears in autocomplete when typing `/f` or `/fork`
- [ ] Description matches "Create a fork of the current conversation at this point"
- [ ] Commands remain in alphabetical order
- [ ] `cargo test -p claudeless -- slash_menu` passes

---

### Phase 2: Implement /fork Command Handler

**Goal:** Handle the `/fork` command execution in the TUI.

**Files:**
- `crates/cli/src/tui/app.rs`

**Implementation:**

Add `/fork` case to `handle_command_inner()`:

```rust
// In handle_command_inner() match statement:

"/fork" => {
    // Check if there's a conversation to fork
    let has_conversation = {
        let sessions = inner.sessions.lock();
        let current = sessions.get_current();
        current.map(|s| !s.turns.is_empty()).unwrap_or(false)
    };

    if has_conversation {
        // TODO: Implement actual fork functionality
        // For now, show a placeholder message
        inner.response_content = "Conversation forked".to_string();
    } else {
        // No conversation to fork - show error
        inner.response_content =
            "Failed to fork conversation: No conversation to fork".to_string();
    }
}
```

**Key Details:**
- The error message format must exactly match the fixture: "Failed to fork conversation: No conversation to fork"
- The response uses the elbow connector format (same as other commands)
- Check `sessions.get_current()` to determine if a conversation exists
- A conversation exists if the current session has at least one turn

**Verification:**
- [ ] `/fork` with no conversation shows error "Failed to fork conversation: No conversation to fork"
- [ ] Error is displayed with elbow connector format (`⎿`)
- [ ] After first message exchange, `/fork` succeeds (no error)

---

### Phase 3: Enable Integration Tests

**Goal:** Remove `#[ignore]` from tests in `tui_fork.rs` and verify they pass.

**Files:**
- `crates/cli/tests/tui_fork.rs`

**Steps:**
1. Remove `#[ignore]` and `// TODO(implement)` from each test
2. Run tests to verify behavior matches Claude Code
3. Fix any discrepancies

**Test Details:**

| Test | Description |
|------|-------------|
| `test_fork_no_conversation_shows_error` | Executes `/fork` with no conversation, verifies error message |
| `test_fork_no_conversation_matches_fixture` | Compares output to `fork_no_conversation.txt` fixture |
| `test_fork_in_autocomplete` | Verifies `/fork` appears in autocomplete with correct description |

**Verification:**
- [ ] All 3 tests pass without `#[ignore]`
- [ ] `cargo test tui_fork` passes
- [ ] Output matches existing fixture `fork_no_conversation.txt`

---

## Key Implementation Details

### Error Message Format

The error output uses the elbow connector format for command output:

```
❯ /fork
  ⎿  Failed to fork conversation: No conversation to fork
```

This format is already handled by `render_conversation_area()` when `is_command_output` is true.

### Session State Check

To determine if a conversation exists:

```rust
let has_conversation = {
    let sessions = inner.sessions.lock();
    let current = sessions.get_current();
    current.map(|s| !s.turns.is_empty()).unwrap_or(false)
};
```

A "conversation" is defined as having at least one turn (user prompt + response pair).

### Command Registry Ordering

The `COMMANDS` array must remain in alphabetical order:
- `cost` (existing)
- `doctor` (existing)
- **`fork` (new)**
- `help` (existing)

### Future Work (Not in Scope)

The actual fork functionality (creating a new session as a copy of the current one) is not required for these tests. The tests only verify:
1. Command appears in autocomplete
2. Error is shown when no conversation exists

Actual fork behavior can be implemented later when tests require it.

---

## Verification Plan

### Unit Tests

**Slash Menu (`slash_menu_tests.rs`):**
- [ ] Add test for `/fork` appearing in filter results
- [ ] Verify `filter_commands("for")` includes fork
- [ ] Verify `filter_commands("f")` includes fork

### Integration Tests

All 3 tests in `tui_fork.rs`:
- [ ] `test_fork_no_conversation_shows_error` - error message correct
- [ ] `test_fork_no_conversation_matches_fixture` - matches fixture
- [ ] `test_fork_in_autocomplete` - appears in autocomplete

### Manual Testing

1. Run `claudeless` with a test scenario
2. Type `/fork` immediately - verify error message
3. Type `/f` - verify fork appears in autocomplete
4. Complete a conversation turn, then `/fork` - verify no error

### Final Checklist

- [ ] `make check` passes
- [ ] All unit tests pass
- [ ] All integration tests pass (no `#[ignore]`)
- [ ] No new clippy warnings
- [ ] `/fork` in correct alphabetical position in COMMANDS
