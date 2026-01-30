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
