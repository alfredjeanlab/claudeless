# ANSI Color Test Support

**Root Feature:** `cl-1aba`

Add test infrastructure for matching ANSI color escape sequences in TUI output, enabling verification that claudeless color rendering matches real Claude Code.

## Overview

Extend the existing TUI test framework to support ANSI-aware comparison against fixtures like `initial_state_ansi.txt`. This enables testing that colors, bold/dim text, and other ANSI attributes render correctly.

## Project Structure

```
crates/cli/
├── src/
│   └── ansi/                    # NEW: ANSI parsing module
│       ├── mod.rs               # Module exports
│       ├── parser.rs            # ANSI escape sequence parser
│       └── parser_tests.rs      # Unit tests
├── tests/
│   ├── common/
│   │   ├── mod.rs               # EXTEND: Add ANSI comparison helpers
│   │   ├── mod_tests.rs         # EXTEND: Add ANSI normalization tests
│   │   └── ansi.rs              # NEW: ANSI test utilities
│   ├── tui_snapshot.rs          # EXTEND: Add ANSI color test
│   └── fixtures/tui/v2.1.12/
│       └── initial_state_ansi.txt  # EXISTS: Reference fixture
```

## Dependencies

No new dependencies required. Use existing:
- `regex = "1"` - For ANSI escape sequence pattern matching

## Implementation Phases

### Phase 1: ANSI Escape Sequence Parser

Create a module to parse ANSI escape sequences from terminal output.

**Files:**
- `crates/cli/src/ansi/mod.rs`
- `crates/cli/src/ansi/parser.rs`
- `crates/cli/src/ansi/parser_tests.rs`

**Key types:**

```rust
// crates/cli/src/ansi/parser.rs

/// Represents a parsed ANSI escape sequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnsiSequence {
    /// 24-bit RGB foreground color: ESC[38;2;R;G;Bm
    FgRgb { r: u8, g: u8, b: u8 },
    /// 24-bit RGB background color: ESC[48;2;R;G;Bm
    BgRgb { r: u8, g: u8, b: u8 },
    /// Reset foreground color: ESC[39m
    FgReset,
    /// Reset background color: ESC[49m
    BgReset,
    /// Reset all attributes: ESC[0m
    Reset,
    /// Bold: ESC[1m
    Bold,
    /// Dim: ESC[2m
    Dim,
    /// Inverse/reverse video: ESC[7m
    Inverse,
    /// Combined reset and dim: ESC[0;2m
    ResetDim,
    /// Other/unknown sequence (preserved as-is)
    Other(String),
}

/// A segment of text with its ANSI attributes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiSpan {
    pub text: String,
    pub sequences: Vec<AnsiSequence>,
}

/// Parse a string containing ANSI escape sequences into spans
pub fn parse_ansi(input: &str) -> Vec<AnsiSpan>;

/// Strip all ANSI escape sequences, returning plain text
pub fn strip_ansi(input: &str) -> String;

/// Extract only the ANSI sequences (for color comparison)
pub fn extract_sequences(input: &str) -> Vec<(usize, AnsiSequence)>;
```

**ANSI patterns to recognize (from fixture analysis):**

| Pattern | Meaning | Example |
|---------|---------|---------|
| `\x1b[38;2;R;G;Bm` | 24-bit foreground | `[38;2;215;119;87m` (orange) |
| `\x1b[48;2;R;G;Bm` | 24-bit background | `[48;2;0;0;0m` (black) |
| `\x1b[39m` | Reset foreground | |
| `\x1b[49m` | Reset background | |
| `\x1b[0m` | Reset all | |
| `\x1b[1m` | Bold | |
| `\x1b[2m` | Dim | |
| `\x1b[7m` | Inverse | |
| `\x1b[0;2m` | Reset + Dim | |

**Verification:** Unit tests in `parser_tests.rs` covering all recognized patterns.

---

### Phase 2: ANSI Normalization Utilities

