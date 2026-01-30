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

### setup-*.capsh & slash-logout.capsh

Captures the initial setup flow and logout states. Challenges:

- Require fresh/unconfigured Claude installation (no existing login)
- Setup flow only appears once per account configuration
- slash_logout requires an active session to log out from
- Would need to mock or reset configuration between captures
- Complex to automate in CI without affecting real user config

These fixtures exist in `crates/cli/tests/fixtures/tui/v2.1.14/`:
- `setup_01_select_theme_dark.txt`
- `setup_01_select_theme_light.txt`
- `setup_01a_syntax_highlighting_disabled.txt`
- `setup_02_login_method.txt`
- `setup_03_login_browser.txt`
- `setup_03_security_notes.txt`
- `setup_04_login_success.txt`
- `setup_05_use_terminal_setup.txt`
- `slash_logout.txt`

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
