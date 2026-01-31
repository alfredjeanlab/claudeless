# --no-session-persistence Implementation Plan

## Overview

Add a `--no-session-persistence` CLI flag that disables session persistence. When enabled, sessions will not be saved to disk (no JSONL files, no sessions-index updates) and cannot be resumed. This flag only works with `--print` mode for non-interactive use cases like CI pipelines or ephemeral queries.

## Project Structure

Key files to modify:
```
crates/cli/src/
├── cli.rs              # Add --no-session-persistence flag
├── main.rs             # Conditionally skip StateWriter operations
└── cli_tests.rs        # Add unit tests for flag validation
```

## Dependencies

No new dependencies required. Uses existing:
- `clap` for CLI argument parsing

## Implementation Phases

### Phase 1: Add CLI Flag

**Goal:** Add `--no-session-persistence` flag that validates it's only used with `--print`.

**Files:** `cli.rs`, `cli_tests.rs`

1. Add the flag to the `Cli` struct in `cli.rs`:
```rust
/// Disable session persistence - sessions will not be saved to disk and
/// cannot be resumed (only works with --print)
#[arg(long)]
pub no_session_persistence: bool,
```

2. Add validation method to `Cli`:
```rust
/// Validate that --no-session-persistence is only used with --print
pub fn validate_no_session_persistence(&self) -> Result<(), &'static str> {
    if self.no_session_persistence && !self.print {
        return Err("--no-session-persistence can only be used with --print mode");
    }
    Ok(())
}
```

3. Add unit tests in `cli_tests.rs`:
```rust
#[test]
fn no_session_persistence_requires_print_mode() {
    let cli = Cli::try_parse_from(["claude", "--no-session-persistence", "prompt"]).unwrap();
    assert!(cli.validate_no_session_persistence().is_err());
}

#[test]
fn no_session_persistence_with_print_mode_succeeds() {
    let cli = Cli::try_parse_from(["claude", "-p", "--no-session-persistence", "prompt"]).unwrap();
    assert!(cli.validate_no_session_persistence().is_ok());
}
```

**Verification:** `cargo test cli_tests::no_session_persistence`

### Phase 2: Validate Flag in Main

**Goal:** Add validation at startup to ensure flag combinations are valid.

**Files:** `main.rs`

1. Add validation after CLI parsing (around line 34):
```rust
let cli = Cli::parse();

// Validate --no-session-persistence usage
if let Err(msg) = cli.validate_no_session_persistence() {
    eprintln!("Error: {}", msg);
    std::process::exit(1);
}
```

**Verification:** Test with invalid flag combination:
```bash
claudeless --no-session-persistence "hello" 2>&1 | grep "only works with --print"
```

### Phase 3: Conditionally Skip State Writing

**Goal:** Skip all state persistence operations when `--no-session-persistence` is enabled.

**Files:** `main.rs`

1. Wrap `StateWriter` creation in a conditional (around line 200-208):
```rust
// Create state writer for recording turns and handling stateful tools
// Skip if --no-session-persistence is enabled
let state_writer = if !cli.no_session_persistence {
    Some(Arc::new(RwLock::new(StateWriter::new(
        session_ctx.session_id.to_string(),
        &session_ctx.project_path,
        session_ctx.launch_timestamp,
        &session_ctx.model,
        &session_ctx.working_directory,
    )?)))
} else {
    None
};
```

2. Guard the queue-operation write (around line 210-213):
```rust
// Write queue-operation for print mode (-p) unless persistence is disabled
if cli.print && !cli.no_session_persistence {
    if let Some(ref writer) = state_writer {
        writer.read().write_queue_operation()?;
    }
}
```

3. Guard turn recording (around line 218-232):
```rust
if tool_calls.is_empty() {
    // Simple turn without tool calls
    if let Some(ref writer) = state_writer {
        writer.write().record_turn(&prompt, &response_text)?;
    }
} else {
    // Turn with tool calls - use granular recording if persistence enabled
    if let Some(ref writer) = state_writer {
        let user_uuid = writer.write().record_user_message(&prompt)?;
        // ... rest of tool call recording logic
    }
}
```

