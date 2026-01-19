# Epic 11b: Audit ~/.claude Directory Format (dotclaude)

**Scope:** dot_claude_projects.rs, dot_claude_plans.rs, dot_claude_todos.rs
**Claimed validation:** Real Claude CLI v2.1.12

## Summary

| Aspect | Status |
|--------|--------|
| Tests written | ✅ 13 tests |
| Tests passing | ✅ All pass |
| Golden files captured | ❌ None |
| Format documented | ✅ In epic-05d5 |
| Validated against real Claude | ⚠️ Observation-based, no samples |

## What Was Tested

### projects/ directory
- Project dir uses normalized path naming (`/` and `.` become `-`)
- `sessions-index.json` created with correct structure
- Session files are `.jsonl` format (not `.json`)
- User message has: uuid, sessionId, timestamp, cwd, message.role, message.content
- Assistant message has: uuid, parentUuid, sessionId, timestamp, requestId, message.role, message.content, message.model

### todos/ directory
- Directory created when TodoWrite tool executes
- File naming: `{sessionId}-agent-{sessionId}.json`
- Content is JSON array with items having: content, status, activeForm
- Status values: pending, in_progress, completed

### plans/ directory
- Directory created when ExitPlanMode tool executes
- File naming: `{adjective}-{verb}-{noun}.md` (three lowercase words)
- Content is markdown (contains `#` heading)

## Verification Method

The tests verify **structural correctness**:
```rust
// Example from dot_claude_projects.rs
assert_eq!(parsed["version"], 1, "version should be 1");
assert!(parsed["entries"].is_array(), "entries should be an array");
assert!(entry["sessionId"].is_string(), "sessionId required");
```

## What's Missing

### No Golden Files
Unlike the TUI tests which have fixtures captured from real Claude (even if unused), the dotclaude tests have **zero captured samples** from real Claude's ~/.claude directory.

The format was derived from:
1. Observation of real Claude CLI behavior
2. Documentation in epic-05d5-state-directory.md

But there's no `tests/fixtures/dotclaude/sessions-index.json` or similar captured from real Claude to compare against.

### Fields May Be Missing
The tests verify required fields exist but don't verify:
- Whether real Claude includes additional fields not in our spec
- Exact formatting (pretty-print vs compact JSON)
- Field ordering (if it matters)
- Edge cases in real Claude's output

### No Regression Protection
Without golden files, if real Claude changes its format, our tests won't detect the drift.

## Comparison: Spec vs Reality

| File | Spec Source | Verified Against Real Claude |
|------|-------------|------------------------------|
| sessions-index.json | epic-05d5 documentation | ❌ No sample captured |
| {uuid}.jsonl | epic-05d5 documentation | ❌ No sample captured |
| {uuid}-agent-{uuid}.json | epic-05d5 documentation | ❌ No sample captured |
| {adj}-{verb}-{noun}.md | epic-05d5 documentation | ❌ No sample captured |

## Git History

```
fd1cd00 - Tests added as failing specs (35 of 51 failing)
0aca65a - Implementation completed, all 13 dotclaude tests pass
```

Test modifications in 0aca65a were **not compromises** - they updated scenario format to properly trigger tool calls. The assertions remained intact.

## Verdict

**Completed:** ✅ Yes - tests pass and verify documented format
**Compromised:** ⚠️ Partially - no golden files means format is assumed, not validated

The dotclaude tests are **better than the TUI tests** (which have orphaned fixtures) because they test what they claim. However, they still lack empirical validation against captured real Claude output.

## Recommendations

1. **Create state directory capture script** (`crates/cli/scripts/capture-state.sh`):

   **What it should do:**
   ```
   1. Create temp working directory
   2. Register cleanup trap: rm temp dir on EXIT/ERR/INT
   3. Run: claude --model haiku -p "Create a todo list with 3 items" (in temp dir)
   4. Copy ~/.claude/projects/{normalized-temp-path}/ to fixtures/
   5. Copy ~/.claude/todos/*.json to fixtures/
   6. Run: claude --model haiku --permission-mode plan -p "Plan a feature"
   7. Copy ~/.claude/plans/*.md to fixtures/
   8. Normalize captured files (only inherently variable fields):
      - UUIDs: "<UUID>"
      - timestamps: "<TIMESTAMP>"
      - paths: keep exact (use consistent working dir for capture)
   9. Cleanup temp dir (trap handles this)
   ```

   **Why haiku?** State file format is identical across models; haiku is cheapest.

2. **Create state comparison script** (`crates/cli/scripts/compare-state.sh`):

   **What it should do:**
   ```
   1. Run real Claude in temp dir (capture state files)
   2. Run simulator in temp dir with CLAUDELESS_STATE_DIR (capture state files)
   3. Normalize (only fields that inherently vary):
      - UUIDs: "<UUID>"
      - timestamps: "<TIMESTAMP>"
      - mtime values: "<MTIME>"
      - paths: keep exact (project path should match if cwd is same)
   4. Diff directory trees (structure)
   5. Diff individual files (content)
   6. Cleanup both temp dirs (trap)
   ```

3. **Capture real ~/.claude samples** - Run real Claude, copy the resulting files as fixtures

2. **Add structural golden file comparison** with normalization for:
   - **Timestamps** - Replace with placeholder `"<TIMESTAMP>"` before comparison
   - **UUIDs** - Replace with placeholder `"<UUID>"` or verify format only (regex)
   - **Session IDs** - Same as UUIDs
   - **File paths** - Normalize to relative or use placeholders for temp dirs
   - **Git branch** - Allow scenario override or placeholder

3. **Scenario configuration for random elements:**
   ```toml
   [identity]
   session_id = "fixed-session-id-for-test"  # Already supported

   [state_output]
   # Control what gets written for deterministic testing
   timestamp_mode = "fixed"  # or "real"
   fixed_timestamp = "2026-01-18T12:00:00Z"
   uuid_mode = "sequential"  # uuid_0, uuid_1, ... for predictability
   ```

4. **Field-level comparison** - Compare structure and non-random values:
   ```rust
   // Verify fields exist and have correct types
   assert!(entry["sessionId"].is_string());
   // Verify fixed content matches
   assert_eq!(entry["version"], 1);
   // Verify format of random fields
   assert!(is_valid_uuid(entry["sessionId"].as_str()));
   ```

5. **Document version** - Pin to specific Claude version and re-capture when updating

## Regression Prevention

Run capture scripts to generate fixtures stored per Claude version:
```
tests/fixtures/dotclaude/
├── v2.1.12/
│   ├── sessions-index.json
│   ├── session.jsonl
│   ├── todo.json
│   └── plan.md
└── v2.2.0/
    └── ...
```

## Required Fixes

See [epic-05x-fix-dotclaude.md](epic-05x-fix-dotclaude.md) for fixes after running comparison.

## Deliverables

1. Implement capture/comparison scripts
2. Capture fixtures to `tests/fixtures/dotclaude/v{version}/`
3. Add fixture comparison tests marked as ignored:
   ```rust
   #[test]
   #[ignore] // FIXME: epic-05x-fix-dotclaude - enable after fixing divergences
   fn test_sessions_index_matches_fixture() {
       let actual = read_state_file("sessions-index.json");
       assert_state_matches_fixture(&actual, "v2.1.12", "sessions-index.json");
   }
   ```
4. Run `cargo test -- --ignored` to verify tests fail as expected
5. Document divergences in epic-05x-fix-dotclaude.md
