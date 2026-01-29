# Test Fixtures Report

This document describes all test fixtures in the claudeless codebase, their locations, contents, and how they're used in tests.

## Directory Structure

```
tests/fixtures/                          # Workspace-level fixtures (MCP)
├── echo_mcp_server.py                   # Python MCP test server
├── mcp-test-init.toml                   # MCP initialization scenario
├── mcp-test-list.toml                   # MCP list operation scenario
├── mcp-test-read.toml                   # MCP read (mock) scenario
├── mcp-test-read-raw.toml               # MCP raw read scenario
├── mcp-test-read-live.toml              # MCP live read scenario
└── mcp-test-write.toml                  # MCP write operation scenario

crates/cli/tests/fixtures/               # CLI-specific fixtures
├── cli/v2.1.12/                         # CLI output fixtures
│   ├── json-output/                     # JSON output format tests
│   │   ├── scenario.toml
│   │   └── output.json
│   └── stream-json/                     # Stream-JSON format tests
│       ├── scenario.toml
│       └── output.jsonl
├── dotclaude/v2.1.12/                   # Claude state directory fixtures
│   ├── sessions-index.json
│   ├── todo-write/                      # Todo creation scenario
│   │   ├── scenario.toml
│   │   ├── todo.json
│   │   └── session.jsonl
│   └── plan-mode/                       # Plan mode scenario
│       ├── scenario.toml
│       ├── plan.md
│       └── session.jsonl
└── tui/                                 # TUI snapshot fixtures
    ├── v2.1.12/                         # 73 snapshot files
    ├── v2.1.14/                         # 17 setup/login snapshots
    ├── v2.1.15/                         # 5 ANSI permission snapshots
    └── v2.1.17/                         # 2 shell mode ANSI snapshots
```

---

## 1. MCP Test Fixtures

**Location:** `tests/fixtures/`

### echo_mcp_server.py

| Attribute | Value |
|-----------|-------|
| **Test Files** | `crates/cli/src/mcp/client_tests.rs`, `crates/cli/src/mcp/server_tests.rs` |
| **Type** | Python script |
| **Purpose** | Implements MCP protocol (JSON-RPC) for testing |

**Contents:**
- Implements `initialize`, `tools/list`, `tools/call` methods
- Tools: `echo` (returns input), `fail` (always errors)

### mcp-test-*.toml

| File | Test Files | Purpose |
|------|------------|---------|
| `mcp-test-init.toml` | `mcp_scenarios.rs` | Tests MCP tool listing (`mcp__<server>__<tool>` format) |
| `mcp-test-list.toml` | `mcp_scenarios.rs` | Tests listing MCP tools and directories |
| `mcp-test-read.toml` | `mcp_scenarios.rs` | Tests mock file reading via MCP |
| `mcp-test-read-raw.toml` | `mcp_scenarios.rs` | Tests raw file content (not normalized) |
| `mcp-test-read-live.toml` | `mcp_scenarios.rs` | Tests real file reading via MCP server |
| `mcp-test-write.toml` | `mcp_scenarios.rs` | Tests file writing via MCP |

---

## 2. CLI Output Fixtures

**Location:** `crates/cli/tests/fixtures/cli/v2.1.12/`

### json-output/

| Attribute | Value |
|-----------|-------|
| **Test Files** | `cli_fixtures.rs` |
| **Type** | Behavior comparison |
| **ANSI** | No |

**Files:**
- `scenario.toml` - Test scenario configuration with pattern-matching responses
- `output.json` - Expected JSON output with placeholders (`<SESSION_ID>`, `<DURATION>`, `<COST>`, etc.)

**Tests:**
- `test_json_output_matches_fixture()` - Compares JSON structure with normalization
- `test_json_output_has_fixture_fields()` - Verifies all expected fields present

### stream-json/

| Attribute | Value |
|-----------|-------|
| **Test Files** | `cli_fixtures.rs` |
| **Type** | Behavior comparison |
| **ANSI** | No |

**Files:**
- `scenario.toml` - Test scenario for streaming JSON output
- `output.jsonl` - Expected NDJSON event sequence

