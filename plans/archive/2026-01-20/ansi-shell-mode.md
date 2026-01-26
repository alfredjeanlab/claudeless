# Implementation Plan: ANSI Colors for Shell Mode & Permission Modes

**Root Feature:** `cl-2a19`

## Overview

Implement ANSI color support for shell mode and complete the styled status bar for all permission modes. The v2.1.15 fixtures capture the expected appearance from real Claude Code, including:

1. **Shell mode** - Status bar is hidden; input shows `❯ \!` with cursor
2. **Permission mode status bars** - Colored indicators with cycle hints:
   - Plan mode: teal icon and text
   - Accept edits: purple icon and text
   - Bypass permissions: red/pink icon and text

Current state: Shell mode is implemented but lacks ANSI tests; `format_status_bar_styled()` only handles Default mode, falling back to plain text for other modes.

## Project Structure

```
crates/cli/src/tui/
├── app.rs                    # Status bar rendering (modify format_status_bar_styled)
├── colors.rs                 # Color constants (add permission mode colors)
└── colors_tests.rs           # Color tests (add permission mode tests)

crates/cli/tests/
├── tui_shell_mode.rs         # Shell mode tests (add ANSI fixture tests)
├── tui_permission.rs         # Permission tests (add ANSI fixture tests)
├── tui_snapshot.rs           # Snapshot tests (add v2.1.15 ANSI tests)
├── common/
│   ├── mod.rs                # Test utilities (add v2.1.15 fixture loader)
│   └── ansi.rs               # ANSI test utilities (verify works with v2.1.15)
└── fixtures/tui/v2.1.15/
    ├── BEHAVIORS.md          # Documentation (exists)
    ├── shell_mode_prefix_ansi.txt
    ├── shell_mode_command_ansi.txt
    ├── permission_default_ansi.txt
    ├── permission_plan_ansi.txt
    ├── permission_accept_edits_ansi.txt
    └── permission_bypass_ansi.txt
```

## Dependencies

No new external dependencies required. Uses existing ANSI infrastructure.

## Implementation Phases

### Phase 1: Add Permission Mode Color Constants

**Goal:** Add RGB color constants for permission mode indicators to `colors.rs`.

**Files:**
- `crates/cli/src/tui/colors.rs`

**Implementation:**

```rust
// Add after existing color constants (line ~18):

/// Teal for plan mode indicator: RGB(72, 150, 140)
pub const PLAN_MODE_COLOR: (u8, u8, u8) = (72, 150, 140);

/// Purple for accept edits mode indicator: RGB(175, 135, 255)
pub const ACCEPT_EDITS_COLOR: (u8, u8, u8) = (175, 135, 255);

/// Red/pink for bypass permissions mode indicator: RGB(255, 107, 128)
pub const BYPASS_PERMISSIONS_COLOR: (u8, u8, u8) = (255, 107, 128);
```

**Verification:**
- [ ] Constants compile without errors
- [ ] Colors match BEHAVIORS.md documentation

---

### Phase 2: Add Styled Permission Mode Formatters

**Goal:** Add helper functions to format colored permission mode status bars.

**Files:**
- `crates/cli/src/tui/colors.rs`

**Implementation:**

```rust
// Add after styled_status_text (around line 165):

/// Format plan mode status bar with teal icon and gray cycle hint.
///
/// Example output:
/// `[reset]  [teal]⏸ plan mode on[gray] (shift+tab to cycle)[/fg]`
pub fn styled_plan_mode_status() -> String {
    let fg_teal = ansi::fg(PLAN_MODE_COLOR.0, PLAN_MODE_COLOR.1, PLAN_MODE_COLOR.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_teal}⏸ plan mode on{fg_gray} (shift+tab to cycle){fg_reset}",
        reset = ansi::RESET,
        fg_teal = fg_teal,
        fg_gray = fg_gray,
        fg_reset = ansi::FG_RESET,
    )
}

/// Format accept edits mode status bar with purple icon and gray cycle hint.
///
/// Example output:
/// `[reset]  [purple]⏵⏵ accept edits on[gray] (shift+tab to cycle)[/fg]`
pub fn styled_accept_edits_status() -> String {
    let fg_purple = ansi::fg(ACCEPT_EDITS_COLOR.0, ACCEPT_EDITS_COLOR.1, ACCEPT_EDITS_COLOR.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_purple}⏵⏵ accept edits on{fg_gray} (shift+tab to cycle){fg_reset}",
        reset = ansi::RESET,
        fg_purple = fg_purple,
        fg_gray = fg_gray,
        fg_reset = ansi::FG_RESET,
    )
}

/// Format bypass permissions mode status bar with red/pink icon and gray cycle hint.
///
/// Example output:
/// `[reset]  [red]⏵⏵ bypass permissions on[gray] (shift+tab to cycle)[/fg]`
pub fn styled_bypass_permissions_status() -> String {
    let fg_red = ansi::fg(BYPASS_PERMISSIONS_COLOR.0, BYPASS_PERMISSIONS_COLOR.1, BYPASS_PERMISSIONS_COLOR.2);
    let fg_gray = ansi::fg(TEXT_GRAY.0, TEXT_GRAY.1, TEXT_GRAY.2);

    format!(
        "{reset}  {fg_red}⏵⏵ bypass permissions on{fg_gray} (shift+tab to cycle){fg_reset}",
        reset = ansi::RESET,
        fg_red = fg_red,
        fg_gray = fg_gray,
        fg_reset = ansi::FG_RESET,
    )
}
```

