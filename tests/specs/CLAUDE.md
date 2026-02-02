# Test Specs

Specs drive claudeless integration tests. Each spec has two parts:

- **`capsh/{name}.capsh`** — A capture script (originally used to drive real Claude Code and record fixtures). In tests, it drives claudeless with the same keystrokes and timing, then compares output against the recorded fixtures in `tests/fixtures/`.
- **`scenarios/{name}.toml`** — Scenario config (session ID, model, default responses) so claudeless can run without a real API.

## How fixtures are generated

The `.capsh` scripts are run against real Claude Code by `tests/capture/capture.sh`, which records terminal snapshots into `tests/fixtures/v{VERSION}/`. See `tests/capture/CLAUDE.md` for the full capture workflow.

## DO NOT modify fixtures

Fixtures in `tests/fixtures/` are immutable ground truth from real Claude Code. If a spec fails, fix claudeless — never change the fixture. See `tests/fixtures/CLAUDE.md`.
