# Plan: Remove Sandbox and Simplify Tool Execution Modes

**Root Feature:** `cl-54cc`

## Summary

Remove all sandbox-related code and simplify tool execution to three modes: `disabled`, `mock`, `live`.

**Removals:**
- `--sandbox-root`, `--allow-real-bash` CLI flags and env vars
- `sandbox_root`, `allow_real_bash` config fields
- `resolve_path()` and path traversal prevention
- `RealMcp` mode and entire `tools/mcp/` module

**Renames:**
- `--tool-execution-mode` → `--tool-mode`
- `CLAUDELESS_TOOL_EXECUTION_MODE` → `CLAUDELESS_TOOL_MODE`
- `Simulated` → `Live`

## Implementation Phases

### Phase 1: Config & CLI Changes

**`crates/cli/src/config.rs`**
- Remove `sandbox_root` and `allow_real_bash` from `ToolExecutionConfig` (lines 118-123)
- Update `ToolExecutionMode` enum: rename `Simulated` → `Live`, remove `RealMcp` (lines 147-160)

**`crates/cli/src/cli.rs`**
- Rename `tool_execution_mode` → `tool_mode`, update env var (lines 143-145)
- Remove `sandbox_root` and `allow_real_bash` fields (lines 147-153)
- Update `CliToolExecutionMode`: rename `Simulated` → `Live`, remove `RealMcp` (lines 156-178)

### Phase 2: Executor Simplification

**`crates/cli/src/tools/executor.rs`**
- Remove `sandbox_root` and `allow_real_bash` from `ExecutionContext` (lines 20, 23)
- Remove `from_config()` method (lines 31-38)
- Update `create_executor()`: `Simulated` → `Live`, remove `RealMcp` case (lines 183-189)

**`crates/cli/src/tools/builtin/mod.rs`**
- Remove `sandbox_root`, `allow_real_bash` from `BuiltinExecutor` (lines 42-43, 69-70)
- Remove `with_sandbox_root()`, `with_real_bash()` methods (lines 76-85)
- Remove `sandbox_root`, `allow_real_bash` from `BuiltinContext` (lines 162, 164)
- Remove entire `resolve_path()` method (lines 171-189)
- Simplify context creation in `execute()` (lines 127-135)

### Phase 3: Update Builtin Tools

**`crates/cli/src/tools/builtin/bash.rs`**
- Remove `allow_real_bash` check - always execute real commands (line 81)
- Remove `sandbox_root` fallback for cwd (lines 39-41)

**`crates/cli/src/tools/builtin/read.rs`**
- Replace `ctx.resolve_path(path)` with `PathBuf::from(path)` (line 49)

**`crates/cli/src/tools/builtin/write.rs`**
- Replace `ctx.resolve_path(path)` with `PathBuf::from(path)` (line 64)

**`crates/cli/src/tools/builtin/edit.rs`**
- Replace `ctx.resolve_path(path)` with `PathBuf::from(path)` (line 87)

**`crates/cli/src/tools/builtin/glob.rs`**
- Replace `ctx.resolve_path()` calls with `PathBuf::from()` (lines 58, 66)
- Remove `ctx.sandbox_root` usage (line 60)

**`crates/cli/src/tools/builtin/grep.rs`**
- Replace `ctx.resolve_path()` with `PathBuf::from()` (line 119)
- Remove `ctx.sandbox_root` usage (line 121)

### Phase 4: Remove MCP Module

**`crates/cli/src/tools/mod.rs`**
- Remove `pub mod mcp;` declaration

**Delete entire directory:**
- `crates/cli/src/tools/mcp/` (all files)

### Phase 5: Update main.rs

**`crates/cli/src/main.rs`**
- Rename `cli.tool_execution_mode` → `cli.tool_mode` (line 216)
- Remove sandbox_root and allow_real_bash context setup (lines 234-237)
- Update `Simulated` → `Live` in match (line 241)

