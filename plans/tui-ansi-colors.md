# TUI ANSI Color Implementation Plan

## Overview

Add ANSI color output to the TUI initial screen to make `test_initial_state_ansi_matches_fixture` pass. This involves styling the logo, version text, model name, working directory, separators, status bar, and prompt placeholder with specific colors matching the real Claude CLI.

## Project Structure

```
crates/cli/src/
├── tui/
│   ├── app.rs           # Main component - add colors to header and status
│   ├── separator.rs     # Update to return styled separator strings
│   └── colors.rs        # NEW: Color constants and styled text helpers
└── ansi/
    └── parser.rs        # Existing ANSI utilities (no changes needed)
```

## Dependencies

No new dependencies required. Uses existing:
- `iocraft` 0.7 - Has `Color::Rgb`, `Weight::Light` for dim text
- Existing `ansi` module - For ANSI sequence types if building raw strings

## Implementation Phases

### Phase 1: Define Color Constants

Create `crates/cli/src/tui/colors.rs` with color definitions matching the fixture.

```rust
//! TUI color definitions matching real Claude CLI.

use iocraft::Color;

/// Orange for logo characters: RGB(215, 119, 87)
pub const LOGO_FG: Color = Color::Rgb { r: 215, g: 119, b: 87 };

/// Black for logo background: RGB(0, 0, 0)
pub const LOGO_BG: Color = Color::Rgb { r: 0, g: 0, b: 0 };

/// Gray for version, model, path, shortcuts: RGB(153, 153, 153)
pub const TEXT_GRAY: Color = Color::Rgb { r: 153, g: 153, b: 153 };

/// Dark gray for separator lines: RGB(136, 136, 136)
pub const SEPARATOR_GRAY: Color = Color::Rgb { r: 136, g: 136, b: 136 };
```

**Verification**: Code compiles, constants are accessible.

### Phase 2: Style Simple Text Elements

Update `render_main_content()` in `app.rs` to add colors to simple text elements:
- Status bar shortcuts (gray)
- Working directory path (gray)
- Model name (gray)
- Version text (gray)

Use iocraft's Text component with `color` property:
```rust
Text(color: TEXT_GRAY, content: format!("? for shortcuts"))
```

**Verification**: Manual test shows gray text for simple elements.

### Phase 3: Style Separator Lines

Update `separator.rs` to optionally return ANSI-styled strings, or update `app.rs` to apply styling when rendering separators.

Option A - Styled Text component:
```rust
Text(color: SEPARATOR_GRAY, weight: Weight::Light, content: make_separator(width))
```