**Tests:**
- `test_stream_json_event_types_match_fixture()` - Compares event sequences
- `test_stream_json_is_valid_ndjson()` - Validates NDJSON format
- `test_stream_json_starts_with_system_init()` - Checks initial event
- `test_stream_json_ends_with_result()` - Checks final event

---

## 3. Claude State Directory Fixtures

**Location:** `crates/cli/tests/fixtures/dotclaude/v2.1.12/`

### todo-write/

| Attribute | Value |
|-----------|-------|
| **Test Files** | `dot_claude_todos.rs` |
| **Type** | State/dotfile comparison |
| **ANSI** | No |

**Files:**
- `scenario.toml` - Scenario with `TodoWrite` tool, loads todos from `todo.json`
- `todo.json` - JSON array with `content`, `status`, `activeForm` fields
- `session.jsonl` - Session transcript (normalized UUIDs, timestamps, message IDs)

**Tests:**
- `test_todos_directory_created()` - Verifies `todos/` directory creation
- `test_todo_file_naming_convention()` - Checks `{sessionId}-agent-{sessionId}.json` format
- `test_todo_file_content_structure()` - Validates JSON array with required fields
- `test_empty_todo_list_format()` - Verifies empty todos are `[]`
- `test_todo_json_matches_fixture()` - Compares against fixture with normalization

### plan-mode/

| Attribute | Value |
|-----------|-------|
| **Test Files** | `dot_claude_plans.rs` |
| **Type** | State/dotfile comparison |
| **ANSI** | No |

**Files:**
- `scenario.toml` - Scenario with `ExitPlanMode` tool
- `plan.md` - Markdown plan file (naming: `{adjective}-{verb}-{noun}.md`)
- `session.jsonl` - Session transcript for plan mode

**Tests:**
- `test_plan_file_naming_convention()` - Validates `{adj}-{verb}-{noun}.md` format
- `test_plan_file_is_markdown()` - Checks markdown format with headings
- `test_plan_md_matches_fixture_content()` - Exact content match
- `test_plan_mode_session_jsonl_matches_fixture()` - Session message sequence match

### sessions-index.json

| Attribute | Value |
|-----------|-------|
| **Test Files** | `dot_claude_projects.rs` |
| **Type** | State/dotfile comparison |
| **ANSI** | No |

**Contents:**
JSON with `version=1` and `entries` array. Entry fields: `sessionId`, `fullPath`, `fileMtime`, `firstPrompt`, `messageCount`, `created`, `modified`, `gitBranch`, `projectPath`, `isSidechain`

**Tests:**
- `test_sessions_index_created()` - Verifies file creation
- `test_sessions_index_matches_fixture()` - Compares structure against fixture

---

## 4. TUI Snapshot Fixtures

**Location:** `crates/cli/tests/fixtures/tui/`

TUI snapshots capture terminal state for visual regression testing. Most tests use normalized snapshots (no ANSI), while `*_ansi.txt` files contain ANSI color codes for color-specific tests.

### v2.1.12 (Default Version) - 73 Files

| Attribute | Value |
|-----------|-------|
| **Test Files** | 20+ test files: `tui_clear.rs`, `tui_compacting.rs`, `tui_fork.rs`, `tui_interaction.rs`, `tui_permission.rs`, `tui_shell_mode.rs`, `tui_shortcuts.rs`, `tui_snapshot.rs`, `tui_tasks.rs`, `tui_thinking.rs`, `tui_trust.rs`, `tui_context.rs`, `tui_exit.rs`, `tui_help.rs`, `tui_hooks.rs`, `tui_memory.rs`, `tui_model.rs`, `tui_responsive.rs`, `tui_setup.rs`, `tui_stash.rs`, `tui_suspend.rs`, `tui_todos.rs` |
| **Type** | TUI capture |
| **ANSI** | Separate `*_ansi.txt` files where needed |

**Categories:**

#### Core State Snapshots
| File | Purpose |
|------|---------|
| `initial_state.txt` | TUI startup state |
| `initial_state_ansi.txt` | Startup with ANSI codes |
| `after_response.txt` | After assistant response |
| `with_input.txt` | With user input |