**Verification:**
- [ ] Each function produces correctly colored output
- [ ] ANSI escape sequences match fixture format

---

### Phase 3: Update format_status_bar_styled()

**Goal:** Handle all permission modes in `format_status_bar_styled()` instead of falling back to plain text.

**Files:**
- `crates/cli/src/tui/app.rs`

**Implementation:**

Update `format_status_bar_styled()` (line 2627) to use the new color helpers:

```rust
fn format_status_bar_styled(state: &RenderState, width: usize) -> String {
    use crate::tui::colors::{
        styled_accept_edits_status, styled_bypass_permissions_status,
        styled_plan_mode_status, styled_status_text,
    };

    // Check for exit hint first (takes precedence)
    if let Some(hint) = &state.exit_hint {
        return match hint {
            ExitHint::CtrlC => "  Press Ctrl-C again to exit".to_string(),
            ExitHint::CtrlD => "  Press Ctrl-D again to exit".to_string(),
            ExitHint::Escape => "  Esc to clear again".to_string(),
        };
    }

    // Handle each permission mode with appropriate styling
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                styled_status_text("? for shortcuts")
            } else {
                // Show "Thinking off" aligned to the right
                let left = styled_status_text("? for shortcuts");
                let left_visual_width = "  ? for shortcuts".len();
                let right = "Thinking off";
                let padding = width.saturating_sub(left_visual_width + right.len());
                format!("{}{:width$}{}", left, "", right, width = padding)
            }
        }
        PermissionMode::Plan => styled_plan_mode_status(),
        PermissionMode::AcceptEdits => styled_accept_edits_status(),
        PermissionMode::BypassPermissions => styled_bypass_permissions_status(),
    }
}
```

**Verification:**
- [ ] All permission modes render with correct colors
- [ ] Default mode behavior unchanged
- [ ] Exit hints still take precedence

---

### Phase 4: Add v2.1.15 Fixture Loader Helper

**Goal:** Add a helper function to load v2.1.15 ANSI fixtures specifically.

**Files:**
- `crates/cli/tests/common/ansi.rs`

**Implementation:**

```rust
// Add after load_ansi_fixture (around line 199):

/// Load a v2.1.15 ANSI fixture file.
/// Used for ANSI color tests against the newer fixture set.
pub fn load_ansi_fixture_v2115(name: &str) -> String {
    let path = super::fixtures_dir()
        .join("v2.1.15")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load v2.1.15 ANSI fixture {:?}: {}", path, e))
}

/// Assert that ANSI-colored TUI output matches a v2.1.15 fixture.
pub fn assert_ansi_matches_fixture_v2115(actual: &str, fixture_name: &str, cwd: Option<&str>) {
    let expected = load_ansi_fixture_v2115(fixture_name);
    let normalized_actual = normalize_ansi_tui(actual, cwd);
    let normalized_expected = normalize_ansi_tui(&expected, cwd);

    if normalized_actual != normalized_expected {
        let diff = diff_ansi_strings(&normalized_expected, &normalized_actual);
        panic!(
            "ANSI TUI output does not match v2.1.15 fixture '{}'\n\n\
             === DIFF (expected vs actual) ===\n{}\n\n\
             === NORMALIZED EXPECTED (escaped) ===\n{}\n\n\
             === NORMALIZED ACTUAL (escaped) ===\n{}\n",
            fixture_name,
            diff,
            escape_ansi_for_display(&normalized_expected),
            escape_ansi_for_display(&normalized_actual)
        );
    }
}
```

