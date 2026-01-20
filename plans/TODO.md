# TODO

Follow-up items from completed features.

## Permission Dialogs (from fix-permission-dialogs.md)

### Context-Sensitive Option 2 Text

The Bash permission dialog hardcodes "Yes, allow reading from etc/ from this project" regardless of the actual command. This should be context-sensitive based on what the command does.

**Current behavior:**
```
 Do you want to proceed?
 ❯ 1. Yes
   2. Yes, allow reading from etc/ from this project  <-- always this
   3. No
```

**Expected behavior:**
- For `cat /etc/passwd`: "Yes, allow reading from etc/ from this project"
- For `npm test`: "Yes, allow npm commands from this project"
- For `rm -rf`: "Yes, allow rm commands from this project"

**Location:** `crates/cli/src/tui/widgets/permission.rs:92-94`

### Session-Level Permission Persistence

The `PermissionSelection::YesSession` choice is implemented in the UI but the actual session-level grant isn't persisted. Currently it just prints "[Permission granted for session]" without actually remembering the grant.

**To implement:**
1. Track session-granted permissions in `TuiAppStateInner`
2. Check against session grants before showing permission dialog
3. Clear session grants when session ends

**Location:** `crates/cli/src/tui/app.rs:949-954`

## Input Shortcuts

### '?' Shortcut Handling

Implement support for '?' input (e.g., '? for shortcuts').

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### '!' Shell Mode Handling

Implement support for '!' prefix (e.g., shell mode).

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Double-Tap Escape to Clear Input

Implement support for double-tapping Escape to clear input.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Ctrl+T to Show Todos

Implement support for Ctrl+T to show todos.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Meta+P to Switch Model

Implement support for Meta+P (Option+P on macOS) to switch models.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Ctrl+_ to Undo

Implement support for Ctrl+_ to undo.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Ctrl+Z to Suspend

Implement support for Ctrl+Z to suspend.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Ctrl+S to Stash Prompt

Implement support for Ctrl+S to stash prompt.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

## Slash Commands

### /fork

Implement the `/fork` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /todos

Implement the `/todos` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /tasks

Implement the `/tasks` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /context

Implement the `/context` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /exit

Implement the `/exit` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /export

Implement the `/export` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /help

Implement the `/help` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /hooks

Implement the `/hooks` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### /memory

Implement the `/memory` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Slash Command Search

Implement incremental search/filtering for slash commands by typing `/[key][key][...]`. Includes type-ahead filtering, arrow key navigation, and tab completion.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

## Scenario Configuration

### Subscription Level in Header

Add scenario-level configuration for the subscription text displayed in the header (e.g., "Opus 4.5 · Claude Max").

Examples: `Claude Max`, `Claude Pro`, `API`

### Fix Model Version String Rendering

Model versions render incorrectly (e.g., "Sonnet 4" instead of "Sonnet 4.5").

### Default Model

Default model should be Opus 4.5 when not specified via `--model` or scenario configuration.

## Testing

### Basic ANSI Color Matching

Add test support for matching ANSI color escape sequences in TUI output. Use captured fixtures (e.g., `initial_state_ansi.txt`) to verify color rendering matches real Claude Code.

**Location:** `crates/cli/tests/fixtures/tui/v2.1.12/`

