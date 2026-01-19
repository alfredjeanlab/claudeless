# Epic 11a: Audit CLI Flag and Output Format Validation

**Scope:** CLI flags, output formats (JSON, stream-JSON, text), error behavior
**Claimed validation:** Real Claude CLI v2.1.12

## Summary

| Aspect | Status |
|--------|--------|
| CLI flags documented | ✅ 23 flags in ACCURACY.md |
| Output format documented | ✅ JSON/stream-JSON/text |
| Comparison script | ✅ Created (`scripts/compare-cli.sh`) |
| Real Claude output captured | ✅ `tests/fixtures/cli/v2.1.12/` |
| Fixture comparison tests | ✅ `tests/cli_fixtures.rs` (6 ignored, 5 passing) |
| Validated empirically | ⚠️ Divergences documented below |

## ACCURACY.md Claims

From `crates/cli/docs/ACCURACY.md`:

```markdown
## Validation Methodology

The accuracy of this simulator was validated by:

1. **Documentation Review**: Comparing behavior against public Claude Code
   documentation.
```

This explicitly admits validation was against **documentation**, not the actual Claude binary.

## Comparison Script

**Status:** ✅ Created as `crates/cli/scripts/compare-cli.sh`

Features:
- Captures JSON and stream-JSON output from real Claude CLI
- Normalizes dynamic fields (session_id, duration, cost, etc.)
- Compares normalized output to simulator
- Supports `capture` subcommand to generate fixtures

Usage:
```bash
./scripts/compare-cli.sh              # Run comparison
./scripts/compare-cli.sh capture v2.1.12  # Capture fixtures
```

## CLI Flag Coverage

### Claimed "Match" (from ACCURACY.md)

| Flag | Documented | Empirically Verified |
|------|------------|---------------------|
| `--print`, `-p` | ✅ | ❌ |
| `--model` | ✅ | ❌ |
| `--output-format` | ✅ | ❌ |
| `--max-tokens` | ✅ | ❌ |
| `--system-prompt` | ✅ | ❌ |
| `--continue`, `-c` | ✅ | ❌ |
| `--resume`, `-r` | ✅ | ❌ |
| `--permission-mode` | ✅ | ❌ |
| ... (23 total) | ✅ | ❌ |

"Empirically Verified" would require running real Claude with each flag and comparing output to simulator.

## Output Format Claims

### JSON Output (`--output-format json`)

ACCURACY.md claims this format:
```json
{
  "type": "result",
  "subtype": "success",
  "cost_usd": 0,
  "is_error": false,
  "duration_ms": 1000,
  "duration_api_ms": 950,
  "num_turns": 1,
  "result": "Response text",
  "session_id": "uuid"
}
```

**Verification:** None. No captured real Claude JSON output exists in the repo.

### Stream JSON (`--output-format stream-json`)

ACCURACY.md claims event sequence:
1. `{"type": "system", "subtype": "init", ...}`
2. `{"type": "assistant", "subtype": "message_start", ...}`
3. `{"type": "content_block_start", ...}`
4. ... etc

**Verification:** None. No captured real Claude stream output exists.

## Tests That Exist

From `tests/smoke_test.rs` and `tests/validation.rs`:
- Tests verify simulator produces valid JSON
- Tests verify field presence
- Tests do NOT compare against real Claude output

```rust
// Example from validation tests
let parsed: serde_json::Value = serde_json::from_str(&output)?;
assert!(parsed["type"].is_string());  // Checks field exists
// Does NOT compare to real Claude output
```

## Error Behavior

ACCURACY.md documents exit codes:
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error |
| 2 | Partial response |
| 130 | Interrupted |

**Verification:** Based on documentation. No tests trigger real Claude errors and compare.

## What Was Delivered vs Planned (Epic 05d)

| Planned | Original Status | Current Status (2026-01-18) |
|---------|-----------------|----------------------------|
| CLI flag audit against real `claude --help` | Documentation review | Documentation review |
| Output format validation against real output | Structural tests only | ✅ Fixtures captured, divergences documented |
| Comparison script for regression testing | Not created | ✅ `scripts/compare-cli.sh` |
| Golden files from real Claude | TUI only (unused) | ✅ `tests/fixtures/cli/v2.1.12/` |
| Error message comparison | Not performed | Not performed |

## What Would Constitute Validation

1. **Flag audit script** - Run `claude --help` and `claudeless --help`, diff systematically
2. **Output capture** - Run real Claude with various flags, capture output as golden files
3. **Structural comparison** - JSON schema validation against real output
4. **Error comparison** - Trigger real errors, compare messages verbatim
5. **Version pinning** - Document exact Claude version tested against

## Git History

No commits show evidence of running real Claude and capturing output. The accuracy claims appear to be based on reading Claude Code documentation.

## Documented Divergences (from test output 2026-01-18)

### JSON Output (`--output-format json`)

| Feature | Real Claude | Simulator |
|---------|-------------|-----------|
| `type` | `"result"` | `"message"` |
| `subtype` | `"success"` | missing |
| `is_error` | `false` | missing |
| `num_turns` | `1` | missing |
| `session_id` | present | missing |
| `total_cost_usd` | present | missing |
| `duration_ms` | present | missing |
| `duration_api_ms` | present | missing |
| `modelUsage` | present | missing |
| `permission_denials` | `[]` | missing |
| `uuid` | present | missing |
| `result` | response text | missing |
| `content` | missing | present (array) |
| `id` | missing | present (msg_*) |
| `model` | missing | present |
| `role` | missing | present |
| `stop_reason` | missing | present |

**Root cause:** Simulator outputs raw Anthropic API message format; real Claude wraps in result envelope.

