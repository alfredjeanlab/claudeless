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

### escape_clear_hint.txt
TUI state after pressing Escape once with input text:
- Same header as with_input
- User's typed text still in the input area
- Status bar shows "Esc to clear again" hint (right-aligned)
- If Escape is pressed again quickly, input is cleared
- If user waits ~2 seconds, the hint disappears and user needs to double-tap again

### shortcuts_display.txt
TUI state after pressing '?' on empty input:
- Header remains
- Shows shortcuts panel with keyboard shortcuts in columns:
  - `! for bash mode` - enter shell command mode
  - `/ for commands` - slash commands
  - `@ for file paths` - file path completion
  - `& for background` - background task mode
  - `double tap esc to clear input` - clear the input field
  - `shift + tab to auto-accept edits` - cycle permission modes
  - `ctrl + o for verbose output` - toggle verbose mode
  - `ctrl + t to show todos` - display todo list
  - `backslash (\) + return (⏎) for newline` - insert newline
  - `ctrl + _ to undo` - undo last action
  - `ctrl + z to suspend` - suspend Claude
  - `cmd + v to paste images` - paste image from clipboard
  - `meta + p to switch model` - change AI model
  - `ctrl + s to stash prompt` - save prompt for later
- Pressing Escape dismisses the shortcuts panel
- Note: '?' only shows shortcuts when input is empty; otherwise types literal '?'

### todos_empty.txt
TUI state after running /todos when no todos exist:
- Header remains
- Shows the /todos command with "❯" prefix
- Shows "No todos currently tracked" with "⎿" prefix
- Empty input field ready for next prompt
- Help shortcut hint returns

### tasks_empty_dialog.txt
TUI state after running /tasks when no background tasks exist:
- Shows a dialog box with rounded corners (╭╮╰╯ characters)
- Header line: "Background tasks"
- Body: "No tasks currently running"
- Footer: "↑/↓ to select · Enter to view · Esc to close"
- Pressing Escape dismisses the dialog and shows "Background tasks dialog dismissed"

### Shell Mode ('\!' prefix)
- **shell_mode_prefix.txt**: Input field showing just `\!` after pressing '!'
- **shell_mode_command.txt**: Input field showing `\!ls -la` after typing a command
- Shell mode is entered by typing '!' at the start of empty input
- The '!' is displayed as `\!` (backslash-escaped) in the input field
- Placeholder hint disappears when shell prefix is entered
- When submitted, the prompt shows `❯ \!command` and Claude executes `Bash(command)`

### Model Variants
- **model_haiku.txt**: Shows "Haiku 4.5 · Claude Max"
- **model_sonnet.txt**: Shows "Sonnet 4.5 · Claude Max"
- **model_opus.txt**: Shows "Opus 4.5 · Claude Max"

### model_picker.txt
TUI state after pressing Meta+P (Option+P on macOS) to open the model picker:
- Shows "Select model" header
- Description: "Switch between Claude models. Applies to this session and future Claude Code sessions."
- Three model options:
  - 1. Default (recommended) - Opus 4.5 · Most capable for complex work
  - 2. Sonnet - Sonnet 4.5 · Best for everyday tasks
  - 3. Haiku - Haiku 4.5 · Fastest for quick answers
- Arrow (❯) indicates cursor position
- Checkmark (✔) indicates currently active model
- Footer shows "Enter to confirm · esc to exit"
- Pressing Up/Down navigates between options
- Pressing Enter confirms selection (changes model)
- Pressing Escape closes picker without changes

### ctrl_z_suspend.txt
TUI output when Ctrl+Z is pressed to suspend Claude Code:
- Claude Code prints "Claude Code has been suspended. Run `fg` to bring Claude Code back."
- Additional note: "Note: ctrl + z now suspends Claude Code, ctrl + _ undoes input."
- The process receives SIGTSTP and suspends, returning control to the shell
- The shell shows "zsh: suspended (signal) claude --model haiku" (or similar for other shells)
- Running `fg` resumes Claude Code, which redraws its TUI with preserved state

