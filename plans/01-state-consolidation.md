# State Management Consolidation

## Problem

Six overlapping abstractions handle "session" and "state" concepts:

| Type | Location | Purpose |
|------|----------|---------|
| `StateDirectory` | `state/directory.rs` | Path resolution, directory structure |
| `StateWriter` | `state/mod.rs` | JSONL writing, index updates |
| `SessionContext` | `session/context.rs` | Runtime config (CLI + scenario merge) |
| `Session` | `state/session.rs` | In-memory conversation history |
| `SessionManager` | `state/session.rs` | Multi-session management |
| `SessionsIndex` | `state/sessions_index.rs` | Session metadata index |

The name collision between `session/` module (runtime config) and `state/session.rs` (persistence) causes confusion.

## Plan

1. **Rename `session/` module to `runtime/`** and `SessionContext` to `RuntimeContext`
   - This module merges CLI args + scenario config — it's runtime configuration, not session state

2. **Consolidate state types into clear layers**:
   - `state/paths.rs` — Path resolution only (extract from StateDirectory)
   - `state/persistence.rs` — JSONL read/write operations
   - `state/index.rs` — Session index management
   - Keep `StateDirectory` as the facade that composes these

3. **Evaluate `SessionManager` usage** — if only used in tests, move to test utilities or remove

4. **Consider merging `Session` into `StateWriter`** — they track overlapping concerns (turns, messages)
