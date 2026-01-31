# Shared Agentic Loop Evaluation

## Summary

**Recommendation: Integrate TUI with Runtime after plan 02 is complete.**

The goal isn't just reducing duplication - it's making feature gaps **structurally impossible**. The `Runtime::execute()` approach from `plans/02-main-orchestration.md` naturally achieves this.

## Current State

### Print Mode (`main.rs:160-408`)
- Synchronous `'response_loop` with labeled continue for Stop hook
- Full implementation: prompt matching → response → tool execution → hooks → state recording
- Native async with `tokio::main`
- Output via `OutputWriter` to stdout

### TUI Mode (`tui/app/commands/execution.rs`)
- Event-driven via iocraft render loop
- `process_prompt()` → `start_streaming_inner()` flow
- Hook continuation via `pending_hook_message` queue
- Uses `block_on()` for hook execution (sync render model)
- **Does not implement tool execution**

## Duplicated Logic Identified

| Logic | Lines | Risk |
|-------|-------|------|
| Scenario matching | ~10 lines each | Low - delegates to `Scenario` methods |
| Token estimation | 1 line each | Trivial |
| JSONL recording | ~20 lines each | Low - different granularity needed |
| Stop hook execution | ~25 lines each | Medium - response parsing could diverge |
| Stop hook response parsing | ~15 lines each | **Highest risk** |

## The Real Problem

The concern isn't duplication per se - it's **feature parity**. TUI mode is missing tool execution entirely. With the current architecture, it's easy to add a feature to print mode and forget TUI (or vice versa).

## Why Runtime Extraction Solves This

From `plans/02-main-orchestration.md`:

```rust
pub struct Runtime {
    context: RuntimeContext,
    scenario: Option<Scenario>,
    executor: Box<dyn ToolExecutor>,
    state: Option<StateWriter>,
    capture: Option<CaptureLog>,
}

impl Runtime {
    /// Execute a single agent turn - shared by both modes
    pub async fn execute(&mut self, prompt: &str) -> Result<TurnResult> {
        // Scenario matching
        // Tool execution loop
        // Hook firing
        // State recording
    }
}
```

**Key insight**: If both modes call `Runtime::execute()`:
- Tool execution is shared → TUI gets it automatically
- New features added to `execute()` → both modes get them
- Mode-specific concerns (output format, UI state) → handled by caller

## Recommended Approach

The Runtime extraction (plan 02) is the right abstraction boundary. `Runtime::execute()` should include the full agent loop:
- Prompt matching
- Response generation
- Tool execution
- Hook firing (with continuation support)
- State recording

## What Stays Mode-Specific

Even with shared turn logic, modes differ in:

| Concern | Print Mode | TUI Mode |
|---------|-----------|----------|
| Output | `OutputWriter` → stdout | `DisplayState` → render |
| Loop control | `'response_loop` continue | `pending_hook_message` queue |
| Async model | Native async | `block_on()` for iocraft |
| User interaction | None (single-shot) | Keyboard, dialogs, permissions |

These differences are **appropriate** - they're presentation layer, not business logic.

## Sequencing

**Prerequisite**: `plans/02-main-orchestration.md`
- Extracts `Runtime` struct with `execute()` method
- Print mode uses `Runtime::execute()` for agent turns

**This plan** (to be implemented after plan 02):
- Integrate TUI mode with the Runtime abstraction
- TUI calls `Runtime::execute()` instead of its own `process_prompt()`
- TUI gains tool execution automatically
- Future features added to `execute()` apply to both modes

## Implementation Steps (after plan 02 complete)

1. **Update TuiConfig to hold Runtime** instead of individual components
   - Replace `hook_executor`, `state_writer` fields with `runtime: Runtime`

2. **Refactor `process_prompt()` in execution.rs**
   - Call `runtime.execute(&prompt).await` (via `block_on()`)
   - Handle `TurnResult` to update display state and tool outputs

3. **Refactor `start_streaming_inner()`**
   - Remove scenario matching (now in Runtime)
   - Remove hook firing (now in Runtime)
   - Keep only UI concerns: mode transitions, spinner, display updates

4. **Handle `TurnResult.hook_continuation`**
   - If Some, queue as `pending_hook_message`
   - Existing render loop check handles re-processing

5. **Remove duplicated code from TUI**
   - Token estimation (use Runtime's)
   - JSONL recording (use Runtime's StateWriter)
   - Hook response parsing (use Runtime's)

## Critical Files

- `crates/cli/src/tui/app/types.rs` - Update TuiConfig
- `crates/cli/src/tui/app/state.rs` - Update TuiAppStateInner
- `crates/cli/src/tui/app/commands/execution.rs` - Refactor to use Runtime
- `crates/cli/src/main.rs` - Pass Runtime to TUI mode

## Verification

1. TUI mode executes tools (currently missing - this is the main win)
2. Adding a feature to `Runtime::execute()` affects both modes
3. `make check` passes
4. Existing TUI fixture tests still pass
