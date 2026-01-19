# Epic 12e: Fix TUI Visual Fidelity - Part 2

## Overview

Complete the TUI visual fidelity work started in Part 1. This epic focuses on:
1. Matching exact visual output to real Claude CLI fixtures
2. Removing backward compatibility code
3. Enabling all ignored fixture comparison tests
4. Consolidating and cleaning up test infrastructure

**Commit reviewed:** 062f022 (feat(tui): Migrate from ratatui/crossterm to iocraft)

## Current State Analysis

### Completed in Part 1
- ✅ iocraft dependency migration
- ✅ App component rewritten with hooks/element! macro
- ✅ Basic trust prompt with `❯` cursor and numbered options
- ✅ Test infrastructure (`normalize_tui`, `assert_tui_matches_fixture`)
- ✅ Fixture files captured from real Claude CLI v2.1.12

### Remaining Gaps
1. **Visual divergences** - Text/layout doesn't match fixtures exactly
2. **Ignored tests** - 6 fixture comparison tests still `#[ignore]`
3. **Backward compatibility files** - `input.rs`, `layout.rs`, `test_helpers.rs` kept for compat
4. **Widget stubs** - Widget files are now just type re-exports

---

## Phase 1: Fix Trust Prompt Visual Fidelity

**Goal:** Make trust prompt output match `trust_prompt.txt` exactly.

### Current vs Expected

**Expected (from fixture):**
```
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 Do you trust the files in this folder?

 /private/var/folders/...

 Claude Code may read, write, or execute files contained in this directory. This can pose security risks, so only use
 files from trusted sources.

 Learn more

 ❯ 1. Yes, proceed
   2. No, exit

 Enter to confirm · Esc to cancel
```

**Current (from app.rs:1076-1121):**
- Uses border box around content
- Different text: "Trusting a folder allows Claude to read and modify..."
- Missing "Learn more" link
- Has "Trust Folder" title in border

### Task Agent Instructions

```
Update render_trust_prompt() in src/tui/app.rs:

1. Remove the bordered View container
2. Use full-width horizontal rule (─) at top
3. Update text content to match fixture exactly:
   - "Do you trust the files in this folder?"
   - Working directory path
   - "Claude Code may read, write, or execute files contained in this directory. This can pose security risks, so only use"
   - "files from trusted sources."
   - Empty line
   - "Learn more" (plain text, not a link)
   - Empty line
   - "❯ 1. Yes, proceed" / "  2. No, exit"
   - Empty line
   - "Enter to confirm · Esc to cancel"

4. Ensure proper spacing (single space indent on all content lines)
5. No border, no title - just horizontal rule separator
```

### Validation

- [ ] Run `cargo test -p claudeless --test tui_fixture_comparison test_trust_prompt_matches_fixture -- --ignored`
- [ ] Test passes (remove `#[ignore]` after confirmed)
- [ ] Visual inspection matches fixture

---

## Phase 2: Fix Initial State Layout

**Goal:** Match `initial_state.txt` - the default TUI when starting.

### Expected Layout (from fixture)
```
 ▐▛███▜▌   Claude Code v2.1.12
▝▜█████▛▘  Haiku 4.5 · Claude Max
  ▘▘ ▝▝    ~/Developer/claudeless

────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
❯ Try "refactor mod.rs"
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  ? for shortcuts
```

### Task Agent Instructions

```
Update render_main_content() in src/tui/app.rs:

1. Header section (3 lines):
   - Line 1: " ▐▛███▜▌   Claude Code v{version}"
   - Line 2: "▝▜█████▛▘  {model} · Claude Max"
   - Line 3: "  ▘▘ ▝▝    {working_directory}"

2. Add empty line after header

3. Input area:
   - Top separator: full-width "─" characters (120 chars)
   - Input line: "❯ " + placeholder or user input
   - Bottom separator: full-width "─" characters

4. Status bar:
   - "  ? for shortcuts" (when default mode)
   - Permission mode indicators per README.md

5. Remove bordered Views - use raw Text with separators
6. Add placeholder text when input is empty: 'Try "refactor mod.rs"'
```

### Validation

- [ ] Run `cargo test -p claudeless --test tui_fixture_comparison test_initial_state_matches_fixture -- --ignored`
- [ ] Test passes (remove `#[ignore]` after confirmed)

---

## Phase 3: Fix Response Display

**Goal:** Match `after_response.txt` format.

### Expected Format
```
❯ Say hello in exactly 3 words

⏺ Hello there friend.
```

