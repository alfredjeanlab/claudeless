# Plan: Responsive Terminal Width Rendering

## Overview

Update the claudeless TUI renderer to be responsive to terminal width. Currently, all separators (`────`) and status bar formatting are hardcoded to 120 characters. This plan adds dynamic width detection and separate unit/integration tests for claudeless's responsive behavior, independent of the fixture-based comparison tests (which remain at 120 chars).

## Project Structure

Key files to modify:

```
crates/cli/src/tui/
├── app.rs              # Main rendering - SEPARATOR, status bar formatting
├── widgets/
│   └── permission.rs   # Already has make_separator(char, width) helper
└── test_helpers.rs     # Add width to test harness
```

Test files to create/modify:

```
crates/cli/src/tui/
├── separator.rs        # New module for separator generation
├── separator_tests.rs  # Unit tests for separator generation
└── responsive_tests.rs # Unit tests for width-aware rendering

crates/cli/tests/
└── tui_responsive.rs   # Integration tests at various widths
```

## Dependencies

New dependency to add to `crates/cli/Cargo.toml`:

```toml
# Terminal size detection (used by iocraft transitively, but we need direct access)
crossterm = "0.28"
```

Note: iocraft already depends on crossterm, so this adds no new transitive dependencies.

## Implementation Phases

### Phase 1: Extract Separator Generation to Dedicated Module

**Goal:** Create a reusable separator module that generates width-aware separators.

**Create `crates/cli/src/tui/separator.rs`:**

```rust
//! Width-aware separator generation for TUI rendering.

/// Default separator character (box drawing horizontal)
pub const SEPARATOR_CHAR: char = '─';

/// Compact separator character (double horizontal)
pub const COMPACT_SEPARATOR_CHAR: char = '═';

/// Light dash character for section dividers
pub const SECTION_DIVIDER_CHAR: char = '╌';

/// Generate a full-width separator line.
pub fn make_separator(width: usize) -> String {
    SEPARATOR_CHAR.to_string().repeat(width)
}

/// Generate a compact separator with centered text.
/// Format: "════...════ {text} ════...════"
pub fn make_compact_separator(text: &str, width: usize) -> String {
    let text_with_spaces = format!(" {} ", text);
    let text_len = text_with_spaces.chars().count();

    if width <= text_len {
        return text_with_spaces;
    }

    let remaining = width - text_len;
    let left_count = remaining / 2;
    let right_count = remaining - left_count;

    format!(
        "{}{}{}",
        COMPACT_SEPARATOR_CHAR.to_string().repeat(left_count),
        text_with_spaces,
        COMPACT_SEPARATOR_CHAR.to_string().repeat(right_count)
    )
}

/// Generate a section divider line.
pub fn make_section_divider(width: usize) -> String {
    SECTION_DIVIDER_CHAR.to_string().repeat(width)
}
```

**Create `crates/cli/src/tui/separator_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn make_separator_generates_correct_width() {
    assert_eq!(make_separator(10).chars().count(), 10);
    assert_eq!(make_separator(80).chars().count(), 80);
    assert_eq!(make_separator(120).chars().count(), 120);
    assert_eq!(make_separator(200).chars().count(), 200);
}

#[test]
fn make_separator_uses_correct_char() {
    let sep = make_separator(5);
    assert!(sep.chars().all(|c| c == SEPARATOR_CHAR));
}

#[test]
fn make_compact_separator_centers_text() {
    let sep = make_compact_separator("Test", 20);
    assert_eq!(sep.chars().count(), 20);
    assert!(sep.contains(" Test "));
}

#[test]
fn make_compact_separator_handles_odd_widths() {
    // Width 21 with " Test " (6 chars) = 15 remaining
    // Left: 7, Right: 8
    let sep = make_compact_separator("Test", 21);
    assert_eq!(sep.chars().count(), 21);
}

#[test]
fn make_compact_separator_handles_narrow_width() {
    // When width is smaller than text, just return the text
    let sep = make_compact_separator("Very Long Text Here", 10);
    assert!(sep.contains("Very Long Text Here"));
}

#[test]
fn make_section_divider_generates_correct_width() {
    assert_eq!(make_section_divider(50).chars().count(), 50);
    assert!(make_section_divider(50).chars().all(|c| c == SECTION_DIVIDER_CHAR));
}
```

