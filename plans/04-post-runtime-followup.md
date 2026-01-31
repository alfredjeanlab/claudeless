# Post-Runtime Integration Follow-up

## Prerequisites

- **Plan 02**: Runtime extraction complete - `Runtime` struct with `execute()` method
- **Plan 03a (shared-agentic-loop)**: TUI integration complete - TUI mode uses `Runtime::execute()`
- **Plan 03b (tool-execution)**: `--tool-mode` CLI removed, default changed to `live`

## Follow-up Work

### 1. Integration Tests for Runtime (from plan 02, step 5)

Add integration tests that construct `Runtime` directly without going through main():

```rust
// tests/runtime_integration.rs
#[tokio::test]
async fn runtime_executes_scenario_response() {
    // Build runtime with simple scenario fixture
    // Execute "hello" prompt
    // Assert response contains expected output
}

#[tokio::test]
async fn runtime_executes_tools() {
    // Build runtime with tool_call scenario and mock executor
    // Execute prompt that triggers tool
    // Assert tools_executed > 0
}

#[tokio::test]
async fn runtime_fires_hooks() {
    // Build runtime with hooks scenario
    // Execute prompt that triggers hook
    // Assert hook_fired is true
}
```

**Test categories needed:**
- Scenario matching and response generation
- Tool execution loop (single tool, multiple tools, nested)
- Hook firing (Stop, PreToolUse, PostToolUse)
- Hook continuation flow
- State recording (verify JSONL output)
- Error handling (scenario not found, tool failure, hook failure)

### 2. Verify TUI Tool Execution (from plan 03 verification)

Create fixture tests that verify TUI mode now executes tools:

```rust
// tests/tui_tool_execution.rs
#[test]
fn tui_executes_bash_tool() {
    // Run claudeless with --tui, bash_tool scenario, and fixture input
    // Assert output contains "Tool executed: Bash"
}
```

**Fixture scenarios to add:**
- `scenarios/fixtures/tui_bash_tool.yaml` - Simple Bash tool call
- `scenarios/fixtures/tui_multi_tool.yaml` - Multiple sequential tools
- `scenarios/fixtures/tui_tool_failure.yaml` - Tool that fails, verify error display

### 3. Verify Live Mode Default (from plan 03b)

After removing `--tool-mode` and changing the default to `live`:

```rust
// tests/tool_mode_default.rs
#[test]
fn scenarios_without_tool_execution_config_use_live_mode() {
    // Run scenario with no [tool_execution] section
    // Assert actual command output appears (not mock, not disabled)
}

#[test]
fn mock_mode_requires_explicit_config() {
    // Run scenario with explicit mode = "mock"
    // Assert mock result appears
}
```

**Scenarios to verify:**
- Existing scenarios that relied on implicit `disabled` default now execute tools
- Scenarios using `mode = "mock"` still work correctly
- No scenario files have stale `--tool-mode` references in comments

### 4. Dead Code Cleanup

After refactoring, remove orphaned code:

| File | What to remove |
|------|----------------|
| `tui/app/commands/execution.rs` | Old `process_prompt()` implementation details now in Runtime |
| `main.rs` | Duplicated tool execution logic now in Runtime |
| `cli.rs` | Any remnants of `--tool-mode` handling |
| `executor.rs` | Verify `DisabledExecutor` fully removed |
| Potentially unused imports | Run `cargo clippy` to identify |

### 5. Feature Parity Audit

Verify both modes have identical behavior for:

| Feature | Print Mode | TUI Mode | Test |
|---------|-----------|----------|------|
| Tool execution | Via Runtime | Via Runtime | `test_tool_parity()` |
| Hook firing | Via Runtime | Via Runtime | `test_hook_parity()` |
| State recording | Via Runtime | Via Runtime | `test_state_parity()` |
| Capture logging | Via Runtime | Via Runtime | `test_capture_parity()` |
| Stop hook continuation | Via Runtime | Via Runtime | `test_stop_hook_parity()` |

Create a test that runs the same scenario in both modes and compares outputs:

```rust
#[test]
fn print_and_tui_produce_same_state_file() {
    // Run same scenario in print mode, capture state file
    // Run same scenario in TUI mode, capture state file
    // Normalize both (ignore timestamps)
    // Assert structures are equal
}
```

### 6. Documentation Updates

Update `docs/` to reflect new architecture:

- **Architecture overview**: Document Runtime as central abstraction
- **Adding features**: Explain that new agent features go in `Runtime::execute()`
- **Mode differences**: Document what remains mode-specific (UI concerns only)
- **Tool execution default**: Document that tools execute in `live` mode by default (from plan 03b)
- **Scenario configuration**: Document `[tool_execution] mode = "mock"` for test scenarios
- **Migration note**: Users who relied on `--tool-mode disabled` must now use scenario config

### 7. Performance Baseline

Establish performance baseline now that architecture is stable:

```bash
# Add to Makefile or scripts/
hyperfine --warmup 3 \
    'claudeless --scenario fixtures/simple.yaml "hello"' \
    'claudeless --scenario fixtures/tool_heavy.yaml "run all"'
```

Track:
- Cold start time (first prompt)
- Tool execution overhead
- TUI render loop impact

## Implementation Order

1. **Dead code cleanup** - Immediate, reduces noise
2. **Integration tests for Runtime** - Core verification of plan 02
3. **TUI tool execution tests** - Core verification of plan 03a
4. **Live mode default tests** - Core verification of plan 03b
5. **Feature parity audit** - Ensures no regressions
6. **Documentation** - Reflects new stable architecture
7. **Performance baseline** - Optional, for future optimization

## Verification

- [ ] `make check` passes
- [ ] New integration tests cover Runtime directly
- [ ] TUI fixture tests verify tool execution
- [ ] Scenarios without `[tool_execution]` config execute tools in live mode
- [ ] No dead code warnings from clippy
- [ ] Feature parity tests pass
- [ ] Documentation reflects both Runtime architecture and live-by-default behavior
