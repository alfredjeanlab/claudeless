# Experimental Scripts

Scripts in this directory capture states that we are still working on reproducing reliably.

## Challenges

### permission-dialogs.capsh

Captures permission request dialogs. Challenges:

- Need Claude to request a specific permission (bash, file edit, etc.)
- Response timing is unpredictable
- Requires actual API interaction

Potential approaches:
- Mock API server that returns permission-requesting responses
- Pre-recorded sessions with known timing
- CI environment with specific prompts

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

## Running Experimental Scripts

```bash
# Run all including experimental (may fail)
RUN_EXPERIMENTAL=1 ./capture-all.sh

# Run single experimental script manually
capsh --frames /tmp/experimental -- claude < experimental/permission-dialogs.capsh
```

## Contributing

If you find a reliable way to capture these states:

1. Update the script with your approach
2. Move to `reliable/` if it now works consistently
3. Update this README