Add ANSI-aware normalization to the test common module, parallel to the existing `normalize_tui()` function.

**Files:**
- `crates/cli/tests/common/ansi.rs` (new)
- `crates/cli/tests/common/mod.rs` (extend)
- `crates/cli/tests/common/mod_tests.rs` (extend)

**Key functions:**

```rust
// crates/cli/tests/common/ansi.rs

use claudeless::ansi::{parse_ansi, strip_ansi, AnsiSequence};

/// Normalize ANSI output for comparison.
///
/// Applies the same normalizations as `normalize_tui()` but preserves
/// ANSI escape sequences. Normalizations are applied to the text
/// content between ANSI sequences.
pub fn normalize_ansi_tui(input: &str, cwd: Option<&str>) -> String;

/// Compare two ANSI strings for semantic equivalence.
///
/// Returns true if both strings have:
/// 1. Same text content (after normalization)
/// 2. Same ANSI sequences at corresponding positions
pub fn compare_ansi_output(actual: &str, expected: &str, cwd: Option<&str>) -> bool;

/// Assert that ANSI-colored TUI output matches a fixture.
///
/// Like `assert_tui_matches_fixture` but compares ANSI sequences.
pub fn assert_ansi_matches_fixture(actual: &str, fixture_name: &str, cwd: Option<&str>);
```

**Normalization strategy:**

1. Parse input into `Vec<AnsiSpan>` (text + sequences)
2. Apply text normalizations (timestamps, paths, etc.) to text portions only
3. Reconstruct the string with original ANSI sequences
4. Compare reconstructed strings

**Verification:** Unit tests covering:
- ANSI sequences preserved through normalization
- Text normalization works within ANSI spans
- Color positions remain correct after normalization

---

### Phase 3: ANSI Color Comparison Logic

Implement the comparison logic that determines if two ANSI-colored outputs match semantically.

**Files:**
- `crates/cli/tests/common/ansi.rs` (extend)
- `crates/cli/tests/common/mod_tests.rs` (extend)

**Comparison rules:**

1. **Text must match** after normalization (existing behavior)
2. **ANSI sequences must match** at corresponding positions
3. **Color equivalence:** RGB colors must match exactly
4. **Attribute equivalence:** Bold, dim, inverse must match

**Diff output for failures:**

```rust
/// Generate a detailed diff showing ANSI differences
pub fn diff_ansi_strings(expected: &str, actual: &str) -> String;
```

Example failure output:
```
Line 1:
  expected: [38;2;215;119;87m ▐[48;2;0;0;0m▛███▜[49m▌[39m
  actual:   [38;2;200;100;80m ▐[48;2;0;0;0m▛███▜[49m▌[39m
  diff: foreground RGB mismatch at column 1: (215,119,87) vs (200,100,80)
```

**Verification:** Unit tests for diff generation and comparison edge cases.

---

### Phase 4: Capture ANSI Output from tmux

Extend tmux helpers to capture ANSI escape sequences (by default, `tmux capture-pane` strips them).

**Files:**
- `crates/cli/tests/common/tmux.rs` (extend)

**Key changes:**

```rust
// crates/cli/tests/common/tmux.rs

/// Capture pane content with ANSI escape sequences preserved.
/// Uses `tmux capture-pane -e` flag for escape sequences.
pub fn capture_pane_ansi(session: &str) -> String {
    let output = std::process::Command::new("tmux")
        .args(["capture-pane", "-e", "-p", "-t", session])
        .output()
        .expect("Failed to capture tmux pane with ANSI");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Wait for content to appear (ANSI-aware version)
pub fn wait_for_content_ansi(session: &str, pattern: &str) -> String;
```

**Note:** The `-e` flag tells tmux to include escape sequences in the output.

**Verification:** Manual test that `capture_pane_ansi` returns ANSI codes.

---

### Phase 5: Integration Test

Add an integration test that verifies ANSI color output matches the captured fixture.

**Files:**
- `crates/cli/tests/tui_snapshot.rs` (extend)