**Verification:** `cargo test separator`

### Phase 2: Add Terminal Width State Tracking

**Goal:** Track terminal width in app state and update on resize events.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Add `terminal_width` field to `TuiAppStateInner`:

```rust
/// Current terminal width
pub terminal_width: u16,
```

2. Add default width constant:

```rust
/// Default terminal width when not detected
pub const DEFAULT_TERMINAL_WIDTH: u16 = 120;
```

3. Initialize width in `TuiAppState::new()`:

```rust
// Get initial terminal width
let terminal_width = crossterm::terminal::size()
    .map(|(w, _)| w)
    .unwrap_or(DEFAULT_TERMINAL_WIDTH);
```

4. Add width getter:

```rust
pub fn terminal_width(&self) -> u16 {
    self.inner.lock().terminal_width
}
```

5. Add width to `RenderState`:

```rust
pub terminal_width: u16,
```

6. Handle `TerminalEvent::Resize` in `use_terminal_events`:

```rust
TerminalEvent::Resize(width, _height) => {
    let mut inner = state.inner.lock();
    inner.terminal_width = width;
}
```

**Verification:** Add a test that verifies width is tracked:

```rust
#[test]
fn terminal_width_defaults_to_120() {
    let state = create_test_app();
    // In tests, terminal isn't available so default is used
    assert_eq!(state.terminal_width(), DEFAULT_TERMINAL_WIDTH);
}
```

### Phase 3: Make Separator Rendering Dynamic

**Goal:** Replace hardcoded SEPARATOR constants with dynamic generation.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Remove old constants:

```rust
// DELETE these lines:
// const SEPARATOR: &str = "────...";
// const COMPACT_SEPARATOR: &str = "══════...";
```

2. Import separator module:

```rust
use super::separator::{make_separator, make_compact_separator};
```

3. Update `render_main_content()` to pass width:

```rust
// Input area with separators
Text(content: make_separator(state.terminal_width as usize))
Text(content: input_display)
Text(content: make_separator(state.terminal_width as usize))
```

4. Update `render_conversation_area()`:

```rust
if state.is_compacted {
    let compact_text = "Conversation compacted · ctrl+o for history";
    content.push_str(&make_compact_separator(compact_text, state.terminal_width as usize));
    content.push('\n');
}
```

5. Update other render functions (`render_trust_prompt`, `render_thinking_dialog`) similarly.

**Changes to `crates/cli/src/tui/widgets/permission.rs`:**

1. Import shared separator module or keep local `make_separator` (already exists).

2. Remove duplicate `make_separator` function if consolidating.

**Verification:** Run existing tests - they should still pass as default width is 120.

### Phase 4: Make Status Bar Width-Aware

**Goal:** Update status bar formatting to use actual terminal width.

**Changes to `crates/cli/src/tui/app.rs`:**

1. Update `format_status_bar()` signature to accept width:

```rust
fn format_status_bar(state: &RenderState, width: usize) -> String {
```

2. Replace hardcoded `120` with `width` parameter:

```rust
// Before:
let total_width: usize = 120;
let padding = 120 - mode_text.len() - "Thinking off".len();

// After:
let total_width = width;
let padding = width.saturating_sub(mode_text.len() + "Thinking off".len());
```

3. Update call site:

```rust
Text(content: format_status_bar(state, state.terminal_width as usize))
```

**Verification:** Status bar should fill to terminal edge at any width.

### Phase 5: Add Unit Tests for Responsive Rendering

**Goal:** Create comprehensive unit tests for width-aware rendering.