### Task Agent Instructions

```
Update response formatting in src/tui/app.rs:

1. User prompts display with "❯ " prefix (not "{username} > ")
2. Claude responses display with "⏺ " prefix
3. Conversation history shows alternating user/assistant messages
4. No borders around response area
5. Preserve the separator lines around input area only
```

### Validation

- [ ] Run `cargo test -p claudeless --test tui_fixture_comparison test_response_format_matches_fixture -- --ignored`
- [ ] Test passes (remove `#[ignore]` after confirmed)

---

## Phase 4: Fix Thinking Dialog

**Goal:** Match `thinking_dialog.txt` format.

### Expected Format (from fixture)
```
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 Toggle thinking mode
 Enable or disable thinking for this session.

 ❯ 1. Enabled ✔  Claude will think before responding
   2. Disabled   Claude will respond without extended thinking

 Enter to confirm · escape to exit
```

### Task Agent Instructions

```
Update render_thinking_dialog() in src/tui/app.rs:

1. Use horizontal rule separator at top (not border)
2. Two-line header:
   - "Toggle thinking mode"
   - "Enable or disable thinking for this session."
3. Options with descriptions:
   - "❯ 1. Enabled ✔  Claude will think before responding"
   - "  2. Disabled   Claude will respond without extended thinking"
4. Footer: "Enter to confirm · escape to exit" (lowercase 'escape')
5. Remove bordered View container
```

### Validation

- [ ] Run `cargo test -p claudeless --test tui_fixture_comparison test_thinking_dialog_matches_fixture -- --ignored`
- [ ] Test passes (remove `#[ignore]` after confirmed)

---

## Phase 5: Fix Permission Mode Display

**Goal:** Match `permission_default.txt` and `permission_plan.txt`.

### Task Agent Instructions

```
Update format_status_bar() in src/tui/app.rs:

1. Default mode: "  ? for shortcuts"
2. Plan mode: "  ⏸ plan mode on (shift+tab to cycle)"
3. Accept edits: "  ⏵⏵ accept edits on (shift+tab to cycle)"
4. Bypass: "  ⏵⏵ bypass permissions on (shift+tab to cycle)"

Note the two-space indent at start of status bar.
```

### Validation

- [ ] Run `cargo test -p claudeless --test tui_fixture_comparison test_permission_default_matches_fixture -- --ignored`
- [ ] Run `cargo test -p claudeless --test tui_fixture_comparison test_permission_plan_matches_fixture -- --ignored`
- [ ] Both tests pass (remove `#[ignore]` after confirmed)

---

## Phase 6: Remove Backward Compatibility Code

**Goal:** Delete files that are no longer needed and consolidate code.

### Files to Delete

1. **`src/tui/input.rs`** - Only contains tests using TuiTestHarness
   - Move tests to `src/tui/test_helpers.rs` or delete if redundant

2. **`src/tui/layout.rs`** - Only contains `TUI_WIDTH`/`TUI_HEIGHT` constants
   - Move constants to `app.rs` or delete if unused

3. **Widget stub files** (if only re-exporting types):
   - `src/tui/widgets/input.rs` - check if needed
   - `src/tui/widgets/response.rs` - check if needed
   - `src/tui/widgets/status.rs` - check if needed
   - `src/tui/widgets/permission.rs` - check if needed

### Task Agent Instructions

```
1. Check each file for actual usage:
   grep -r "tui::input::" crates/cli/src/
   grep -r "tui::layout::" crates/cli/src/
   grep -r "widgets::input::" crates/cli/src/
   etc.

2. For each unused module:
   - Remove from src/tui/mod.rs exports
   - Delete the file
   - Fix any compilation errors

3. If tests in input.rs are valuable, move them to test_helpers.rs

4. Update src/tui/mod.rs to reflect new structure

5. Ensure no dead_code warnings remain (remove #[allow(dead_code)])
```

### Validation

- [ ] `cargo check -p claudeless` succeeds with no warnings
- [ ] `cargo test -p claudeless` passes
- [ ] No `#[allow(dead_code)]` annotations in tui module
- [ ] Files deleted: `input.rs`, `layout.rs`, widget stubs

---

## Phase 7: Enable All Fixture Tests

**Goal:** Remove `#[ignore]` from all fixture comparison tests.

### Task Agent Instructions

