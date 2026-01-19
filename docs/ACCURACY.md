# Claudelessulator Accuracy Report

Last validated: 2026-01-18
Claude Code version: v2.1.12 (golden files), documentation (CLI flags)

## Validation Status

| Feature | Method | Status | Notes |
|---------|--------|--------|-------|
| CLI flags | Documentation review | ✅ Complete | 23 flags documented |
| JSON output structure | Golden file comparison | ⚠️ Diverges | See epic-05x-fix-cli.md |
| Stream-JSON events | Golden file comparison | ⚠️ Diverges | See epic-05x-fix-cli.md |
| Hook protocol | Schema validation | ✅ Complete | |
| State directory | Structure validation | ✅ Complete | |

**Note:** Golden files captured from Claude v2.1.12 are in `tests/fixtures/cli/v2.1.12/`.
Ignored tests document divergences: `cargo test --test cli_fixtures -- --ignored`

## Summary

| Category | Match | Partial | Mismatch | Not Validated |
|----------|-------|---------|----------|---------------|
| CLI Flags | 23 | 1 | 0 | 5 |
| Output Formats | 3 | 0 | 0 | 0 |
| Hook Protocol | 7 | 0 | 0 | 0 |
| State Directory | 5 | 0 | 0 | 0 |
| Error Behavior | 5 | 0 | 0 | 0 |
| Permission Modes | 6 | 0 | 0 | 0 |

## CLI Flags

### Implemented (Match Real Claude)

| Flag | Description |
|------|-------------|
| `--print`, `-p` | Print mode for non-interactive use |
| `--model` | Specify model to use |
| `--output-format` | Output format (text/json/stream-json) |
| `--max-tokens` | Maximum tokens in response |
| `--system-prompt` | Custom system prompt |
| `--continue`, `-c` | Continue previous conversation |
| `--resume`, `-r` | Resume by session ID |
| `--allowedTools` | Restrict available tools |
| `--disallowedTools` | Blacklist specific tools |
| `--permission-mode` | Permission handling mode |
| `--cwd` | Working directory |
| `--input-format` | Input format (text/stream-json) |
| `--session-id` | Specify session UUID |
| `--verbose` | Verbose output |
| `--debug`, `-d` | Debug mode |
| `--include-partial-messages` | Include partial chunks in stream |
| `--fallback-model` | Fallback on overload |
| `--max-budget-usd` | Maximum spend limit |
| `--allow-dangerously-skip-permissions` | Enable permission bypass option |
| `--dangerously-skip-permissions` | Bypass all permissions (requires allow) |
| `--strict-mcp-config` | Only use specified MCP configs |
| `--mcp-debug` | Show MCP server debug info |
| `--input-file` | Read prompt from file |

### Partial Implementation

| Flag | Status | Notes |
|------|--------|-------|
| `--mcp-config` | Partial | Config parsing only, no server execution |

### Permission Modes

| Mode | Status | Notes |
|------|--------|-------|
| `default` | Match | Interactive prompts (simulated) |
| `acceptEdits` | Match | Auto-allow edit operations |
| `bypassPermissions` | Match | Skip all permission checks |
| `delegate` | Match | Use hooks for decisions |
| `dontAsk` | Match | Deny by default |
| `plan` | Match | No execution mode |

### MCP Support (Partial)

The simulator provides partial MCP support focused on tool injection:

| Feature | Status | Notes |
|---------|--------|-------|
| Config file parsing | Implemented | JSON and JSON5 |
| Server name in output | Implemented | `mcp_servers` array |
| Tool registration | Implemented | Via config or API |
| Actual server execution | Not supported | Simulated only |
| Dynamic tool discovery | Not supported | Manual registration |
| Server health checks | Not supported | Always "running" |

### Not Supported (By Design)

| Flag | Reason |
|------|--------|
| `--chrome` | Chrome integration out of scope |

### Low Priority (Not Validated)

- `--add-dir` - Additional directories
- `--agent` - Custom agent
- `--betas` - Beta headers
- `--json-schema` - Structured output schema
- `--tools` - Built-in tool list

## Output Formats

### JSON (`--output-format json`)

**Current behavior (diverges from real Claude):**
```json
{
  "type": "message",
  "role": "assistant",
  "content": [{"type": "text", "text": "Response"}],
  "model": "claude-sonnet-4-20250514",
  "stop_reason": "end_turn",
  "usage": {"input_tokens": 100, "output_tokens": 8}
}
```

**Target behavior (real Claude v2.1.12):**
```json
{
  "type": "result",
  "subtype": "success",
  "cost_usd": 0.001,
  "is_error": false,
  "duration_ms": 1000,
  "duration_api_ms": 950,
  "num_turns": 1,
  "result": "Response text",
  "session_id": "uuid"
}
```

See `tests/fixtures/cli/v2.1.12/json_output.normalized.json` for golden file.

### Stream JSON (`--output-format stream-json`)

