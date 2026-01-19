#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
#

# Compare real Claude CLI output against claudeless output
# Uses haiku model to minimize costs (validates format, not quality)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_DIR="$(cd "$CRATE_DIR/../.." && pwd)"

# Configuration
MODEL="${MODEL:-claude-3-5-haiku-latest}"
TEST_PROMPT="${TEST_PROMPT:-Hello}"
VERBOSE="${VERBOSE:-0}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "$1"
}

log_verbose() {
    if [[ "$VERBOSE" == "1" ]]; then
        echo -e "$1"
    fi
}

# Create temp directory and register cleanup
TMPDIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT ERR INT

# Normalize JSON output for comparison
# Replaces dynamic fields with placeholders
normalize_json() {
    local input="$1"
    # Use jq to normalize:
    # - session_id -> "<SESSION_ID>"
    # - duration_ms -> "<DURATION>"
    # - duration_api_ms -> "<DURATION>"
    # - cost_usd -> "<COST>"
    # - Any timestamp fields -> "<TIMESTAMP>"
    # - request_id -> "<REQUEST_ID>"
    jq '
        walk(
            if type == "object" then
                with_entries(
                    if .key == "session_id" then .value = "<SESSION_ID>"
                    elif .key == "duration_ms" then .value = "<DURATION>"
                    elif .key == "duration_api_ms" then .value = "<DURATION>"
                    elif .key == "cost_usd" then .value = "<COST>"
                    elif .key == "total_cost_usd" then .value = "<COST>"
                    elif .key == "costUSD" then .value = "<COST>"
                    elif .key == "timestamp" then .value = "<TIMESTAMP>"
                    elif .key == "request_id" then .value = "<REQUEST_ID>"
                    elif .key == "uuid" then .value = "<UUID>"
                    elif .key == "id" and (.value | type) == "string" and (.value | startswith("msg_")) then .value = "<MESSAGE_ID>"
                    elif .key == "cwd" then .value = "<CWD>"
                    elif .key == "path" and (.value | type) == "string" and (.value | contains("/")) then .value = "<PATH>"
                    elif .key == "plugins" then .value = "<PLUGINS>"
                    elif .key == "mcp_servers" then .value = "<MCP_SERVERS>"
                    elif .key == "result" then .value = "<RESPONSE_TEXT>"
                    elif .key == "text" then .value = "<RESPONSE_TEXT>"
                    elif .key == "content" and (.value | type) == "array" then .value = "<CONTENT>"
                    elif .key == "usage" then .value = "<USAGE>"
                    elif .key == "modelUsage" then .value = "<MODEL_USAGE>"
                    elif .key == "input_tokens" then .value = "<TOKENS>"
                    elif .key == "output_tokens" then .value = "<TOKENS>"
                    elif .key == "cache_creation_input_tokens" then .value = "<TOKENS>"
                    elif .key == "cache_read_input_tokens" then .value = "<TOKENS>"
                    else .
                    end
                )
            else .
            end
        )
    ' <<< "$input" | jq -S '.'  # Sort keys for consistent comparison
}

# Normalize JSON to compact single-line format (for NDJSON)
normalize_json_compact() {
    local input="$1"
    jq '
        walk(
            if type == "object" then
                with_entries(
                    if .key == "session_id" then .value = "<SESSION_ID>"
                    elif .key == "duration_ms" then .value = "<DURATION>"
                    elif .key == "duration_api_ms" then .value = "<DURATION>"
                    elif .key == "cost_usd" then .value = "<COST>"
                    elif .key == "total_cost_usd" then .value = "<COST>"
                    elif .key == "costUSD" then .value = "<COST>"
                    elif .key == "timestamp" then .value = "<TIMESTAMP>"
                    elif .key == "request_id" then .value = "<REQUEST_ID>"
                    elif .key == "uuid" then .value = "<UUID>"
                    elif .key == "id" and (.value | type) == "string" and (.value | startswith("msg_")) then .value = "<MESSAGE_ID>"
                    elif .key == "cwd" then .value = "<CWD>"
                    elif .key == "path" and (.value | type) == "string" and (.value | contains("/")) then .value = "<PATH>"
                    elif .key == "plugins" then .value = "<PLUGINS>"
                    elif .key == "mcp_servers" then .value = "<MCP_SERVERS>"
                    elif .key == "result" then .value = "<RESPONSE_TEXT>"
                    elif .key == "text" then .value = "<RESPONSE_TEXT>"
                    elif .key == "content" and (.value | type) == "array" then .value = "<CONTENT>"
                    elif .key == "usage" then .value = "<USAGE>"
                    elif .key == "modelUsage" then .value = "<MODEL_USAGE>"
                    elif .key == "input_tokens" then .value = "<TOKENS>"
                    elif .key == "output_tokens" then .value = "<TOKENS>"
                    elif .key == "cache_creation_input_tokens" then .value = "<TOKENS>"
                    elif .key == "cache_read_input_tokens" then .value = "<TOKENS>"
                    else .
                    end
                )
            else .
            end
        )
    ' <<< "$input" | jq -Sc '.'  # Sort keys, compact for NDJSON
}

