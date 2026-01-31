# TUI Fixture Capture

Capture TUI fixtures from real Claude Code for testing claudeless.

## Setup

Capture scripts require an OAuth token to authenticate with Claude CLI.

```bash
# Generate a long-lived token
claude setup-token

# Save to .env (gitignored)
echo "CLAUDE_CODE_OAUTH_TOKEN=<your-token>" > tests/capture/.env
```

See `.env.example` for the template.

## Capture Scenarios

The capture system supports three scenarios based on config state:

| Scenario | Config | Shows | Use For |
|----------|--------|-------|---------|
| **Trusted** | Pre-configured | Straight to prompt | Most captures |
| **Trust dialog** | Auth only | Trust dialog | Permission fixtures |
| **Full onboarding** | Empty | Theme, login, security, trust | Onboarding fixtures |

### Scenario 1: Trusted (Default)

Most capture scripts use this. The workspace is pre-trusted, so Claude goes straight to the prompt.

**Explore with tmux:**
```bash
# Create isolated config with pre-trusted workspace
workspace=$(mktemp -d)
config_dir=$(mktemp -d)
cat > "$config_dir/.claude.json" << EOF
{
  "hasCompletedOnboarding": true,
  "lastOnboardingVersion": "$(claude --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')",
  "projects": {
    "$(cd "$workspace" && pwd -P)": {
      "hasTrustDialogAccepted": true,
      "allowedTools": []
    }
  }
}
EOF

# Run Claude with isolated config
tmux new-session -d -s claude-test -x 80 -y 24 -c "$workspace" \
  "CLAUDE_CONFIG_DIR=$config_dir CLAUDE_CODE_OAUTH_TOKEN=\$CLAUDE_CODE_OAUTH_TOKEN claude --model haiku"
tmux attach -t claude-test

# Cleanup
tmux kill-session -t claude-test
```

**Capture script:** (default behavior)
```capsh
# Captures help dialog
# Args: --model haiku

wait "for shortcuts" 10s
send "/help" 100 <Enter>
wait "claude.com" 10s
snapshot "help_response"

send <C-u> 100 "/exit" 100 <Enter>
wait 500
kill TERM
```

### Scenario 2: Trust Dialog Only

For capturing trust/permission dialogs. Has auth but workspace not trusted.

**Explore with tmux:**
```bash
# Create config with auth but NO project trust
workspace=$(mktemp -d)
config_dir=$(mktemp -d)
cat > "$config_dir/.claude.json" << EOF
{
  "hasCompletedOnboarding": true,
  "lastOnboardingVersion": "$(claude --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')",
  "projects": {}
}
EOF

# Run - will show trust dialog
tmux new-session -d -s claude-test -x 80 -y 24 -c "$workspace" \
  "CLAUDE_CONFIG_DIR=$config_dir CLAUDE_CODE_OAUTH_TOKEN=\$CLAUDE_CODE_OAUTH_TOKEN claude --model haiku"
tmux attach -t claude-test
```

**Capture script:**
```capsh
# Captures trust dialog
# Args: --model haiku
# Config: auth-only

wait "trust this folder" 10s
snapshot "trust_dialog"

send <Enter>
wait "for shortcuts" 10s
snapshot "after_trust"

send <C-u> 100 "/exit" 100 <Enter>
wait 500
kill TERM
```

### Scenario 3: Full Onboarding

For capturing the complete first-run experience. Empty config, no auth.

**Explore with tmux:**
```bash
# Create empty config dir (no auth, no onboarding)
config_dir=$(mktemp -d)
workspace=$(mktemp -d)

# Run - will show full onboarding (theme, login, security, trust)
tmux new-session -d -s claude-test -x 80 -y 24 -c "$workspace" \
  "CLAUDE_CONFIG_DIR=$config_dir claude --model haiku"
tmux attach -t claude-test
```

**Capture script:**
```capsh
# Captures onboarding flow
# Args: --model haiku
# Config: empty

# Theme selection
wait "Dark mode" 10s
snapshot "theme_picker"
send <Enter>

# Login selection
wait "Claude account" 10s
snapshot "login_picker"
# ... manual auth required ...
```

Note: Full onboarding requires manual OAuth flow and cannot be fully automated.

## How to Explore

Use capsh or tmux to explore Claude CLI behavior before writing capture scripts.

Always use `--model haiku` unless otherwise instructed.

### Quick exploration with capsh

```bash
# Explore with frame capture (uses default trusted config)
source tests/capture/.env
workspace=$(mktemp -d)
cd "$workspace"
capsh --frames /tmp/explore -- claude --model haiku <<'EOF'
wait "for shortcuts" 10s
wait 30s
EOF

# Check what was captured
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

Write `.capsh` scripts in `tests/capture/capsh/`.
See [docs/CAPSH.md](../../docs/CAPSH.md) for full DSL reference.

### Script header

```capsh
# Description of what this script captures.
# Args: --model haiku
# Workspace: (temp)
# Config: trusted|auth-only|empty
```

| Header | Default | Description |
|--------|---------|-------------|
| `Args` | `--model haiku` | Claude CLI arguments |
| `Workspace` | `(temp)` | Working directory (temp or path) |
| `Config` | `trusted` | Config scenario (see above) |

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
│   ├── state/              # Isolated Claude config/state after capture
│   ├── state.before.txt    # File listing before capture
│   ├── state.after.txt     # File listing after capture
│   ├── state.diff          # Diff of state changes
│   ├── 000001.txt          # Plain text frames
│   ├── 000001.ansi.txt     # ANSI frames
│   ├── recording.jsonl     # Timing log
│   └── latest.txt          # Latest frame

tests/fixtures/v{VERSION}/
├── {snapshot_name}.txt       # Extracted fixtures
└── {snapshot_name}.ansi.txt
```

### Adding new fixtures

1. Explore the behavior you want to capture
2. Create a `.capsh` script in `tests/capture/capsh/`
3. Run `./tests/capture/capture.sh --script your-script` to test
4. Verify fixtures in `tests/fixtures/v{VERSION}/`

### Special cases: tmux scripts

Some fixtures can't be captured with capsh (Ctrl-C, Ctrl-D) due to raw mode. These use tmux scripts in `tests/capture/tmux/*.sh` which are automatically run by `capture.sh`.

To run tmux scripts individually:

```bash
./tests/capture/tmux/ctrl-c-exit-hint.sh
./tests/capture/tmux/ctrl-d-exit-hint.sh
```

Tmux scripts directly control a tmux session and capture pane output to fixtures.