4. Update tool execution to handle `Option<Arc<RwLock<StateWriter>>>`:
```rust
// Create executor with state writer for stateful tools
let executor: Box<dyn ToolExecutor> = match execution_mode {
    ToolExecutionMode::Live => {
        let mut builtin = BuiltinExecutor::new();
        if let Some(ref writer) = state_writer {
            builtin = builtin.with_state_writer(Arc::clone(writer));
        }
        let mcp = mcp_manager
            .as_ref()
            .map(|m| McpToolExecutor::new(Arc::clone(m)));
        Box::new(CompositeExecutor::new(mcp, builtin))
    }
    _ => create_executor(execution_mode),
};
```

**Verification:**
```bash
# Should NOT create any files in ~/.claude/projects/
claudeless -p --no-session-persistence "hello" --output-format json

# Verify no session files created
ls ~/.claude/projects/*/  # Should not have new .jsonl file
```

### Phase 4: Integration Testing

**Goal:** Add integration tests to verify the flag works correctly end-to-end.

**Files:** `tests/` or manual testing scripts

1. Create test case that verifies no files are written:
```bash
#!/bin/bash
# Test: --no-session-persistence doesn't write files

# Setup: Record project dir state before
BEFORE=$(ls -la ~/.claude/projects/*/ 2>/dev/null | wc -l)

# Run with --no-session-persistence
claudeless -p --no-session-persistence "test query"

# Verify: No new files created
AFTER=$(ls -la ~/.claude/projects/*/ 2>/dev/null | wc -l)
if [ "$AFTER" -gt "$BEFORE" ]; then
    echo "FAIL: New files were created despite --no-session-persistence"
    exit 1
fi
echo "PASS: No new files created"
```

2. Test that resume fails appropriately after non-persistent session:
```bash
# Get session ID from output, try to resume - should fail
SESSION_ID=$(claudeless -p --no-session-persistence "test" -o json | jq -r '.session_id')
claudeless -r "$SESSION_ID" "continue" 2>&1 | grep -q "Session not found"
```

**Verification:** `make check`

## Key Implementation Details

### State Bypassing Strategy

Rather than modifying `StateWriter` internals, we use `Option<StateWriter>` wrapping in main.rs. This keeps `StateWriter` unchanged and concentrates the no-persistence logic in one place.

### What Gets Skipped

When `--no-session-persistence` is enabled:
- No `.jsonl` session file created in `~/.claude/projects/`
- No `sessions-index.json` updates
- No `queue-operation` line written
- No turn/message recording
- No todo file creation
- No plan file creation

### What Still Works

- Output formatting (text, json, stream-json) - works normally
- MCP servers - still initialized and used
- Tool execution - works but results not persisted
- Session ID generation - still generated for output consistency

### Flag Naming

Using `--no-session-persistence` (with hyphens) to match Claude CLI conventions like `--no-verify` patterns and existing long flags.

## Verification Plan

1. **Unit tests:**
   - `cargo test cli_tests::no_session_persistence_requires_print_mode`
   - `cargo test cli_tests::no_session_persistence_with_print_mode_succeeds`

2. **Integration tests:**
```bash
# Flag requires --print mode
claudeless --no-session-persistence "test" 2>&1 | grep -q "only works with --print"
echo "PASS: Flag validation works"

# No files created with flag
TEMP_STATE_DIR=$(mktemp -d)
CLAUDELESS_STATE_DIR=$TEMP_STATE_DIR claudeless -p --no-session-persistence "test"
[ -z "$(ls $TEMP_STATE_DIR/projects/*/*.jsonl 2>/dev/null)" ]
echo "PASS: No session files created"

# Normal operation still creates files
CLAUDELESS_STATE_DIR=$TEMP_STATE_DIR claudeless -p "test"
[ -n "$(ls $TEMP_STATE_DIR/projects/*/*.jsonl 2>/dev/null)" ]
echo "PASS: Normal operation creates files"

rm -rf $TEMP_STATE_DIR
```

3. **Full check:**
```bash
make check
```

4. **Manual verification:**
```bash
# Text output works
claudeless -p --no-session-persistence "hello"

# JSON output works
claudeless -p --no-session-persistence "hello" -o json | jq .

# Stream JSON output works
claudeless -p --no-session-persistence "hello" -o stream-json | head -1 | jq .
```
