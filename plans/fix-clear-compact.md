# Plan: Fix /clear and /compact Support

## Overview

Implement and fix the `/clear` and `/compact` slash commands in the TUI to match the real Claude CLI v2.1.12 behavior. The `/clear` command is currently unimplemented, and `/compact` has rendering discrepancies.

## Project Structure

Key files to modify:

```
crates/cli/src/tui/
├── app.rs              # Main TUI state and command handling (primary changes)
└── streaming.rs        # Token streaming (possibly for compact delay)

crates/cli/src/config.rs        # Scenario config for compact_delay_ms
crates/cli/src/state/session.rs # Session/turn tracking for tool summaries
```

Test files:
```
crates/cli/tests/
├── tui_clear.rs        # Clear command tests
├── tui_compacting.rs   # Compact command tests
└── fixtures/tui/v2.1.12/
    ├── clear_after.txt
    ├── compact_during.txt
    └── compact_after.txt
```

## Dependencies

No new external dependencies required. Uses existing:
- `iocraft` for TUI rendering
- `chrono` for timestamps

## Implementation Phases

### Phase 1: Implement /clear Command

**Goal:** Make `test_clear_after_matches_fixture` pass.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Add `/clear` handling in `handle_command_inner()`:

```rust
"/clear" => {
    // Clear conversation display and session
    inner.conversation_display.clear();
    inner.response_content = "(no content)".to_string();
    inner.is_command_output = true;

    // Clear session turns
    {
        let mut sessions = inner.sessions.lock();
        sessions.current_session().turns.clear();
    }

    // Reset token counts
    inner.status.input_tokens = 0;
    inner.status.output_tokens = 0;
}
```

2. Update `render_conversation_area()` to format clear output:
   - After `/clear`, the display shows:
     ```
     ❯ /clear
       ⎿  (no content)
     ```
   - The `⎿` (elbow connector) indicates the response to the command

3. Add special case rendering for `/clear` output with elbow connector.

**Expected fixture match (`clear_after.txt`):**
```
 ▐▛███▜▌   Claude Code v2.1.12
▝▜█████▛▘  Haiku 4.5 · Claude Max
  ▘▘ ▝▝    ~/Developer/claudeless

❯ /clear
  ⎿  (no content)

────────────...
```

### Phase 2: Fix /compact Symbol and In-Progress Message

**Goal:** Fix the compacting-in-progress display to match `compact_during.txt`.

**Current behavior:**
```
* Compacting conversation... (ctrl+c to interrupt)
```

**Expected behavior:**
```
✻ Compacting conversation… (ctrl+c to interrupt)
```

**Changes to `crates/cli/src/tui/app.rs`:**

1. Fix the symbol in `handle_command_inner()`:

```rust
"/compact" => {
    inner.mode = AppMode::Responding;
    inner.is_compacting = true;
    inner.compacting_started = Some(std::time::Instant::now());
    // Use correct symbol (✻) and ellipsis (…)
    inner.response_content =
        "✻ Compacting conversation… (ctrl+c to interrupt)".to_string();
    inner.conversation_display.clear();
}
```

2. Update render to show only the compacting message without prefix during compacting:
   - During compact, show: `❯ /compact` followed by newline, then `✻ Compacting...`
   - Remove the `⏺` prefix for this special message

### Phase 3: Fix /compact Completion Display

**Goal:** Fix the post-compact display to match `compact_after.txt`.

**Expected behavior after compact:**
```
══════════════════════════════════════ Conversation compacted · ctrl+o for history ═══════════════════════════════════════

❯ /compact
  ⎿  Compacted (ctrl+o to see full summary)
  ⎿  Read Cargo.toml (14 lines)
```

**Changes to `crates/cli/src/tui/app.rs`:**

1. Update `check_compacting()` to generate proper completion output:

```rust
pub fn check_compacting(&self) {
    let mut inner = self.inner.lock();
    if inner.is_compacting {
        if let Some(started) = inner.compacting_started {
            let delay_ms = inner.config.compact_delay_ms.unwrap_or(500);
            if started.elapsed() >= std::time::Duration::from_millis(delay_ms) {
                inner.is_compacting = false;
                inner.compacting_started = None;
                inner.mode = AppMode::Input;

                // Build tool summary from session turns
                let tool_summary = build_tool_summary(&inner.sessions);

                // Set response with elbow connector format
                inner.response_content = format!(
                    "Compacted (ctrl+o to see full summary){}",
                    if tool_summary.is_empty() { String::new() }
                    else { format!("\n{}", tool_summary) }
                );
                inner.is_command_output = true;
            }
        }
    }
}
```

2. Add `build_tool_summary()` helper function to extract tool call summaries from session turns:

```rust
fn build_tool_summary(sessions: &Arc<Mutex<SessionManager>>) -> String {
    let sessions = sessions.lock();
    let Some(session) = sessions.get_current() else { return String::new() };

    let mut summaries = Vec::new();
    for turn in &session.turns {
        for tool_call in &turn.tool_calls {
            if let Some(summary) = format_tool_summary(&tool_call) {
                summaries.push(summary);
            }
        }
    }
    summaries.join("\n")
}

fn format_tool_summary(tool: &TurnToolCall) -> Option<String> {
    match tool.tool.as_str() {
        "Read" => {
            let path = tool.input.get("file_path")?.as_str()?;
            let lines = tool.output.as_ref()
                .map(|o| o.lines().count())
                .unwrap_or(0);
            Some(format!("  ⎿  Read {} ({} lines)", path, lines))
        }
        "Write" => {
            let path = tool.input.get("file_path")?.as_str()?;
            Some(format!("  ⎿  Wrote {}", path))
        }
        "Edit" => {
            let path = tool.input.get("file_path")?.as_str()?;
            Some(format!("  ⎿  Edited {}", path))
        }
        "Bash" => {
            let cmd = tool.input.get("command")?.as_str()?;
            let short_cmd = if cmd.len() > 30 {
                format!("{}...", &cmd[..27])
            } else {
                cmd.to_string()
            };
            Some(format!("  ⎿  Ran `{}`", short_cmd))
        }
        _ => None
    }
}
```

