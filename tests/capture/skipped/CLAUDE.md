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

### api-usage-billing.capsh

Captures the initial TUI state when logged in with API Usage Billing. Challenges:

- Requires being logged in with an Anthropic Console account (API billing)
- Cannot be automated without switching login state
- Does not work with Claude subscription accounts (Max, Pro, Team, Enterprise)

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