#### Clear/Compact Operations
| File | Purpose |
|------|---------|
| `clear_before.txt` | Before /clear |
| `clear_after.txt` | After /clear |
| `compact_before.txt` | Before compaction |
| `compact_during.txt` | During compaction |
| `compact_after.txt` | After compaction |

#### Permission Dialogs
| File | Purpose |
|------|---------|
| `permission_default.txt` | Default permission mode |
| `permission_plan.txt` | Plan mode |
| `permission_bash_command.txt` | Bash permission prompt |
| `permission_edit_file.txt` | Edit permission prompt |
| `permission_write_file.txt` | Write permission prompt |
| `permission_trust_folder.txt` | Trust folder prompt |
| `permission_bypass.txt` | Bypass mode |
| `permission_accept_edits.txt` | Accept edits |

#### Model Selection
| File | Purpose |
|------|---------|
| `model_haiku.txt` | Haiku selected |
| `model_sonnet.txt` | Sonnet selected |
| `model_opus.txt` | Opus selected |
| `model_picker.txt` | Model selection dialog |

#### Thinking Dialog
| File | Purpose |
|------|---------|
| `thinking_dialog.txt` | Extended thinking dialog |
| `thinking_dialog_enabled_selected.txt` | Enabled state |
| `thinking_dialog_disabled_selected.txt` | Disabled state |
| `thinking_dialog_mid_conversation.txt` | Mid-conversation toggle |
| `thinking_off_status.txt` | Thinking disabled status |

#### Tasks/Hooks/Shell
| File | Purpose |
|------|---------|
| `tasks_empty_dialog.txt` | Empty task list |
| `trust_prompt.txt` | Trust prompt |
| `hooks_dialog.txt` | Hooks configuration |
| `hooks_matcher_dialog.txt` | Hook matcher |
| `shell_mode_prefix.txt` | Shell mode prefix |
| `shell_mode_command.txt` | Shell mode command |

#### Slash Commands/Autocomplete
| File | Purpose |
|------|---------|
| `context_autocomplete.txt` | /context autocomplete |
| `context_usage.txt` | /context usage display |
| `exit_autocomplete.txt` | /exit autocomplete |
| `export_autocomplete.txt` | /export autocomplete |
| `export_filename_dialog.txt` | Export filename prompt |
| `export_method_dialog.txt` | Export method selection |
| `help_autocomplete.txt` | /help autocomplete |
| `help_general_tab.txt` | Help general tab |
| `help_commands_tab.txt` | Help commands tab |
| `hooks_autocomplete.txt` | /hooks autocomplete |
| `slash_search_filter.txt` | /search filter |
| `slash_search_menu.txt` | /search menu |
| `slash_search_tab_complete.txt` | /search tab completion |

#### Exit/Suspend Hints
| File | Purpose |
|------|---------|
| `ctrl_c_exit_hint.txt` | Ctrl+C hint |
| `ctrl_d_exit_hint.txt` | Ctrl+D hint |
| `escape_clear_hint.txt` | Escape hint |
| `ctrl_z_suspend.txt` | Ctrl+Z suspend |
| `ctrl_s_stash_active.txt` | Ctrl+S stash |

#### Other
| File | Purpose |
|------|---------|
| `fork_no_conversation.txt` | Fork without conversation |
| `shortcuts_display.txt` | /shortcuts display |
| `status_bar_extended.txt` | Extended status bar |
| `todos_empty.txt` | Empty todos |

### v2.1.14 - 17 Files (Setup/Login)

| Attribute | Value |
|-----------|-------|
| **Test Files** | `tui_setup.rs` |
| **Type** | TUI capture |
| **ANSI** | No |

