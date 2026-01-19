# Epic 11c: Audit TUI Visual Fidelity

**Scope:** tui_*.rs tests, tests/fixtures/tui/
**Claimed validation:** Real Claude CLI v2.1.12

## Summary

| Aspect | Status |
|--------|--------|
| Tests written | ✅ 38 tests |
| Tests passing | ✅ All pass |
| Golden files captured | ✅ 27 fixture files |
| Fixtures used in tests | ❌ Zero references |
| Format matches fixtures | ❌ Known divergences |

## Critical Finding: Orphaned Fixtures

The fixtures directory contains **27 files captured from real Claude CLI v2.1.12**:

```
tests/fixtures/tui/
├── README.md                              # Documents capture method
├── initial_state.txt                      # Real Claude startup screen
├── trust_prompt.txt                       # Real trust dialog
├── after_response.txt                     # Real response format
├── model_haiku.txt / model_sonnet.txt / model_opus.txt
├── permission_*.txt                       # Permission dialogs
├── thinking_dialog*.txt                   # Thinking toggle
└── ... (27 total files)
```

**But no test code references these fixtures.**

```bash
$ grep -r "fixtures/tui" crates/cli/tests/*.rs
# No results

$ grep -r "include_str" crates/cli/tests/*.rs
# No results
```

## What Tests Actually Do

Tests use **keyword matching**, not fixture comparison:

```rust
// tui_trust.rs - actual test
assert!(capture.to_lowercase().contains("trust"));
assert!(capture.contains("files") && capture.contains("folder"));

// What it SHOULD do
let expected = include_str!("fixtures/tui/trust_prompt.txt");
assert_eq!(normalize(capture), normalize(expected));
```

## Known Format Divergences

### Trust Prompt

**Real Claude (from fixture):**
```
Do you trust the files in this folder?

/private/var/folders/.../tmp.WGSAfa6ySq

Claude Code may read, write, or execute files contained in this directory.

❯ 1. Yes, proceed
  2. No, exit

Enter to confirm · Esc to cancel
```

**Simulator (from widgets/trust.rs):**
```rust
Line::from(vec![
    Span::styled("[Yes]", yes_style),
    Span::styled("[No/Exit]", no_style),
]),
```

| Element | Real Claude | Simulator |
|---------|-------------|-----------|
| Options format | `❯ 1. Yes, proceed` | `[Yes]` |
| Selection indicator | `❯` cursor | Color highlighting |
| Option text | "Yes, proceed" / "No, exit" | "Yes" / "No/Exit" |

### Response Marker

**Real Claude:** `⏺ Hello there friend.`
**Simulator:** `⏺ {response}` ✅ (This one matches)

### Permission Mode Status

**Real Claude (from fixture):**
```
⏸ plan mode on (shift+tab to cycle)
```

**Simulator:** Uses similar format ✅

## Fixture README Documents Real Behavior

From `tests/fixtures/tui/README.md`:
```markdown
# Claude TUI Snapshots

Captured from real Claude CLI for comparison testing.

**Behavior observed with:** claude --version 2.1.12 (Claude Code)

## Capture Method

Captured using tmux:
tmux new-session -d -s claude-tui -x 120 -y 40
tmux send-keys -t claude-tui 'claude --model haiku' Enter
sleep 3
tmux capture-pane -t claude-tui -p
```

The capture methodology is documented, but never executed in tests.

**Note:** Uses `--model haiku` for cost efficiency. TUI format is identical across models.

## Git History

```
fd1cd00 - Fixtures added with 27 real Claude captures
838bc82 - TUI implementation (uses different format)
d4f1e8d - Additional TUI features
```

The fixtures were added in the same commit as the tests, but tests were written with keyword assertions instead of fixture comparison.

## Verdict

**Completed:** ⚠️ Partially - tests pass but don't verify visual fidelity
**Compromised:** ✅ Yes - fixtures exist but are orphaned, format diverges

The TUI tests give false confidence. They pass because they check for keywords ("trust", "files", "folder") rather than verifying the actual visual output matches real Claude.

## Recommendations

