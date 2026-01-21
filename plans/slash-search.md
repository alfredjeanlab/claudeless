# Implementation Plan: Slash Command Incremental Search

**Test File:** `crates/cli/tests/tui_slash_search.rs` (26 tests, currently `#[ignore]`)

## Overview

Implement incremental search/filtering for slash commands by typing `/[key][key][...]`. When the user types `/`, a command menu appears with all available commands in alphabetical order. As additional characters are typed, the menu filters using fuzzy matching. Arrow keys navigate the selection, Tab completes the selected command, and Escape closes the menu.

**Key behaviors to implement:**
1. Typing `/` opens the slash command autocomplete menu
2. Menu shows commands in alphabetical order with descriptions
3. First command is highlighted (selected) by default
4. Additional characters filter commands using fuzzy/subsequence matching
5. Down/Up arrows navigate through the filtered list
6. Tab completes the selected command and closes the menu
7. Commands with arguments show hints (e.g., `/add-dir  <path>`)
8. Escape closes menu but keeps typed text; second Escape clears input

## Project Structure

```
crates/cli/src/tui/
├── app.rs                    # Key handling, state transitions
├── app_tests.rs              # Unit tests for slash menu state
├── mod.rs                    # Module exports
├── slash_menu.rs             # NEW: Command registry and filtering
├── slash_menu_tests.rs       # NEW: Unit tests for filtering
└── widgets/
    ├── mod.rs                # Export new widget
    ├── slash_menu.rs         # NEW: Menu rendering widget
    └── slash_menu_tests.rs   # NEW: Widget rendering tests

crates/cli/tests/
├── tui_slash_search.rs       # Integration tests (enable from #[ignore])
└── fixtures/tui/v2.1.12/
    ├── slash_menu_open.txt           # NEW: Fixture for menu open
    ├── slash_menu_filtered_co.txt    # NEW: Fixture for /co filter
    ├── slash_menu_navigation.txt     # NEW: Fixture for arrow nav
    └── slash_tab_complete.txt        # NEW: Fixture after Tab
```

## Dependencies

No new external dependencies required. The fuzzy matching algorithm will be implemented in-house using subsequence matching.

## Implementation Phases

### Phase 1: Capture Real Claude Code Behavior

**Goal:** Create reference fixtures by capturing real Claude Code v2.1.12 behavior.

**Steps:**
1. Follow `docs/prompts/tui-test-capture-guide.md`
2. Capture these key states:
   - Menu open (after typing `/`)
   - Menu filtered (after typing `/co`)
   - Menu navigation (after pressing Down)
   - After Tab completion (showing argument hint)
   - After Escape (menu closed, text preserved)

**Commands to capture:**
```bash
tmux kill-session -t claude-slash 2>/dev/null
tmux new-session -d -s claude-slash -x 120 -y 20
tmux send-keys -t claude-slash 'claude --model haiku' Enter
sleep 3

# Capture: Menu open
tmux send-keys -t claude-slash '/'
sleep 0.5
tmux capture-pane -t claude-slash -p > fixtures/slash_menu_open.txt

# Capture: Filtered
tmux send-keys -t claude-slash 'co'
sleep 0.3
tmux capture-pane -t claude-slash -p > fixtures/slash_menu_filtered_co.txt

# etc.
```

**Verification:**
- [ ] All key UI states captured as fixtures
- [ ] Command list matches what tests expect

---

### Phase 2: Define Command Registry

**Goal:** Create a static registry of all slash commands with descriptions and argument hints.

**Files:**
- `crates/cli/src/tui/slash_menu.rs` (new)
- `crates/cli/src/tui/slash_menu_tests.rs` (new)

**Implementation:**

