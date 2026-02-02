#!/bin/bash
# Common library for capsh fixture capture
# shellcheck disable=SC2034  # Variables used by sourcing scripts

set -euo pipefail

# Get script directory for relative paths
if [[ -n "${BASH_SOURCE[0]:-}" ]]; then
    COMMON_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    CAPTURE_DIR="$(dirname "$COMMON_SCRIPT_DIR")"
else
    # Fallback for direct sourcing
    CAPTURE_DIR="${CAPTURE_DIR:-$(pwd)}"
fi

# Load capture environment (OAuth token, etc.)
load_capture_env() {
    local env_file="$CAPTURE_DIR/.env"

    if [[ -f "$env_file" ]]; then
        # shellcheck source=/dev/null
        source "$env_file"
    fi

    if [[ -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" ]]; then
        echo -e "${RED}Error: CLAUDE_CODE_OAUTH_TOKEN not set${NC}" >&2
        echo "" >&2
        echo "Capture scripts require an OAuth token to authenticate with Claude CLI." >&2
        echo "" >&2
        echo "Setup:" >&2
        echo "  1. Run: claude setup-token" >&2
        echo "  2. Add to tests/capture/.env:" >&2
        echo "     CLAUDE_CODE_OAUTH_TOKEN=<your-token>" >&2
        echo "" >&2
        echo "See: tests/capture/.env.example" >&2
        return 1
    fi

    export CLAUDE_CODE_OAUTH_TOKEN
}

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

# Parse workspace from script header comment
# Usage: parse_workspace script.capsh
# Looks for: # Workspace: /path/to/dir
#        or: # Workspace: (temp)
parse_workspace() {
    local script="$1"
    local ws_line
    ws_line=$(grep -E '^# Workspace:' "$script" | head -1) || true

    if [[ -n "$ws_line" ]]; then
        local ws="${ws_line#\# Workspace:}"
        ws="${ws## }"  # trim leading space
        if [[ "$ws" == "(temp)" || "$ws" == "temp" ]]; then
            echo ""  # Will use temp dir
        else
            echo "$ws"
        fi
    else
        echo ""  # Default to temp dir
    fi
}

# Parse config mode from script header comment
# Usage: parse_config_mode script.capsh
# Looks for: # Config: trusted|auth-only|empty
# Returns: trusted (default), auth-only, or empty
parse_config_mode() {
    local script="$1"
    local config_line
    config_line=$(grep -E '^# Config:' "$script" | head -1) || true

    if [[ -n "$config_line" ]]; then
        local mode="${config_line#\# Config:}"
        mode="${mode## }"  # trim leading space
        mode="${mode%% *}" # trim trailing content
        case "$mode" in
            auth-only|empty)
                echo "$mode"
                ;;
            *)
                echo "trusted"
                ;;
        esac
    else
        echo "trusted"
    fi
}

# Sanitize state files to normalize paths and redact sensitive data
# Usage: sanitize_state config_dir
sanitize_state() {
    local config_dir="$1"

    # Find all text files and sanitize them (including .json.backup.* files)
    find "$config_dir" -type f \( -name "*.json" -o -name "*.json.backup.*" -o -name "*.jsonl" -o -name "*.txt" -o -name "*.md" \) | while read -r file; do
        # Replace /Users/{username}/ with /Users/alfred/
        # Strip common subdirs (Developer, Desktop, Documents)
        # Replace /private/var/folders/... temp paths with /tmp/workspace/
        # Replace userID values with <user_id>
        sed -i.bak \
            -e 's|/Users/[^/]*/Developer/|/Users/alfred/|g' \
            -e 's|/Users/[^/]*/Desktop/|/Users/alfred/|g' \
            -e 's|/Users/[^/]*/Documents/|/Users/alfred/|g' \
            -e 's|/Users/[^/]*/|/Users/alfred/|g' \
            -e 's|/private/var/folders/[^"]*|/tmp/workspace|g' \
            -e 's|"userID": *"[^"]*"|"userID": "<user_id>"|g' \
            -e 's|"accountUuid": *"[^"]*"|"accountUuid": "<account_uuid>"|g' \
            -e 's|"organizationUuid": *"[^"]*"|"organizationUuid": "<org_uuid>"|g' \
            -e 's|"emailAddress": *"[^"]*"|"emailAddress": "user@example.com"|g' \
            -e 's|"displayName": *"[^"]*"|"displayName": "User"|g' \
            -e 's|"organizationName": *"[^"]*"|"organizationName": "Organization"|g' \
            "$file"
        rm -f "$file.bak"
    done
}