Option B - Raw ANSI in separator function (if iocraft doesn't properly combine dim + color):
```rust
pub fn make_styled_separator(width: usize) -> String {
    format!("\x1b[2m\x1b[38;2;136;136;136m{}\x1b[0m", "─".repeat(width))
}
```

**Verification**: Separators appear in dim dark gray.

### Phase 4: Style Logo with Background Colors

The logo requires complex inline styling where some characters have a black background:
```
 ▐[bg]▛███▜[/bg]▌   Claude Code v2.1.12
▝▜[bg]█████[/bg]▛▘  Haiku 4.5 · Claude Max
  ▘▘ ▝▝    ~/Developer/claudeless
```

Approach: Build styled strings with ANSI escape codes since iocraft Text doesn't support inline background changes.

Create helper function in `colors.rs`:
```rust
/// Format logo line 1 with proper colors
pub fn styled_logo_line1(version_str: &str) -> String {
    format!(
        "{FG} ▐{BG}▛███▜{BG_RESET}▌{FG_RESET}   {BOLD}Claude Code{RESET} {GRAY}{version_str}{FG_RESET}",
        FG = "\x1b[38;2;215;119;87m",     // orange foreground
        BG = "\x1b[48;2;0;0;0m",           // black background
        BG_RESET = "\x1b[49m",             // reset background
        FG_RESET = "\x1b[39m",             // reset foreground
        BOLD = "\x1b[1m",                   // bold
        RESET = "\x1b[0m",                  // reset all
        GRAY = "\x1b[38;2;153;153;153m",   // gray foreground
    )
}

/// Format logo line 2 with proper colors
pub fn styled_logo_line2(model_str: &str) -> String {
    format!(
        "{FG}▝▜{BG}█████{BG_RESET}▛▘{FG_RESET}  {GRAY}{model_str}{FG_RESET}",
        FG = "\x1b[38;2;215;119;87m",
        BG = "\x1b[48;2;0;0;0m",
        BG_RESET = "\x1b[49m",
        FG_RESET = "\x1b[39m",
        GRAY = "\x1b[38;2;153;153;153m",
    )
}

/// Format logo line 3 with proper colors
pub fn styled_logo_line3(path_str: &str) -> String {
    format!(
        "{FG}  ▘▘ ▝▝  {FG_RESET}  {GRAY}{path_str}{FG_RESET}",
        FG = "\x1b[38;2;215;119;87m",
        FG_RESET = "\x1b[39m",
        GRAY = "\x1b[38;2;153;153;153m",
    )
}
```

Update `format_header_lines()` to use these styled functions.

**Verification**: Logo displays with orange text, black background on middle characters.

### Phase 5: Style Prompt Placeholder

The placeholder "Try ..." has special styling:
```
❯ [7mT[0;2mry "write a test for scenario.rs"[0m
```

This is:
- `❯ ` - normal
- `T` - inverse video (ANSI 7)
- `ry "..."` - dim (reset + dim, ANSI 0;2)

Create helper function:
```rust
/// Format placeholder prompt with proper styling
pub fn styled_placeholder(text: &str) -> String {
    // Assumes text starts with capital letter that gets inverse
    let first_char = text.chars().next().unwrap_or('T');
    let rest = &text[first_char.len_utf8()..];
    format!(
        "❯ {INV}{first}{RESET_DIM}{rest}{RESET}",
        INV = "\x1b[7m",
        RESET_DIM = "\x1b[0;2m",
        RESET = "\x1b[0m",
        first = first_char,
    )
}
```

**Verification**: Placeholder shows "T" in inverse, rest dimmed.

### Phase 6: Integration and Test

1. Update `format_header_lines()` in `app.rs` to use styled functions
2. Update `render_main_content()` to use styled separators and status bar
3. Update input display logic for styled placeholder
4. Remove `#[ignore]` from `test_initial_state_ansi_matches_fixture`
5. Run test and iterate on any mismatches

**Verification**: `cargo test test_initial_state_ansi_matches_fixture` passes.

## Key Implementation Details

### ANSI Escape Sequence Reference

| Code | Meaning |
|------|---------|
| `\x1b[38;2;R;G;Bm` | 24-bit foreground color |
| `\x1b[48;2;R;G;Bm` | 24-bit background color |
| `\x1b[39m` | Reset foreground |
| `\x1b[49m` | Reset background |
| `\x1b[0m` | Reset all attributes |
| `\x1b[1m` | Bold |
| `\x1b[2m` | Dim |
| `\x1b[7m` | Inverse/reverse video |
| `\x1b[0;2m` | Reset + dim combined |

### Fixture Color Values

| Element | Color | RGB |
|---------|-------|-----|
| Logo foreground | Orange | (215, 119, 87) |
| Logo background (middle chars) | Black | (0, 0, 0) |
| Version/model/path/shortcuts | Gray | (153, 153, 153) |
| Separator lines | Dark gray + dim | (136, 136, 136) |

### iocraft Color Usage

For simple cases, use iocraft's Color enum:
```rust
color: Color::Rgb { r: 153, g: 153, b: 153 }
```

For complex inline styling (logo, placeholder), use raw ANSI strings since iocraft's Text component doesn't support:
- Background colors on Text
- Inline style changes within a single Text content

### Constraint Compliance

Per `docs/LIMITATIONS.md`:
- Focus on initial screen only (other screens lower priority)
- Colors must match fixture exactly
- Prefer no colors to wrong colors
- Avoid bright/bold coloring without fixture coverage

## Verification Plan

1. **Unit tests**: Verify styled helper functions produce expected ANSI codes
   - Add tests in `colors_tests.rs` for each styled_* function

2. **Snapshot test**: The main verification
   ```bash
   cargo test test_initial_state_ansi_matches_fixture -- --nocapture
   ```

3. **Visual inspection**: Run TUI manually and compare against real Claude
   ```bash
   cargo run -- --scenario scenarios/basic.yaml --tui --claude-version 2.1.12
   ```

4. **Full test suite**: Ensure no regressions
   ```bash
   make check
   ```

5. **Diff debugging**: If test fails, use existing `diff_ansi_strings()` utility in test output to identify specific mismatches
