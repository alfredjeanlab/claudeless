# TUI Positional Prompt Implementation Plan

## Overview

Fix claudeless to process positional prompt arguments in TTY mode, matching real Claude CLI behavior. When a user runs `claudeless "Hello"` in a terminal, the prompt should be processed immediately after startup (and after trust is granted if needed), then remain in interactive mode for follow-up input.

## Project Structure

Key files involved:

```
crates/cli/src/
├── cli.rs                           # CLI arg parsing (prompt: Option<String>)
├── main.rs                          # Entry point, TUI mode branching
├── runtime/
│   └── core.rs                      # Runtime::cli() accessor
└── tui/
    ├── app.rs                       # TuiApp, App component, render loop
    ├── app/
    │   ├── types.rs                 # TuiConfig, AppMode
    │   ├── state.rs                 # TuiAppState, TuiAppStateInner
    │   ├── dialogs.rs               # handle_trust_key - trust grant handling
    │   └── commands/
    │       └── execution.rs         # process_prompt() implementation
    └── mod.rs                       # Module re-exports
```

## Dependencies

No new external dependencies required. Uses existing:
- `clap` for CLI argument parsing
- `parking_lot::Mutex` for state management
- `tokio` for async execution

## Implementation Phases

### Phase 1: Add Initial Prompt to TuiConfig

**Files:** `crates/cli/src/tui/app/types.rs`

Add a field to carry the positional prompt through to TUI initialization:

```rust
// In TuiConfig struct
pub struct TuiConfig {
    // ... existing fields ...
    /// Initial prompt from CLI positional argument
    pub initial_prompt: Option<String>,
}
```

Update both constructor methods:
- `TuiConfig::from_runtime()` - extract `runtime.cli().prompt.clone()`
- `TuiConfig::from_scenario()` - accept optional prompt parameter

**Verification:** `cargo check` passes, existing tests still pass.

### Phase 2: Store Pending Initial Prompt in State

**Files:** `crates/cli/src/tui/app/state.rs`

Add a field to track the pending initial prompt:

```rust
// In TuiAppStateInner struct
pub struct TuiAppStateInner {
    // ... existing fields ...
    /// Pending initial prompt from CLI positional arg (processed once on startup)
    pub pending_initial_prompt: Option<String>,
}
```

In `TuiAppState::new()`:
- If `config.trusted` is true and `config.initial_prompt` is `Some`, store it in `pending_initial_prompt`
- If not trusted (trust dialog shown), store it anyway - will be processed after trust grant

```rust
// In TuiAppState::new()
let pending_initial_prompt = config.initial_prompt.clone();

Self {
    inner: Arc::new(Mutex::new(TuiAppStateInner {
        // ... existing fields ...
        pending_initial_prompt,
    })),
}
```

**Verification:** `cargo check` passes, no runtime behavior change yet.

### Phase 3: Process Initial Prompt After Trust Grant

**Files:** `crates/cli/src/tui/app/dialogs.rs`

Modify `handle_trust_key()` to process the initial prompt after trust is granted:

```rust
// In handle_trust_key(), after granting trust
KeyCode::Enter => {
    if let Some(prompt) = inner.dialog.as_trust() {
        match prompt.selected {
            TrustChoice::Yes => {
                inner.trust_granted = true;
                inner.dialog.dismiss();
                // Check for pending initial prompt
                if let Some(initial) = inner.pending_initial_prompt.take() {
                    drop(inner);
                    self.process_prompt(initial);
                } else {
                    inner.mode = AppMode::Input;
                }
            }
            // ... No case unchanged ...
        }
    }
}

// Same pattern for Y/y shortcut
KeyCode::Char('y') | KeyCode::Char('Y') => {
    inner.trust_granted = true;
    inner.dialog.dismiss();
    if let Some(initial) = inner.pending_initial_prompt.take() {
        drop(inner);
        self.process_prompt(initial);
    } else {
        inner.mode = AppMode::Input;
    }
}
```

**Verification:** Test with untrusted directory + positional prompt. After pressing Y, prompt should process.

### Phase 4: Process Initial Prompt for Trusted Directories

**Files:** `crates/cli/src/tui/app/state.rs`, `crates/cli/src/tui/app.rs`

For already-trusted directories, process the initial prompt in the first render cycle. Add a method and hook it into the App component:

