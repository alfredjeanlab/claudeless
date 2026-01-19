# Implementation Plan: Fix Permission Dialogs

## Overview

Fix and implement the TUI permission dialogs system to match real Claude Code behavior. This involves:
1. Fixing the permission pattern syntax bug (glob vs prefix matching)
2. Implementing rich permission dialogs for Bash, Edit, and Write tools
3. Fixing the trust folder dialog to match fixtures
4. Implementing the extended status bar with "Use meta+t to toggle thinking"

## Project Structure

Key files to modify/create:

```
crates/cli/src/
├── permission/
│   ├── pattern.rs          # Fix :* prefix syntax
│   ├── pattern_tests.rs    # Update tests for new syntax
│   └── check.rs            # Minor updates if needed
├── tui/
│   ├── app.rs              # Update permission dialog rendering
│   └── widgets/
│       ├── mod.rs          # Export new widget
│       ├── permission.rs   # NEW: Rich permission dialog widget
│       └── permission_tests.rs # NEW: Unit tests
│
crates/cli/tests/
├── tui_permission.rs       # Existing tests (update some)
├── settings_permissions.rs # Update tests for new syntax
└── fixtures/tui/v2.1.12/
    ├── permission_bash_command.txt  # Reference fixtures
    ├── permission_edit_file.txt
    ├── permission_write_file.txt
    └── permission_trust_folder.txt
```

## Dependencies

No new external dependencies required. Uses existing:
- `glob` crate for file pattern matching
- `iocraft` for TUI rendering

## Implementation Phases

### Phase 1: Fix Permission Pattern Syntax

**Goal:** Change pattern matching from glob-style `npm *` to Claude's prefix-style `npm:*`.

**Files:**
- `crates/cli/src/permission/pattern.rs`
- `crates/cli/src/permission/pattern_tests.rs`

**Changes:**

1. Update `CompiledPattern` enum to add `Prefix` variant:
```rust
pub enum CompiledPattern {
    /// Exact string match
    Exact(String),
    /// Prefix match (for :* patterns like "Bash(npm:*)")
    Prefix(String),
    /// Glob pattern (for file patterns like "Write(*.md)")
    Glob(Pattern),
}
```

2. Update `ToolPattern::parse()` to detect `:*` suffix:
```rust
pub fn parse(s: &str) -> Option<Self> {
    // ... existing tool name extraction ...

    // Check for :* suffix (prefix matching) - Claude's actual syntax
    if arg.ends_with(":*") {
        let prefix = &arg[..arg.len() - 2];
        return Some(Self {
            tool,
            argument: Some(CompiledPattern::Prefix(prefix.to_string())),
        });
    }

    // Existing glob/exact logic for file patterns
}
```

3. Update `ToolPattern::matches()` for `Prefix`:
```rust
CompiledPattern::Prefix(prefix) => input.starts_with(prefix),
```

4. Update all tests to use `:*` syntax where appropriate.

**Verification:**
- [ ] `ToolPattern::parse("Bash(npm:*)")` returns `Prefix("npm")`
- [ ] Pattern with `Prefix("npm")` matches "npm", "npm test", "npm install"
- [ ] Pattern with `Prefix("npm")` does NOT match "npx", "pnpm"
- [ ] `Write(*.md)` still uses glob matching (file patterns unchanged)
- [ ] `cargo test -p claudeless -- pattern` passes

---

### Phase 2: Implement Rich Permission Dialog Widget

**Goal:** Create a reusable permission dialog widget that renders the correct multi-option format.

**Files:**
- `crates/cli/src/tui/widgets/permission.rs` (NEW)
- `crates/cli/src/tui/widgets/permission_tests.rs` (NEW)
- `crates/cli/src/tui/widgets/mod.rs` (update exports)

**Data Structures:**

```rust
/// Type of permission being requested
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionType {
    /// Bash command execution
    Bash {
        command: String,
        description: Option<String>,
    },
    /// File edit with diff
    Edit {
        file_path: String,
        diff_lines: Vec<DiffLine>,
    },
    /// New file creation
    Write {
        file_path: String,
        content_preview: Vec<String>,
    },
}

/// A line in a diff preview
#[derive(Clone, Debug)]
pub struct DiffLine {
    pub line_num: Option<u32>,
    pub kind: DiffKind,
    pub content: String,
}

#[derive(Clone, Debug)]
pub enum DiffKind {
    Context,
    Added,
    Removed,
    NoNewline,
}

/// User's selection in the permission dialog
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionSelection {
    Yes,           // Option 1: Yes (single request)
    YesSession,    // Option 2: Yes, allow for session
    No,            // Option 3: No
}

/// State for rich permission dialog
#[derive(Clone, Debug)]
pub struct RichPermissionDialog {
    pub permission_type: PermissionType,
    pub selected: PermissionSelection,
}
```

**Rendering Logic:**

The widget renders different content based on `PermissionType`:

