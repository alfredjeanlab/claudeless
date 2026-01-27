# TODO

Follow-up items from completed features.

## Input Shortcuts

### Ctrl+_ to Undo

Integration tests for Ctrl+_ are marked `#[ignore]` because tmux cannot reliably send Ctrl+_. Unit tests verify this behavior works correctly.

**Tests:** `tui_interaction.rs` Ctrl+_ tests (integration tests `#[ignore]`, unit tests active)

**Status:** Behavior implemented; integration tests skipped due to tmux limitation.

## TUI Color Rendering

### ANSI Color Output

Implement ANSI color output in the TUI to match real Claude Code's color scheme.

**Test:** `test_initial_state_ansi_matches_fixture` in `tui_snapshot.rs`.

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

### Shell Mode ANSI Colors

Shell mode (! prefix) needs pink/magenta styling to match real Claude Code.

**Test:** `test_bash_mode_pink_colors` in `tui_shell_mode.rs` (currently `#[ignore]`)

## Slash Commands

### Slash Command Search

Implement incremental search/filtering for slash commands by typing `/[key][key][...]`. Includes type-ahead filtering, arrow key navigation, and tab completion.

Follow `docs/prompts/tui-test-capture-guide.md` to capture expected behavior from real Claude Code.

## Scenario Configuration

### Subscription Level in Header

Add scenario-level configuration for the subscription text displayed in the header (e.g., "Opus 4.5 · Claude Max").

**Fixtures captured:**
- `Claude Max` - model_haiku.txt, model_sonnet.txt, model_opus.txt
- `API Usage Billing` - api_usage_billing.txt (v2.1.14)

**Examples to capture:** `Claude Pro`, `Free`, `Enterprise` (or `Team`?)
