# Re-export List Cleanup

## Problem

`state/mod.rs` re-exports 35+ items, making module boundaries meaningless:

```rust
pub use session::{
    append_assistant_message_jsonl, append_error_jsonl, append_result_jsonl,
    append_turn_jsonl, append_user_message_jsonl, write_queue_operation,
    AssistantMessage, AssistantMessageLine, AssistantMessageParams, ContentBlock,
    ErrorLine, QueueOperationLine, ResultLine, Session, SessionManager,
    ToolResultContent, ToolResultMessageLine, ToolResultUserMessage, Turn,
    TurnParams, TurnToolCall, Usage, UserMessage, UserMessageContent,
    UserMessageLine, UserMessageParams,
};
```

This exposes internal serialization types (`*Line`, `*Params`) as public API.

## Plan

1. **Audit each re-exported type** for actual external usage:
   - Search for `use claudeless::state::{Type}` in tests and external code
   - Types only used within `state/` module become `pub(crate)` or private

2. **Define explicit public API** — likely just:
   ```rust
   // Facade types
   pub use directory::StateDirectory;
   pub use mod::StateWriter;

   // Domain types used externally
   pub use session::{Session, Turn, ContentBlock};
   pub use todos::{TodoItem, TodoStatus};
   pub use settings::{ClaudeSettings, PermissionSettings};
   ```

3. **Keep `io` utilities as `pub(crate)`** — they're internal helpers:
   ```rust
   pub(crate) use io::{files_in, json_files_in, ensure_parent_exists, ...};
   ```

4. **Apply same audit to other modules** — `mcp/mod.rs`, `tools/mod.rs`, `permission/mod.rs`