### Stash Prompt (Ctrl+S)
- **ctrl_s_stash_active.txt**: TUI state after pressing Ctrl+S to stash input text
  - Shows "› Stashed (auto-restores after submit)" message above the input area
  - Input field is cleared (shows placeholder prompt again)
  - Status bar shows "? for shortcuts"
  - Pressing Ctrl+S again restores the stashed text
  - After submitting a different prompt and receiving a response, the stashed text auto-restores

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

## ANSI Color Fixtures

Fixtures with the `_ansi.txt` suffix include ANSI escape sequences for color testing.

### initial_state_ansi.txt
Same content as `initial_state.txt` but with ANSI escape sequences preserved.
Used for testing that claudeless color rendering matches real Claude Code.

Key colors observed:
- **Orange** `(215, 119, 87)`: Logo characters
- **Black** `(0, 0, 0)`: Logo background
- **Gray** `(153, 153, 153)`: Version, model, path, shortcuts
- **Dark gray** `(136, 136, 136)`: Separator lines

### ANSI Capture Method

To capture ANSI-colored output from tmux, use the `-e` flag:
```bash
tmux new-session -d -s claude-tui -x 120 -y 40
tmux send-keys -t claude-tui 'claude --model haiku' Enter
sleep 3
tmux capture-pane -e -t claude-tui -p > initial_state_ansi.txt
```

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

### /exit Command
- **exit_autocomplete.txt**: TUI state showing /exit in the autocomplete dropdown
  - Shows "/exit                   Exit the REPL"
  - Also shows other matching commands like /export, /context, /clear, /compact
- The /exit command (or partial match like /ex) exits the TUI
- Displays a random farewell message (e.g., "Goodbye!", "Bye!", "See ya!", "Catch you later!")
- Returns to shell prompt after exit

### /context Command
- **context_autocomplete.txt**: TUI state showing /context in the autocomplete dropdown
  - Shows "/context                Visualize current context usage as a colored grid"
  - Also shows other matching commands like /clear, /compact, /status
- **context_usage.txt**: TUI state after executing /context command
  - Shows a 10x9 colored grid representing context usage
  - Grid uses symbols: ⛀ (current token), ⛶ (free space), ⛝ (autocompact buffer), ⛁ (category marker)
  - Shows "Estimated usage by category" header
  - Lists categories with token counts and percentages:
    - System prompt: tokens (%)
    - System tools: tokens (%)
    - Memory files: tokens (%)
    - Messages: tokens (%)
    - Free space: tokens (%)
    - Autocompact buffer: tokens (%)
  - Shows "Memory files · /memory" section with list of loaded CLAUDE.md files
  - Returns to normal input prompt after display

### /help Command
- **help_autocomplete.txt**: TUI state showing /help in the autocomplete dropdown
  - Shows "/help                   Show help and available commands"
  - Autocomplete appears when typing /help
- **help_general_tab.txt**: TUI state showing the /help dialog general tab
  - Shows a multi-tab dialog with: general, commands, custom-commands tabs
  - Tab navigation hint: "(←/→ or tab to cycle)"
  - General tab shows overview text and keyboard shortcuts:
    - "/" for commands
    - "&" for background
    - "ctrl + o" for verbose output
    - "backslash (\\) + return (⏎)" for newline
    - "cmd + v" to paste images
    - "ctrl + s" to stash prompt
  - Shows "For more help: https://code.claude.com/docs/en/overview" at bottom
- **help_commands_tab.txt**: TUI state showing the /help dialog commands tab
  - Shows browseable list of default slash commands
  - First command shown: /add-dir with description "Add a new working directory"
  - Arrow (❯) indicates cursor position
  - Down arrow (↓) indicates more commands below
  - Pressing Up/Down navigates between commands
- Pressing Tab or arrow keys cycles between tabs
- Pressing Escape dismisses the dialog and shows "Help dialog dismissed"

### /export Command
- **export_autocomplete.txt**: TUI state showing /export in the autocomplete dropdown
  - Shows "/export                 Export the current conversation to a file or clipboard"
  - Also shows other matching commands like /remote-env
