# Skipped Scripts

Scripts in this directory capture states that we are still working on reproducing reliably.

## Challenges

### after-response.capsh

Captures UI state after Claude responds. Challenges:

- Need Claude to actually respond (requires API)
- Response content varies
- Timing is unpredictable

### compact-states.capsh

Captures context compaction states. Challenges:

- Need enough context to trigger compaction
- Compaction timing is model-dependent
- Requires multiple interactions

### setup-*.capsh & slash-logout.capsh

Captures the initial setup flow and logout states. Challenges:

- Require fresh/unconfigured Claude installation (no existing login)
- Setup flow only appears once per account configuration
- slash_logout requires an active session to log out from
- Would need to mock or reset configuration between captures
- Complex to automate in CI without affecting real user config

These fixtures exist in `crates/cli/tests/fixtures/tui/v2.1.14/`:
- `setup_01_select_theme_dark.txt`
- `setup_01_select_theme_light.txt`
- `setup_01a_syntax_highlighting_disabled.txt`
- `setup_02_login_method.txt`
- `setup_03_login_browser.txt`
- `setup_03_security_notes.txt`
- `setup_04_login_success.txt`
- `setup_05_use_terminal_setup.txt`
- `slash_logout.txt`

### failed_to_open_socket / failed_to_open_socket_no_version

Error states when Claude Code cannot connect to the API. Challenges:

- Requires simulating network failure (firewall block, DNS failure, etc.)
- Error conditions are transient and environment-dependent
- Can't reliably trigger FailedToOpenSocket errors in CI

Note: Reference fixtures exist in `crates/cli/tests/fixtures/tui/v2.1.14/` but were captured manually, not via capsh scripts.

### Permission Dialogs (permission_*.txt)

Captures permission request dialogs. These fixtures exist in `fixtures/tui/v2.1.12/` but cannot be reliably captured by automation because they require Claude to use tools that trigger the permission system.

**Fixtures:**
- `permission_bash_command.txt` - Bash command execution prompt
- `permission_write_file.txt` - New file creation prompt
- `permission_edit_file.txt` - File edit prompt with diff view
- `permission_accept_edits.txt` - Status bar showing "accept edits on"
- `permission_bypass.txt` - Status bar showing "bypass permissions on"
- `permission_plan.txt` - Status bar showing "plan mode on"
- `permission_trust_folder.txt` - Folder trust dialog (without header)

**How to trigger manually:**
1. **Bash command**: Ask Claude to run a shell command (e.g., "run ls -la")
2. **Write file**: Ask Claude to create a new file (e.g., "create hello.txt with 'Hello World'")
3. **Edit file**: Ask Claude to modify an existing file
4. **Accept edits mode**: Press Shift+Tab to cycle to "accept edits" mode
5. **Bypass permissions**: Press Shift+Tab to cycle to "bypass permissions" mode
6. **Plan mode**: Press Shift+Tab to cycle to "plan mode"

**Challenges:**
- Requires Claude API to generate tool calls
- Tool call content varies based on prompt
- Timing is unpredictable

### trust_prompt.txt

Captures the folder trust dialog shown when entering an untrusted directory.

**How to trigger:**
```bash
# Create temp directory and run Claude in it
cd $(mktemp -d)
claude --model haiku
# Trust dialog appears immediately
```

**Challenges:**
- Requires running in an untrusted folder
- Dialog appears before normal TUI is ready
- Dismissing creates trust entry, preventing re-capture

## Simulator Fixture Differences

These fixture tests are ignored because the simulator renders differently than the real CLI:

### compact_before.txt / compact_during.txt / compact_after.txt

The simulator has these differences from real Claude CLI:

1. **Header format**: Simulator shows `Claudeless 0.1.0` with logo, real CLI shows `Claude Code v2.1.12`
2. **Tool output format**: Real CLI shows `⏺ Read(Cargo.toml)` as a distinct block, simulator shows response inline
3. **Compaction summary**: Real CLI shows `Read Cargo.toml (14 lines)` in summary, simulator doesn't track tool calls

To fix these tests:
1. Update `normalize_tui()` to strip the simulator header, OR
2. Match the simulator's conversation display format to real CLI (significant work), OR
3. Regenerate fixtures from simulator output and accept differences

The `compact_during.txt` test specifically requires capturing the transient "Compacting..." spinner state,
which is timing-sensitive and may need retries in CI.

## Running Skipped Scripts

```bash
# Run all including skipped (may fail)
RUN_SKIPPED=1 ./capture.sh

# Run single skipped script manually
capsh --frames /tmp/skipped -- claude < skipped/permission-dialogs.capsh
```

## Contributing

If you find a reliable way to capture these states:

1. Update the script with your approach
2. Move to `capsh/` if it now works consistently
3. Update this README