# Normalize stream-JSON output (newline-delimited JSON)
normalize_stream_json() {
    local input="$1"
    while IFS= read -r line; do
        if [[ -n "$line" ]]; then
            normalize_json_compact "$line"
        fi
    done <<< "$input"
}

compare_json_output() {
    log "${YELLOW}=== Comparing JSON output ===${NC}"

    local real_output sim_output

    # Capture real Claude output
    log_verbose "Running: claude --model $MODEL -p --output-format json \"$TEST_PROMPT\""
    if ! real_output=$(claude --model "$MODEL" -p --output-format json "$TEST_PROMPT" 2>&1); then
        log "${RED}Failed to run real Claude CLI${NC}"
        echo "$real_output"
        return 1
    fi

    echo "$real_output" > "$TMPDIR/real.json"
    log_verbose "Real output saved to $TMPDIR/real.json"

    # Capture simulator output
    log_verbose "Running: cargo run -p claudeless -- --model $MODEL -p --output-format json \"$TEST_PROMPT\""
    if ! sim_output=$(cd "$REPO_DIR" && cargo run -p claudeless -- --model "$MODEL" -p --output-format json "$TEST_PROMPT" 2>&1); then
        log "${RED}Failed to run claudeless${NC}"
        echo "$sim_output"
        return 1
    fi

    echo "$sim_output" > "$TMPDIR/sim.json"
    log_verbose "Simulator output saved to $TMPDIR/sim.json"

    # Normalize both outputs
    local real_normalized sim_normalized
    real_normalized=$(normalize_json "$real_output")
    sim_normalized=$(normalize_json "$sim_output")

    echo "$real_normalized" > "$TMPDIR/real.normalized.json"
    echo "$sim_normalized" > "$TMPDIR/sim.normalized.json"

    # Compare
    if diff -u "$TMPDIR/real.normalized.json" "$TMPDIR/sim.normalized.json" > "$TMPDIR/json.diff"; then
        log "${GREEN}JSON output: MATCH${NC}"
        return 0
    else
        log "${RED}JSON output: DIFFER${NC}"
        cat "$TMPDIR/json.diff"
        return 1
    fi
}

compare_stream_json_output() {
    log "${YELLOW}=== Comparing stream-json output ===${NC}"

    local real_output sim_output

    # Capture real Claude output
    # Note: --verbose is required for stream-json with -p
    log_verbose "Running: claude --model $MODEL -p --output-format stream-json --verbose \"$TEST_PROMPT\""
    if ! real_output=$(claude --model "$MODEL" -p --output-format stream-json --verbose "$TEST_PROMPT" 2>&1); then
        log "${RED}Failed to run real Claude CLI${NC}"
        echo "$real_output"
        return 1
    fi

    echo "$real_output" > "$TMPDIR/real.stream.jsonl"

    # Capture simulator output
    log_verbose "Running: cargo run -p claudeless -- --model $MODEL -p --output-format stream-json \"$TEST_PROMPT\""
    if ! sim_output=$(cd "$REPO_DIR" && cargo run -p claudeless -- --model "$MODEL" -p --output-format stream-json "$TEST_PROMPT" 2>&1); then
        log "${RED}Failed to run claudeless${NC}"
        echo "$sim_output"
        return 1
    fi

    echo "$sim_output" > "$TMPDIR/sim.stream.jsonl"

    # Normalize both outputs
    local real_normalized sim_normalized
    real_normalized=$(normalize_stream_json "$real_output")
    sim_normalized=$(normalize_stream_json "$sim_output")

    echo "$real_normalized" > "$TMPDIR/real.stream.normalized.jsonl"
    echo "$sim_normalized" > "$TMPDIR/sim.stream.normalized.jsonl"

    # Compare event types and structure (not exact content)
    # Extract just the type/subtype fields for structural comparison
    local real_types sim_types
    real_types=$(jq -r '[.type, .subtype // "none"] | @tsv' < "$TMPDIR/real.stream.normalized.jsonl" 2>/dev/null || true)
    sim_types=$(jq -r '[.type, .subtype // "none"] | @tsv' < "$TMPDIR/sim.stream.normalized.jsonl" 2>/dev/null || true)

    echo "$real_types" > "$TMPDIR/real.types.txt"
    echo "$sim_types" > "$TMPDIR/sim.types.txt"

    if diff -u "$TMPDIR/real.types.txt" "$TMPDIR/sim.types.txt" > "$TMPDIR/stream.diff"; then
        log "${GREEN}Stream-JSON event sequence: MATCH${NC}"
        return 0
    else
        log "${RED}Stream-JSON event sequence: DIFFER${NC}"
        cat "$TMPDIR/stream.diff"
        return 1
    fi
}