- **export_method_dialog.txt**: Dialog for selecting export method
  - Shows "Export Conversation" header
  - Two options:
    1. Copy to clipboard - Copy the conversation to your system clipboard
    2. Save to file - Save the conversation to a file in the current directory
  - Shows "esc to cancel" footer
  - Arrow (❯) indicates cursor position
- **export_filename_dialog.txt**: Dialog for entering filename when saving to file
  - Shows "Export Conversation" header
  - Shows "Enter filename:" prompt
  - Default filename is based on date and conversation content (e.g., "2026-01-20-conversation-export.txt")
  - Shows "Enter to save · esc to go back" footer
- Selecting clipboard shows "Conversation copied to clipboard" confirmation
- Selecting file and pressing Enter shows "Conversation exported to: <filename>" confirmation
- Pressing Escape in method dialog shows "Export cancelled"
- Pressing Escape in filename dialog returns to method selection

### /hooks Command
- **hooks_autocomplete.txt**: TUI state showing /hooks in the autocomplete dropdown
  - Shows "/hooks                  Manage hook configurations for tool events"
  - Autocomplete appears when typing /hooks
- **hooks_dialog.txt**: TUI state showing the /hooks dialog with hook types list
  - Shows "Hooks" header with count of active hooks (e.g., "4 hooks")
  - Shows scrollable list of hook types with descriptions:
    - PreToolUse - Before tool execution
    - PostToolUse - After tool execution
    - PostToolUseFailure - After tool execution fails
    - Notification - When notifications are sent
    - UserPromptSubmit - When the user submits a prompt
    - SessionStart - When a new session is started
    - Stop - Right before Claude concludes its response
    - SubagentStart - When a subagent (Task tool call) is started
    - SubagentStop - Right before a subagent (Task tool call) concludes its response
    - PreCompact - Before conversation compaction
    - SessionEnd - When a session is ending
    - PermissionRequest - When a permission dialog is displayed
    - Setup - Repo setup hooks for init and maintenance
    - Disable all hooks (special action at end of list)
  - Arrow (❯) indicates cursor position
  - Up/Down arrows (↑/↓) indicate scrolling availability
  - Footer shows "Enter to confirm · esc to cancel"
- **hooks_matcher_dialog.txt**: TUI state after selecting a hook type
  - Shows hook-specific header: "[HookType] - Tool Matchers"
  - Shows help text about exit codes:
    - Exit code 0 - stdout/stderr not shown
    - Exit code 2 - show stderr to model and block tool call
    - Other exit codes - show stderr to user only but continue with tool call
  - Shows "+ Add new matcher..." option
  - Shows "No matchers configured yet" if no matchers exist
  - Pressing Escape returns to main hooks list
- Pressing Escape in main dialog shows "Hooks dialog dismissed"

### Slash Command Search
- **slash_search_menu.txt**: TUI state showing the slash command autocomplete menu
  - Appears when typing `/` at empty input or start of line
  - Shows commands in alphabetical order (add-dir, agents, chrome, clear, compact, config, ...)
  - Each command shows its name and description
  - First command is highlighted (selected) by default with a different color (light purple)
  - Commands are shown in a dropdown below the input line
- **slash_search_filter.txt**: TUI state showing filtered slash command menu
  - After typing `/co`, shows commands matching "co" (compact, context, config)
  - Uses fuzzy/subsequence matching - characters must appear in order but not consecutively
  - Results update as each character is typed
- **slash_search_tab_complete.txt**: TUI state after Tab completion
  - Shows completed command in input (e.g., `/add-dir`)
  - If command takes arguments, shows argument hint (e.g., `<path>`)
  - Autocomplete menu closes after Tab
- Navigation:
  - Down arrow moves selection to next command
  - Up arrow moves selection to previous command
  - Tab completes input to selected command and closes menu
  - Escape closes menu but keeps typed text, shows "Esc to clear again"

## Capture Method

Captured using tmux:
```bash
tmux new-session -d -s claude-tui -x 120 -y 40
tmux send-keys -t claude-tui 'claude --model haiku' Enter
sleep 3
tmux capture-pane -t claude-tui -p
```
