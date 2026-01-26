# Implementation Plan: /exit Slash Command

**Root Feature:** `cl-1999`

## Overview

Implement the `/exit` slash command to allow users to exit the TUI with a farewell message. When executed, it:
- Displays a random farewell message (e.g., "Goodbye!", "Bye!", "See ya!", "Catch you later!")
- Triggers application exit with `ExitReason::UserQuit`

This is a simple command that sets the exit flag and displays output directly in the response area (no dialog mode needed).

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add /exit handler to handle_command_inner()
│   └── slash_menu.rs          # Add "exit" command to COMMANDS array
```

Tests:
```
crates/cli/tests/
├── tui_exit.rs                # Remove #[ignore] from 2 tests (lines 367-393, 399-443)
└── fixtures/tui/v2.1.12/
    └── exit_autocomplete.txt  # Already exists - shows expected autocomplete output
```

## Dependencies

No new dependencies required. Uses existing:
- `ExitReason::UserQuit` enum variant
- `rand` crate (already a dependency) for random farewell selection

## Implementation Phases

### Phase 1: Add /exit to Command Registry

Add the `/exit` command to the static COMMANDS array in alphabetical order.

**File:** `crates/cli/src/tui/slash_menu.rs`

Insert after the `/doctor` entry (line 80) and before `/fork`:
```rust
SlashCommand {
    name: "exit",
    description: "Exit the REPL",
    argument_hint: None,
},
SlashCommand {
    name: "export",
    description: "Export the current conversation to a file or clipboard",
    argument_hint: None,
},
```

Note: `/export` also needs to be added as it appears in the `exit_autocomplete.txt` fixture between `/exit` and `/context`. This matches Claude Code v2.1.12's command set.

**Verification:**
- `cargo build`
- Type `/exit` in TUI - should appear in autocomplete with "Exit the REPL" description

---

### Phase 2: Implement /exit Command Handler

Add the command handler that displays a farewell message and triggers exit.

**File:** `crates/cli/src/tui/app.rs`

Add a helper function for farewell messages (near other format functions):
```rust
/// Generate a random farewell message for /exit command
fn random_farewell() -> &'static str {
    const FAREWELLS: &[&str] = &[
        "Goodbye!",
        "Bye!",
        "See ya!",
        "Catch you later!",
    ];
    use rand::seq::SliceRandom;
    FAREWELLS.choose(&mut rand::rng()).copied().unwrap_or("Goodbye!")
}
```

Add match arm in `handle_command_inner()` (after `/doctor` and before `/fork`):
```rust
"/exit" => {
    inner.response_content = Self::random_farewell().to_string();
    inner.should_exit = true;
    inner.exit_reason = Some(ExitReason::UserQuit);
}
```

**Verification:**
- Running `/exit` displays a farewell message and exits the TUI
- Exit code is 0 (same as Ctrl+D exit)

---

### Phase 3: Enable Tests and Final Verification

Remove `#[ignore]` from the `/exit` command tests.

**File:** `crates/cli/tests/tui_exit.rs`

Remove `#[ignore]` and `// TODO(implement)` comments from:
- `test_tui_exit_command_shows_autocomplete` (line 368-369)
- `test_tui_exit_command_exits_with_farewell` (line 400-401)

**Verification:**
```bash
cargo test --test tui_exit -- test_tui_exit_command
make check
```

## Key Implementation Details

### Command Registry Placement

The `/exit` command must be inserted in alphabetical order in the COMMANDS array. Per the `exit_autocomplete.txt` fixture, the order should be:
1. `/exit` - "Exit the REPL"
2. `/export` - "Export the current conversation to a file or clipboard"
3. `/context` - "Visualize current context usage as a colored grid"

### Farewell Messages

The test `test_tui_exit_command_exits_with_farewell` checks for any of these messages:
- "Goodbye!"
- "Bye!"
- "See ya!"
- "Catch you later!"

Using random selection adds personality while keeping deterministic test behavior (any of the valid messages passes).

### Exit Flow

1. User types `/exit` and presses Enter
2. `submit_input()` detects slash command, calls `handle_command_inner()`
3. `/exit` handler sets:
   - `response_content` = farewell message
   - `should_exit` = true
   - `exit_reason` = `Some(ExitReason::UserQuit)`
4. TUI renders final frame with farewell message (prefixed with `⎿`)
5. `TuiApp::run()` returns `ExitReason::UserQuit`
6. `main.rs` maps `UserQuit` to exit code 0

### Comparison with Ctrl+D Exit

`/exit` behaves similarly to double Ctrl+D:
- Both use `ExitReason::UserQuit`
- Both exit with code 0
- Difference: `/exit` shows farewell message, Ctrl+D shows no message

## Verification Plan

1. **Unit Tests:**
   - `test_tui_exit_command_shows_autocomplete` - verifies autocomplete shows "/exit" with "Exit the REPL"
   - `test_tui_exit_command_exits_with_farewell` - verifies command exits with farewell message

2. **Integration:**
   - `make check` passes (includes lint, format, clippy, tests, build, audit)

3. **Manual Testing:**
   - Launch TUI: `cargo run -- --scenario <file>`
   - Type `/exit` - should appear in autocomplete dropdown
   - Press Enter - should show farewell message and return to shell
   - Verify exit code: `echo $?` should show `0`

## Files Modified Summary

| File | Changes |
|------|---------|
| `crates/cli/src/tui/slash_menu.rs` | Add `/exit` and `/export` entries to COMMANDS array |
| `crates/cli/src/tui/app.rs` | Add `random_farewell()` function and `/exit` handler in `handle_command_inner()` |
| `crates/cli/tests/tui_exit.rs` | Remove `#[ignore]` from 2 tests |