1. **Bash command** (per `permission_bash_command.txt`):
```
────────────────────────────────────────────────────────────────────────────────
 Bash command

   cat /etc/passwd | head -5
   Display first 5 lines of /etc/passwd

 Do you want to proceed?
 ❯ 1. Yes
   2. Yes, allow reading from etc/ from this project
   3. No

 Esc to cancel · Tab to add additional instructions
```

2. **Edit file** (per `permission_edit_file.txt`):
```
────────────────────────────────────────────────────────────────────────────────
 Edit file hello.txt
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
 1 -Hello World
 1   No newline at end of file
 2 +Hello Universe
 3   No newline at end of file
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
 Do you want to make this edit to hello.txt?
 ❯ 1. Yes
   2. Yes, allow all edits during this session (shift+tab)
   3. No

 Esc to cancel · Tab to add additional instructions
```

3. **Write file** (per `permission_write_file.txt`):
```
────────────────────────────────────────────────────────────────────────────────
 Create file hello.txt
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
  1 Hello World
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
 Do you want to create hello.txt?
 ❯ 1. Yes
   2. Yes, allow all edits during this session (shift+tab)
   3. No

 Esc to cancel · Tab to add additional instructions
```

**Key Implementation Details:**
- Dashed separator: `╌` character (U+254C)
- Selection indicator: `❯` for selected, spaces for unselected
- Line numbers right-aligned in diff view
- Diff symbols: `-` for removed, `+` for added, space for context

**Verification:**
- [ ] Unit tests for each render format
- [ ] Key handling: Up/Down cycles selection, Enter confirms, Esc cancels
- [ ] String output matches fixture format

---

### Phase 3: Integrate Rich Permission Dialog into TUI App

**Goal:** Replace the current basic permission dialog with the rich version.

**Files:**
- `crates/cli/src/tui/app.rs`

**Changes:**

1. Update `PermissionRequest` to use rich dialog types:
```rust
pub struct PermissionRequest {
    pub dialog: RichPermissionDialog,
}
```

2. Update `render_permission_dialog()` to use the new widget
3. Update `handle_permission_key()` for 3-option selection (Up/Down/1/2/3 keys)
4. Update `show_permission_request()` to accept rich permission data

**Key Binding Updates:**
- `1`, `Y`, `y`: Select "Yes" and confirm
- `2`: Select "Yes, allow for session" and confirm
- `3`, `N`, `n`: Select "No" and confirm
- `Up`/`Down`: Navigate options
- `Enter`: Confirm current selection
- `Esc`: Cancel (select No)

**Verification:**
- [ ] Rich dialog appears when permission is needed
- [ ] All three options are selectable
- [ ] Keyboard shortcuts work correctly

---

### Phase 4: Fix Trust Folder Dialog

**Goal:** Update trust folder dialog to match `permission_trust_folder.txt` fixture.

**Files:**
- `crates/cli/src/tui/app.rs` (update `render_trust_prompt`)

**Current vs Expected:**

The current implementation is close but needs minor adjustments to match the fixture exactly:

Expected format:
```
────────────────────────────────────────────────────────────────────────────────
 Do you trust the files in this folder?

 /private/var/folders/t5/6tq8cxtj20z035rv8hsnzwvh0000gn/T/tmp.4wnhxcEF1K

 Claude Code may read, write, or execute files contained in this directory. This can pose security risks, so only use
 files from trusted sources.

 Learn more

 ❯ 1. Yes, proceed
   2. No, exit

 Enter to confirm · Esc to cancel
```

**Changes:**
- Verify exact text matches fixture
- Update un-ignore `test_permission_trust_folder_matches_fixture`

**Verification:**
- [ ] `test_permission_trust_folder_matches_fixture` passes

---

### Phase 5: Implement Extended Status Bar

**Goal:** Show "Use meta+t to toggle thinking" on the right side of status bar in non-default permission modes.

**Files:**
- `crates/cli/src/tui/app.rs` (update `format_status_bar`)

**Expected Format (from `status_bar_extended.txt`):**
```
  ⏵⏵ accept edits on (shift+tab to cycle)                                                                    Use meta+t to toggle thinking
```

**Changes:**

Update `format_status_bar()`:
```rust
fn format_status_bar(state: &RenderState) -> String {
    let left_text = match &state.permission_mode {
        PermissionMode::Default => "  ? for shortcuts".to_string(),
        PermissionMode::Plan => "  ⏸ plan mode on (shift+tab to cycle)".to_string(),
        PermissionMode::AcceptEdits => "  ⏵⏵ accept edits on (shift+tab to cycle)".to_string(),
        PermissionMode::BypassPermissions => "  ⏵⏵ bypass permissions on (shift+tab to cycle)".to_string(),
        // ... other modes
    };

    // Only show right side in non-default modes
    if state.permission_mode == PermissionMode::Default {
        return left_text;
    }

    // Calculate padding to right-align "Use meta+t to toggle thinking"
    let right_text = "Use meta+t to toggle thinking";
    let terminal_width = 140; // Match fixture width
    let padding = terminal_width - left_text.len() - right_text.len();
    format!("{}{:width$}{}", left_text, "", right_text, width = padding)
}
```

