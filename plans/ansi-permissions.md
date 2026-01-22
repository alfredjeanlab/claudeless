# ANSI Colors for Permission Modes

**Branch:** `feature/ansi-permissions`

Add ANSI color rendering to permission mode indicators in the TUI status bar, matching the exact output from Claude Code v2.1.15.

## Overview

The status bar currently renders permission modes (Plan, Accept Edits, Bypass Permissions) as plain text when calling `format_status_bar_styled()`. The v2.1.15 fixtures show that each mode should have a distinct color:

| Mode | Color | RGB |
|------|-------|-----|
| Default | Gray | `(153, 153, 153)` |
| Plan | Teal | `(72, 150, 140)` |
| Accept Edits | Purple | `(175, 135, 255)` |
| Bypass Permissions | Red/Pink | `(255, 107, 128)` |

The mode icon and text use the mode's color, while the cycle hint "(shift+tab to cycle)" uses gray.

## Project Structure

```
crates/cli/
├── src/tui/
│   ├── colors.rs          # MODIFY: Add permission mode color constants
│   └── app.rs             # MODIFY: Update format_status_bar_styled()
└── tests/
    ├── tui_permission.rs  # MODIFY: Add ANSI fixture tests
    └── fixtures/tui/v2.1.15/
        ├── permission_default_ansi.txt      # EXISTS: Reference fixture
        ├── permission_plan_ansi.txt         # EXISTS: Reference fixture
        ├── permission_accept_edits_ansi.txt # EXISTS: Reference fixture
        └── permission_bypass_ansi.txt       # EXISTS: Reference fixture
```

## Dependencies

No new dependencies required. Uses existing:
- `colors.rs` module for ANSI escape sequence generation

## Implementation Phases

### Phase 1: Add Permission Mode Color Constants

Add color constants for each permission mode to `colors.rs`.

**File:** `crates/cli/src/tui/colors.rs`

**Changes:**

```rust
// Permission mode colors (from v2.1.15 fixtures)
pub const PLAN_MODE: (u8, u8, u8) = (72, 150, 140);         // Teal
pub const ACCEPT_EDITS_MODE: (u8, u8, u8) = (175, 135, 255); // Purple
pub const BYPASS_MODE: (u8, u8, u8) = (255, 107, 128);       // Red/Pink
```

**Verification:** `cargo build` succeeds, constants are accessible.

---

### Phase 2: Implement Styled Permission Status Function

Add a helper function to generate ANSI-colored permission status strings.

**File:** `crates/cli/src/tui/colors.rs`

**Add function:**

```rust
use crate::permission::PermissionMode;

/// Generate styled permission status text with ANSI colors.
///
/// Format for non-default modes:
/// `[mode_color][icon] [mode_text][gray] (shift+tab to cycle)[reset]`
pub fn styled_permission_status(mode: &PermissionMode) -> String {
    match mode {
        PermissionMode::Default => {
            format!("{}? for shortcuts{}", fg_gray(), FG_RESET)
        }
        PermissionMode::Plan => {
            let (r, g, b) = PLAN_MODE;
            format!(
                "{}⏸ plan mode on{} (shift+tab to cycle){}",
                fg(r, g, b),
                fg_gray(),
                FG_RESET
            )
        }
        PermissionMode::AcceptEdits => {
            let (r, g, b) = ACCEPT_EDITS_MODE;
            format!(
                "{}⏵⏵ accept edits on{} (shift+tab to cycle){}",
                fg(r, g, b),
                fg_gray(),
                FG_RESET
            )
        }
        PermissionMode::BypassPermissions => {
            let (r, g, b) = BYPASS_MODE;
            format!(
                "{}⏵⏵ bypass permissions on{} (shift+tab to cycle){}",
                fg(r, g, b),
                fg_gray(),
                FG_RESET
            )
        }
        // Delegate and DontAsk modes use gray (same as default cycle hint)
        PermissionMode::Delegate => {
            format!(
                "{}delegate mode (shift+tab to cycle){}",
                fg_gray(),
                FG_RESET
            )
        }
        PermissionMode::DontAsk => {
            format!(
                "{}don't ask mode (shift+tab to cycle){}",
                fg_gray(),
                FG_RESET
            )
        }
    }
}

/// Helper to generate gray foreground ANSI sequence
fn fg_gray() -> String {
    let (r, g, b) = TEXT_GRAY;
    fg(r, g, b)
}
```

**Verification:** Unit tests for each permission mode's colored output.

---

### Phase 3: Update format_status_bar_styled()

Replace the fallback `_ => format_status_bar(state, width)` with proper ANSI-colored rendering for all permission modes.

**File:** `crates/cli/src/tui/app.rs`

**Current code (lines ~2627-2653):**