```rust
// crates/cli/src/tui/slash_menu.rs

/// A slash command definition
#[derive(Clone, Debug)]
pub struct SlashCommand {
    /// Command name without the leading `/` (e.g., "add-dir")
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// Optional argument hint (e.g., "<path>")
    pub argument_hint: Option<&'static str>,
}

impl SlashCommand {
    /// Full command with leading slash
    pub fn full_name(&self) -> String {
        format!("/{}", self.name)
    }
}

/// All available slash commands, in alphabetical order
pub static COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "add-dir",
        description: "Add a new working directory",
        argument_hint: Some("<path>"),
    },
    SlashCommand {
        name: "agents",
        description: "Manage agent configurations",
        argument_hint: None,
    },
    SlashCommand {
        name: "clear",
        description: "Clear conversation history",
        argument_hint: None,
    },
    SlashCommand {
        name: "compact",
        description: "Compact conversation (keep a summary in context)",
        argument_hint: None,
    },
    SlashCommand {
        name: "config",
        description: "Open configuration settings",
        argument_hint: None,
    },
    SlashCommand {
        name: "context",
        description: "View current context usage",
        argument_hint: None,
    },
    SlashCommand {
        name: "help",
        description: "Show help and available commands",
        argument_hint: None,
    },
    SlashCommand {
        name: "hooks",
        description: "Manage hook configurations for tool events",
        argument_hint: None,
    },
    // Add remaining commands from Claude Code...
];
```

**Verification:**
- [ ] Commands are in alphabetical order
- [ ] All commands from tests are included
- [ ] `cargo test -p claudeless -- slash_menu` passes

---

### Phase 3: Implement Fuzzy Filtering

**Goal:** Filter commands based on typed characters using subsequence matching.

**Files:**
- `crates/cli/src/tui/slash_menu.rs`
- `crates/cli/src/tui/slash_menu_tests.rs`

**Implementation:**

```rust
/// Check if `query` matches `text` using fuzzy subsequence matching
/// Returns true if all characters in query appear in text in order
pub fn fuzzy_matches(query: &str, text: &str) -> bool {
    let query = query.to_lowercase();
    let text = text.to_lowercase();

    let mut query_chars = query.chars().peekable();

    for text_char in text.chars() {
        if let Some(&query_char) = query_chars.peek() {
            if text_char == query_char {
                query_chars.next();
            }
        }
    }

    query_chars.peek().is_none()
}

/// Filter commands by a query string (without leading `/`)
pub fn filter_commands(query: &str) -> Vec<&'static SlashCommand> {
    COMMANDS
        .iter()
        .filter(|cmd| fuzzy_matches(query, cmd.name))
        .collect()
}
```

**Tests:**

```rust
#[test]
fn test_fuzzy_matches_prefix() {
    assert!(fuzzy_matches("co", "compact"));
    assert!(fuzzy_matches("co", "config"));
    assert!(fuzzy_matches("co", "context"));
}

#[test]
fn test_fuzzy_matches_subsequence() {
    assert!(fuzzy_matches("hk", "hooks")); // h_oo_k_s
    assert!(fuzzy_matches("ad", "add-dir")); // _a_d_d-dir
}

#[test]
fn test_fuzzy_no_match() {
    assert!(!fuzzy_matches("xyz", "compact"));
}

#[test]
fn test_filter_commands_co() {
    let results = filter_commands("co");
    let names: Vec<_> = results.iter().map(|c| c.name).collect();
    assert!(names.contains(&"compact"));
    assert!(names.contains(&"config"));
    assert!(names.contains(&"context"));
}
```

**Verification:**
- [ ] `fuzzy_matches("co", "compact")` returns true
- [ ] `fuzzy_matches("hel", "help")` returns true
- [ ] `filter_commands("")` returns all commands
- [ ] `filter_commands("co")` returns compact, config, context

---

### Phase 4: Add Slash Menu State to TUI

**Goal:** Track slash menu state in `TuiAppStateInner`.

**Files:**
- `crates/cli/src/tui/app.rs`

**Implementation:**