### Phase 6: Update Tests

**Delete tests:**
- `tools/builtin/mod_tests.rs`: `test_sandbox_path_resolution`, `test_no_sandbox_path_resolution`
- `tools/builtin/bash_tests.rs`: `test_bash_without_real_execution`
- `tools/executor_tests.rs`: `test_execution_context_from_config`
- Entire `tools/mcp/*_tests.rs` files (deleted with module)

**Update tests:**
- `config_tests.rs`: Update `test_parse_tool_execution_simulated` → test `live` mode, remove sandbox assertions
- `executor_tests.rs`: Update `Simulated` → `Live` references
- `bash_tests.rs`: Remove `allow_real_bash` from context in remaining tests
- `scenario_fields.rs`: Remove `sandbox_root` from TOML fixtures, update `simulated` → `live`
- `tests/dot_claude_plans.rs`, `tests/dot_claude_todos.rs`: Update `simulated` → `live`

**Update fixtures:**
- `tests/fixtures/dotclaude/v2.1.12/plan-mode/scenario.toml`: `simulated` → `live`
- `tests/fixtures/dotclaude/v2.1.12/todo-write/scenario.toml`: `simulated` → `live`

### Phase 7: Update Documentation

**`docs/USAGE.md`**
- Rename `--tool-execution-mode` → `--tool-mode` in table
- Rename env var `CLAUDELESS_TOOL_EXECUTION_MODE` → `CLAUDELESS_TOOL_MODE`
- Remove `--sandbox-root`, `--allow-real-bash` rows
- Update modes table: `simulated` → `live`, remove `real-mcp`
- Remove sandboxed execution example

**`docs/SCENARIOS.md`**
- Update Tool Execution section (lines 296-327)
- Update modes: `simulated` → `live`, remove `real_mcp`
- Remove `sandbox_root`, `allow_real_bash` from config example

**`docs/LIMITATIONS.md`**
- Remove "MCP Server Execution" from future work

**`scenarios/full-featured.toml`**
- Change `mode = "simulated"` → `mode = "live"`
- Remove `sandbox_root` line

## Verification

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

Manual verification:
```bash
claudeless --tool-mode live -p "echo hello"
# Should execute real bash and print "hello"
```

## Files Changed

| File | Action |
|------|--------|
| `crates/cli/src/config.rs` | Edit |
| `crates/cli/src/cli.rs` | Edit |
| `crates/cli/src/tools/executor.rs` | Edit |
| `crates/cli/src/tools/builtin/mod.rs` | Edit |
| `crates/cli/src/tools/builtin/bash.rs` | Edit |
| `crates/cli/src/tools/builtin/read.rs` | Edit |
| `crates/cli/src/tools/builtin/write.rs` | Edit |
| `crates/cli/src/tools/builtin/edit.rs` | Edit |
| `crates/cli/src/tools/builtin/glob.rs` | Edit |
| `crates/cli/src/tools/builtin/grep.rs` | Edit |
| `crates/cli/src/tools/mod.rs` | Edit |
| `crates/cli/src/tools/mcp/*` | Delete |
| `crates/cli/src/main.rs` | Edit |
| `crates/cli/src/config_tests.rs` | Edit |
| `crates/cli/src/tools/executor_tests.rs` | Edit |
| `crates/cli/src/tools/builtin/mod_tests.rs` | Edit |
| `crates/cli/src/tools/builtin/bash_tests.rs` | Edit |
| `crates/cli/tests/scenario_fields.rs` | Edit |
| `crates/cli/tests/dot_claude_plans.rs` | Edit |
| `crates/cli/tests/dot_claude_todos.rs` | Edit |
| `crates/cli/tests/fixtures/*/scenario.toml` | Edit |
| `docs/USAGE.md` | Edit |
| `docs/SCENARIOS.md` | Edit |
| `docs/LIMITATIONS.md` | Edit |
| `scenarios/full-featured.toml` | Edit |
