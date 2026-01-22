# Implementation Plan: /help Slash Command

## Overview

Implement the `/help` slash command to show a multi-tab help dialog. When executed, it displays a dialog with three tabs:

1. **general** - Overview text and keyboard shortcuts
2. **commands** - Browseable list of default slash commands (from `slash_menu::COMMANDS`)
3. **custom-commands** - Browseable list of custom/project commands (initially empty)

The dialog supports:
- Tab or Left/Right arrow keys to cycle between tabs
- Up/Down arrow keys to navigate within command lists
- Selection cursor (`❯`) indicating current command in lists
- Escape to dismiss the dialog with "Help dialog dismissed" message

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs                 # Add AppMode::HelpDialog, /help handler, key handling, rendering
│   └── widgets/
│       ├── mod.rs             # Export HelpDialog, HelpTab
│       ├── help.rs            # NEW: HelpDialog struct, HelpTab enum
│       └── help_tests.rs      # NEW: Unit tests for HelpDialog

crates/cli/tests/
└── tui_help.rs                # Remove #[ignore] from 7 tests
```

## Dependencies

No new dependencies required. Uses existing:
- `crossterm` for keyboard input handling
- `slash_menu::COMMANDS` for command list
- Box-drawing characters for dialog borders

## Implementation Phases

### Phase 1: Create HelpDialog Widget

Create a new widget module for the help dialog state.

**File:** `crates/cli/src/tui/widgets/help.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Help dialog widget.
//!
//! Shown when user executes `/help` to display help and available commands.

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;

/// Available tabs in the help dialog
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HelpTab {
    #[default]
    General,
    Commands,
    CustomCommands,
}

impl HelpTab {
    /// Get all tabs in order
    pub fn all() -> &'static [HelpTab] {
        &[HelpTab::General, HelpTab::Commands, HelpTab::CustomCommands]
    }

    /// Get the next tab (wraps around)
    pub fn next(self) -> HelpTab {
        match self {
            HelpTab::General => HelpTab::Commands,
            HelpTab::Commands => HelpTab::CustomCommands,
            HelpTab::CustomCommands => HelpTab::General,
        }
    }

    /// Get the previous tab (wraps around)
    pub fn prev(self) -> HelpTab {
        match self {
            HelpTab::General => HelpTab::CustomCommands,
            HelpTab::Commands => HelpTab::General,
            HelpTab::CustomCommands => HelpTab::Commands,
        }
    }

    /// Get display name for the tab
    pub fn name(self) -> &'static str {
        match self {
            HelpTab::General => "general",
            HelpTab::Commands => "commands",
            HelpTab::CustomCommands => "custom-commands",
        }
    }
}

/// State for the /help dialog
#[derive(Clone, Debug)]
pub struct HelpDialog {
    /// Currently active tab
    pub active_tab: HelpTab,
    /// Selected command index in Commands tab (0-based)
    pub commands_selected: usize,
    /// Selected command index in CustomCommands tab (0-based)
    pub custom_selected: usize,
    /// Claude version string for display
    pub version: String,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self::new("2.1.12".to_string())
    }
}