```rust
// Add to TuiAppStateInner:

/// Slash command menu state (None if menu is closed)
pub slash_menu: Option<SlashMenuState>,

/// State of the slash command autocomplete menu
#[derive(Clone, Debug)]
pub struct SlashMenuState {
    /// Characters typed after `/` (the filter query)
    pub filter: String,
    /// Index of the currently selected command in the filtered list
    pub selected_index: usize,
    /// Cached filtered commands (updated when filter changes)
    pub filtered_commands: Vec<&'static SlashCommand>,
}

impl SlashMenuState {
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            selected_index: 0,
            filtered_commands: filter_commands(""),
        }
    }

    /// Update the filter and refresh the command list
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter.clone();
        self.filtered_commands = filter_commands(&filter);
        // Reset selection if it's out of bounds
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = 0;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.filtered_commands.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if !self.filtered_commands.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_commands.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get the currently selected command
    pub fn selected_command(&self) -> Option<&'static SlashCommand> {
        self.filtered_commands.get(self.selected_index).copied()
    }
}
```

**Verification:**
- [ ] `SlashMenuState::new()` creates state with all commands
- [ ] `set_filter("co")` updates filtered list
- [ ] `select_next()` wraps at end
- [ ] `select_prev()` wraps at beginning

---

### Phase 5: Handle Key Events for Slash Menu

**Goal:** Update `handle_input_key()` to manage slash menu interactions.

**Files:**
- `crates/cli/src/tui/app.rs`

**Key Handling Logic:**

```rust
// In handle_input_key():

// Handle slash menu navigation when menu is open
if inner.slash_menu.is_some() {
    match event.code {
        KeyCode::Down => {
            if let Some(ref mut menu) = inner.slash_menu {
                menu.select_next();
            }
            return;
        }
        KeyCode::Up => {
            if let Some(ref mut menu) = inner.slash_menu {
                menu.select_prev();
            }
            return;
        }
        KeyCode::Tab => {
            // Complete the selected command
            if let Some(ref menu) = inner.slash_menu {
                if let Some(cmd) = menu.selected_command() {
                    inner.input_buffer = cmd.full_name();
                    if cmd.argument_hint.is_some() {
                        inner.input_buffer.push_str("  "); // Space before arg hint
                    }
                    inner.cursor_pos = inner.input_buffer.len();
                }
            }
            inner.slash_menu = None; // Close menu
            return;
        }
        KeyCode::Esc => {
            // Close menu but keep text
            inner.slash_menu = None;
            // Note: existing escape handling shows "Esc to clear again"
            return;
        }
        _ => {}
    }
}

// Handle character input
if let KeyCode::Char(c) = event.code {
    // Insert character at cursor
    inner.input_buffer.insert(inner.cursor_pos, c);
    inner.cursor_pos += 1;

    // Update slash menu if input starts with /
    if inner.input_buffer.starts_with('/') {
        let filter = inner.input_buffer[1..].to_string();
        if let Some(ref mut menu) = inner.slash_menu {
            menu.set_filter(filter);
        } else {
            let mut menu = SlashMenuState::new();
            menu.set_filter(filter);
            inner.slash_menu = Some(menu);
        }
    } else {
        inner.slash_menu = None;
    }
}

// Handle backspace - update or close menu
if event.code == KeyCode::Backspace {
    // ... existing backspace handling ...

    // Update slash menu
    if inner.input_buffer.starts_with('/') {
        let filter = inner.input_buffer[1..].to_string();
        if let Some(ref mut menu) = inner.slash_menu {
            menu.set_filter(filter);
        }
    } else {
        inner.slash_menu = None;
    }
}
```

**Verification:**
- [ ] Typing `/` opens menu
- [ ] Typing additional chars updates filter
- [ ] Down arrow moves selection
- [ ] Tab completes and closes menu
- [ ] Escape closes menu without clearing input
- [ ] Backspace updates filter

---

### Phase 6: Implement Menu Rendering Widget

**Goal:** Create a widget to render the slash command menu.

**Files:**
- `crates/cli/src/tui/widgets/slash_menu.rs` (new)
- `crates/cli/src/tui/widgets/mod.rs` (update exports)

**Implementation:**

