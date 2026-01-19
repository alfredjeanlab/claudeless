# CLI Output Fixtures

Captured from real Claude CLI for validation testing.

**Behavior observed with:** claude --version 2.1.12 (Claude Code)

## Directory Structure

```
cli/
├── README.md
└── v{version}/
    ├── json-output/
    │   ├── scenario.toml      # Test scenario
    │   └── output.json        # Normalized expected output
    └── stream-json/
        ├── scenario.toml      # Test scenario
        └── output.jsonl       # Normalized expected output
```

## Normalized Fields

The following fields are replaced with placeholders in normalized files:

| Field | Placeholder |
|-------|-------------|
| `session_id` | `<SESSION_ID>` |
| `duration_ms` | `<DURATION>` |
| `duration_api_ms` | `<DURATION>` |
| `cost_usd` / `total_cost_usd` | `<COST>` |
| `timestamp` | `<TIMESTAMP>` |
| `request_id` | `<REQUEST_ID>` |
| `uuid` | `<UUID>` |
| `id` (msg_*) | `<MESSAGE_ID>` |
| `cwd` | `<CWD>` |
| `result` / `text` | `<RESPONSE_TEXT>` |
| `content` (arrays) | `<CONTENT>` |
| `usage` | `<USAGE>` |
| `modelUsage` | `<MODEL_USAGE>` |
| `plugins` | `<PLUGINS>` |
| `mcp_servers` | `<MCP_SERVERS>` |

## Capture Method

Use the compare-cli.sh script:
```bash
./crates/claudeless/scripts/compare-cli.sh capture v2.1.12
```