### Stream-JSON Output (`--output-format stream-json`)

| Event | Real Claude | Simulator |
|-------|-------------|-----------|
| 1 | `{type: "system", subtype: "init", tools: [...], ...}` | `{type: "message_start", ...}` |
| 2 | `{type: "assistant", message: {...}}` | `{type: "content_block_start"}` |
| 3 | `{type: "result", subtype: "success", ...}` | `{type: "content_block_delta"}` (multiple) |
| 4+ | - | `content_block_stop`, `message_delta`, `message_stop` |

**Root cause:** Simulator outputs raw API streaming events; real Claude wraps in higher-level events.

### System Init Event (stream-json)

Real Claude includes in `system.init`:
- `tools` - list of available tools
- `agents` - list of available agents
- `slash_commands` - available slash commands
- `plugins` - loaded plugins
- `claude_code_version` - version string
- `permissionMode`, `apiKeySource`, `output_style`

Simulator has none of this context.

## Verdict

**Completed:** ⚠️ Partially - infrastructure created, divergences documented
**Compromised:** ✅ Yes - ACCURACY.md claims do not match empirical results

The CLI validation now has proper tooling, but significant divergences exist between simulator output and real Claude CLI output.

## Recommendations

1. ✅ **Create comparison script** (`crates/cli/scripts/compare-cli.sh`) - DONE:

   **What it should do:**
   ```
   1. Create temp directory
   2. Register cleanup trap (rm temp dir on EXIT/ERR/INT)
   3. Run: claude --model haiku -p --output-format json "Hello" > real.json
   4. Run: claudeless --model haiku -p --output-format json "Hello" > sim.json
   5. Normalize (only fields that inherently vary):
      - session_id: "<SESSION_ID>"
      - duration_ms: "<DURATION>"
      - duration_api_ms: "<DURATION>"
      - cost_usd: "<COST>" (real has cost, sim is 0)
      - timestamps: "<TIMESTAMP>"
      - paths: keep exact (should match if cwd is same)
   6. Diff normalized outputs
   7. Exit 0 if match, 1 if differ
   ```

   **Why haiku?** Validation checks output *format*, not response *quality*. Haiku is cheapest and produces structurally identical output.

2. **Fields to normalize/exclude in comparison:**

   | Field | Treatment |
   |-------|-----------|
   | `session_id` | Exclude (random UUID) |
   | `duration_ms` | Exclude (timing varies) |
   | `duration_api_ms` | Exclude (timing varies) |
   | `cost_usd` | Compare as 0 for simulator |
   | `timestamp` | Exclude or verify format only |
   | `request_id` | Exclude (random) |
   | `type`, `subtype` | Compare exactly |
   | `result` | Compare exactly (for deterministic prompts) |
   | `is_error` | Compare exactly |

3. **Scenario configuration for deterministic output:**
   ```toml
   [output]
   # Fixed values for comparison testing
   session_id = "test-session-12345"
   duration_ms = 1000
   cost_usd = 0.0

   [identity]
   request_id_prefix = "req_test_"  # Predictable request IDs
   ```

4. ✅ **Capture golden files** with metadata - DONE:
   ```
   tests/fixtures/cli/
   ├── json_output.json          # Normalized (no timestamps/UUIDs)
   ├── json_output.meta.json     # Documents which fields were stripped
   ├── stream_json_events.jsonl  # Event sequence (normalized)
   └── README.md                 # Claude version, capture date
   ```

5. ✅ **Add regression tests** comparing structure and static fields - DONE (`tests/cli_fixtures.rs`):
   ```rust
   #[test]
   fn test_json_output_structure() {
       let real = include_str!("fixtures/cli/json_output.json");
       let real: Value = serde_json::from_str(real).unwrap();

       let sim_output = run_simulator(&["--output-format", "json", "-p", "test"]);
       let sim: Value = serde_json::from_str(&sim_output).unwrap();

       // Compare static fields
       assert_eq!(sim["type"], real["type"]);
       assert_eq!(sim["subtype"], real["subtype"]);
       assert_eq!(sim["is_error"], real["is_error"]);

       // Verify format of dynamic fields
       assert!(sim["session_id"].is_string());
       assert!(sim["duration_ms"].is_number());
   }
   ```

6. **Update ACCURACY.md** - Be explicit about what was validated how:
   ```markdown
   ## Validation Status

   | Feature | Method | Last Validated |
   |---------|--------|----------------|
   | JSON output structure | Golden file comparison | 2026-01-18 (v2.1.12) |
   | CLI flags | Documentation review | 2026-01-18 |
   | Stream events | Golden file comparison | Not yet |
   ```

7. **Version lock** - Pin validation to specific Claude Code version

## Regression Prevention

Run capture scripts to generate fixtures stored per Claude version:
```
tests/fixtures/cli/
├── v2.1.12/
│   ├── json_output.normalized.json
│   └── stream_json.normalized.jsonl
└── v2.2.0/
    └── ...
```

## Required Fixes

See [epic-05x-fix-cli.md](epic-05x-fix-cli.md) for fixes after running comparison.

## Deliverables

1. ✅ Implement capture/comparison scripts - `scripts/compare-cli.sh`
2. ✅ Capture fixtures to `tests/fixtures/cli/v{version}/` - `tests/fixtures/cli/v2.1.12/`
3. ✅ Add fixture comparison tests marked as ignored - `tests/cli_fixtures.rs` (6 ignored tests)
4. ✅ Run `cargo test -- --ignored` to verify tests fail as expected - Done, 6 failures documented
5. ✅ Document divergences - See "Documented Divergences" section above