impl HelpDialog {
    pub fn new(version: String) -> Self {
        Self {
            active_tab: HelpTab::General,
            commands_selected: 0,
            custom_selected: 0,
            version,
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
    }

    /// Move selection up in current command list
    pub fn select_prev(&mut self, total_commands: usize) {
        match self.active_tab {
            HelpTab::Commands => {
                if total_commands > 0 {
                    if self.commands_selected == 0 {
                        self.commands_selected = total_commands - 1;
                    } else {
                        self.commands_selected -= 1;
                    }
                }
            }
            HelpTab::CustomCommands => {
                // Similar logic for custom commands when implemented
            }
            HelpTab::General => {}
        }
    }

    /// Move selection down in current command list
    pub fn select_next(&mut self, total_commands: usize) {
        match self.active_tab {
            HelpTab::Commands => {
                if total_commands > 0 {
                    self.commands_selected = (self.commands_selected + 1) % total_commands;
                }
            }
            HelpTab::CustomCommands => {
                // Similar logic for custom commands when implemented
            }
            HelpTab::General => {}
        }
    }
}
```

**Tasks:**
1. Create `crates/cli/src/tui/widgets/help.rs` with `HelpTab` enum and `HelpDialog` struct
2. Add unit tests in `crates/cli/src/tui/widgets/help_tests.rs`:
   - Test `HelpTab::next()` and `HelpTab::prev()` cycling
   - Test `HelpDialog::next_tab()` and `HelpDialog::prev_tab()`
   - Test `HelpDialog::select_next()` and `HelpDialog::select_prev()` with bounds

### Phase 2: Wire Up AppMode and State

Integrate the HelpDialog into the TUI application state.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `HelpDialog` variant to `AppMode` enum (around line 139):
   ```rust
   /// Showing help dialog
   HelpDialog,
   ```

2. Add `help_dialog` field to `RenderState` struct (around line 184):
   ```rust
   /// Help dialog state (None if not showing)
   pub help_dialog: Option<HelpDialog>,
   ```

3. Add `help_dialog` field to `TuiAppStateInner` struct

4. Update `render_state()` to include `help_dialog`

5. Export from `widgets/mod.rs`:
   ```rust
   pub mod help;
   pub use help::{HelpDialog, HelpTab};
   ```

### Phase 3: Implement Command Handler

Update the `/help` command to open the dialog instead of showing inline text.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Update `/help` match arm in `handle_command_inner()` (around line 1385):
   ```rust
   "/help" | "/?" => {
       inner.mode = AppMode::HelpDialog;
       let version = inner.claude_version.clone().unwrap_or_else(|| "2.1.12".to_string());
       inner.help_dialog = Some(HelpDialog::new(version));
   }
   ```

### Phase 4: Implement Key Handler

Add keyboard handling for the help dialog.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add `AppMode::HelpDialog` to the match in `handle_key_event()` (around line 549):
   ```rust
   AppMode::HelpDialog => self.handle_help_dialog_key(key),
   ```

2. Add `AppMode::HelpDialog` to escape handling in `handle_escape()` (around line 1063):
   ```rust
   AppMode::HelpDialog => {
       inner.help_dialog = None;
       inner.mode = AppMode::Input;
   }
   ```

3. Implement `handle_help_dialog_key()`:
   ```rust
   fn handle_help_dialog_key(&self, key: KeyEvent) {
       use crate::tui::slash_menu::COMMANDS;
       let mut inner = self.inner.lock();

       let Some(ref mut dialog) = inner.help_dialog else {
           return;
       };

       match key.code {
           KeyCode::Esc => {
               inner.mode = AppMode::Input;
               inner.help_dialog = None;
               inner.response_content = "Help dialog dismissed".to_string();
               inner.is_command_output = true;
           }
           KeyCode::Tab | KeyCode::Right => dialog.next_tab(),
           KeyCode::Left => dialog.prev_tab(),
           KeyCode::BackTab => dialog.prev_tab(), // Shift+Tab
           KeyCode::Up => dialog.select_prev(COMMANDS.len()),
           KeyCode::Down => dialog.select_next(COMMANDS.len()),
           _ => {}
       }
   }
   ```

### Phase 5: Implement Render Function

Add the dialog rendering logic.

**File:** `crates/cli/src/tui/app.rs`

**Tasks:**
1. Add conditional rendering in the main render function (around line 2092):
   ```rust
   if state.mode == AppMode::HelpDialog {
       if let Some(ref dialog) = state.help_dialog {
           return render_help_dialog(dialog, width);
       }
   }
   ```

2. Implement `render_help_dialog()`:
   ```rust
   /// Render help dialog
   fn render_help_dialog(dialog: &HelpDialog, width: usize) -> AnyElement<'static> {
       use crate::tui::slash_menu::COMMANDS;
       use crate::tui::widgets::HelpTab;

       let inner_width = width.saturating_sub(2);
       let h_line = "─".repeat(inner_width);
       let bottom_border = format!("╰{}╯", h_line);

       // Build tab header: "─Claude Code v2.1.12─ general ─ commands ─ custom-commands ─(←/→ or tab to cycle)─..."
       let tab_header = format!(
           " ─Claude Code v{}─ {} ─ {} ─ {} ─(←/→ or tab to cycle){}",
           dialog.version,
           HelpTab::General.name(),
           HelpTab::Commands.name(),
           HelpTab::CustomCommands.name(),
           "─".repeat(inner_width.saturating_sub(70))
       );

       let footer = " For more help: https://code.claude.com/docs/en/overview";