**Test implementation:**

```rust
// crates/cli/tests/tui_snapshot.rs

/// Compare ANSI-colored initial state against real Claude fixture
#[test]
fn test_initial_state_ansi_matches_fixture() {
    let scenario = write_scenario(
        r#"
        {
            "default_response": "Hello!",
            "trusted": true,
            "claude_version": "2.1.12"
        }
        "#,
    );

    let session = "claudeless-fixture-initial-ansi";

    // Start TUI and wait for ready (using plain text pattern)
    tmux::kill_session(session);
    tmux::new_session(session, 120, 40);
    let cmd = format!(
        "{} --scenario {} --tui",
        claudeless_bin(),
        scenario.path().display()
    );
    tmux::send_line(session, &cmd);
    tmux::wait_for_content(session, TUI_READY_PATTERN);

    // Capture with ANSI sequences
    let capture = tmux::capture_pane_ansi(session);
    tmux::kill_session(session);

    // Compare against ANSI fixture
    assert_ansi_matches_fixture(&capture, "initial_state_ansi.txt", None);
}
```

**Verification:** Test passes when run against claudeless TUI.

---

### Phase 6: Documentation and Cleanup

Update documentation and remove the TODO item.

**Files:**
- `plans/TODO.md` (update - remove completed item)
- `crates/cli/tests/fixtures/tui/CLAUDE.md` (update - document ANSI fixture usage)

**Documentation updates:**

1. Add section to `fixtures/tui/CLAUDE.md` explaining ANSI fixtures
2. Document the `_ansi.txt` naming convention
3. Add examples of how to capture new ANSI fixtures

**Verification:** Documentation is accurate and helpful.

## Key Implementation Details

### ANSI Escape Sequence Regex

The core parsing regex for ANSI sequences:

```rust
// Matches ESC [ followed by semicolon-separated numbers, ending with 'm'
const ANSI_PATTERN: &str = r"\x1b\[([0-9;]*)m";
```

Parse the parameter string to determine sequence type:
- `38;2;R;G;B` → `FgRgb { r, g, b }`
- `48;2;R;G;B` → `BgRgb { r, g, b }`
- `0` → `Reset`
- `1` → `Bold`
- `2` → `Dim`
- `7` → `Inverse`
- `39` → `FgReset`
- `49` → `BgReset`
- `0;2` → `ResetDim`

### Color Values in Fixtures

Key colors observed in `initial_state_ansi.txt`:

| Color | RGB | Usage |
|-------|-----|-------|
| Orange | `(215, 119, 87)` | Logo characters |
| Black | `(0, 0, 0)` | Logo background |
| Gray | `(153, 153, 153)` | Version, model, path, shortcuts |
| Dark gray | `(136, 136, 136)` | Separator lines |

### Normalization Interaction

When normalizing ANSI text:

1. **DO NOT** normalize the ANSI sequences themselves
2. **DO** normalize the text content between sequences
3. **Preserve** the relative positions of sequences to text

Example:
```
Input:  [38;2;153;153;153mv2.1.12[39m
Output: [38;2;153;153;153m<VERSION>[39m
```

## Verification Plan

### Unit Tests (Phase 1-3)

Run with:
```bash
cargo test -p claudeless ansi
cargo test -p claudeless --test '*' -- ansi
```

Tests should cover:
- [ ] Parsing all ANSI sequence types
- [ ] Stripping ANSI from mixed content
- [ ] Normalizing text within ANSI spans
- [ ] Comparing identical ANSI strings
- [ ] Detecting ANSI mismatches
- [ ] Generating useful diff output

### Integration Tests (Phase 5)

Run with:
```bash
cargo test -p claudeless --test tui_snapshot -- ansi
```

Tests should:
- [ ] Pass when claudeless colors match fixtures
- [ ] Fail with helpful diff when colors differ

### Full Verification

```bash
make check
```

All existing tests should continue to pass, plus new ANSI tests.