```rust
// crates/cli/src/tui/widgets/slash_menu.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::slash_menu::SlashMenuState;

pub struct SlashMenuWidget<'a> {
    state: &'a SlashMenuState,
}

impl<'a> SlashMenuWidget<'a> {
    pub fn new(state: &'a SlashMenuState) -> Self {
        Self { state }
    }
}

impl Widget for SlashMenuWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let max_visible = (area.height as usize).saturating_sub(2); // Account for borders

        let lines: Vec<Line> = self
            .state
            .filtered_commands
            .iter()
            .enumerate()
            .take(max_visible)
            .map(|(i, cmd)| {
                let is_selected = i == self.state.selected_index;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    Style::default()
                };

                Line::from(vec![
                    Span::styled(format!("/{:<12}", cmd.name), style),
                    Span::styled("  ", style),
                    Span::styled(cmd.description, style.fg(Color::DarkGray)),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE));

        paragraph.render(area, buf);
    }
}
```

**Integration in render:**

```rust
// In app.rs render() method, after rendering input:

if let Some(ref menu) = inner.slash_menu {
    // Calculate menu area (above input line)
    let menu_height = menu.filtered_commands.len().min(10) as u16;
    let menu_area = Rect {
        x: input_area.x,
        y: input_area.y.saturating_sub(menu_height + 1),
        width: input_area.width.min(60),
        height: menu_height,
    };

    SlashMenuWidget::new(menu).render(menu_area, buf);
}
```

**Verification:**
- [ ] Menu renders with correct commands
- [ ] Selected command is highlighted
- [ ] Descriptions are shown
- [ ] Menu doesn't overflow screen

---

### Phase 7: Add Argument Hint Display

**Goal:** Show argument hints after Tab completion for commands that take arguments.

**Files:**
- `crates/cli/src/tui/app.rs`

**Implementation:**

The argument hint should be displayed as part of the input line, styled differently (e.g., dimmed):

```rust
// In render_input_line():

// After the input buffer, show argument hint if applicable
if inner.slash_menu.is_none() && inner.input_buffer.starts_with('/') {
    // Find matching command
    let cmd_name = inner.input_buffer.trim_start_matches('/');
    if let Some(cmd) = COMMANDS.iter().find(|c| c.name == cmd_name) {
        if let Some(hint) = cmd.argument_hint {
            // Render hint in dim style
            let hint_span = Span::styled(
                format!("  {}", hint),
                Style::default().fg(Color::DarkGray),
            );
            // ... render hint after cursor
        }
    }
}
```

**Verification:**
- [ ] `/add-dir` shows `<path>` hint
- [ ] `/clear` shows no hint
- [ ] Hint disappears when menu is open
- [ ] Hint appears immediately after Tab completion

---

### Phase 8: Enable Integration Tests

**Goal:** Remove `#[ignore]` from tests in `tui_slash_search.rs` and verify they pass.

**Files:**
- `crates/cli/tests/tui_slash_search.rs`

**Steps:**
1. Remove `#[ignore]` and `// TODO(implement)` from tests one by one
2. Run each test to verify behavior matches Claude Code
3. Fix any discrepancies

**Test Categories:**
1. Slash Command Menu Tests (2 tests)
   - `test_tui_slash_opens_command_menu`
   - `test_tui_slash_menu_shows_descriptions`

2. Incremental Filtering Tests (3 tests)
   - `test_tui_slash_filters_commands`
   - `test_tui_slash_filters_progressively`
   - `test_tui_slash_fuzzy_matches`

3. Arrow Key Navigation Tests (2 tests)
   - `test_tui_slash_down_arrow_navigation`
   - `test_tui_slash_up_arrow_navigation`

4. Tab Completion Tests (4 tests)
   - `test_tui_slash_tab_completes_first_command`
   - `test_tui_slash_tab_shows_argument_hint`
   - `test_tui_slash_tab_closes_menu`

5. Escape Behavior Tests (2 tests)
   - `test_tui_slash_escape_closes_menu_keeps_text`
   - `test_tui_slash_escape_from_filtered_keeps_text`

**Verification:**
- [ ] All 26 tests pass without `#[ignore]`
- [ ] `cargo test tui_slash` passes

---

## Key Implementation Details

### Fuzzy Matching Algorithm

The fuzzy matching uses **subsequence matching**: all characters in the query must appear in the command name in the same order, but not necessarily consecutively.

