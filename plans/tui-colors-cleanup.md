# TUI Colors Cleanup Plan

**Root Feature:** `cl-ee5e`

## Overview

Review and clean up the TUI ANSI colors implementation to eliminate code duplication, centralize color handling, enhance test coverage, and ensure consistency with fixture expectations. The goal is to make the color system more maintainable while verifying all color output matches captured fixtures.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs              # Main TUI component (has inline ANSI at line 2479)
│   ├── colors.rs           # Color constants + styled text helpers (241 lines)
│   ├── colors_tests.rs     # Unit tests for color helpers (102 lines)
│   └── mod.rs              # Module exports
├── ansi/
│   ├── mod.rs              # ANSI module exports
│   ├── parser.rs           # ANSI sequence parser (221 lines)
│   └── parser_tests.rs     # Parser unit tests
crates/cli/tests/
├── tui_snapshot.rs         # Snapshot tests including ANSI test
├── common/
│   └── ansi.rs             # ANSI-aware test utilities (355 lines)
└── fixtures/tui/v2.1.12/
    └── initial_state_ansi.txt  # Reference ANSI fixture
```

## Dependencies

No new dependencies required. Uses existing:
- `iocraft` 0.7 - TUI rendering framework
- `regex` - For ANSI sequence parsing

## Implementation Phases

### Phase 1: Audit and Document Current State

Verify the current implementation and document any issues.

**Tasks:**
1. Run `test_initial_state_ansi_matches_fixture` to verify it passes
2. Run `make check` to establish baseline
3. Document all locations where ANSI escape codes are generated:
   - `crates/cli/src/tui/colors.rs` (private `ansi` module)
   - `crates/cli/src/tui/app.rs` line 2479-2480 (hardcoded)
   - `crates/cli/src/ansi/parser.rs` (`AnsiSequence::to_escape_code`)

**Verification:**
```bash
cargo test test_initial_state_ansi_matches_fixture -- --nocapture
make check
```

### Phase 2: Centralize ANSI Escape Code Generation

Consolidate duplicated ANSI escape code generation into a single location.

**Tasks:**
1. Make the `ansi` module in `colors.rs` public as `pub mod escape`
2. Update `app.rs` `render_stash_indicator()` to use `colors::escape::fg()` instead of inline format string
3. Consider whether `AnsiSequence::to_escape_code()` in parser.rs should delegate to the centralized module (evaluate trade-off: parser may need to remain self-contained for testing)

**Before (app.rs:2477-2480):**
```rust
let (r, g, b) = super::colors::LOGO_FG;
let accent_fg = format!("\x1b[38;2;{};{};{}m", r, g, b);
let reset = "\x1b[0m";
```

**After:**
```rust
use super::colors::escape;
let accent_fg = escape::fg(LOGO_FG.0, LOGO_FG.1, LOGO_FG.2);
let reset = escape::RESET;
```

**Verification:**
```bash
cargo test test_initial_state_ansi_matches_fixture
cargo clippy --all-targets
```

### Phase 3: Eliminate Dead Code and Unused Constants

Review and remove any unused color constants or helper functions.

**Tasks:**
1. Check if all color constants are used:
   - `LOGO_FG` - used in styled_logo_* and stash indicator
   - `LOGO_BG` - used in styled_logo_line1, styled_logo_line2
   - `TEXT_GRAY` - used in multiple styled_* functions
   - `SEPARATOR_GRAY` - used in styled_separator
   - `PLAN_MODE`, `ACCEPT_EDITS_MODE`, `BYPASS_MODE` - used in styled_permission_status
2. Check if all `ansi` module constants are used:
   - `FG_RESET`, `BG_RESET`, `RESET`, `BOLD`, `DIM`, `INVERSE`, `RESET_DIM`
3. Use `cargo clippy` and `#[deny(dead_code)]` to identify unused items
4. Remove any identified dead code

**Verification:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

### Phase 4: Enhance Unit Test Coverage

Add comprehensive tests for color rendering and edge cases.

**Tasks:**
1. Add tests for `styled_permission_status()` for all permission modes:
   - `PermissionMode::Default`
   - `PermissionMode::Plan`
   - `PermissionMode::AcceptEdits`
   - `PermissionMode::BypassPermissions`
   - `PermissionMode::Delegate`
   - `PermissionMode::DontAsk`
2. Add tests verifying exact ANSI sequence output matches fixture format
3. Add tests for edge cases:
   - Empty input to styled functions
   - Unicode characters in paths/text
   - Very long separator widths

**New tests to add to `colors_tests.rs`:**
```rust
#[test]
fn styled_permission_status_plan_mode() {
    let status = styled_permission_status(&PermissionMode::Plan);
    assert!(status.contains("\x1b[38;2;72;150;140m")); // Teal color
    assert!(status.contains("plan mode on"));
}

#[test]
fn styled_permission_status_accept_edits_mode() {
    let status = styled_permission_status(&PermissionMode::AcceptEdits);
    assert!(status.contains("\x1b[38;2;175;135;255m")); // Purple color
    assert!(status.contains("accept edits on"));
}

#[test]
fn styled_permission_status_bypass_mode() {
    let status = styled_permission_status(&PermissionMode::BypassPermissions);
    assert!(status.contains("\x1b[38;2;255;107;128m")); // Red/Pink color
    assert!(status.contains("bypass permissions on"));
}

#[test]
fn styled_placeholder_handles_unicode() {
    let placeholder = styled_placeholder("Tëst");
    assert!(placeholder.contains("\x1b[7mT")); // Inverse on T
}

#[test]
fn styled_logo_line3_handles_long_paths() {
    let long_path = "~/".to_string() + &"a".repeat(200);
    let line = styled_logo_line3(&long_path);
    assert!(line.contains(&long_path));
}
```