**Create `crates/cli/src/tui/responsive_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// Test separator rendering at various widths
mod separator_rendering {
    use super::*;

    #[test]
    fn separator_width_matches_terminal() {
        for width in [80, 100, 120, 150, 200] {
            let sep = make_separator(width);
            assert_eq!(
                sep.chars().count(),
                width,
                "Separator should be {} chars wide",
                width
            );
        }
    }

    #[test]
    fn compact_separator_width_matches_terminal() {
        let text = "Conversation compacted · ctrl+o for history";
        for width in [80, 100, 120, 150, 200] {
            let sep = make_compact_separator(text, width);
            assert_eq!(
                sep.chars().count(),
                width,
                "Compact separator should be {} chars wide",
                width
            );
        }
    }
}

/// Test status bar formatting at various widths
mod status_bar_rendering {
    use super::*;

    fn create_render_state(width: u16) -> RenderState {
        RenderState {
            terminal_width: width,
            // ... other fields with defaults
        }
    }

    #[test]
    fn status_bar_fits_terminal_width() {
        for width in [80, 100, 120, 150] {
            let state = create_render_state(width);
            let bar = format_status_bar(&state, width as usize);
            assert!(
                bar.chars().count() <= width as usize,
                "Status bar should fit within {} chars, got {}",
                width,
                bar.chars().count()
            );
        }
    }

    #[test]
    fn status_bar_thinking_off_aligned_right() {
        let state = create_render_state(100);
        let bar = format_status_bar(&state, 100);
        // "Thinking off" should be at the right edge
        if bar.contains("Thinking off") {
            assert!(bar.trim_end().ends_with("Thinking off"));
        }
    }
}
```

**Verification:** `cargo test responsive`

### Phase 6: Add Integration Tests for Responsive Behavior

**Goal:** Create integration tests that verify rendering at different terminal widths.

**Create `crates/cli/tests/tui_responsive.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for responsive terminal width rendering.
//!
//! These tests verify claudeless adapts to different terminal widths,
//! independent of the fixture comparison tests (which use 120 chars).

mod common;

use common::{start_tui_ext, tmux, write_scenario, TUI_READY_PATTERN};

/// Separator should span full terminal width at 80 columns
#[test]
fn test_separator_width_80() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = "claudeless-responsive-80";

    let capture = start_tui_ext(session, &scenario, 80, 24, TUI_READY_PATTERN);
    tmux::kill_session(session);

    // Find separator line and verify width
    let separator_line = capture
        .lines()
        .find(|line| line.chars().all(|c| c == '─'))
        .expect("Should have separator line");

    assert_eq!(
        separator_line.chars().count(),
        80,
        "Separator should be 80 chars at 80-column terminal"
    );
}

/// Separator should span full terminal width at 100 columns
#[test]
fn test_separator_width_100() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = "claudeless-responsive-100";

    let capture = start_tui_ext(session, &scenario, 100, 24, TUI_READY_PATTERN);
    tmux::kill_session(session);

    let separator_line = capture
        .lines()
        .find(|line| line.chars().all(|c| c == '─'))
        .expect("Should have separator line");

    assert_eq!(separator_line.chars().count(), 100);
}

/// Separator should span full terminal width at 150 columns
#[test]
fn test_separator_width_150() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = "claudeless-responsive-150";

    let capture = start_tui_ext(session, &scenario, 150, 24, TUI_READY_PATTERN);
    tmux::kill_session(session);

    let separator_line = capture
        .lines()
        .find(|line| line.chars().all(|c| c == '─'))
        .expect("Should have separator line");

    assert_eq!(separator_line.chars().count(), 150);
}

/// Compact separator should span full width after /compact
#[test]
fn test_compact_separator_width() {
    let scenario = write_scenario(r#"{ "default_response": "ok", "trusted": true }"#);
    let session = "claudeless-compact-width";

    let capture = start_tui_ext(session, &scenario, 100, 24, TUI_READY_PATTERN);

    // Type a message and wait for response
    tmux::send_line(session, "hello");
    let _ = tmux::wait_for_content(session, "ok");

    // Type /compact
    tmux::send_line(session, "/compact");
    let capture = tmux::wait_for_content(session, "compacted");

    tmux::kill_session(session);

    // Find compact separator line
    let compact_line = capture
        .lines()
        .find(|line| line.contains("compacted"))
        .expect("Should have compact separator");

    assert_eq!(
        compact_line.chars().count(),
        100,
        "Compact separator should be 100 chars at 100-column terminal"
    );
}

/// Permission dialog separators should span full width
#[test]
fn test_permission_dialog_width() {
    // Create scenario that triggers permission prompt
    let scenario = write_scenario(r#"
    {
        "default_response": "ok",
        "trusted": false,
        "tools": [
            {
                "name": "Bash",
                "input": {"command": "ls"},
                "output": "file.txt"
            }
        ]
    }
    "#);
    let session = "claudeless-permission-width";

    let capture = start_tui_ext(session, &scenario, 100, 30, TUI_READY_PATTERN);

    // Trigger permission by typing a message
    tmux::send_line(session, "list files");
    let capture = tmux::wait_for_content(session, "Allow");

    tmux::kill_session(session);

    // Find separator in permission dialog
    let separator_line = capture
        .lines()
        .find(|line| line.chars().all(|c| c == '─') && line.chars().count() > 50)
        .expect("Should have separator line in permission dialog");

    assert_eq!(
        separator_line.chars().count(),
        100,
        "Permission separator should be 100 chars"
    );
}
```