# Write a minimal Claude config with pre-trusted workspace
# Usage: write_claude_config config_dir workspace_path [version]
write_claude_config() {
    local config_dir="$1"
    local workspace_path="$2"
    local version="${3:-$(detect_version)}"

    mkdir -p "$config_dir"
    cat > "$config_dir/.claude.json" << EOF
{
  "hasCompletedOnboarding": true,
  "lastOnboardingVersion": "$version",
  "projects": {
    "$workspace_path": {
      "hasTrustDialogAccepted": true,
      "allowedTools": []
    }
  }
}
EOF
}

# Write a config with auth but no workspace trust (for trust dialog captures)
# Usage: write_auth_only_config config_dir [version]
write_auth_only_config() {
    local config_dir="$1"
    local version="${2:-$(detect_version)}"

    mkdir -p "$config_dir"
    cat > "$config_dir/.claude.json" << EOF
{
  "hasCompletedOnboarding": true,
  "lastOnboardingVersion": "$version",
  "projects": {}
}
EOF
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

    # Parse workspace from script header (default: temp dir)
    local workspace
    workspace=$(parse_workspace "$script")
    if [[ -z "$workspace" ]]; then
        workspace=$(mktemp -d)
    fi
    # Resolve to absolute path (macOS /tmp -> /private/tmp)
    workspace="$(cd "$workspace" && pwd -P)"

    # Parse config mode from script header (default: trusted)
    local config_mode
    config_mode=$(parse_config_mode "$script")

    # Create isolated config dir based on mode
    local config_dir="$raw_dir/state"
    local use_oauth_token="1"
    case "$config_mode" in
        trusted)
            write_claude_config "$config_dir" "$workspace"
            ;;
        auth-only)
            write_auth_only_config "$config_dir"
            ;;
        empty)
            mkdir -p "$config_dir"
            use_oauth_token=""
            ;;
    esac

    # Snapshot state before capture (relative paths)
    (cd "$raw_dir" && find state -type f | sort) > "$raw_dir/state.before.txt"

    echo -e "Running: ${CYAN}$script_name${NC}"
    [[ -n "$claude_args" ]] && echo -e "  Args: ${CYAN}$claude_args${NC}"
    [[ "$config_mode" != "trusted" ]] && echo -e "  Config: ${MAGENTA}$config_mode${NC}"
    echo -e "  ${DIM}Workspace: $workspace${NC}"

    # Run capsh with isolated config
    # Note: We run capsh from the workspace directory so claude detects it correctly
    # Only pass OAuth token if not in 'empty' mode (onboarding capture)
    # shellcheck disable=SC2086  # intentional word splitting for claude_args
    local exit_code=0
    (
        cd "$workspace"
        CLAUDE_CONFIG_DIR="$config_dir" \
        CLAUDE_CODE_OAUTH_TOKEN="${use_oauth_token:+${CLAUDE_CODE_OAUTH_TOKEN:-}}" \
        timeout "$capture_timeout" capsh --frames "$raw_dir" -- \
            claude $claude_args < "$script"
    ) || exit_code=$?

    # Exit codes: 0=success, 143=killed by SIGTERM (expected from kill TERM), 124=timeout
    if [[ $exit_code -ne 0 && $exit_code -ne 143 ]]; then
        if [[ $exit_code -eq 124 ]]; then
            echo -e "${RED}Error: $script_name timed out after ${capture_timeout}s${NC}" >&2
        else
            echo -e "${RED}Error: capsh failed for $script_name (exit $exit_code)${NC}" >&2
        fi
        return 1
    fi

    # Sanitize state files (normalize paths, redact PII)
    sanitize_state "$config_dir"

    # Snapshot state after capture and generate diff (relative paths)
    (cd "$raw_dir" && find state -type f | sort) > "$raw_dir/state.after.txt"
    diff "$raw_dir/state.before.txt" "$raw_dir/state.after.txt" > "$raw_dir/state.diff" || true

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

# Capture tmux pane to fixture file
# Usage: capture_tmux_pane session_name fixture_name fixtures_dir
capture_tmux_pane() {
    local session_name="$1"
    local fixture_name="$2"
    local fixtures_dir="$3"

    mkdir -p "$fixtures_dir"

    # Capture pane and strip shell prompt lines (everything before Claude logo)
    tmux capture-pane -t "$session_name" -p | sed -n '/▐▛███▜▌/,$p' > "$fixtures_dir/${fixture_name}.tmux.txt"

    echo -e "${DIM}  $fixture_name${NC}"
}
