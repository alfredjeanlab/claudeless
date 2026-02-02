#!/bin/bash
# Capture trust prompt dialog using tmux.
#
# Creates a temp folder with .claude/settings.json containing allowedTools,
# then runs claude --model haiku from that folder to trigger the trust prompt.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

VERSION=$(detect_version)
if [[ -z "$VERSION" ]]; then
    echo -e "${RED}Error: Could not detect Claude CLI version${NC}" >&2
    exit 1
fi

FIXTURES_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")/fixtures/v${VERSION}"
mkdir -p "$FIXTURES_DIR"

SESSION="claude-trust-$$"
COLS=80
ROWS=24

# Create temp directory with .claude/settings.json
TEMP_DIR=$(mktemp -d)
mkdir -p "$TEMP_DIR/.claude"
cat > "$TEMP_DIR/.claude/settings.json" <<'EOF'
{
  "allowedTools": ["Bash", "Read"],
  "permissions": {
    "allow": ["Bash", "Read"]
  }
}
EOF

# Cleanup on exit
cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "${CYAN}Running:${NC} trust-prompt-dialog (tmux)"

# Start tmux session
tmux new-session -d -s "$SESSION" -x $COLS -y $ROWS

# Change to temp directory and run Claude
tmux send-keys -t "$SESSION" "cd '$TEMP_DIR' && claude --model haiku" Enter

# Wait for trust prompt to appear (look for "trust the files")
for i in {1..30}; do
    if tmux capture-pane -t "$SESSION" -p | grep -qi "trust"; then
        break
    fi
    sleep 0.5
done

# Give it a moment to fully render
sleep 0.5

# Capture the pane and strip shell prompt lines (keep from horizontal rule onward)
tmux capture-pane -t "$SESSION" -p | sed -n '/^─/,$p' > "$FIXTURES_DIR/trust_prompt.tmux.txt"

echo -e "${GREEN}✓${NC} Captured trust_prompt"

# Send Escape to cancel and exit
tmux send-keys -t "$SESSION" Escape
sleep 1