```
1. In tui_fixture_comparison.rs, remove #[ignore] from:
   - test_trust_prompt_matches_fixture
   - test_initial_state_matches_fixture
   - test_response_format_matches_fixture
   - test_permission_default_matches_fixture
   - test_permission_plan_matches_fixture
   - test_thinking_dialog_matches_fixture

2. Remove FIXME comments referencing epic-05x-fix-tui

3. Update module doc comment to remove "known divergences" section

4. Run full test suite:
   cargo test -p claudeless --test tui_fixture_comparison

5. If any test still fails, fix the rendering code
```

### Validation

- [ ] All fixture comparison tests pass without `--ignored` flag
- [ ] No `#[ignore]` attributes remain in tui_fixture_comparison.rs
- [ ] No FIXME comments referencing this epic remain

---

## Phase 8: Consolidate Legacy Tests

**Goal:** Migrate keyword-based tests to fixture comparison where appropriate.

### Tests to Review

From `tui_trust.rs`:
- `test_trust_prompt_mentions_files` → covered by fixture test
- `test_trust_prompt_mentions_security` → covered by fixture test
- `test_trust_prompt_has_yes_no_options` → covered by fixture test
- Keep behavioral tests: `test_trust_prompt_yes_proceeds`, `test_trust_prompt_escape_cancels`

From `tui_compacting.rs`:
- Review `contains()` assertions - replace with fixtures where applicable

### Task Agent Instructions

```
1. For each test file (tui_trust.rs, tui_thinking.rs, etc.):
   a. Identify tests that check visual output with contains()
   b. If fixture test covers same case, delete the redundant test
   c. Keep behavioral tests that test interactions, not visuals

2. Update test file doc comments to clarify:
   - "Visual tests" → in tui_fixture_comparison.rs
   - "Behavioral tests" → in tui_*.rs

3. Ensure no redundant coverage between fixture tests and keyword tests
```

### Validation

- [ ] No tests duplicate fixture comparison coverage
- [ ] Behavioral tests remain functional
- [ ] `cargo test -p claudeless --test 'tui_*' -- --test-threads=1` passes

---

## Phase 9: Final Validation

**Goal:** Full integration test of all TUI functionality.

### Task Agent Instructions

```
1. Run comparison script:
   ./crates/cli/scripts/compare-tui.sh

2. Run full test suite:
   cargo test -p claudeless --test 'tui_*' -- --test-threads=1

3. Run make check:
   make check

4. Manual verification checklist:
   - [ ] cargo run -p claudeless -- --scenario tests/fixtures/scenarios/basic.toml --tui
   - [ ] Trust prompt appears (if untrusted)
   - [ ] Input area has placeholder text
   - [ ] Typing shows user input
   - [ ] Enter submits and shows response
   - [ ] Meta+T opens thinking dialog
   - [ ] Thinking dialog has correct format
   - [ ] Status bar shows correct mode
```

### Validation

- [ ] `scripts/compare-tui.sh` exits 0 (or documents acceptable differences)
- [ ] All TUI tests pass
- [ ] `make check` passes
- [ ] No `#[ignore]` tests remain with FIXME references

---

## Architecture Notes

### Rendering Pattern

All rendering now happens in `app.rs` with these functions:
- `render_main_content()` - Main layout
- `render_trust_prompt()` - Trust dialog
- `render_thinking_dialog()` - Thinking toggle
- `render_permission_dialog()` - Permission prompts

### Text Formatting

Use full-width separators (120 chars of `─`):
```rust
const SEPARATOR: &str = "────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────";
```

### Status Bar Format

| Mode | Display |
|------|---------|
| Default | `  ? for shortcuts` |
| Plan | `  ⏸ plan mode on (shift+tab to cycle)` |
| Accept Edits | `  ⏵⏵ accept edits on (shift+tab to cycle)` |
| Bypass | `  ⏵⏵ bypass permissions on (shift+tab to cycle)` |

---

## Final Checklist

- [ ] Phase 1: Trust prompt matches fixture
- [ ] Phase 2: Initial state matches fixture
- [ ] Phase 3: Response format matches fixture
- [ ] Phase 4: Thinking dialog matches fixture
- [ ] Phase 5: Permission modes match fixtures
- [ ] Phase 6: Backward compatibility files removed
- [ ] Phase 7: All fixture tests enabled (no `#[ignore]`)
- [ ] Phase 8: Legacy tests consolidated
- [ ] Phase 9: Full validation passes
- [ ] `make check` passes
- [ ] No warnings in `cargo check -p claudeless`
