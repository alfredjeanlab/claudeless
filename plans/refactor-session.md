# Plan: Refactor session.rs

## Overview

Refactor `crates/cli/src/state/session.rs` (931 lines) to be under the 750-line limit by extracting JSONL format types into a submodule and DRYing up repetitive patterns.

## Project Structure

```
crates/cli/src/state/
├── session.rs              # Core Session, Turn, SessionManager (~420 lines)
├── session/
│   └── jsonl.rs            # JSONL format types and write functions (~520 lines)
├── session_tests.rs        # Tests (unchanged)
```

## Dependencies

No new dependencies required. Uses existing:
- `chrono`
- `serde`/`serde_json`
- `std::collections::HashMap`
- `std::path::{Path, PathBuf}`

## Implementation Phases

### Phase 1: Create session submodule structure

Create `session/jsonl.rs` with all JSONL-related types:

1. Create directory `crates/cli/src/state/session/`
2. Create `crates/cli/src/state/session/jsonl.rs` containing:
   - `UserMessage`, `UserMessageLine`
   - `ToolResultContent`, `ToolResultUserMessage`, `ToolResultMessageLine`
   - `ContentBlock`, `CacheCreation`, `Usage`
   - `AssistantMessage`, `AssistantMessageLine`
   - `QueueOperationLine`
   - `TurnParams`, `UserMessageParams`, `UserMessageContent`, `AssistantMessageParams`
   - `write_queue_operation()`, `append_turn_jsonl()`, `append_user_message_jsonl()`, `append_assistant_message_jsonl()`

**Verification**: `cargo check` passes

### Phase 2: DRY up JSONL module with shared helpers

Extract common patterns to reduce line count:

1. **Extract file append helper**:
```rust
fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}
```

2. **Extract common message context** (optional, for maintainability):
```rust
/// Common fields shared by all message line types.
pub struct MessageContext<'a> {
    pub session_id: &'a str,
    pub cwd: &'a str,
    pub version: &'a str,
    pub git_branch: &'a str,
    pub timestamp: DateTime<Utc>,
}
```

3. **Trim verbose doc comments** - Remove comments that merely restate the obvious (e.g., "/// Type is always 'user'." on a `&'static str` field)

**Verification**: `cargo check` passes, `cargo test` passes

### Phase 3: Update session.rs imports and re-exports

1. Convert `session.rs` to a module with submodules:
   - Add `mod jsonl;` declaration
   - Add `pub use jsonl::*;` to maintain public API

2. Keep in `session.rs`:
   - `TurnToolCall` struct
   - `Turn` struct and impl
   - `Session` struct and impl
   - `SessionManager` struct and impl
   - `generate_session_id()` function
   - Test module reference

**Verification**: `cargo check` passes, existing tests pass

### Phase 4: Update mod.rs imports

Update `crates/cli/src/state/mod.rs` to verify re-exports work correctly. No changes should be needed since `session.rs` re-exports everything from `jsonl.rs`.

**Verification**: Full `make check` passes

## Key Implementation Details

### Module conversion pattern

When converting `session.rs` to have submodules, use the folder-based module pattern:

```
# Before (single file)
session.rs

# After (module with submodules)
session.rs          # becomes the module root, contains pub mod + re-exports + core types
session/
  jsonl.rs          # extracted JSONL types
```

Note: The `session.rs` file stays at the same path; Rust allows both `session.rs` and `session/` to coexist when `session.rs` declares `mod jsonl`.

### Line count breakdown

**session.rs (~420 lines)**:
- License/imports: ~15 lines
- `TurnToolCall`: ~8 lines
- `Turn` struct + impl: ~50 lines
- `Session` struct + impl: ~105 lines
- `SessionManager` struct + impl: ~210 lines
- `generate_session_id()`: ~10 lines
- Module declarations + re-exports: ~10 lines
- Test reference: ~5 lines

**session/jsonl.rs (~520 lines)**:
- License/imports: ~15 lines
- User message types: ~55 lines
- Tool result types: ~75 lines
- Content block types: ~45 lines
- Cache/Usage types: ~45 lines
- Assistant message types: ~70 lines
- Queue operation: ~35 lines
- TurnParams + append_turn_jsonl: ~80 lines
- UserMessageParams + append_user_message_jsonl: ~100 lines
- AssistantMessageParams + append_assistant_message_jsonl: ~60 lines

Both files comfortably under 750 lines.

### Public API preservation

The `mod.rs` currently re-exports:
```rust
pub use session::{
    append_assistant_message_jsonl, append_turn_jsonl, append_user_message_jsonl,
    write_queue_operation, AssistantMessage, AssistantMessageLine, AssistantMessageParams,
    ContentBlock, QueueOperationLine, Session, SessionManager, ToolResultContent,
    ToolResultMessageLine, ToolResultUserMessage, Turn, TurnParams, TurnToolCall, Usage,
    UserMessage, UserMessageContent, UserMessageLine, UserMessageParams,
};
```

All these exports must remain available from `session.rs` via `pub use jsonl::*;`.

## Verification Plan

1. **Phase 1**: `cargo check` - ensures new module compiles
2. **Phase 2**: `cargo check && cargo test --lib` - ensures DRY changes don't break functionality
3. **Phase 3**: `cargo check && cargo test --lib` - ensures re-exports work
4. **Phase 4**: `make check` - full verification including:
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `quench check --fix` (verifies line count < 750)
   - `cargo test --all`
   - `cargo build --all`

### Test coverage

Existing tests in `session_tests.rs` cover:
- Session creation and modification
- Turn management
- Session expiration
- Save/load persistence
- SessionManager operations

JSONL write functions are tested indirectly through integration tests. No changes to test files needed.