1. **Create TUI capture script** (`crates/cli/scripts/capture-tui.sh`):

   **What it should do:**
   ```
   1. Generate unique tmux session name (e.g., claude-capture-$$)
   2. Register cleanup trap: kill tmux session on EXIT/ERR/INT
   3. Create tmux session: tmux new-session -d -s $SESSION -x 120 -y 40
   4. Send command: tmux send-keys "claude --model haiku" Enter
   5. Wait for startup (sleep or poll for expected content)
   6. Capture: tmux capture-pane -p > output.txt
   7. Optionally send more keys (trust prompt, input, etc.) and re-capture
   8. Kill session (trap handles this)
   9. Output captured file path
   ```

   **Why haiku?** TUI format is identical across models; haiku is cheapest.

2. **Create TUI comparison script** (`crates/cli/scripts/compare-tui.sh`):

   **What it should do:**
   ```
   1. Capture real Claude TUI (using capture script above)
   2. Capture simulator TUI (same method)
   3. Normalize (only fields that inherently vary):
      - timestamps: "<TIME>"
      - session IDs: "<SESSION>"
      - paths: keep exact unless temp dir (e.g., /var/folders/... → "<TEMPDIR>")
      - whitespace: preserve interior spacing (visual alignment matters)
        - only strip trailing whitespace per line
        - do NOT collapse multiple spaces (breaks column alignment)
        - do NOT strip leading whitespace (breaks indentation)
   4. Diff normalized outputs
   5. Cleanup both tmux sessions (trap)
   ```

3. **Wire up fixture comparison** with normalization for dynamic content:
   - **Timestamps** - Replace with `<TIME>`
   - **Working directory** - Replace temp paths with `<TEMPDIR>` placeholder
   - **Session IDs** - Replace with `<SESSION>`
   - **Whitespace** - Strip trailing only; preserve leading and interior spacing
   - **Terminal size** - Capture and test at same dimensions (120x40)

2. **Scenario configuration for TUI elements:**
   ```toml
   [tui]
   # Control random/dynamic elements for deterministic testing
   suggested_prompt = "refactor mod.rs"  # Override random suggestion
   version_string = "v2.1.12"            # Match fixture version
   working_directory_display = "~/Developer/claudeless"  # Display override
   ```

3. **Fix format divergences** - Update trust prompt to match real Claude:
   ```
   # Real Claude format (from fixture)
   ❯ 1. Yes, proceed
     2. No, exit

   # Not bracket buttons: [Yes] [No/Exit]
   ```

4. **Implement fixture comparison helper:**
   ```rust
   fn compare_tui_output(actual: &str, fixture: &str) -> bool {
       let normalized_actual = normalize_tui(actual);
       let normalized_fixture = normalize_tui(fixture);
       normalized_actual == normalized_fixture
   }

   fn normalize_tui(s: &str) -> String {
       s.replace(|c: char| c.is_ascii_digit() && /* in timestamp */, "X")
        .replace("/tmp/...", "<CWD>")
        // etc
   }
   ```

5. **Add visual regression tests** - Snapshot testing with normalization

6. **Re-capture fixtures** - If format intentionally differs, document why and update fixtures to match simulator (with explanation)

## Regression Prevention

Run capture scripts to generate fixtures stored per Claude version:
```
tests/fixtures/tui/
├── v2.1.12/
│   ├── initial_state.txt
│   ├── trust_prompt.txt
│   ├── after_response.txt
│   └── ...
└── v2.2.0/
    └── ...
```

## Required Fixes

See [epic-05x-fix-tui.md](epic-05x-fix-tui.md) for fixes.

Known divergences already identified:

1. **Trust prompt format** - bracket buttons vs numbered list with cursor
2. **Fixtures not wired** - keyword matching instead of fixture comparison

## Deliverables

1. Implement capture/comparison scripts
2. Capture fixtures to `tests/fixtures/tui/v{version}/`
3. Add fixture comparison tests marked as ignored:
   ```rust
   #[test]
   #[ignore] // FIXME: epic-05x-fix-tui - enable after fixing divergences
   fn test_trust_prompt_matches_fixture() {
       let capture = capture_tui_state("trust_prompt");
       assert_tui_matches_fixture(&capture, "v2.1.12", "trust_prompt.txt");
   }
   ```
4. Run `cargo test -- --ignored` to verify tests fail as expected
5. Document divergences in epic-05x-fix-tui.md
