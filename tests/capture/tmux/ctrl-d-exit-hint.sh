#!/bin/bash
# Capture "Press Ctrl-D to exit" hint using tmux.
#
# Ctrl-D cannot be captured via capsh because TUI runs in raw mode,
# but tmux can send the actual EOF signal to trigger the hint.

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

SESSION="claude-ctrl-d-$$"
COLS=80
ROWS=24

# Cleanup on exit
cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo -e "${CYAN}Running:${NC} ctrl-d-exit-hint (tmux)"

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

# Send first Ctrl-D to trigger hint
tmux send-keys -t "$SESSION" C-d

# Wait for hint to appear (check for the hint text)
for i in {1..10}; do
    if tmux capture-pane -t "$SESSION" -p | grep -q "Ctrl-D"; then
        break
    fi
    sleep 0.5
done

# Capture the pane to fixture
capture_tmux_pane "$SESSION" "ctrl_d_exit_hint" "$FIXTURES_DIR"

echo -e "${GREEN}✓${NC} Captured ctrl_d_exit_hint"

# Send second Ctrl-D to actually exit
tmux send-keys -t "$SESSION" C-d
sleep 1
