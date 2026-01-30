# Skipped Scripts

Scripts in this directory capture states that we are still working on reproducing reliably.

## API/Response Fixtures

These fixtures require actual API responses from Claude and cannot be captured with mock/offline scenarios.

### after_response

Captures UI state after Claude responds.

- Requires Claude to actually respond (requires API)
- Response content varies
- Timing is unpredictable

### clear_before / clear_after

Captures the `/clear` command states before and after clearing conversation.

- Requires active conversation with responses
- Need to capture state before clear, then after
- Depends on having API responses to clear

### thinking_dialog_disabled_selected

Captures the thinking dialog with "disabled" option selected.

- Requires mid-conversation state (thinking toggle affects next response)
- Need to navigate dialog during active session
- Selection state depends on prior API interactions

### thinking_dialog_enabled_selected

Captures the thinking dialog with "enabled" option selected.

- Requires mid-conversation state (thinking toggle affects next response)
- Need to navigate dialog during active session
- Selection state depends on prior API interactions

## Other Challenges

### compact-states.capsh

Captures context compaction states. Challenges:

- Need enough context to trigger compaction
- Compaction timing is model-dependent
- Requires multiple interactions

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