**Verification:**
- [ ] Helper loads v2.1.15 fixtures correctly
- [ ] Error messages are descriptive

---

### Phase 5: Add Shell Mode ANSI Tests

**Goal:** Add tests that verify shell mode ANSI output matches v2.1.15 fixtures.

**Files:**
- `crates/cli/tests/tui_shell_mode.rs`

**Implementation:**

Add new test section at end of file:

```rust
// =============================================================================
// Shell Mode ANSI Color Tests (v2.1.15)
// =============================================================================

/// Behavior observed with: claude --version 2.1.15 (Claude Code)
///
/// Shell mode prefix ANSI output matches v2.1.15 fixture
#[test]
fn test_tui_shell_prefix_ansi_matches_fixture_v2115() {
    use common::ansi::assert_ansi_matches_fixture_v2115;

    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.15"
        }
        "#,
    );

    let session = "claudeless-shell-prefix-ansi";
    let previous = start_tui(session, &scenario);

    // Press '!' to enter shell mode
    tmux::send_keys(session, "!");
    let capture = tmux::wait_for_change(session, &previous);

    tmux::kill_session(session);

    assert_ansi_matches_fixture_v2115(&capture, "shell_mode_prefix_ansi.txt", None);
}

/// Behavior observed with: claude --version 2.1.15 (Claude Code)
///
/// Shell mode with command ANSI output matches v2.1.15 fixture
#[test]
fn test_tui_shell_command_ansi_matches_fixture_v2115() {
    use common::ansi::assert_ansi_matches_fixture_v2115;

    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.15"
        }
        "#,
    );

    let session = "claudeless-shell-command-ansi";
    let previous = start_tui(session, &scenario);

    // Enter shell mode and type a command
    tmux::send_keys(session, "!");
    tmux::wait_for_change(session, &previous);
    tmux::send_keys(session, "ls -la");
    let capture = tmux::wait_for_content(session, "ls -la");

    tmux::kill_session(session);

    assert_ansi_matches_fixture_v2115(&capture, "shell_mode_command_ansi.txt", None);
}
```

**Verification:**
- [ ] Shell mode prefix test passes
- [ ] Shell mode command test passes
- [ ] Status bar hidden in shell mode (as per fixtures)

---

### Phase 6: Add Permission Mode ANSI Tests

**Goal:** Add tests that verify permission mode ANSI output matches v2.1.15 fixtures.

**Files:**
- `crates/cli/tests/tui_permission.rs` (or new file `tui_permission_ansi.rs`)

**Implementation:**

```rust
// =============================================================================
// Permission Mode ANSI Color Tests (v2.1.15)
// =============================================================================

/// Behavior observed with: claude --version 2.1.15 (Claude Code)
///
/// Plan mode ANSI output matches v2.1.15 fixture (teal colored)
#[test]
fn test_tui_permission_plan_ansi_matches_fixture_v2115() {
    use common::ansi::assert_ansi_matches_fixture_v2115;

    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.15",
            "permission_mode": "plan"
        }
        "#,
    );

    let session = "claudeless-perm-plan-ansi";
    // Start with plan mode wait pattern
    let capture = start_tui_ext(session, &scenario, 120, 20, "plan mode on");

    tmux::kill_session(session);

    assert_ansi_matches_fixture_v2115(&capture, "permission_plan_ansi.txt", None);
}

/// Behavior observed with: claude --version 2.1.15 (Claude Code)
///
/// Accept edits mode ANSI output matches v2.1.15 fixture (purple colored)
#[test]
fn test_tui_permission_accept_edits_ansi_matches_fixture_v2115() {
    use common::ansi::assert_ansi_matches_fixture_v2115;

    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.15",
            "permission_mode": "accept-edits"
        }
        "#,
    );

    let session = "claudeless-perm-accept-ansi";
    let capture = start_tui_ext(session, &scenario, 120, 20, "accept edits on");

    tmux::kill_session(session);

    assert_ansi_matches_fixture_v2115(&capture, "permission_accept_edits_ansi.txt", None);
}

/// Behavior observed with: claude --version 2.1.15 (Claude Code)
///
/// Bypass permissions mode ANSI output matches v2.1.15 fixture (red/pink colored)
#[test]
fn test_tui_permission_bypass_ansi_matches_fixture_v2115() {
    use common::ansi::assert_ansi_matches_fixture_v2115;

    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.15",
            "permission_mode": "bypass-permissions"
        }
        "#,
    );

    let session = "claudeless-perm-bypass-ansi";
    let capture = start_tui_ext(session, &scenario, 120, 20, "bypass permissions on");

    tmux::kill_session(session);

    assert_ansi_matches_fixture_v2115(&capture, "permission_bypass_ansi.txt", None);
}
```

