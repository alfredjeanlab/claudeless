# Claude TUI Snapshots

Captured from real Claude CLI for comparison testing.

**Behavior observed with:** claude --version 2.1.12 (Claude Code)

## Captures

### initial_state.txt
The initial TUI state when claude starts interactively:
- Logo with version info
- Model name (e.g., "Haiku 4.5 · Claude Max")
- Working directory
- Placeholder prompt hint (e.g., 'Try "refactor mod.rs"')
- Help shortcut hint ("? for shortcuts")

### with_input.txt
TUI state after user types in the input field:
- Same header as initial
- User's typed text in the input area
- No placeholder hint when input is present

### after_response.txt
TUI state after Claude responds:
- Header remains
- Shows the user's prompt prefixed with "❯"
- Shows Claude's response prefixed with "⏺"
- New empty input field ready for next prompt
- Help shortcut hint returns

### Model Variants
- **model_haiku.txt**: Shows "Haiku 4.5 · Claude Max"
- **model_sonnet.txt**: Shows "Sonnet 4.5 · Claude Max"
- **model_opus.txt**: Shows "Opus 4.5 · Claude Max"

### Permission Mode Variants
- **permission_default.txt**: Shows "? for shortcuts"
- **permission_plan.txt**: Shows "⏸ plan mode on (shift+tab to cycle)"
- **permission_accept_edits.txt**: Shows "⏵⏵ accept edits on (shift+tab to cycle)"
- **permission_bypass.txt**: Shows "⏵⏵ bypass permissions on (shift+tab to cycle)"

### status_bar_extended.txt
Extended status bar (visible in non-default permission modes):
- Left: Permission mode indicator with cycle hint
- Right: "Use meta+t to toggle thinking"
- With file changes: Shows git diff stats like "19 files +627 -5608"

## Key UI Elements

- **Logo**: ASCII art logo with version
- **Model indicator**: Shows current model and account type (e.g., "Haiku 4.5 · Claude Max")
- **Working directory**: Shows current path
- **Prompt prefix**: "❯" for user messages
- **Response prefix**: "⏺" for Claude's responses
- **Input separator**: Line of "─" characters
- **Status bar (default)**: "? for shortcuts"
- **Status bar (non-default modes)**: "[icon] [mode] on (shift+tab to cycle) · [file stats] Use meta+t to toggle thinking"

## Permission Mode Icons
- Default: (no icon, shows "? for shortcuts")
- Plan: ⏸
- Accept Edits: ⏵⏵
- Bypass Permissions: ⏵⏵

## Capture Method

Captured using tmux:
```bash
tmux new-session -d -s claude-tui -x 120 -y 40
tmux send-keys -t claude-tui 'claude --model haiku' Enter
sleep 3
tmux capture-pane -t claude-tui -p
```
