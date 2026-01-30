# Skipped Scripts

Scripts in this directory capture states that we are still working on reproducing reliably.

## Challenges

### after-response.capsh

Captures UI state after Claude responds. Challenges:

- Need Claude to actually respond (requires API)
- Response content varies
- Timing is unpredictable

### compact-states.capsh

Captures context compaction states. Challenges:

- Need enough context to trigger compaction
- Compaction timing is model-dependent
- Requires multiple interactions

### failed_to_open_socket / failed_to_open_socket_no_version

Error states when Claude Code cannot connect to the API. Challenges:

- Requires simulating network failure (firewall block, DNS failure, etc.)
- Error conditions are transient and environment-dependent
- Can't reliably trigger FailedToOpenSocket errors in CI

Note: Reference fixtures exist in `crates/cli/tests/fixtures/tui/v2.1.14/` but were captured manually, not via capsh scripts.

## Running Skipped Scripts

```bash
# Run all including skipped (may fail)
RUN_SKIPPED=1 ./capture.sh

# Run single skipped script manually
capsh --frames /tmp/skipped -- claude < skipped/permission-dialogs.capsh
```

## Contributing

If you find a reliable way to capture these states:

1. Update the script with your approach
2. Move to `capsh/` if it now works consistently
3. Update this README
