# Fix Status Bar "Use meta+t to toggle thinking" Bug

## Problem

Claudeless incorrectly outputs "Use meta+t to toggle thinking" in the status bar for permission modes (plan, accept-edits, bypass-permissions), but the real Claude CLI v2.1.15 does not display this text.

### Affected Tests
- `test_permission_plan_matches_fixture`
- `test_permission_plan_ansi_matches_fixture`
- `test_permission_accept_edits_ansi_matches_fixture`
- `test_permission_bypass_ansi_matches_fixture`

### Expected vs Actual

**Fixture (real CLI):**
```
  ⏸ plan mode on (shift+tab to cycle)
```

**Claudeless (current):**
```
  ⏸ plan mode on (shift+tab to cycle)                                            Use meta+t to toggle thinking
```

## Root Cause Investigation

1. Find where status bar is rendered in `crates/cli/src/tui/app.rs`
2. Identify the code that adds "Use meta+t to toggle thinking"
3. Determine the condition under which this text should/shouldn't appear

## Proposed Fix

The "Use meta+t to toggle thinking" hint likely should only appear in default mode (not plan/accept-edits/bypass modes), or possibly only when thinking is available as a feature.

### Steps

1. **Locate status bar rendering code**
   - Search for "meta+t" or "toggle thinking" in `app.rs`
   - Find the function that renders the status bar line

2. **Check conditional logic**
   - The hint may be unconditionally rendered
   - Should only appear in specific conditions (TBD based on real CLI behavior)

3. **Fix the conditional**
   - Option A: Only show in default permission mode
   - Option B: Never show (if real CLI never shows it)
   - Option C: Show based on thinking feature availability

4. **Update tests**
   - Remove `#[ignore]` from the 4 affected tests
   - Verify all tests pass

## Files to Modify

- `crates/cli/src/tui/app.rs` - Status bar rendering logic

## Verification

```bash
cargo test test_permission_plan_matches_fixture
cargo test test_permission_plan_ansi_matches_fixture
cargo test test_permission_accept_edits_ansi_matches_fixture
cargo test test_permission_bypass_ansi_matches_fixture
```
