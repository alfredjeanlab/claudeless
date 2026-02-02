#!/bin/bash
# Capture Ctrl-Z suspend message using tmux.
#
# Ctrl-Z cannot be captured via capsh because TUI runs in raw mode,
# but tmux can send the actual SIGTSTP signal to trigger the suspend.

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

SESSION="claude-ctrl-z-$$"
COLS=80
ROWS=24

# Cleanup on exit
cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo -e "${CYAN}Running:${NC} ctrl-z-suspend (tmux)"

# Start tmux session with Claude
tmux new-session -d -s "$SESSION" -x $COLS -y $ROWS
tmux send-keys -t "$SESSION" 'claude --model haiku' Enter

# Wait for Claude to be ready (look for the prompt)
for i in {1..30}; do
    if tmux capture-pane -t "$SESSION" -p | grep -q "for shortcuts"; then
        break
    fi
    sleep 0.5
done

# Send Ctrl-Z to suspend Claude
tmux send-keys -t "$SESSION" C-z

# Wait for suspend message to appear
for i in {1..10}; do
    if tmux capture-pane -t "$SESSION" -p | grep -q "has been suspended"; then
        break
    fi
    sleep 0.5
done

# Capture the suspend message (it appears after the TUI exits)
# The message starts with "Claude Code has been suspended"
tmux capture-pane -t "$SESSION" -p | grep -A1 "Claude Code has been suspended" > "$FIXTURES_DIR/ctrl_z_suspend.tmux.txt"

echo -e "${GREEN}✓${NC} Captured ctrl_z_suspend"

# Resume and exit Claude cleanly
tmux send-keys -t "$SESSION" 'fg' Enter
sleep 0.5
tmux send-keys -t "$SESSION" C-c
sleep 0.5
tmux send-keys -t "$SESSION" C-c
sleep 1