| File | Purpose |
|------|---------|
| `setup_01_select_theme_dark.txt` | Theme selection (dark) |
| `setup_01_select_theme_light.txt` | Theme selection (light) |
| `setup_01a_syntax_highlighting_disabled.txt` | Syntax highlighting option |
| `setup_02_login_method.txt` | Login method selection |
| `setup_03_login_browser.txt` | Browser login flow |
| `setup_03_security_notes.txt` | Security notes |
| `setup_04_login_success.txt` | Login success |
| `setup_05_use_terminal_setup.txt` | Terminal setup completion |
| `failed_to_open_socket.txt` | Socket error (with version) |
| `failed_to_open_socket_no_version.txt` | Socket error (no version) |
| `slash_logout.txt` | /logout command |
| `api_usage_billing.txt` | API usage/billing display |
| `initial_state.txt` | Initial state v2.1.14 |

### v2.1.15 - 5 Files (ANSI Permission Dialogs)

| Attribute | Value |
|-----------|-------|
| **Test Files** | `tui_permission.rs` |
| **Type** | TUI capture |
| **ANSI** | Yes |

| File | Purpose |
|------|---------|
| `permission_bash_command_ansi.txt` | Bash permission with ANSI |
| `permission_edit_file_ansi.txt` | Edit permission with ANSI |
| `permission_write_file_ansi.txt` | Write permission with ANSI |
| `permission_trust_folder_ansi.txt` | Trust folder with ANSI |
| `permission_default_ansi.txt` | Default permission with ANSI |

### v2.1.17 - 2 Files (Shell Mode ANSI)

| Attribute | Value |
|-----------|-------|
| **Test Files** | `tui_shell_mode.rs` |
| **Type** | TUI capture |
| **ANSI** | Yes |

| File | Purpose |
|------|---------|
| `shell_mode_prefix_ansi.txt` | Shell mode prefix with ANSI |
| `shell_mode_command_ansi.txt` | Shell mode command with ANSI |

---

## Normalization

All fixtures use normalization before comparison to ensure deterministic tests:

| Field | Placeholder |
|-------|-------------|
| Timestamps (HH:MM:SS) | `<TIME>` |
| UUIDs | `<SESSION>` or `<UUID>` |
| Temp/working paths | `<PATH>` |
| Version strings | `<VERSION>` |
| Model names | `<MODEL>` |
| Session IDs | `<SESSION_ID>` |
| Duration | `<DURATION>` |
| Cost | `<COST>` |
| Message IDs | `<MESSAGE_ID>` |
| Request IDs | `<REQUEST_ID>` |
| Tool Use IDs | `<TOOL_USE_ID>` |
| Timestamps | `<TIMESTAMP>` |

Additional normalization:
- Non-breaking spaces → regular spaces
- Strip trailing whitespace per line
- Strip leading/trailing empty lines

---

## Summary Table

| Fixture Group | Location | Files | Test Files | Type |
|---------------|----------|-------|------------|------|
| MCP Server | `tests/fixtures/` | 1 | `mcp/client_tests.rs`, `mcp/server_tests.rs` | Python script |
| MCP Scenarios | `tests/fixtures/` | 6 | `mcp_scenarios.rs` | TOML scenarios |
| CLI JSON Output | `cli/v2.1.12/json-output/` | 2 | `cli_fixtures.rs` | Behavior |
| CLI Stream-JSON | `cli/v2.1.12/stream-json/` | 2 | `cli_fixtures.rs` | Behavior |
| State: Todos | `dotclaude/v2.1.12/todo-write/` | 3 | `dot_claude_todos.rs` | State |
| State: Plans | `dotclaude/v2.1.12/plan-mode/` | 3 | `dot_claude_plans.rs` | State |
| State: Index | `dotclaude/v2.1.12/` | 1 | `dot_claude_projects.rs` | State |
| TUI v2.1.12 | `tui/v2.1.12/` | 73 | 20+ tui_*.rs | TUI capture |
| TUI v2.1.14 | `tui/v2.1.14/` | 17 | `tui_setup.rs` | TUI capture |
| TUI v2.1.15 | `tui/v2.1.15/` | 5 | `tui_permission.rs` | TUI capture (ANSI) |
| TUI v2.1.17 | `tui/v2.1.17/` | 2 | `tui_shell_mode.rs` | TUI capture (ANSI) |