**Verification:**
```bash
cargo test colors -- --nocapture
```

### Phase 5: Add Terminal Capability Detection (Optional)

Add infrastructure for graceful degradation when terminal doesn't support 24-bit color.

**Tasks:**
1. Add `supports_true_color()` function to detect terminal capabilities
2. Add `ColorMode` enum: `TrueColor`, `Color256`, `Basic`, `None`
3. Update styled functions to accept optional color mode parameter
4. Default to `TrueColor` for backward compatibility

**Note:** This phase may be deferred if fixture tests are the primary verification method and all test environments support true color.

**Sketch:**
```rust
pub enum ColorMode {
    TrueColor,  // 24-bit RGB
    Color256,   // 256 color palette
    Basic,      // 8/16 basic colors
    None,       // No colors
}

pub fn detect_color_mode() -> ColorMode {
    // Check COLORTERM, TERM environment variables
    if std::env::var("COLORTERM").map_or(false, |v| v == "truecolor" || v == "24bit") {
        return ColorMode::TrueColor;
    }
    // ... fallback logic
}
```

**Verification:**
```bash
COLORTERM=truecolor cargo test test_initial_state_ansi_matches_fixture
NO_COLOR=1 cargo run -- --scenario scenarios/basic.yaml --tui
```

### Phase 6: Final Verification and Cleanup

Ensure all tests pass and no regressions exist.

**Tasks:**
1. Run full test suite with `make check`
2. Visually inspect TUI output matches expectations
3. Verify `test_initial_state_ansi_matches_fixture` passes
4. Ensure no colors are applied without fixture coverage
5. Update documentation if needed

**Verification:**
```bash
make check
cargo test test_initial_state_ansi_matches_fixture -- --nocapture
cargo run -- --scenario scenarios/basic.yaml --tui --claude-version 2.1.12
```

## Key Implementation Details

### Current ANSI Duplication Points

| Location | What it generates | Should use |
|----------|-------------------|------------|
| `colors.rs:33-34` | `fg(r,g,b)` function | Canonical source |
| `colors.rs:38-39` | `bg(r,g,b)` function | Canonical source |
| `colors.rs:43-61` | Reset/style constants | Canonical source |
| `app.rs:2479` | Inline `format!` for fg | Should use `colors::escape::fg()` |
| `parser.rs:90-103` | `to_escape_code()` | Self-contained for parser (acceptable) |

### Color Constants Reference

| Constant | RGB | Usage |
|----------|-----|-------|
| `LOGO_FG` | (215, 119, 87) | Orange logo characters |
| `LOGO_BG` | (0, 0, 0) | Black background on middle logo chars |
| `TEXT_GRAY` | (153, 153, 153) | Version, model, path, shortcuts |
| `SEPARATOR_GRAY` | (136, 136, 136) | Separator lines (+ dim) |
| `PLAN_MODE` | (72, 150, 140) | Teal for plan mode indicator |
| `ACCEPT_EDITS_MODE` | (175, 135, 255) | Purple for accept edits indicator |
| `BYPASS_MODE` | (255, 107, 128) | Red/pink for bypass indicator |

### ANSI Sequence Reference

| Code | Meaning | Constant |
|------|---------|----------|
| `\x1b[38;2;R;G;Bm` | 24-bit foreground | `fg(r,g,b)` |
| `\x1b[48;2;R;G;Bm` | 24-bit background | `bg(r,g,b)` |
| `\x1b[39m` | Reset foreground | `FG_RESET` |
| `\x1b[49m` | Reset background | `BG_RESET` |
| `\x1b[0m` | Reset all | `RESET` |
| `\x1b[1m` | Bold | `BOLD` |
| `\x1b[2m` | Dim | `DIM` |
| `\x1b[7m` | Inverse | `INVERSE` |
| `\x1b[0;2m` | Reset+dim | `RESET_DIM` |

## Verification Plan

1. **Baseline verification**: Run existing tests before changes
   ```bash
   make check
   ```

2. **After each phase**: Run incremental tests
   ```bash
   cargo test test_initial_state_ansi_matches_fixture -- --nocapture
   cargo clippy --all-targets --all-features -- -D warnings
   ```

3. **Unit test coverage**: Verify all styled functions have tests
   ```bash
   cargo test colors -- --nocapture
   ```

4. **Full regression check**: After all changes complete
   ```bash
   make check
   ```

5. **Visual verification**: Manual inspection
   ```bash
   cargo run -- --scenario scenarios/basic.yaml --tui --claude-version 2.1.12
   ```
