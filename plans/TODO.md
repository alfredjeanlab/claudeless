# TODO

Follow-up items from completed features.

## Input Shortcuts

### '?' Shortcut Handling

Implement support for '?' input (e.g., '? for shortcuts').

**Tests:** `tui_shortcuts.rs` (currently `#[ignore]`)

### '!' Shell Mode Handling

Implement support for '!' prefix (e.g., shell mode).

**Tests:** `tui_shell_mode.rs` (currently `#[ignore]`)

### Double-Tap Escape to Clear Input

Implement support for double-tapping Escape to clear input.

**Tests:** `tui_interaction.rs` escape handling tests (currently `#[ignore]`)

### Ctrl+T to Show Todos

Implement support for Ctrl+T to show todos.

**Tests:** `tui_todos.rs` (currently `#[ignore]`)

### Meta+P to Switch Model

Implement support for Meta+P (Option+P on macOS) to switch models.

**Tests:** `tui_model.rs` model picker tests (currently `#[ignore]`)

### Ctrl+_ to Undo

Implement support for Ctrl+_ to undo.

**Tests:** `tui_interaction.rs` Ctrl+_ tests (currently `#[ignore]`)

### Ctrl+Z to Suspend

Implement support for Ctrl+Z to suspend.

**Tests:** `tui_suspend.rs` (currently `#[ignore]`)

### Ctrl+S to Stash Prompt

Implement support for Ctrl+S to stash prompt.

**Tests:** `tui_stash.rs` (currently `#[ignore]`)

## Slash Commands

### /fork

Implement the `/fork` command.

**Tests:** `tui_fork.rs` (currently `#[ignore]`)

### /todos

Implement the `/todos` command.

**Tests:** `tui_todos.rs` (currently `#[ignore]`)

### /tasks

Implement the `/tasks` command.

**Tests:** `tui_tasks.rs` (currently `#[ignore]`)

### /context

Implement the `/context` command.

**Tests:** `tui_context.rs` (currently `#[ignore]`)

### /exit

Implement the `/exit` command.

**Tests:** `tui_exit.rs` (currently `#[ignore]`)

### /export

Implement the `/export` command.

**Tests:** `tui_export.rs` (currently `#[ignore]`)

### /help

Implement the `/help` command.

**Tests:** `tui_help.rs` (currently `#[ignore]`)

### /hooks

Implement the `/hooks` command.

**Tests:** `tui_hooks.rs` (currently `#[ignore]`)

### /memory

Implement the `/memory` command.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

### Slash Command Search

Implement incremental search/filtering for slash commands by typing `/[key][key][...]`. Includes type-ahead filtering, arrow key navigation, and tab completion.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

## TUI Color Rendering

### ANSI Color Output

Implement ANSI color output in the TUI to match real Claude Code's color scheme.

**Test:** `test_initial_state_ansi_matches_fixture` in `tui_snapshot.rs` (currently `#[ignore]`).

**Colors to implement** (from `initial_state_ansi.txt` fixture):
- **Orange** `(215, 119, 87)`: Logo characters
- **Black** `(0, 0, 0)`: Logo background
- **Gray** `(153, 153, 153)`: Version, model, path, shortcuts
- **Dark gray** `(136, 136, 136)`: Separator lines (with dim attribute)

**Key elements to style:**
- Logo (foreground + background colors)
- Version text (gray)
- Model name (gray)
- Working directory path (gray)
- Separator lines (dim + dark gray)
- Status bar shortcuts (gray)
- Prompt placeholder (dim)

**Location:** `crates/cli/src/tui/app.rs` - Add iocraft styles to components.

## Scenario Configuration

### Subscription Level in Header

Add scenario-level configuration for the subscription text displayed in the header (e.g., "Opus 4.5 · Claude Max").

**Fixtures captured:**
- `Claude Max` - model_haiku.txt, model_sonnet.txt, model_opus.txt
- `API Usage Billing` - api_usage_billing.txt (v2.1.14)

**Examples to capture:** `Claude Pro`, `Free`, `Enterprise` (or `Team`?)