capture_fixtures() {
    local version="$1"
    local base_dir="$CRATE_DIR/tests/fixtures/cli/$version"

    log "${YELLOW}=== Capturing CLI fixtures for Claude $version ===${NC}"
    log "Using model: $MODEL"
    log "Test prompt: $TEST_PROMPT"
    echo ""

    # ==========================================================================
    # JSON Output Fixture
    # ==========================================================================
    local json_dir="$base_dir/json-output"
    mkdir -p "$json_dir"

    log "Capturing JSON output..."
    log_verbose "Running: claude --model $MODEL -p --output-format json \"$TEST_PROMPT\""

    local real_json
    if ! real_json=$(claude --model "$MODEL" -p --output-format json "$TEST_PROMPT" 2>&1); then
        log "${RED}Failed to capture JSON output${NC}"
        echo "$real_json"
        return 1
    fi

    # Save raw output (gitignored) and normalized output
    echo "$real_json" > "$json_dir/output.raw.json"
    normalize_json "$real_json" > "$json_dir/output.json"
    log "  Captured: json-output/output.json"

    # ==========================================================================
    # Stream-JSON Output Fixture
    # ==========================================================================
    local stream_dir="$base_dir/stream-json"
    mkdir -p "$stream_dir"

    log "Capturing stream-JSON output..."
    log_verbose "Running: claude --model $MODEL -p --output-format stream-json --verbose \"$TEST_PROMPT\""

    local real_stream
    if ! real_stream=$(claude --model "$MODEL" -p --output-format stream-json --verbose "$TEST_PROMPT" 2>&1); then
        log "${RED}Failed to capture stream-JSON output${NC}"
        echo "$real_stream"
        return 1
    fi

    # Save raw output (gitignored) and normalized output
    echo "$real_stream" > "$stream_dir/output.raw.jsonl"
    normalize_stream_json "$real_stream" > "$stream_dir/output.jsonl"
    log "  Captured: stream-json/output.jsonl"

    # ==========================================================================
    # Summary
    # ==========================================================================
    echo ""
    log "${GREEN}Fixtures captured to $base_dir${NC}"
    echo ""
    log "Directory structure:"
    find "$base_dir" -type f -name "*.json" -o -name "*.jsonl" -o -name "*.toml" | sort | sed "s|$base_dir/|  |"
}

usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [COMMAND]

Compare real Claude CLI output against claudeless.

Commands:
    compare         Run comparison tests (default)
    capture VERSION Capture fixtures for the specified Claude version
    help            Show this help message

Options:
    -v, --verbose   Show detailed output
    -m, --model     Model to use (default: claude-3-5-haiku-latest)
    -p, --prompt    Test prompt to use (default: "Hello")

Environment Variables:
    MODEL           Model to use
    TEST_PROMPT     Test prompt to use
    VERBOSE         Set to 1 for verbose output

Examples:
    $(basename "$0")                        # Run comparison
    $(basename "$0") capture v2.1.12        # Capture fixtures
    VERBOSE=1 $(basename "$0")              # Verbose comparison
EOF
}

main() {
    local command="compare"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -v|--verbose)
                VERBOSE=1
                shift
                ;;
            -m|--model)
                MODEL="$2"
                shift 2
                ;;
            -p|--prompt)
                TEST_PROMPT="$2"
                shift 2
                ;;
            compare)
                command="compare"
                shift
                ;;
            capture)
                command="capture"
                shift
                if [[ $# -eq 0 ]]; then
                    log "${RED}Error: capture requires a version argument${NC}"
                    usage
                    exit 1
                fi
                CAPTURE_VERSION="$1"
                shift
                ;;
            help|-h|--help)
                usage
                exit 0
                ;;
            *)
                log "${RED}Unknown option: $1${NC}"
                usage
                exit 1
                ;;
        esac
    done

    case "$command" in
        compare)
            local failed=0
            compare_json_output || failed=1
            echo
            compare_stream_json_output || failed=1

            echo
            if [[ $failed -eq 0 ]]; then
                log "${GREEN}All comparisons passed!${NC}"
            else
                log "${RED}Some comparisons failed${NC}"
            fi
            exit $failed
            ;;
        capture)
            capture_fixtures "$CAPTURE_VERSION"
            ;;
    esac
}

main "$@"
