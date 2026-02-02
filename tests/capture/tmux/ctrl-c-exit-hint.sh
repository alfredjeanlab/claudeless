#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
# Capture "Press Ctrl-C again to exit" hint using tmux.
#
# Ctrl-C cannot be captured via capsh because TUI runs in raw mode,
# but tmux can send the actual signal to trigger the hint.

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

SESSION="claude-ctrl-c-$$"
COLS=80
ROWS=24

# Cleanup on exit
cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo -e "${CYAN}Running:${NC} ctrl-c-exit-hint (tmux)"

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

# Send first Ctrl-C to trigger hint
tmux send-keys -t "$SESSION" C-c

# Wait for hint to appear (check for the hint text)
for i in {1..10}; do
    if tmux capture-pane -t "$SESSION" -p | grep -q "Ctrl-C again"; then
        break
    fi
    sleep 0.5
done

# Capture the pane to fixture
capture_tmux_pane "$SESSION" "ctrl_c_exit_hint" "$FIXTURES_DIR"

echo -e "${GREEN}✓${NC} Captured ctrl_c_exit_hint"

# Send second Ctrl-C to actually exit
tmux send-keys -t "$SESSION" C-c
sleep 1