```rust
// In TuiAppState (state.rs)
impl TuiAppState {
    /// Check for and process pending initial prompt.
    /// Returns true if a prompt was processed.
    pub fn check_initial_prompt(&self) -> bool {
        let pending = {
            let mut inner = self.inner.lock();
            // Only process if in Input mode (not during trust dialog)
            if inner.mode != AppMode::Input {
                return false;
            }
            inner.pending_initial_prompt.take()
        };

        if let Some(prompt) = pending {
            self.process_prompt(prompt);
            true
        } else {
            false
        }
    }
}
```

In `App` component (app.rs), add the check alongside existing periodic checks:

```rust
// After check_pending_hook_message()
state_clone.check_initial_prompt();
```

**Verification:** Test with trusted directory + positional prompt. Should process immediately on startup.

### Phase 5: Update Main to Pass Prompt Through

**Files:** `crates/cli/src/main.rs`, `crates/cli/src/tui/app/types.rs`

Update `TuiConfig::from_runtime()` to include the initial prompt:

```rust
// In types.rs, TuiConfig::from_runtime()
pub fn from_runtime(runtime: &Runtime, allow_bypass_permissions: bool, is_tty: bool) -> Self {
    let cli = runtime.cli();
    // ... existing code ...

    Self {
        // ... existing fields ...
        initial_prompt: cli.prompt.clone(),
    }
}
```

Ensure `Default::default()` for TuiConfig sets `initial_prompt: None`.

**Verification:** Full integration test with scenario file.

### Phase 6: Add Integration Tests

**Files:** `crates/cli/tests/positional_prompt.rs` (new file)

Create integration tests covering:
1. Positional prompt with trusted directory - immediate processing
2. Positional prompt with untrusted directory - processing after trust grant
3. No positional prompt - normal welcome screen behavior
4. Positional prompt with `--print` flag - should use print mode (no TUI)

```rust
#[test]
fn test_positional_prompt_tty_trusted() {
    // Test that positional prompt is processed in TTY mode
    // Use scenario with trusted = true
}

#[test]
fn test_positional_prompt_tty_untrusted() {
    // Test that positional prompt is processed after trust grant
    // Use scenario with trusted = false
}
```

**Verification:** All new tests pass, existing tests unchanged.

## Key Implementation Details

### State Flow for Trusted Directory

```
CLI parses "Hello" -> TuiConfig.initial_prompt = Some("Hello")
    -> TuiAppState::new() stores in pending_initial_prompt
    -> Mode = Input (trusted)
    -> First render: check_initial_prompt() sees Input mode
    -> Takes pending_initial_prompt, calls process_prompt("Hello")
    -> Mode = Thinking -> Responding -> Input
    -> User can now enter follow-up prompts
```

### State Flow for Untrusted Directory

```
CLI parses "Hello" -> TuiConfig.initial_prompt = Some("Hello")
    -> TuiAppState::new() stores in pending_initial_prompt
    -> Mode = Trust (trust dialog shown)
    -> User presses Y
    -> handle_trust_key() takes pending_initial_prompt
    -> Calls process_prompt("Hello") instead of just mode = Input
    -> Mode = Thinking -> Responding -> Input
    -> User can now enter follow-up prompts
```

### Why Not Use `pending_hook_message`?

The existing `pending_hook_message` field is for stop hook continuations which can trigger multiple times during a session. The initial prompt is a one-time startup event with different semantics:
- Processed exactly once
- Must wait for trust grant in untrusted directories
- Should not interfere with hook processing

A dedicated `pending_initial_prompt` field keeps these concerns separated.

## Verification Plan

### Unit Tests
- `TuiConfig::from_runtime()` extracts prompt correctly
- `TuiAppState::new()` initializes `pending_initial_prompt` from config
- `check_initial_prompt()` returns true only once, only in Input mode

### Integration Tests
1. **TTY + Trusted + Prompt**: `claudeless --scenario trusted.toml "Hello"`
   - Expect: Prompt processed, response shown, then input mode

2. **TTY + Untrusted + Prompt**: `claudeless --scenario untrusted.toml "Hello"`
   - Expect: Trust dialog shown, after Y pressed, prompt processed

3. **TTY + No Prompt**: `claudeless --scenario trusted.toml`
   - Expect: Welcome screen with placeholder, no auto-processing

4. **Non-TTY + Prompt**: `echo | claudeless --scenario test.toml "Hello"`
   - Expect: Uses print mode (not TUI), outputs response

5. **Print Mode**: `claudeless --scenario test.toml -p "Hello"`
   - Expect: Print mode output, no TUI

### Manual Testing
- Run in tmux session: `claudeless --scenario test.toml "Hello"`
- Verify prompt is processed without needing to type anything
- Verify can enter follow-up prompts after response
