# TUI Fixture Capture

Capture TUI fixtures from real Claude Code for testing claudeless.

## How to Explore

Use capsh interactively to explore Claude CLI behavior before writing capture scripts.

Always use `--model haiku` unless otherwise instructed.

### Quick exploration

```bash
# Explore with frame capture
capsh --frames /tmp/explore -- claude --model haiku <<'EOF'
wait "for shortcuts" 10s
wait 30s
EOF

# Check what was captured
ls /tmp/explore/
cat /tmp/explore/latest.txt
```

### Interactive exploration with tmux

```bash
# Start a tmux session
tmux new-session -d -s claude-test -x 80 -y 24
tmux send-keys -t claude-test 'claude --model haiku' Enter

# Attach to watch
tmux attach -t claude-test

# In another terminal, send keys and capture
tmux send-keys -t claude-test '/help' Enter
tmux capture-pane -t claude-test -p > /tmp/help.txt

# Cleanup
tmux kill-session -t claude-test
```


## How to Write Captures

Write `.capsh` scripts to capture fixtures.
See [docs/CAPSH.md](../../docs/CAPSH.md) for full DSL reference.

### Script structure

```capsh
# Description of what this script captures.
# Args: --model haiku

wait "for shortcuts" 10s    # Wait for Claude to be ready

# Perform actions
send "/help"
wait "claude.com" 10s
snapshot "help_response"

# Exit cleanly
send <C-u> 100 "/exit" 100 <Enter>
wait 500
kill TERM
```

### Key patterns

```capsh
# Wait for Claude ready state
wait "for shortcuts" 10s

# Send text with inline delays
send "/help" 100 <Enter>

# Named snapshots become fixtures
snapshot "fixture_name"

# Clean exit sequence
send <C-u> 100 "/exit" 100 <Enter>
wait 500
kill TERM
```

### Common keys

| Key | Description |
|-----|-------------|
| `<Enter>` | Submit input |
| `<Esc>` | Cancel/clear |
| `<Tab>` | Tab complete |
| `<C-u>` | Clear input line |
| `<M-p>` | Model picker |
| `<M-t>` | Thinking toggle |

### Running scripts

Always use `--script` to run only the script being authored (unless otherwise instructed).

```bash
# Run single script (preferred when authoring)
./tests/capture/capture.sh --script help-dialog

# Retry failed scripts
./tests/capture/capture.sh --retry

# Run all capture scripts (ONLY if directly instructed)
./tests/capture/capture.sh
```

### Output structure

```
tests/capture/output/v{VERSION}/
├── {script-name}/
│   ├── 000001.txt       # Plain text frames
│   ├── 000001.ansi.txt  # ANSI frames
│   ├── recording.jsonl  # Timing log
│   └── latest.txt       # Latest frame

tests/fixtures/v{VERSION}/
├── {snapshot_name}.txt       # Extracted fixtures
└── {snapshot_name}.ansi.txt
```

### Adding new fixtures

1. Explore the behavior you want to capture
2. Create a `.capsh` script in `tests/capture/scripts/`
3. Run `./tests/capture/capture.sh --script your-script` to test
4. Verify fixtures in `tests/fixtures/v{VERSION}/`
5. Update `TODO.md` to mark as finished

### Special cases: tmux scripts

Some fixtures can't be captured with capsh (Ctrl-C, Ctrl-D) due to raw mode. These use tmux scripts in `tests/capture/tmux/*.sh` which are automatically run by `capture.sh`.

To run tmux scripts individually:

```bash
./tests/capture/tmux/ctrl-c-exit-hint.sh
./tests/capture/tmux/ctrl-d-exit-hint.sh
```

Tmux scripts directly control a tmux session and capture pane output to fixtures.
