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

- **CLI Flags**: See `claude -h` for comparison with real CLI
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
- **Slash Commands**: Basic commands implemented (`/clear`, `/compact`, `/fork`, `/help`, `/context`, `/exit`, `/todos`, `/tasks`, `/export`, `/hooks`, `/memory`); wider support needed
- **Unimplemented CLI Flags**: See CLI Flags section below
- **Tools**: `WebSearch`, `WebFetch`, `NotebookEdit`, `AskUserQuestion`, `EnterPlanMode`, `Task`, `KillShell`, `TaskOutput`, `Skill`
- **Subagents**: Agent spawning and management
- **TUI Setup Flow**: Theme selection, login flow, logout command, connection error handling

## Known TODOs

Known divergences with failing tests.
Run: `cargo test -- --ignored`

- [ ] **TUI permission dialogs**: Rich dialogs and ANSI styling not implemented
  - `test_permission_bash_command_matches_fixture`
  - `test_permission_edit_file_matches_fixture`
  - `test_permission_write_file_matches_fixture`
  - `test_permission_trust_folder_matches_fixture`
  - `test_status_bar_extended_matches_fixture`
  - `test_permission_plan_matches_fixture`
  - `test_permission_plan_ansi_matches_fixture`
  - `test_permission_accept_edits_ansi_matches_fixture`
  - `test_permission_bypass_ansi_matches_fixture`
- [ ] **TUI setup flow**: Theme selection, login, logout, connection errors
  - `test_tui_setup_theme_selection_dark_mode_default`
  - `test_tui_setup_theme_selection_light_mode`
  - `test_tui_setup_theme_selection_ansi_mode`
  - `test_tui_setup_theme_ctrl_t_toggles_syntax_highlighting`
  - `test_tui_setup_login_method_shows_options`
  - `test_tui_setup_full_login_flow_to_initial_state`
  - `test_tui_slash_logout_exits_to_shell`
  - `test_tui_failed_to_open_socket_exits`
  - `test_tui_failed_to_open_socket_shows_helpful_message`
- [ ] **CLI flags**: Missing flags for full compatibility
  - `test_stream_json_starts_with_system_init`
  - `test_add_dir_flag_should_be_accepted`
  - `test_json_schema_flag_should_be_accepted`
  - `test_tools_flag_should_be_accepted`
  - `test_agent_flag_should_be_accepted`
  - `test_append_system_prompt_flag_should_be_accepted`
- [ ] **TUI thinking dialog**: Extra separator lines in rendering
  - `test_thinking_dialog_matches_fixture`
  - `test_thinking_dialog_enabled_selected_matches_fixture`
  - `test_thinking_dialog_disabled_selected_matches_fixture`
  - `test_thinking_off_status_matches_fixture`
  - `test_thinking_dialog_mid_conversation_matches_fixture`
- [ ] **TUI /compact fixture matching**: Fixture tests need tool_calls recorded in session
  - `test_compact_before_matches_fixture`
  - `test_compact_during_matches_fixture`
  - `test_compact_after_matches_fixture`
- [ ] **TUI interaction**: Status bar visibility and input handling
  - `test_input_display_matches_fixture`
  - `test_tui_ctrl_underscore_undoes_last_word`
  - `test_tui_ctrl_underscore_clears_all_input`
- [ ] **TUI trust dialog**: Trust prompt handling
  - `test_trust_prompt_escape_cancels`
  - `test_trust_prompt_matches_fixture`
- [ ] **TUI tasks view**: Empty state rendering
  - `test_tasks_empty_matches_fixture`
- [ ] **TUI shell mode**: Bash mode pink styling
  - `test_tui_shell_prefix_ansi_matches_fixture_v2117`
- [ ] **TUI export**: Slash command autocomplete
  - `test_tui_export_command_shows_autocomplete`

---

## CLI Flags

### Partial → Full

| Flag | Status |
|------|--------|
| `--mcp-config` | ✓ Full: config parsing + server execution |

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
| `usage` | Rich cache/server metrics | Simulated token-based usage |
| `modelUsage` | Per-model detailed metrics | Simulated per-model usage |
| `cost_usd` | Actual API cost | Simulated (~$3/M in, $15/M out) |
| `duration_ms` | Actual timing | Simulated |

### Stream-JSON System Init Event

Missing fields:
- `agents`, `apiKeySource`, `claude_code_version`, `cwd`
- `output_style`, `permissionMode`, `plugins`, `skills`, `slash_commands`

---

## MCP

| Feature | Status |
|---------|--------|
| Server spawning | ✓ Implemented via `McpClient::connect()` |
| Protocol initialization | ✓ Implemented (`initialize` + `notifications/initialized`) |
| Dynamic tool discovery | ✓ Implemented via `tools/list` |
| Tool execution | ✓ Implemented via `tools/call` |
| Graceful shutdown | ✓ Implemented with timeout and force-kill |
| Multi-server management | ✓ Implemented via `McpManager` |
| Tool routing | ✓ Implemented (tool → server mapping) |
| Resources protocol | Not implemented (tools only) |
| Prompts protocol | Not implemented (tools only) |
| Server health checks | Basic (process exit detection) |

### MCP Configuration

Supported config formats:
- JSON (`.json`)
- JSON5 (`.json5`) with comments

Supported flags:
- `--mcp-config <path>` - Load MCP config file
- `--strict-mcp-config` - Fail on invalid config
- `--mcp-debug` - Enable MCP debug output