**Verification:**
- [ ] Plan mode shows teal (72, 150, 140) indicator
- [ ] Accept edits shows purple (175, 135, 255) indicator
- [ ] Bypass permissions shows red/pink (255, 107, 128) indicator
- [ ] All tests match v2.1.15 fixtures

---

## Key Implementation Details

### ANSI Color Codes Reference

From the v2.1.15 fixtures:

| Element | RGB | ANSI Escape |
|---------|-----|-------------|
| Logo (orange) | (215, 119, 87) | `[38;2;215;119;87m` |
| Text (gray) | (153, 153, 153) | `[38;2;153;153;153m` |
| Separator (dark gray) | (136, 136, 136) | `[38;2;136;136;136m` |
| Plan mode (teal) | (72, 150, 140) | `[38;2;72;150;140m` |
| Accept edits (purple) | (175, 135, 255) | `[38;2;175;135;255m` |
| Bypass permissions (red) | (255, 107, 128) | `[38;2;255;107;128m` |

### Shell Mode Status Bar Behavior

Per v2.1.15 fixtures, when in shell mode:
- **Status bar is empty** (line 8 shows just `[0m`)
- Input shows `❯ \!` with cursor block `[7m [0m` after

This differs from normal mode where status bar shows "? for shortcuts" or permission mode indicator.

### Permission Mode Status Bar Format

Each non-default permission mode follows this pattern:
```
[0m]  [mode-color]{icon} {text}[gray-color] (shift+tab to cycle)[39m]
```

Where:
- `[0m]` resets from previous line's styling
- Two spaces indent
- Mode-specific color followed by icon and status text
- Gray color for the cycle hint
- `[39m]` resets foreground at end

### Test Fixture Normalization

The ANSI test infrastructure normalizes:
- Version strings → `<VERSION>`
- Paths → `<PATH>`
- Model names → `<MODEL>`
- Timestamps → `<TIME>`
- Placeholder prompts → `<PLACEHOLDER>`
- Trailing `[39m]` sequences (iocraft optimization)

---

## Verification Plan

### Unit Tests

**Colors (`colors_tests.rs`):**
- [ ] `test_styled_plan_mode_status_contains_teal`
- [ ] `test_styled_accept_edits_status_contains_purple`
- [ ] `test_styled_bypass_permissions_status_contains_red`
- [ ] `test_permission_mode_colors_match_spec`

### Integration Tests

**Shell Mode ANSI (`tui_shell_mode.rs`):**
- [ ] `test_tui_shell_prefix_ansi_matches_fixture_v2115`
- [ ] `test_tui_shell_command_ansi_matches_fixture_v2115`

**Permission Mode ANSI (`tui_permission.rs`):**
- [ ] `test_tui_permission_plan_ansi_matches_fixture_v2115`
- [ ] `test_tui_permission_accept_edits_ansi_matches_fixture_v2115`
- [ ] `test_tui_permission_bypass_ansi_matches_fixture_v2115`

### Final Checklist

- [ ] `make check` passes
- [ ] All ANSI color tests pass
- [ ] Colors match v2.1.15 BEHAVIORS.md spec
- [ ] No regressions in existing tests
- [ ] Shell mode hides status bar (per fixtures)
- [ ] Permission modes show colored indicators

---

## Appendix: v2.1.15 Fixture Excerpt

### shell_mode_prefix_ansi.txt (line 6-8)
```
[0m❯ \![7m [0m
[2m[38;2;136;136;136m────...
[0m
```
Note: Empty status bar line (just `[0m]`)

### permission_plan_ansi.txt (line 8)
```
[0m  [38;2;72;150;140m⏸ plan mode on[38;2;153;153;153m (shift+tab to cycle)[39m
```

### permission_accept_edits_ansi.txt (line 8)
```
[0m  [38;2;175;135;255m⏵⏵ accept edits on[38;2;153;153;153m (shift+tab to cycle)[39m
```

### permission_bypass_ansi.txt (line 8)
```
[0m  [38;2;255;107;128m⏵⏵ bypass permissions on[38;2;153;153;153m (shift+tab to cycle)[39m
```