3. Add the separator line rendering. Update `render_conversation_area()` to prepend the compact separator when compacted:

```rust
// Full-width compact separator (120 chars with centered text)
const COMPACT_SEPARATOR: &str =
    "══════════════════════════════════════ Conversation compacted · ctrl+o for history ═════════════════════════════════════";
```

4. Track `is_compacted` state to show separator at top of conversation area.

### Phase 4: Add Scenario Support for compact_delay_ms

**Goal:** Allow tests to control compact delay for deterministic test timing.

**Changes to `crates/cli/src/config.rs`:**

1. Add `compact_delay_ms` field to `ScenarioConfig`:

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ScenarioConfig {
    // ... existing fields ...

    /// Delay in milliseconds before compact completes (default: 500)
    #[serde(default)]
    pub compact_delay_ms: Option<u64>,
}
```

**Changes to `crates/cli/src/tui/app.rs`:**

1. Add `compact_delay_ms` to `TuiConfig`:

```rust
pub struct TuiConfig {
    // ... existing fields ...
    pub compact_delay_ms: Option<u64>,
}
```

2. Pass through from scenario in `TuiConfig::from_scenario()`.

### Phase 5: Update Response Rendering for Command Output

**Goal:** Ensure command outputs (like `/clear` and `/compact`) use the elbow connector format.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Update `render_conversation_area()` to handle command output specially:

```rust
fn render_conversation_area(state: &RenderState) -> AnyElement<'static> {
    let mut content = String::new();

    // Add compact separator if compacted
    if state.is_compacted {
        content.push_str(COMPACT_SEPARATOR);
        content.push('\n');
    }

    // Add conversation display
    if !state.conversation_display.is_empty() {
        content.push_str(&state.conversation_display);
    }

    // Add current response
    if !state.response_content.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\n");
        }

        if state.is_command_output {
            // Command output uses elbow connector, not ⏺
            // Format each line with proper indentation
            for (i, line) in state.response_content.lines().enumerate() {
                if i == 0 {
                    content.push_str(&format!("  ⎿  {}", line));
                } else {
                    content.push_str(&format!("\n  ⎿  {}", line));
                }
            }
        } else {
            content.push_str(&format!("⏺ {}", state.response_content));
        }
    }
    // ... rest of function
}
```

2. Add `is_compacted` to `RenderState` and `TuiAppStateInner`.

### Phase 6: Tests and Verification

**Enable ignored tests:**

1. Remove `#[ignore]` from `test_clear_after_matches_fixture` in `tui_clear.rs`
2. Remove `#[ignore]` from the following in `tui_compacting.rs`:
   - `test_compact_before_matches_fixture` (may need response format fixes)
   - `test_compact_during_matches_fixture`
   - `test_compact_after_matches_fixture`

**Update LIMITATIONS.md:**

Remove the following from Known TODOs:
```
- [ ] **TUI /clear command**: Not implemented
- [ ] **TUI /compact**: Rendering differs from real Claude
```

## Key Implementation Details

### Elbow Connector Format

The real Claude CLI uses `⎿` (Box Drawings Light Arc Up and Right, U+23BF) as an elbow connector for command output, indented with 2 spaces:

```
❯ /command
  ⎿  output line 1
  ⎿  output line 2
```

### Compact Separator

The separator uses double-line box drawing characters (`═`, U+2550) and is 120 characters wide with centered text:

```
══════════════════════════════════════ Conversation compacted · ctrl+o for history ═════════════════════════════════════
```

### State Tracking

New state fields needed in `TuiAppStateInner`:
- `is_compacted: bool` - Whether conversation has been compacted (for showing separator)
- `compact_delay_ms: Option<u64>` - Configurable delay from scenario

### Tool Summary Generation

Tool summaries shown after compact are derived from `TurnToolCall` records in session turns. Format varies by tool:
- Read: `Read {path} ({lines} lines)`
- Write: `Wrote {path}`
- Edit: `Edited {path}`
- Bash: `Ran \`{command}\``

## Verification Plan

1. Run `cargo fmt --all -- --check`
2. Run `cargo clippy --all-targets --all-features -- -D warnings`
3. Run specific tests:
   ```bash
   cargo test test_clear -- --nocapture
   cargo test test_compact -- --nocapture
   ```
4. Run all tests: `cargo test --all`
5. Run full check: `make check`

**Manual verification:**
```bash
# Start TUI and test commands
claudeless scenarios/full-featured.toml

# In the TUI:
# 1. Enter a prompt, get response
# 2. Type /clear - verify "(no content)" output
# 3. Enter another prompt, get response
# 4. Type /compact - verify in-progress and completion messages
```

## Files Changed

| File | Action |
|------|--------|
| `crates/cli/src/tui/app.rs` | Edit |
| `crates/cli/src/config.rs` | Edit |
| `crates/cli/tests/tui_clear.rs` | Edit (remove #[ignore]) |
| `crates/cli/tests/tui_compacting.rs` | Edit (remove #[ignore]) |
| `docs/LIMITATIONS.md` | Edit (update Known TODOs) |
