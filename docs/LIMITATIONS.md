# Claudeless Limitations

Claudeless simulates the Claude CLI for testing by simulating:

- TUI text layout
- CLI interface
- error messages
- `~/.claude` filesystem modifications

The state directory location is controlled by `CLAUDELESS_CONFIG_DIR` (highest priority) or `CLAUDE_CONFIG_DIR`.
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
- **Hook Protocol**: All 9 events (pre/post tool execution, notification, permission, session start/end, prompt submit, pre-compaction, stop)
- **Permission Modes**: All 6 modes (default, acceptEdits, bypassPermissions, delegate, dontAsk, plan)
- **MCP Config**: JSON/JSON5 parsing, tool registration, `--mcp-config` / `--strict-mcp-config` / `--mcp-debug`
- **State Directory**: projects, todos, plans, sessions, settings.json
- **Built-in Tools**: Bash, Read, Write, Edit, Glob, Grep, TodoWrite, ExitPlanMode, AskUserQuestion (sandboxed, with TUI elicitation dialog)
- **Scenario System**: Pattern matching, multi-turn conversations, failure injection, mock responses
- **Slash Commands**: 24 commands in menu with fuzzy search filtering (`/clear`, `/compact`, `/config`, `/context`, `/exit`, `/export`, `/fork`, `/help`, `/hooks`, `/init`, `/login`, `/logout`, `/mcp`, `/memory`, `/model`, `/permissions`, `/plan`, `/pr-comments`, `/review`, `/status`, `/tasks`, `/terminal-setup`, `/todos`, `/vim`)
- **ANSI Colors**: Logo, header, separators, status bar, permission mode indicators, bash mode styling

## Out of Scope

- **Chrome Integration**: Browser features not simulated
- **IDE Integration**: `--ide` flag not supported
- **Settings Screens**: Status, Config, Usage screens
- **Subcommands**: `doctor`, `install`, `mcp`, `plugin`, `setup-token`, `update`

## Future Work

- **MCP Resources/Prompts**: Only tools protocol supported; resources and prompts not implemented
- **Tools**: `WebSearch`, `WebFetch`, `NotebookEdit`, `EnterPlanMode`, `Task`, `KillShell`, `TaskOutput`, `Skill`
- **Subagents**: Agent spawning and management
- **TUI Setup Flow**: Theme selection, login flow, logout command, connection error handling
- **Stream-JSON System Init**: Missing fields (`agents`, `apiKeySource`, `claude_code_version`, `cwd`, `output_style`, `permissionMode`, `plugins`, `skills`, `slash_commands`)

## Known TODOs

Known divergences with ignored tests.
Run: `cargo test -- --ignored`

- [ ] **TUI setup flow** (9 tests): Theme selection, login, logout, connection errors
  - `test_tui_setup_theme_selection_dark_mode_default`
  - `test_tui_setup_theme_selection_light_mode`
  - `test_tui_setup_theme_selection_ansi_mode`
  - `test_tui_setup_theme_ctrl_t_toggles_syntax_highlighting`
  - `test_tui_setup_login_method_shows_options`
  - `test_tui_setup_full_login_flow_to_initial_state`
  - `test_tui_slash_logout_exits_to_shell`
  - `test_tui_failed_to_open_socket_exits`
  - `test_tui_failed_to_open_socket_shows_helpful_message`
- [ ] **Flaky TUI tests** (9 tests): Timing-sensitive tmux tests that fail intermittently on CI
  - `test_tui_model_picker_shows_available_models` (picker render timing)
  - `test_tui_ctrl_z_shows_keybinding_note` (tmux timing)
  - `test_fork_success_with_conversation` (tmux timing)
  - `test_tui_export_command_shows_autocomplete` (tmux timing)
  - `test_tui_export_arrow_navigation` (tmux timing)
  - `test_tui_export_clipboard_shows_confirmation` (tmux timing)
  - `test_tui_export_file_shows_filename_dialog` (tmux timing)
  - `test_tui_export_filename_escape_returns_to_method` (tmux timing)
  - `test_tui_slash_tab_closes_menu` (tmux timing)
- [ ] **Stream-JSON output** (3 tests): System init event and `-p` verbose mode
  - `test_stream_json_starts_with_system_init`
  - `test_stream_json_print_requires_verbose`
  - `test_init_event_has_extended_fields`
- [ ] **TUI interaction** (2 tests): Ctrl+_ undo (tmux cannot send Ctrl+_; unit tests verify behavior)
  - `test_tui_ctrl_underscore_undoes_last_word`
  - `test_tui_ctrl_underscore_clears_all_input`
- [ ] **TUI trust dialog** (1 test)
  - `test_trust_prompt_escape_cancels`

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
