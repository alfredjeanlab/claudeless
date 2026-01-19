# Claudeless Limitations

Claudeless simulates the Claude CLI for testing by simulating:

- TUI text layout
- CLI interface
- error messages
- `~/.claude` filesystem modifications

The state directory location is controlled by `CLAUDELESS_STATE_DIR`.
Simulated failures can be injected via scenarios.
Any costs, tokens, and timing are simulated and no API calls are made.

## Simulation

Any costs, tokens, and timing values are deterministic to enable predictable test assertions.

- **State Directory**: Uses temp directory by default
- **Real API Costs**: `cost_usd` always 0 (simulator makes no API calls)
- **Token Counts**: Estimated (~4 chars/token), not actual tokenization
- **Timing**: `duration_ms` values simulated

## Supported Behavior

Built-in tools execute in a sandbox and scenarios control responses.

- **CLI Flags**: 23 flags fully implemented (see `claude -h` for comparison)
- **Output Formats**: text, json, stream-json with result wrapper
- **Hook Protocol**: All 7 events (pre/post tool execution, notification, permission, session start/end, prompt submit)
- **Permission Modes**: All 6 modes (default, acceptEdits, bypassPermissions, delegate, dontAsk, plan)
- **MCP Config**: JSON/JSON5 parsing, tool registration, `--mcp-config` / `--strict-mcp-config` / `--mcp-debug`
- **State Directory**: projects, todos, plans, sessions, settings.json
- **Built-in Tools**: Bash, Read, Write, Edit, Glob, Grep, TodoWrite, ExitPlanMode (sandboxed)
- **Scenario System**: Pattern matching, multi-turn conversations, failure injection, mock responses

## Out of Scope

- **Chrome Integration**: Browser features not simulated
- **IDE Integration**: `--ide` flag not supported
- **Settings**: Status, Config, Usage screens
- **Colors**: TUI colors may not exactly match, however text outputs are expected to match exactly
- **Subcommands**: `doctor`, `install`, `mcp`, `plugin`, `setup-token`, `update`

## Future Work

- **MCP Resources/Prompts**: Only tools protocol supported; resources and prompts not implemented
- **Slash Commands**: Wider `/command` support
- **Unimplemented CLI Flags**: See CLI Flags section below
- **Tools**: `WebSearch`, `WebFetch`, `NotebookEdit`, `AskUserQuestion`, `EnterPlanMode`, `Task`, `KillShell`, `TaskOutput`, `Skill`
- **Subagents**: Agent spawning and management

## Known TODOs

Known divergences with failing tests.  
Run: `cargo test -- --ignored`

- [ ] **JSON output format**: `usage` and `modelUsage` fields empty
  - `test_json_output_uses_result_wrapper_format`
  - `test_json_output_result_contains_response_text`
- [ ] **TUI /compact fixture matching**: Fixture tests need tool_calls recorded in session
  - `test_compact_before_matches_fixture`
  - `test_compact_during_matches_fixture`
  - `test_compact_after_matches_fixture`
- [ ] **TUI permission dialogs**: Rich dialogs for bash/edit/write/trust not implemented
  - `test_permission_bash_command_matches_fixture`
  - `test_permission_edit_file_matches_fixture`
  - `test_permission_write_file_matches_fixture`
  - `test_permission_trust_folder_matches_fixture`
  - `test_status_bar_extended_matches_fixture`
- [ ] **TUI thinking dialog**: Extra separator lines in rendering
  - `test_thinking_dialog_enabled_selected_matches_fixture`
  - `test_thinking_dialog_disabled_selected_matches_fixture`
  - `test_thinking_off_status_matches_fixture`
  - `test_thinking_dialog_mid_conversation_matches_fixture`
- [ ] **TUI status bar**: Visibility differs during input
  - `test_input_display_matches_fixture`

---

## CLI Flags

### Partial

| Flag | Issue |
|------|-------|
| `--mcp-config` | Config parsing only, no server execution |

### Not Implemented

| Flag | Notes |
|------|-------|
| `--add-dir` | Additional directories |
| `--agent` | Custom agent |
| `--agents` | Agent definitions JSON |
| `--append-system-prompt` | Append to system prompt |
| `--betas` | Beta headers |
| `--disable-slash-commands` | Disable skills |
| `--file` | File resources |
| `--fork-session` | Fork session on resume |
| `--json-schema` | Structured output schema |
| `--no-session-persistence` | Disable session persistence |
| `--plugin-dir` | Plugin directories |
| `--replay-user-messages` | Replay user messages |
| `--setting-sources` | Settings sources |
| `--settings` | Settings file/JSON |
| `--tools` | Built-in tool list |

---

## Output Format Divergences

### JSON Output (`--output-format json`)

| Field | Real Claude | Claudeless |
|-------|-------------|------------|
| `usage` | Rich cache/server metrics | Empty `{}` |
| `modelUsage` | Per-model detailed metrics | Empty `{}` |
| `total_cost_usd` | Actual API cost | Always `0` |
| `duration_ms` | Actual timing | Simulated |

### Stream-JSON System Init Event

Missing fields:
- `agents`, `apiKeySource`, `claude_code_version`, `cwd`
- `output_style`, `permissionMode`, `plugins`, `skills`, `slash_commands`

---

## MCP

| Feature | Status |
|---------|--------|
| Actual server execution | Not implemented (returns stub) |
| Dynamic tool discovery | Not implemented (manual registration) |
| Server health checks | Not implemented (always "running") |
