# Fix Shell Preamble Stripping for Mid-Session Captures

## Problem

When TUI tests capture output mid-session (after sending input or triggering dialogs), the captured output includes shell prompt lines from before the TUI started. The `normalize_tui()` function attempts to strip this preamble by finding the TUI logo (`▐▛███▜▌`), but this doesn't work correctly for mid-session captures.

### Affected Tests
- `test_permission_bash_command_matches_fixture`
- `test_permission_edit_file_matches_fixture`
- `test_permission_write_file_matches_fixture`
- `test_permission_trust_folder_matches_fixture`
- `test_status_bar_extended_matches_fixture`
- `test_tasks_empty_matches_fixture`
- `test_thinking_dialog_matches_fixture`
- `test_trust_prompt_matches_fixture`

### Expected vs Actual

**Fixture:**
```
 ▐▛███▜▌   Claude Code v2.1.12
▝▜█████▛▘  Haiku 4.5 · Claude Max
  ▘▘ ▝▝    ~/Developer/claudeless

────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 Toggle thinking mode
...
```

**Captured (before normalization):**
```
/path/to/claudeless --scenario /tmp/xxx.json --tui
8hsnzwvh0000gn/T/.tmpXXX.json --tui

claudeless/crates/cli on  main is 📦 v0.1.0 [rust]
❯ /path/to/claudeless --scenario ...
rv8hsnzwvh0000gn/T/.tmpXXX.json --tui
 ▐▛███▜▌   Claude Code v2.1.12
...
```

## Root Cause

The preamble appears because:
1. tmux captures the entire visible buffer
2. Shell prompt and command echo appear before TUI renders
3. `normalize_tui()` finds logo but the text before it on the same line gets kept

## Proposed Fixes

### Option A: Improve `normalize_tui()` in `common/mod.rs`

Make the preamble stripping more robust:

```rust
// Current approach finds logo and strips to line start
if let Some(logo_pos) = result.find("▐▛███▜▌") {
    let line_start = result[..logo_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
    result = result[line_start..].to_string();
}
```

Issues:
- Logo may appear on a line with garbage text before it
- Need to strip entire lines before the logo line

**Fix:**
```rust
// Find logo and strip ALL content before the logo's line
if let Some(logo_pos) = result.find("▐▛███▜▌") {
    // Find the start of the line containing the logo
    let line_start = result[..logo_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
    // Find where actual logo line starts (may have junk prefix)
    let logo_line = &result[line_start..];
    if let Some(logo_in_line) = logo_line.find("▐▛███▜▌") {
        // Check if there's junk before the logo on this line
        let prefix = &logo_line[..logo_in_line].trim();
        if !prefix.is_empty() && !prefix.starts_with(' ') {
            // There's junk - find the space-logo pattern
            if let Some(clean_start) = logo_line.find(" ▐▛███▜▌") {
                result = result[line_start + clean_start..].to_string();
            }
        } else {
            result = result[line_start..].to_string();
        }
    }
}
```

### Option B: Fix test capture timing

The shell preamble appears because we capture too early. Ensure TUI has fully rendered before capturing:

1. Wait for the TUI logo to appear
2. Wait an additional frame for full render
3. Then wait for the specific content pattern

### Option C: Strip lines not matching TUI patterns

After finding the logo, validate each line:
- TUI lines start with specific patterns (logo chars, `─`, `❯`, `⏸`, etc.)
- Shell lines start with paths, prompts (`$`, `%`, `❯` without TUI)

## Recommended Approach

**Option A** is most robust. The fix should:

1. Find the TUI logo character `▐`
2. Find the start of that line
3. Check if the line starts with ` ▐` (space + logo) as expected
4. If not, there's garbage - find the clean pattern ` ▐▛███▜▌`
5. Strip everything before the clean TUI header

## Files to Modify

- `crates/cli/tests/common/mod.rs` - `normalize_tui()` function
- `crates/cli/tests/common/ansi.rs` - `normalize_ansi_tui()` function (same fix)

## Steps

1. Update `normalize_tui()` to robustly strip preamble
2. Update `normalize_ansi_tui()` with equivalent logic
3. Remove `#[ignore]` from affected tests
4. Run tests to verify

## Verification

```bash
cargo test test_permission_bash_command_matches_fixture
cargo test test_permission_edit_file_matches_fixture
cargo test test_permission_write_file_matches_fixture
cargo test test_permission_trust_folder_matches_fixture
cargo test test_status_bar_extended_matches_fixture
cargo test test_tasks_empty_matches_fixture
cargo test test_thinking_dialog_matches_fixture
cargo test test_trust_prompt_matches_fixture
```