**Verification:** `cargo test tui_responsive`

## Key Implementation Details

### Terminal Width Detection

Use crossterm for initial width detection:

```rust
use crossterm::terminal::size;

let width = size().map(|(w, _)| w).unwrap_or(DEFAULT_TERMINAL_WIDTH);
```

Handle resize events from iocraft:

```rust
TerminalEvent::Resize(width, _height) => {
    inner.terminal_width = width;
}
```

### Unicode Character Widths

Some separator characters are multi-byte but single-width:
- `─` (U+2500) - box drawing horizontal
- `═` (U+2550) - double horizontal
- `╌` (U+254C) - light triple dash

Use `.chars().count()` for visual width, not `.len()` for byte count.

### Minimum Width Handling

Establish a minimum supported width to prevent layout issues:

```rust
const MIN_TERMINAL_WIDTH: u16 = 40;

let effective_width = terminal_width.max(MIN_TERMINAL_WIDTH);
```

### Status Bar Truncation

For very narrow terminals, truncate status bar content gracefully:

```rust
if width < 60 {
    // Show abbreviated status
    return "  ?".to_string();
}
```

### Test Isolation

The responsive tests are separate from fixture comparison tests:
- Fixture tests (`tui_snapshot.rs`) - Compare against real Claude CLI at 120 chars
- Responsive tests (`tui_responsive.rs`) - Verify claudeless adapts to different widths

This ensures fixture tests remain stable while responsive behavior is tested independently.

## Verification Plan

### Unit Tests

```bash
# Run separator unit tests
cargo test separator

# Run responsive unit tests
cargo test responsive

# Run all TUI tests
cargo test tui
```

### Integration Tests

```bash
# Run responsive integration tests
cargo test tui_responsive

# Run all integration tests (including fixture comparison)
cargo test --test '*'
```

### Full Verification

```bash
# Run make check (includes all linting and tests)
make check
```

### Manual Verification

1. Start claudeless in different terminal sizes:

```bash
# In a small terminal (80x24)
resize -s 24 80; claudeless scenarios/full-featured.toml --tui

# In a medium terminal (100x30)
resize -s 30 100; claudeless scenarios/full-featured.toml --tui

# In a large terminal (150x40)
resize -s 40 150; claudeless scenarios/full-featured.toml --tui
```

2. Verify:
   - Separators span the full terminal width
   - Status bar content is properly aligned
   - Compact separator after `/compact` fills width
   - Permission dialogs have full-width separators

3. Test dynamic resizing:
   - Start claudeless at 120 cols
   - Resize terminal to 80 cols
   - Verify separators update (on next render)
   - Resize to 150 cols
   - Verify separators update

## Files Changed

| File | Action | Description |
|------|--------|-------------|
| `crates/cli/Cargo.toml` | Edit | Add crossterm dependency |
| `crates/cli/src/tui/mod.rs` | Edit | Add separator module |
| `crates/cli/src/tui/separator.rs` | Create | Separator generation logic |
| `crates/cli/src/tui/separator_tests.rs` | Create | Separator unit tests |
| `crates/cli/src/tui/app.rs` | Edit | Add width tracking, dynamic separators |
| `crates/cli/src/tui/responsive_tests.rs` | Create | Responsive rendering unit tests |
| `crates/cli/src/tui/widgets/permission.rs` | Edit | Use shared separator or pass width |
| `crates/cli/tests/tui_responsive.rs` | Create | Integration tests at various widths |