       match dialog.active_tab {
           HelpTab::General => {
               element! {
                   View(flex_direction: FlexDirection::Column, width: 100pct) {
                       Text(content: tab_header.clone())
                       Text(content: "")
                       Text(content: "")
                       Text(content: "  Claude understands your codebase, makes edits with your permission, and executes commands — right from your terminal.")
                       Text(content: "  / for commands    ctrl + o for verbose output              cmd + v to paste images")
                       Text(content: "  & for background  backslash (\\) + return (⏎) for newline   ctrl + s to stash prompt")
                       Text(content: "")
                       Text(content: footer)
                   }
               }.into()
           }
           HelpTab::Commands => {
               // Show browseable command list with selection cursor
               let header = "  Browse default commands:";

               // Get visible commands (show selected + surrounding)
               let selected = dialog.commands_selected;

               // For simplicity, show first command selected, next command with down arrow hint
               let cmd = COMMANDS.get(selected);
               let next_cmd = COMMANDS.get(selected + 1);

               element! {
                   View(flex_direction: FlexDirection::Column, width: 100pct) {
                       Text(content: h_line.clone())
                       Text(content: format!("  Claude Code v{}  {}   {}   {}  (←/→ or tab to cycle)",
                           dialog.version,
                           HelpTab::General.name(),
                           HelpTab::Commands.name(),
                           HelpTab::CustomCommands.name()))
                       Text(content: "")
                       Text(content: header)
                       Text(content: format!("  ❯ /{}", cmd.map(|c| c.name).unwrap_or("")))
                       Text(content: format!("    {}", cmd.map(|c| c.description).unwrap_or("")))
                       Text(content: format!("  ↓ /{}", next_cmd.map(|c| c.name).unwrap_or("")))
                       Text(content: footer)
                   }
               }.into()
           }
           HelpTab::CustomCommands => {
               element! {
                   View(flex_direction: FlexDirection::Column, width: 100pct) {
                       Text(content: tab_header)
                       Text(content: "")
                       Text(content: "  Browse custom commands:")
                       Text(content: "  (no custom commands configured)")
                       Text(content: "")
                       Text(content: footer)
                   }
               }.into()
           }
       }
   }
   ```

### Phase 6: Enable Tests

Remove `#[ignore]` attributes from the test file and verify all tests pass.

**File:** `crates/cli/tests/tui_help.rs`

**Tasks:**
1. Remove `#[ignore]` from all 7 test functions
2. Run tests: `cargo test --test tui_help`
3. Adjust rendering output to match fixture expectations if needed

## Key Implementation Details

### Tab Header Format

From the fixture `help_general_tab.txt`, the exact format is:
```
 ─Claude Code v2.1.12─ general ─ commands ─ custom-commands ─(←/→ or tab to cycle)─────────
```

Note the spaces around tab names and the separator characters.

### Commands Tab Format

From `help_commands_tab.txt`:
```
  Browse default commands:
  ❯ /add-dir
    Add a new working directory
  ↓ /agents
```

- Selected command has `❯` cursor at column 2
- Description is indented 4 spaces
- Next command has `↓` hint to indicate more below

### Selection State

- When entering Commands tab, first command (`/add-dir`) should be auto-selected
- Up/Down arrows move selection, wrapping at boundaries
- Selection index persists when switching back to the tab

### Dismiss Behavior

- Escape key closes dialog
- Shows message "Help dialog dismissed"
- Returns to Input mode with clean input area (no `/help` text remaining)

## Verification Plan

### Unit Tests (Phase 1)

```bash
cargo test --lib -- tui::widgets::help
```

Verify:
- `HelpTab::next()` cycles: General -> Commands -> CustomCommands -> General
- `HelpTab::prev()` cycles in reverse
- `HelpDialog::select_next()` wraps at max commands
- `HelpDialog::select_prev()` wraps at 0

### Integration Tests (Phase 6)

```bash
cargo test --test tui_help
```

All 7 tests should pass:
1. `test_tui_help_command_shows_autocomplete` - `/help` appears in autocomplete
2. `test_tui_help_shows_dialog_with_general_tab` - Dialog opens with general tab
3. `test_tui_help_tab_shows_commands_tab` - Tab key switches to commands tab
4. `test_tui_help_tab_cycles_through_all_tabs` - Full tab cycling works
5. `test_tui_help_commands_arrow_navigation` - Up/Down arrows select commands
6. `test_tui_help_escape_dismisses_dialog` - Escape shows dismiss message
7. `test_tui_help_dismiss_returns_to_clean_input` - Returns to clean state

### Final Verification

```bash
make check
```

Ensures:
- `cargo fmt` passes
- `cargo clippy` passes
- All tests pass
- Build succeeds
- `cargo audit` passes
- `cargo deny check` passes