**Current behavior (diverges from real Claude):**
1. `{"type": "message_start", ...}` - Missing system init
2. `{"type": "content_block_start", ...}`
3. `{"type": "content_block_delta", ...}` - Text chunks
4. `{"type": "content_block_stop", ...}`
5. `{"type": "message_delta", ...}` - Missing assistant wrapper
6. `{"type": "message_stop"}` - Missing result event

**Target behavior (real Claude v2.1.12):**
1. `{"type": "system", "subtype": "init", ...}` - Session initialization
2. `{"type": "assistant", "subtype": "message_start", ...}` - Message begins
3. `{"type": "content_block_start", ...}` - Content block starts
4. `{"type": "content_block_delta", ...}` - Text chunks
5. `{"type": "content_block_stop", ...}` - Block ends
6. `{"type": "assistant", "subtype": "message_delta", ...}` - Usage stats
7. `{"type": "assistant", "subtype": "message_stop"}` - Message ends
8. `{"type": "result", ...}` - Final result

See `tests/fixtures/cli/v2.1.12/stream_json.normalized.jsonl` for golden file.

### Text Output

Plain text output for human consumption. Matches real Claude behavior.

## Hook Protocol

The simulator implements the full hook protocol:

| Event | Status |
|-------|--------|
| `pre_tool_execution` | Match |
| `post_tool_execution` | Match |
| `notification` | Match |
| `permission_request` | Match |
| `session_start` | Match |
| `session_end` | Match |
| `prompt_submit` | Match |

Hook payloads include:
- `event` - Event type (snake_case)
- `session_id` - Session UUID
- `payload` - Event-specific data

Hook responses support:
- `proceed` - Boolean to allow/block
- `modified_payload` - Optional modified payload
- `error` - Error message if blocked
- `data` - Custom return data

## State Directory

The simulator creates a temporary directory structure matching `~/.claude`:

```
<temp>/
├── projects/       # Project-specific settings
├── todos/          # Todo list storage
├── plans/          # Plan files
├── sessions/       # Session data
└── settings.json   # Global settings
```

**Safety**: By default, the simulator uses a temp directory to prevent
accidentally modifying real Claude state. Use `CLAUDELESS_STATE_DIR` to
specify a custom location.

## Error Behavior

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (auth, network, permission bypass validation, etc.) |
| 2 | Partial response / interrupted |
| 130 | Interrupted by signal (Ctrl+C) |

### Error Result Format

Errors use the same result wrapper format:

```json
{
  "type": "result",
  "subtype": "error",
  "is_error": true,
  "error": "Error message",
  "cost_usd": 0,
  "duration_ms": 100
}
```

Rate limit errors include `retry_after`:

```json
{
  "type": "result",
  "subtype": "error",
  "is_error": true,
  "error": "Rate limited. Retry after 60 seconds.",
  "retry_after": 60
}
```

### Permission Bypass Errors

Using `--dangerously-skip-permissions` without `--allow-dangerously-skip-permissions`
produces an error:

```
Error: --dangerously-skip-permissions requires --allow-dangerously-skip-permissions to be set.
This is a safety measure. Only use this in sandboxed environments with no internet access.
```

## Known Limitations

1. **MCP Server Execution**: The simulator does not actually execute MCP servers.
   Tools are registered manually or via templates, not discovered from servers.

2. **Permission Prompts**: Interactive permission prompts are simulated, not
   actually displayed. Tests use `--dangerously-skip-permissions` or
   `--permission-mode bypassPermissions`.

3. **Chrome Integration**: Browser-based features are not simulated.

4. **Real API Costs**: The `cost_usd` field is always 0 since no real API
   calls are made.

5. **Token Counts**: Token counts are estimated (4 chars per token) rather
   than using actual tokenization.

6. **Timing**: `duration_ms` values are simulated and don't reflect actual
   processing time.

## Validation Methodology

The accuracy of this simulator was validated by:

1. **Golden File Comparison** (CLI output formats):
   - Fixtures captured from real Claude CLI v2.1.12
   - Comparison script: `scripts/compare-cli.sh`
   - Fixture tests: `tests/cli_fixtures.rs` (ignored until divergences fixed)
   - Current status: Divergences documented in `plans/epic-05x-fix-cli.md`

2. **Documentation Review** (CLI flags):
   - Compared flags against public Claude Code documentation
   - 23 flags documented with implementation status
   - No automated comparison against `claude --help` output

3. **Schema Validation** (output structure):
   - JSON/stream-JSON parsed and validated
   - Field presence verified
   - Event sequence tracked

4. **Property Testing**: Using property-based tests to verify invariants.

5. **Integration Tests**: Testing complete flows from CLI args to output.

### Validation Tools

- `scripts/compare-cli.sh` - Compare real vs simulated output
- `scripts/compare-cli.sh capture vX.Y.Z` - Capture new golden files
- `cargo test --test cli_fixtures -- --ignored` - Run failing fixture tests

The validation module (`claudeless::validation`) provides programmatic access
to:

- `CliAudit` - CLI flag implementation status
- `AccuracyReport` - Full accuracy report generation
- Output samples - Golden examples of real Claude output
