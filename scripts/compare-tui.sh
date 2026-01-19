#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
#
# compare-tui.sh - Compare real Claude TUI with simulator TUI
#
# Captures and compares TUI output from real Claude CLI and the simulator,
# applying normalization for dynamic content.
#
# Usage:
#   compare-tui.sh [options]
#
# Options:
#   --real COMMAND       Command to run real Claude (default: 'claude --model haiku')
#   --sim COMMAND        Command to run simulator (default: auto-detected)
#   --scenario FILE      Scenario file for simulator
#   -k, --keys KEYS      Send additional keys after startup
#   -d, --delay SECS     Delay before capture (default: 2)
#   -o, --output DIR     Output directory for captures (default: /tmp)
#   --no-cleanup         Keep temporary files after comparison
#   --help               Show this help message
#
# Examples:
#   # Basic comparison
#   compare-tui.sh --scenario test.json
#
#   # Compare trust prompts
#   compare-tui.sh --scenario untrusted.json
#
#   # Compare after accepting trust
#   compare-tui.sh --scenario untrusted.json -k 'Enter'

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Defaults
REAL_CMD="claude --model haiku"
SIM_CMD=""
SCENARIO=""
KEYS=""
DELAY=2
OUTPUT_DIR="/tmp/compare-tui-$$"
CLEANUP=true

# Cleanup function
cleanup() {
    if [[ "$CLEANUP" == true ]] && [[ -d "$OUTPUT_DIR" ]]; then
        rm -rf "$OUTPUT_DIR" 2>/dev/null || true
    fi
}

trap cleanup EXIT ERR INT TERM

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --real)
            REAL_CMD="$2"
            shift 2
            ;;
        --sim)
            SIM_CMD="$2"
            shift 2
            ;;
        --scenario)
            SCENARIO="$2"
            shift 2
            ;;
        -k|--keys)
            KEYS="$2"
            shift 2
            ;;
        -d|--delay)
            DELAY="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --no-cleanup)
            CLEANUP=false
            shift
            ;;
        --help)
            head -35 "$0" | tail -32
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Auto-detect simulator if not specified
if [[ -z "$SIM_CMD" ]]; then
    # Look for built binary
    if [[ -f "$SCRIPT_DIR/../../../target/debug/claudeless" ]]; then
        SIM_BIN="$SCRIPT_DIR/../../../target/debug/claudeless"
    elif [[ -f "$SCRIPT_DIR/../../../target/release/claudeless" ]]; then
        SIM_BIN="$SCRIPT_DIR/../../../target/release/claudeless"
    else
        echo "Error: Cannot find claudeless binary. Run 'cargo build' first." >&2
        exit 1
    fi

    if [[ -n "$SCENARIO" ]]; then
        SIM_CMD="$SIM_BIN --scenario $SCENARIO --tui"
    else
        echo "Error: --scenario required when using auto-detected simulator" >&2
        exit 1
    fi
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build capture command arguments
CAPTURE_ARGS=(-d "$DELAY")
if [[ -n "$KEYS" ]]; then
    CAPTURE_ARGS+=(-k "$KEYS")
fi

# Capture real Claude TUI
echo "Capturing real Claude TUI..." >&2
"$SCRIPT_DIR/capture-tui.sh" "${CAPTURE_ARGS[@]}" -o "$OUTPUT_DIR/real.txt" "$REAL_CMD" >/dev/null

# Capture simulator TUI
echo "Capturing simulator TUI..." >&2
"$SCRIPT_DIR/capture-tui.sh" "${CAPTURE_ARGS[@]}" -o "$OUTPUT_DIR/sim.txt" "$SIM_CMD" >/dev/null

# Normalize function - applies to both outputs
normalize() {
    local input="$1"
    local output="$2"

    sed -E '
        # Replace timestamps (HH:MM:SS or HH:MM patterns)
        s/[0-9]{1,2}:[0-9]{2}(:[0-9]{2})?/<TIME>/g

        # Replace session IDs (typical UUID or session patterns)
        s/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/<SESSION>/gi
        s/session-[a-zA-Z0-9]+/<SESSION>/g

        # Replace temp directory paths (macOS and Linux patterns)
        s|/private/var/folders/[^/]+/[^/]+/[^/]+/[^[:space:]]+|<TEMPDIR>|g
        s|/tmp/[^[:space:]]+|<TEMPDIR>|g
        s|/var/tmp/[^[:space:]]+|<TEMPDIR>|g

        # Strip trailing whitespace per line (preserve leading and interior)
        s/[[:space:]]+$//
    ' "$input" > "$output"
}

# Normalize both captures
normalize "$OUTPUT_DIR/real.txt" "$OUTPUT_DIR/real_normalized.txt"
normalize "$OUTPUT_DIR/sim.txt" "$OUTPUT_DIR/sim_normalized.txt"

# Compare
echo "" >&2
echo "=== Comparison ===" >&2

if diff -u "$OUTPUT_DIR/real_normalized.txt" "$OUTPUT_DIR/sim_normalized.txt" > "$OUTPUT_DIR/diff.txt" 2>&1; then
    echo "MATCH: Simulator output matches real Claude TUI" >&2
    exit 0
else
    echo "DIVERGENCE: Differences found between real Claude and simulator" >&2
    echo "" >&2
    echo "--- Real Claude (normalized)" >&2
    echo "+++ Simulator (normalized)" >&2
    echo "" >&2
    cat "$OUTPUT_DIR/diff.txt"

    if [[ "$CLEANUP" == false ]]; then
        echo "" >&2
        echo "Files preserved in: $OUTPUT_DIR" >&2
        echo "  real.txt             - Raw real Claude capture" >&2
        echo "  sim.txt              - Raw simulator capture" >&2
        echo "  real_normalized.txt  - Normalized real Claude" >&2
        echo "  sim_normalized.txt   - Normalized simulator" >&2
        echo "  diff.txt             - Diff output" >&2
    fi
    exit 1
fi
