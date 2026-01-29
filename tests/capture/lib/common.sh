#!/bin/bash
# Common library for capsh fixture capture
# shellcheck disable=SC2034  # Variables used by sourcing scripts

set -euo pipefail

# Colors for output
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
DIM='\033[0;90m'
NC='\033[0m' # No Color

# Detect Claude CLI version
detect_version() {
    claude --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1
}

# Extract named snapshots from recording.jsonl and copy to fixtures directory
# Usage: extract_fixtures frames_dir fixtures_dir
extract_fixtures() {
    local frames_dir="$1"
    local fixtures_dir="$2"
    local recording="$frames_dir/recording.jsonl"

    if [[ ! -f "$recording" ]]; then
        echo -e "${RED}Error: No recording.jsonl found in $frames_dir${NC}" >&2
        return 1
    fi

    mkdir -p "$fixtures_dir"

    local count=0
    while IFS= read -r line; do
        # Look for snapshot entries with names: {"ms":...,"snapshot":"000001","name":"fixture_name"}
        if [[ "$line" =~ \"snapshot\":\"([0-9]+)\" ]]; then
            local frame_num="${BASH_REMATCH[1]}"

            # Check if this snapshot has a name
            if [[ "$line" =~ \"name\":\"([^\"]+)\" ]]; then
                local fixture_name="${BASH_REMATCH[1]}"

                local plain_frame="$frames_dir/${frame_num}.txt"
                local ansi_frame="$frames_dir/${frame_num}.ansi.txt"

                if [[ -f "$plain_frame" ]]; then
                    cp "$plain_frame" "$fixtures_dir/${fixture_name}.txt"
                    ((count++)) || true
                    echo -e "${DIM}  $fixture_name${NC}"
                else
                    echo -e "${YELLOW}Warning: Frame $frame_num not found for $fixture_name${NC}" >&2
                fi

                if [[ -f "$ansi_frame" ]]; then
                    cp "$ansi_frame" "$fixtures_dir/${fixture_name}.ansi.txt"
                fi
            fi
        fi
    done < "$recording"

    if [[ $count -eq 0 ]]; then
        echo -e "${YELLOW}Warning: No named snapshots found${NC}" >&2
    fi
}

# Timeouts for capture scripts (seconds)
KEYBOARD_TIMEOUT="${KEYBOARD_TIMEOUT:-30}"    # For UI/keyboard interactions
THINKING_TIMEOUT="${THINKING_TIMEOUT:-300}"   # For API calls/thinking operations

# Default Claude CLI args (use Haiku for speed)
DEFAULT_CLAUDE_ARGS="${DEFAULT_CLAUDE_ARGS:---model haiku}"

# Parse CLI args from script header comment
# Usage: parse_claude_args script.capsh
# Looks for: # Args: --model sonnet
#        or: # Args: (none)
parse_claude_args() {
    local script="$1"
    local args_line
    args_line=$(grep -E '^# Args:' "$script" | head -1) || true

    if [[ -n "$args_line" ]]; then
        local args="${args_line#\# Args:}"
        args="${args## }"  # trim leading space
        if [[ "$args" == "(none)" || "$args" == "none" ]]; then
            echo ""
        else
            echo "$args"
        fi
    else
        echo "$DEFAULT_CLAUDE_ARGS"
    fi
}

# Run a capsh script and capture output
# Usage: run_capture script.capsh raw_output_base fixtures_dir [timeout]
run_capture() {
    local script="$1"
    local raw_output_base="$2"
    local fixtures_dir="$3"
    local capture_timeout="${4:-$KEYBOARD_TIMEOUT}"

    local script_name
    script_name=$(basename "$script" .capsh)
    local raw_dir="$raw_output_base/$script_name"

    mkdir -p "$raw_dir" "$fixtures_dir"

    # Get Claude CLI args from script header or use default
    local claude_args
    claude_args=$(parse_claude_args "$script")

    echo -e "Running: ${CYAN}$script_name${NC}"
    [[ -n "$claude_args" ]] && echo -e "  Args: ${CYAN}$claude_args${NC}"

    # Run capsh with timeout
    # shellcheck disable=SC2086  # intentional word splitting for claude_args
    local exit_code=0
    timeout "$capture_timeout" capsh --frames "$raw_dir" -- claude $claude_args < "$script" || exit_code=$?

    # Exit codes: 0=success, 143=killed by SIGTERM (expected from kill TERM), 124=timeout
    if [[ $exit_code -ne 0 && $exit_code -ne 143 ]]; then
        if [[ $exit_code -eq 124 ]]; then
            echo -e "${RED}Error: $script_name timed out after ${capture_timeout}s${NC}" >&2
        else
            echo -e "${RED}Error: capsh failed for $script_name (exit $exit_code)${NC}" >&2
        fi
        return 1
    fi

    # Extract named fixtures from recording
    extract_fixtures "$raw_dir" "$fixtures_dir"
}

# Get the project root directory
get_project_root() {
    git rev-parse --show-toplevel 2>/dev/null || pwd
}

# Check if claude CLI is available
check_claude() {
    if ! command -v claude &>/dev/null; then
        echo -e "${RED}Error: claude CLI not found in PATH${NC}" >&2
        return 1
    fi
}

# Check if capsh is available
check_capsh() {
    if ! command -v capsh &>/dev/null; then
        echo -e "${RED}Error: capsh not found in PATH${NC}" >&2
        echo "Build with: cargo build --release -p capsh" >&2
        return 1
    fi
}