```
Query: "co"
Matches: "compact" (c_o_mpact), "config" (c_o_nfig), "context" (c_o_ntext)
Doesn't match: "clear" (no 'o' after 'c')

Query: "hk"
Matches: "hooks" (h_oo_k_s)
```

### State Machine

```
┌──────────────────────────────────────────────────────────┐
│                    Input Mode                             │
│                   (slash_menu: None)                      │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Type '/'
                           ▼
┌──────────────────────────────────────────────────────────┐
│                 Slash Menu Open                           │
│              (slash_menu: Some(_))                        │
│                                                           │
│  - Filter updates on each character typed                 │
│  - Down/Up changes selected_index                         │
│  - Tab → complete + close                                 │
│  - Escape → close (keep text)                            │
│  - Backspace → update filter or close if empty           │
│  - Enter → execute command if valid                      │
└──────────────────────────────────────────────────────────┘
                           │
                           │ Tab / Escape / Enter
                           ▼
┌──────────────────────────────────────────────────────────┐
│                    Input Mode                             │
│              (slash_menu: None)                           │
│              (input_buffer may have command)              │
└──────────────────────────────────────────────────────────┘
```

### Menu Positioning

The menu renders **above** the input line, floating over the conversation area:

```
┌────────────────────────────────────────────────────────────┐
│  (conversation area)                                       │
│                                                            │
│  ┌──────────────────────────────────────────┐              │
│  │ /add-dir   Add a new working directory   │ ← menu      │
│  │ /agents    Manage agent configurations   │              │
│  │ /clear     Clear conversation history    │              │
│  └──────────────────────────────────────────┘              │
├────────────────────────────────────────────────────────────┤
│ ❯ /a                                                       │ ← input
└────────────────────────────────────────────────────────────┘
```

### Command List (from tests)

Based on the test file, these commands should be included:

| Command | Description | Argument |
|---------|-------------|----------|
| `/add-dir` | Add a new working directory | `<path>` |
| `/agents` | Manage agent configurations | - |
| `/clear` | Clear conversation history | - |
| `/compact` | Compact conversation (keep a summary in context) | - |
| `/config` | Open configuration settings | - |
| `/context` | View current context usage | - |
| `/help` | Show help and available commands | - |
| `/hooks` | Manage hook configurations for tool events | - |

(Additional commands to be captured from Claude Code during Phase 1)

---

## Verification Plan

### Unit Tests

**Slash Menu Module (`slash_menu_tests.rs`):**
- [ ] `test_fuzzy_matches_prefix` - prefix matching works
- [ ] `test_fuzzy_matches_subsequence` - non-consecutive matching works
- [ ] `test_fuzzy_case_insensitive` - matching is case-insensitive
- [ ] `test_filter_commands_empty` - returns all commands
- [ ] `test_filter_commands_co` - filters correctly
- [ ] `test_filter_commands_no_match` - returns empty

**Slash Menu State (`app_tests.rs`):**
- [ ] `test_slash_menu_state_new` - initial state correct
- [ ] `test_slash_menu_set_filter` - updates filtered list
- [ ] `test_slash_menu_select_next` - wraps correctly
- [ ] `test_slash_menu_select_prev` - wraps correctly
- [ ] `test_slash_menu_selected_command` - returns correct command

### Integration Tests

All 26 tests in `tui_slash_search.rs`:
- [ ] Slash Command Menu Tests (2)
- [ ] Incremental Filtering Tests (3)
- [ ] Arrow Key Navigation Tests (2)
- [ ] Tab Completion Tests (4+)
- [ ] Escape Behavior Tests (2+)

### Manual Testing

1. Run `claudeless` with a test scenario
2. Type `/` - verify menu opens with all commands
3. Type `co` - verify menu filters to compact, config, context
4. Press Down - verify selection moves
5. Press Tab - verify completion and menu closes
6. Press Escape - verify menu closes but text remains
7. Verify argument hints appear for commands that take args

### Final Checklist

- [ ] `make check` passes
- [ ] All unit tests pass
- [ ] All integration tests pass (no `#[ignore]`)
- [ ] No new clippy warnings
- [ ] Fixtures captured and documented
- [ ] Manual testing complete