```rust
fn format_status_bar_styled(state: &TuiAppState, width: usize) -> String {
    match &state.permission_mode {
        PermissionMode::Default => {
            if state.thinking_enabled {
                styled_status_text("? for shortcuts")
            } else {
                // Show "Thinking off" aligned to the right
                ...
            }
        }
        _ => format_status_bar(state, width),  // <-- REMOVE THIS FALLBACK
    }
}
```

**Updated code:**

```rust
fn format_status_bar_styled(state: &TuiAppState, width: usize) -> String {
    use crate::tui::colors::styled_permission_status;

    let status = styled_permission_status(&state.permission_mode);

    // Handle thinking indicator for default mode
    if matches!(state.permission_mode, PermissionMode::Default) && !state.thinking_enabled {
        // ... existing thinking off logic ...
    }

    format!("{}{}", RESET, status)
}
```

**Key considerations:**

1. The status bar starts with `[0m` (RESET) based on fixtures
2. Two leading spaces before the content: `"  [38;2;..."`
3. The ANSI sequences must match the fixture format exactly

**Verification:** Visual inspection in TUI, comparison with fixtures.

---

### Phase 4: Add ANSI Fixture Tests

Add integration tests that compare the styled status bar output against the v2.1.15 ANSI fixtures.

**File:** `crates/cli/tests/tui_permission.rs`

**Add tests:**

```rust
#[test]
fn test_permission_default_ansi_matches_fixture() {
    // Start TUI with default permission mode
    // Capture with ANSI sequences
    // Compare against permission_default_ansi.txt
}

#[test]
fn test_permission_plan_ansi_matches_fixture() {
    // Start TUI, cycle to plan mode
    // Capture with ANSI sequences
    // Compare against permission_plan_ansi.txt
}

#[test]
fn test_permission_accept_edits_ansi_matches_fixture() {
    // Start TUI, cycle to accept edits mode
    // Capture with ANSI sequences
    // Compare against permission_accept_edits_ansi.txt
}

#[test]
fn test_permission_bypass_ansi_matches_fixture() {
    // Start TUI with bypass enabled, cycle to bypass mode
    // Capture with ANSI sequences
    // Compare against permission_bypass_ansi.txt
}
```

**Note:** Use existing `assert_ansi_matches_fixture()` helper from `common/ansi.rs`.

**Verification:** All ANSI tests pass.

---

### Phase 5: Verification and Cleanup

Run full test suite and verify visual output matches Claude Code v2.1.15.

**Commands:**

```bash
make check
```

**Manual verification:**

1. Start claudeless TUI
2. Verify default mode shows gray "? for shortcuts"
3. Press Shift+Tab, verify teal "⏸ plan mode on"
4. Press Shift+Tab, verify purple "⏵⏵ accept edits on"
5. With bypass enabled, verify red "⏵⏵ bypass permissions on"

**Verification:** All tests pass, visual output matches fixtures.

## Key Implementation Details

### ANSI Sequence Format

The fixtures use 24-bit RGB color sequences:

```
\x1b[38;2;R;G;Bm  - Set foreground to RGB color
\x1b[39m          - Reset foreground
\x1b[0m           - Reset all attributes
```

### Status Bar Line Format

From fixtures, the status bar line structure is:

```
[0m  [mode_color][icon] [mode_text][gray] (shift+tab to cycle)[39m
    ↑                                                         ↑
    2 spaces                                            fg reset
```

### Unicode Width Considerations

The icons ⏸ (U+23F8) and ⏵ (U+23F5) each have display width of 1. When ⏵⏵ is used, it's 2 characters displayed.

### Fixture Reference (permission_plan_ansi.txt line 8)

```
[0m  [38;2;72;150;140m⏸ plan mode on[38;2;153;153;153m (shift+tab to cycle)[39m
```

Breakdown:
- `[0m` - Reset all
- `  ` - Two spaces
- `[38;2;72;150;140m` - Teal foreground
- `⏸ plan mode on` - Icon and text
- `[38;2;153;153;153m` - Gray foreground
- ` (shift+tab to cycle)` - Cycle hint (note leading space)
- `[39m` - Reset foreground

## Verification Plan

### Unit Tests

```bash
cargo test -p claudeless -- permission
cargo test -p claudeless -- colors
```

### Integration Tests

```bash
cargo test -p claudeless --test tui_permission
```

### Full Suite

```bash
make check
```

### Expected Results

- [ ] Color constants defined in `colors.rs`
- [ ] `styled_permission_status()` returns correct ANSI for each mode
- [ ] `format_status_bar_styled()` uses colored output for all modes
- [ ] All ANSI fixture tests pass
- [ ] Visual output matches Claude Code v2.1.15
- [ ] `make check` passes
