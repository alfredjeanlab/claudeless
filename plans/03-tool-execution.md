# Plan: Remove --tool-mode CLI Argument

## Summary

Move tool execution mode configuration from CLI argument to scenario-only, remove `disabled` mode, and change the default to `live`.

## Current State

- `--tool-mode` CLI flag in `SimulatorOptions` overrides scenario config
- Three modes: `disabled` (default), `mock`, `live`
- Scenario files can already specify `[tool_execution] mode = "..."`
- Resolution precedence: CLI > scenario > default(disabled)

## Changes

### 1. Remove CLI Argument (`crates/cli/src/cli.rs`)

- Delete `tool_mode` field from `SimulatorOptions` (line 208)
- Delete `CliToolExecutionMode` enum (lines 217-225)
- Delete `From<CliToolExecutionMode>` impl (lines 227-234)

### 2. Remove Disabled Mode (`crates/cli/src/config.rs`)

- Remove `Disabled` variant from `ToolExecutionMode` enum (line 218)
- Change `#[default]` attribute from `Disabled` to `Live` (line 222)

### 3. Remove DisabledExecutor (`crates/cli/src/tools/executor.rs`)

- Delete `DisabledExecutor` struct and impl (lines 86-110)
- Remove `Disabled` match arm from `create_executor()` (line 169)
- Remove `Disabled` match arm from `create_executor_with_mcp()` (line 197)

### 4. Simplify main.rs Resolution (`crates/cli/src/main.rs`)

Change lines 298-310 from:
```rust
let execution_mode = cli
    .simulator
    .tool_mode
    .clone()
    .map(ToolExecutionMode::from)
    .or_else(|| {
        scenario
            .as_ref()
            .and_then(|s| s.config().tool_execution.as_ref())
            .map(|te| te.mode.clone())
    })
    .unwrap_or_default();
```

To:
```rust
let execution_mode = scenario
    .as_ref()
    .and_then(|s| s.config().tool_execution.as_ref())
    .map(|te| te.mode.clone())
    .unwrap_or_default();  // Now defaults to Live
```

Also remove the `if execution_mode != ToolExecutionMode::Disabled` guard (line 312) since tools always execute now.

### 5. Update Tests

**`crates/cli/tests/mcp_scenarios.rs`** - Update all tests using `--tool-mode`:

| Test | Current | Change |
|------|---------|--------|
| `test_read_scenario_mock_mode` | `--tool-mode mock` | Add `[tool_execution] mode = "mock"` to fixture |
| `test_write_scenario_mock_mode` | `--tool-mode mock` | Add to fixture |
| `test_list_scenario_mock_mode` | `--tool-mode mock` | Add to fixture |
| `test_read_live_scenario_qualified_name_routes_to_mcp` | `--tool-mode live` | Already has fixture, verify `mode = "live"` |
| `test_read_raw_scenario_loads` | `--tool-mode disabled` | Remove test or change to `mock` |
| `test_disabled_mode_no_tool_results` | `--tool-mode disabled` | **Delete test** |
| `test_mock_mode_returns_canned_results` | `--tool-mode mock` | Add to fixture |
| `test_mcp_scenarios_have_tool_calls` | `--tool-mode mock` | Add to fixtures |

**`crates/cli/src/tools/executor_tests.rs`** - Remove disabled mode test (line 66)

**`crates/cli/src/config_tests.rs`** - Update default mode assertion (line 225-226)

### 6. Update Scenario Fixtures

Add `[tool_execution]` section to fixtures that need mock mode:
- `tests/fixtures/mcp-test-read.toml`
- `tests/fixtures/mcp-test-write.toml`
- `tests/fixtures/mcp-test-list.toml`

### 7. Update Documentation (`docs/USAGE.md`)

- Remove `--tool-mode` from CLI options table (line 26)
- Update example that uses `--tool-mode live` (line 186)
- Document `[tool_execution]` in scenario config section

## Files to Modify

1. `crates/cli/src/cli.rs` - Remove CLI argument
2. `crates/cli/src/config.rs` - Remove Disabled variant, change default
3. `crates/cli/src/tools/executor.rs` - Remove DisabledExecutor
4. `crates/cli/src/main.rs` - Simplify resolution logic
5. `crates/cli/tests/mcp_scenarios.rs` - Update/remove tests
6. `crates/cli/src/tools/executor_tests.rs` - Remove disabled test
7. `crates/cli/src/config_tests.rs` - Update default assertion
8. `tests/fixtures/mcp-test-*.toml` - Add tool_execution sections
9. `docs/USAGE.md` - Update documentation

## Verification

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`
4. Manual test: `claudeless --scenario scenarios/full-featured.toml -p "test"` should execute tools by default
