# Test Fixtures

These fixtures are captured from the real Claude Code CLI using the capture system in `tests/capture/`. They represent ground-truth terminal output (TUI snapshots, state diffs) from actual Claude Code sessions.

## How they are generated

Capture scripts in `tests/capture/capsh/` drive real Claude Code inside a pty, take named snapshots at specific moments, and extract the terminal content into fixture files. The capture tool (`tests/capture/capture.sh`) handles isolated config, tmux sessions, and frame extraction. See `tests/capture/CLAUDE.md` for details.

## What they are for

Claudeless tests compare their own TUI rendering against these fixtures to verify pixel-perfect compatibility with real Claude Code behavior.

## DO NOT MODIFY THESE FILES

These fixtures are immutable ground truth. Under no circumstances should any AI agent, automated tool, or code editor directly modify, "fix", or "update" any file in this directory.

If a test fails because claudeless output does not match a fixture:

- Fix claudeless to match the fixture.
- If you cannot fix claudeless, leave the test failing and explain why.
- NEVER change the fixture to match claudeless.

The only way fixtures should change is by re-running the capture system against a new version of Claude Code, which produces new versioned fixture directories.
