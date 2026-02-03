# TODO

Follow-up items from completed features.

## Input Shortcuts

### Ctrl+_ to Undo

Integration tests for Ctrl+_ are marked `#[ignore]` because tmux cannot reliably send Ctrl+_. Unit tests verify this behavior works correctly.

**Tests:** `tui_interaction.rs` Ctrl+_ tests (integration tests `#[ignore]`, unit tests active)

**Status:** Behavior implemented; integration tests skipped due to tmux limitation.

## Scenario Configuration

### Subscription Level in Header

Subscription text is configurable via the `provider` field in scenario config (defaults to "Claude Max").

**Fixtures captured:**
- `Claude Max` - model_haiku.txt, model_sonnet.txt, model_opus.txt
- `API Usage Billing` - api_usage_billing.txt (v2.1.14)

**Examples to capture:** `Claude Pro`, `Free`, `Enterprise` (or `Team`?)

**Status:** Default implemented; additional fixture variants not yet captured.

## Stream-JSON Output

### System Init Event

The stream-json system init event is missing several fields present in real Claude Code output.

**Missing fields:** `agents`, `apiKeySource`, `claude_code_version`, `cwd`, `output_style`, `permissionMode`, `plugins`, `skills`, `slash_commands`

**Tests:** `test_stream_json_starts_with_system_init` in `stream_json.rs` (`#[ignore]`)

**Status:** Deferred.

## TUI Setup Flow

Setup wizard screens (theme selection, login/logout, connection errors) are not implemented. All 9 setup-related integration tests are `#[ignore]`.

**Tests:** `tui_setup.rs` (9 tests, all `#[ignore]`)

**Status:** Deferred.
