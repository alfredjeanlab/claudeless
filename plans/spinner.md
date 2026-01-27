# Plan: Integrate Spinner Message Component

## Overview

Add an animated spinner component to the claudeless TUI that matches Claude Code's "Thinking..." animation behavior.

## Current State

- **No animation**: Status messages like "✻ Compacting conversation…" are static text
- **Timer exists**: 100ms render loop already in place (`app.rs:80-88`)
- **State management ready**: `TuiAppState` supports tracking additional fields

## Claude Code Reference Implementation

From `checkout/v2.1.12/cli/misc/021-chunk-1000.js:2446-2455`:
```
Frames (macOS): ["·", "✢", "✳", "✶", "✻", "✽"]
Full cycle: forward + reverse = 12 frames
Timing: 120ms per frame
```

---

## Implementation Steps

### 1. Add Spinner Module

**Create:** `crates/cli/src/tui/spinner.rs`

```rust
/// Spinner animation frames (platform-aware)
pub fn spinner_frames() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["·", "✢", "✳", "✶", "✻", "✽"]
    } else {
        &["·", "✢", "*", "✶", "✻", "✽"]
    }
}

/// Full animation cycle (forward + reverse for breathing effect)
pub fn spinner_cycle() -> Vec<&'static str> {
    let frames = spinner_frames();
    let mut cycle: Vec<&str> = frames.to_vec();
    cycle.extend(frames.iter().rev().skip(1).take(frames.len() - 1));
    cycle
}

/// Whimsical verbs for status messages
pub const SPINNER_VERBS: &[&str] = &[
    "Thinking", "Computing", "Pondering", "Processing",
    "Contemplating", "Cogitating", "Deliberating", "Musing",
];
```

### 2. Add Spinner State

**Edit:** `crates/cli/src/tui/app/state/display.rs`

Add fields:
```rust
pub spinner_frame: usize,
pub spinner_verb: String,
```

### 3. Add Frame Advancement Logic

**Edit:** `crates/cli/src/tui/app/state/mod.rs`

Add method:
```rust
pub fn advance_spinner(&mut self) {
    let cycle_len = spinner::spinner_cycle().len();
    self.display.spinner_frame = (self.display.spinner_frame + 1) % cycle_len;
}
```

### 4. Integrate with Timer Loop

**Edit:** `crates/cli/src/tui/app.rs`

In the periodic timer (currently 100ms), call `advance_spinner()` when in Responding/Thinking mode:
```rust
// Change timer from 100ms to 120ms to match Claude Code
tokio::time::sleep(Duration::from_millis(120)).await;
// In the loop body:
if matches!(state.mode, AppMode::Responding | AppMode::Thinking) {
    state.advance_spinner();
}
```

### 5. Create Spinner Render Function

**Edit:** `crates/cli/src/tui/app/render/content.rs`

Add function:
```rust
fn render_spinner(state: &RenderState) -> String {
    let cycle = spinner::spinner_cycle();
    let frame = cycle[state.spinner_frame % cycle.len()];
    let verb = &state.spinner_verb;
    format!("{} {}…", frame, verb)
}
```

### 6. Display Spinner in Conversation Area

**Edit:** `crates/cli/src/tui/app/render/content.rs:79-108`

Replace static "✻ Compacting..." with animated spinner when in responding mode:
```rust
if state.is_compacting {
    // Show compacting with spinner
    format!("{} Compacting conversation… (ctrl+c to interrupt)", spinner_char)
} else if matches!(state.mode, AppMode::Responding) {
    render_spinner(state)
}
```

### 7. Reset Spinner on Mode Change

**Edit:** `crates/cli/src/tui/app/commands.rs`

When entering Responding mode, reset spinner and pick random verb:
```rust
state.display.spinner_frame = 0;
state.display.spinner_verb = spinner::random_verb().to_string();
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/cli/src/tui/mod.rs` | Add `mod spinner;` |
| `crates/cli/src/tui/spinner.rs` | **New file** - frames, cycle, verbs |
| `crates/cli/src/tui/app/state/display.rs` | Add `spinner_frame`, `spinner_verb` |
| `crates/cli/src/tui/app/state/mod.rs` | Add `advance_spinner()` method |
| `crates/cli/src/tui/app/types.rs` | Add spinner fields to `RenderState` |
| `crates/cli/src/tui/app.rs` | Change timer to 120ms, call advance_spinner |
| `crates/cli/src/tui/app/render/content.rs` | Use spinner in response rendering |
| `crates/cli/src/tui/app/commands.rs` | Reset spinner on mode transitions |

---

## Verification

1. **Build**: `cargo build --all`
2. **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
3. **Test**: `cargo test --all`
4. **Manual test**: Run the CLI and verify:
   - Spinner animates during response streaming
   - Spinner shows during compacting
   - Animation is smooth (breathing effect)
   - Verb changes between requests
5. **Full check**: `make check`