**Verification:**
- [ ] Extended status bar appears in non-default modes
- [ ] "Use meta+t to toggle thinking" is right-aligned
- [ ] `test_status_bar_extended_matches_fixture` passes

---

### Phase 6: Wire Up Scenario Tool Use to Permission Dialogs

**Goal:** Connect scenario tool use responses to trigger permission dialogs in the TUI.

**Files:**
- `crates/cli/src/tui/app.rs`
- `crates/cli/src/scenario.rs` (if needed)

**Implementation:**

When processing a response that contains tool use:
1. Parse tool use from response (e.g., `<tool_use>...</tool_use>` or similar format)
2. Check permission using `PermissionChecker`
3. If `NeedsPrompt` result, show rich permission dialog
4. Wait for user response
5. Continue or abort based on selection

**Test Scenarios:**

Create test scenarios that trigger each permission type:

```toml
# scenarios/test-bash-permission.toml
name = "bash-permission-test"
[[responses]]
pattern = { contains = "run command" }
response = { tool_use = { tool = "Bash", input = { command = "cat /etc/passwd | head -5", description = "Display first 5 lines" } } }

# scenarios/test-edit-permission.toml
name = "edit-permission-test"
[[responses]]
pattern = { contains = "edit file" }
response = { tool_use = { tool = "Edit", input = { file_path = "hello.txt", old_string = "Hello World", new_string = "Hello Universe" } } }
```

**Verification:**
- [ ] `test_permission_bash_command_matches_fixture` passes
- [ ] `test_permission_edit_file_matches_fixture` passes
- [ ] `test_permission_write_file_matches_fixture` passes

---

## Key Implementation Details

### Character Constants

```rust
/// Full-width horizontal separator (─)
const SEPARATOR: &str = "─".repeat(120);

/// Dashed separator for content areas (╌)
const DASHED_SEPARATOR: &str = "╌".repeat(120);

/// Selection indicator
const SELECTED: &str = " ❯ ";
const UNSELECTED: &str = "   ";
```

### Permission Option Text

| Tool Type | Option 1 | Option 2 | Option 3 |
|-----------|----------|----------|----------|
| Bash | "Yes" | "Yes, allow [action] from this project" | "No" |
| Edit | "Yes" | "Yes, allow all edits during this session (shift+tab)" | "No" |
| Write | "Yes" | "Yes, allow all edits during this session (shift+tab)" | "No" |

### Diff Rendering

```rust
fn render_diff_line(line: &DiffLine) -> String {
    let prefix = match line.kind {
        DiffKind::Removed => "-",
        DiffKind::Added => "+",
        DiffKind::Context | DiffKind::NoNewline => " ",
    };

    match line.line_num {
        Some(n) => format!(" {:2} {}{}", n, prefix, line.content),
        None => format!("    {}{}", prefix, line.content),
    }
}
```

---

## Verification Plan

### Unit Tests

**Pattern Matching (`pattern_tests.rs`):**
- [ ] `test_prefix_matches_exact` - "npm" matches "npm"
- [ ] `test_prefix_matches_with_args` - "npm" matches "npm test"
- [ ] `test_prefix_no_partial_match` - "npm" does NOT match "npx"
- [ ] `test_prefix_with_spaces` - "rm -rf" matches "rm -rf /tmp"
- [ ] `test_colon_star_parsing` - "Bash(npm:*)" parses to Prefix
- [ ] `test_file_glob_unchanged` - "Write(*.md)" still uses glob

**Permission Widget (`permission_tests.rs`):**
- [ ] `test_bash_dialog_format` - Output matches fixture
- [ ] `test_edit_dialog_format` - Output matches fixture with diff
- [ ] `test_write_dialog_format` - Output matches fixture
- [ ] `test_selection_cycling` - Up/Down changes selection
- [ ] `test_keyboard_shortcuts` - 1/2/3/y/n keys work

### Integration Tests

**TUI Permission Tests (`tui_permission.rs`):**
- [ ] `test_permission_bash_command_matches_fixture` - un-ignore, passes
- [ ] `test_permission_edit_file_matches_fixture` - un-ignore, passes
- [ ] `test_permission_write_file_matches_fixture` - un-ignore, passes
- [ ] `test_permission_trust_folder_matches_fixture` - un-ignore, passes
- [ ] `test_status_bar_extended_matches_fixture` - un-ignore, passes

### Manual Testing

1. Run `claudeless` with scenario that triggers Bash permission
2. Verify dialog matches fixture visually
3. Test all keyboard interactions (1, 2, 3, y, n, Enter, Esc, Up, Down)
4. Verify session-level permission grants work

### Final Checklist

- [ ] All ignored tests in `tui_permission.rs` pass
- [ ] Pattern syntax updated to use `:*`
- [ ] `make check` passes
- [ ] No new clippy warnings
- [ ] Documentation updated in LIMITATIONS.md (remove fixed items)
