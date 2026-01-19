#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
#
# capture-tui.sh - Capture Claude TUI output via tmux
#
# Captures the TUI state of a running claude (or claudeless) session
# for visual fidelity comparison testing.
#
# Usage:
#   capture-tui.sh [options] [command]
#
# Options:
#   -o, --output FILE    Output file (default: stdout)
#   -w, --width WIDTH    Terminal width (default: 120)
#   -h, --height HEIGHT  Terminal height (default: 40)
#   -d, --delay SECS     Delay before capture in seconds (default: 2)
#   -k, --keys KEYS      Send additional keys after startup
#   -K, --keys-delay S   Delay after sending keys (default: 1)
#   --help               Show this help message
#
# Examples:
#   # Capture real claude TUI
#   capture-tui.sh -o initial_state.txt 'claude --model haiku'
#
#   # Capture simulator TUI
#   capture-tui.sh -o sim_state.txt 'claudeless --scenario test.json --tui'
#
#   # Capture after sending keys (trust prompt response)
#   capture-tui.sh -k 'Enter' 'claude --model haiku'

set -euo pipefail

# Defaults
OUTPUT=""
WIDTH=120
HEIGHT=40
DELAY=2
KEYS=""
KEYS_DELAY=1
COMMAND=""

# Generate unique session name
SESSION="claude-capture-$$"

# Cleanup function
cleanup() {
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        tmux kill-session -t "$SESSION" 2>/dev/null || true
    fi
}

# Register cleanup on exit
trap cleanup EXIT ERR INT TERM

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT="$2"
            shift 2
            ;;
        -w|--width)
            WIDTH="$2"
            shift 2
            ;;
        -h|--height)
            HEIGHT="$2"
            shift 2
            ;;
        -d|--delay)
            DELAY="$2"
            shift 2
            ;;
        -k|--keys)
            KEYS="$2"
            shift 2
            ;;
        -K|--keys-delay)
            KEYS_DELAY="$2"
            shift 2
            ;;
        --help)
            head -35 "$0" | tail -32
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
        *)
            COMMAND="$1"
            shift
            ;;
    esac
done

if [[ -z "$COMMAND" ]]; then
    echo "Error: No command specified" >&2
    echo "Usage: capture-tui.sh [options] <command>" >&2
    exit 1
fi

# Check tmux is available
if ! command -v tmux &>/dev/null; then
    echo "Error: tmux is required but not installed" >&2
    exit 1
fi

# Create tmux session
tmux new-session -d -s "$SESSION" -x "$WIDTH" -y "$HEIGHT"

# Send command to start the TUI
tmux send-keys -t "$SESSION" "$COMMAND" Enter

# Wait for TUI to start
sleep "$DELAY"

# Send additional keys if specified
if [[ -n "$KEYS" ]]; then
    tmux send-keys -t "$SESSION" "$KEYS"
    sleep "$KEYS_DELAY"
fi

# Capture the pane content
if [[ -n "$OUTPUT" ]]; then
    tmux capture-pane -t "$SESSION" -p > "$OUTPUT"
    echo "$OUTPUT"
else
    tmux capture-pane -t "$SESSION" -p
fi

# Cleanup happens via trap
