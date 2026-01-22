# Claude TUI Snapshots (v2.1.15)

Captured from real Claude CLI for comparison testing.

**Behavior observed with:** claude --version 2.1.15 (Claude Code)

## Permission Mode ANSI Fixtures

These fixtures capture the permission mode indicators WITH ANSI color codes preserved.
Used for testing color rendering in claudeless matches real Claude Code.

### permission_default_ansi.txt
Default permission mode with ANSI escape sequences:
- Status bar shows "? for shortcuts" in gray `(153, 153, 153)`
- This is the initial state when claude starts

### permission_plan_ansi.txt
Plan mode with ANSI escape sequences:
- Status bar shows "⏸ plan mode on (shift+tab to cycle)"
- Plan mode icon "⏸" and text "plan mode on" in teal `(72, 150, 140)`
- Cycle hint "(shift+tab to cycle)" in gray `(153, 153, 153)`

### permission_accept_edits_ansi.txt
Accept edits mode with ANSI escape sequences:
- Status bar shows "⏵⏵ accept edits on (shift+tab to cycle)"
- Accept edits icon "⏵⏵" and text "accept edits on" in purple `(175, 135, 255)`
- Cycle hint "(shift+tab to cycle)" in gray `(153, 153, 153)`

### permission_bypass_ansi.txt
Bypass permissions mode with ANSI escape sequences:
- Status bar shows "⏵⏵ bypass permissions on (shift+tab to cycle)"
- Bypass icon "⏵⏵" and text "bypass permissions on" in red/pink `(255, 107, 128)`
- Cycle hint "(shift+tab to cycle)" in gray `(153, 153, 153)`
- Only available when started with `--dangerously-skip-permissions`

## Common ANSI Colors

Key colors observed across all permission mode fixtures:
- **Orange** `(215, 119, 87)`: Logo characters
- **Black** `(0, 0, 0)`: Logo background
- **Gray** `(153, 153, 153)`: Version, model, path, shortcuts, cycle hints
- **Dark gray** `(136, 136, 136)`: Separator lines
- **Teal** `(72, 150, 140)`: Plan mode indicator
- **Purple** `(175, 135, 255)`: Accept edits mode indicator
- **Red/Pink** `(255, 107, 128)`: Bypass permissions mode indicator

## Permission Mode Cycle Order

When pressing Shift+Tab to cycle permission modes:
1. Default ("? for shortcuts")
2. Accept edits ("⏵⏵ accept edits on")
3. Plan mode ("⏸ plan mode on")
4. (cycles back to Default)

Note: Bypass mode is only available when started with `--dangerously-skip-permissions`.

## Shell Mode ANSI Fixtures

These fixtures capture the shell mode ('!' prefix) WITH ANSI color codes preserved.

### shell_mode_prefix_ansi.txt
Shell mode entry with ANSI escape sequences:
- Input shows `❯ \!` with cursor positioned after the backslash-exclamation
- The placeholder hint disappears when shell prefix is entered
- Status bar is hidden in shell mode

### shell_mode_command_ansi.txt
Shell mode with command input with ANSI escape sequences:
- Input shows `❯ \!ls -la` with cursor at end
- Demonstrates multi-word command input in shell mode

### Shell Mode Behavior
- Typing '!' at the start of empty input enters shell mode
- The '!' prefix is displayed as '\!' in the input field
- Shell mode allows direct bash command execution
- Commands are shown as `\!command` in the input
- Backspace on `\!` exits shell mode and shows placeholder again
- When submitted, the prompt shows `❯ \!command` and Claude executes `Bash(command)`

## Capture Method

Captured using tmux with ANSI preservation:
```bash
tmux kill-session -t claude-perm 2>/dev/null
tmux new-session -d -s claude-perm -x 120 -y 20
tmux send-keys -t claude-perm 'claude --model haiku' Enter
sleep 4
# Capture with -e flag to preserve ANSI escape sequences
tmux capture-pane -e -t claude-perm -p | tail -n +6 | head -8 > permission_default_ansi.txt
# Press shift+tab to cycle modes
tmux send-keys -t claude-perm BTab
sleep 0.5
tmux capture-pane -e -t claude-perm -p | tail -n +6 | head -8 > permission_accept_edits_ansi.txt
# Continue cycling...
```

For bypass mode, start with:
```bash
tmux send-keys -t claude-bypass 'claude --model haiku --dangerously-skip-permissions' Enter
```

For shell mode fixtures:
```bash
tmux kill-session -t claude-shell 2>/dev/null
tmux new-session -d -s claude-shell -x 120 -y 20
tmux send-keys -t claude-shell 'claude --model haiku' Enter
sleep 5
# Press ! to enter shell mode
tmux send-keys -t claude-shell '!'
sleep 0.5
tmux capture-pane -e -t claude-shell -p | tail -n +5 | head -8 > shell_mode_prefix_ansi.txt
# Type a command
tmux send-keys -t claude-shell 'ls -la'
sleep 0.5
tmux capture-pane -e -t claude-shell -p | tail -n +5 | head -8 > shell_mode_command_ansi.txt
tmux kill-session -t claude-shell
```
