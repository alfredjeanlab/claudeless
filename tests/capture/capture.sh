#!/bin/bash
# Master script for capturing TUI fixtures from Claude CLI
#
# Usage: capture.sh [OPTIONS]
#
# Environment variables:
#   RUN_SKIPPED=1  Run skipped scripts (may fail)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Parse arguments
RUN_SKIPPED="${RUN_SKIPPED:-0}"
RETRY_MODE=0
SINGLE_SCRIPT=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --retry)
            RETRY_MODE=1
            shift
            ;;
        --script)
            SINGLE_SCRIPT="$2"
            shift 2
            ;;
        --skip-skipped)
            RUN_SKIPPED=0
            shift
            ;;
        --run-skipped)
            RUN_SKIPPED=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Capture TUI fixtures from real Claude CLI using capsh scripts."
            echo ""
            echo "Options:"
            echo "  --script <name>         Run only the specified script (without .capsh)"
            echo "  --retry                 Re-run failed scripts or scripts with missing fixtures"
            echo "  --skip-skipped          Skip skipped scripts (default)"
            echo "  --run-skipped           Run skipped scripts (may fail)"
            echo "  -h, --help              Show this help"
            echo ""
            echo "Environment variables:"
            echo "  RUN_SKIPPED=1           Same as --run-skipped"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Check dependencies
check_claude
check_capsh

# Detect version
VERSION=$(detect_version)
if [[ -z "$VERSION" ]]; then
    echo -e "${RED}Error: Could not detect Claude CLI version${NC}" >&2
    exit 1
fi

echo -e "${BOLD}Claude CLI version:${NC} ${GREEN}$VERSION${NC}"

# Raw output goes in git-ignored directory
RAW_OUTPUT="$SCRIPT_DIR/output/v${VERSION}"
# Fixtures go in tests/fixtures/
FIXTURES_DIR="$(dirname "$SCRIPT_DIR")/fixtures/v${VERSION}"
# Track failures
FAILURES_FILE="$RAW_OUTPUT/.failures"

# Check if script should run (retry mode and single script logic)
should_run_script() {
    local script_name="$1"

    # Single script mode: only run the specified script
    if [[ -n "$SINGLE_SCRIPT" ]]; then
        [[ "$script_name" == "$SINGLE_SCRIPT" ]]
        return $?
    fi

    if [[ "$RETRY_MODE" != "1" ]]; then
        return 0  # Not in retry mode, run everything
    fi

    # In retry mode: run if in failures file OR if recording doesn't exist
    if [[ -f "$FAILURES_FILE" ]] && grep -q "^${script_name}$" "$FAILURES_FILE"; then
        return 0
    fi

    # Also run if recording doesn't exist (script never ran successfully)
    if [[ ! -f "$RAW_OUTPUT/$script_name/recording.jsonl" ]]; then
        return 0
    fi

    return 1  # Skip in retry mode
}

# Record failure
record_failure() {
    local script_name="$1"
    echo "$script_name" >> "$FAILURES_FILE"
}

# Clear failure (on success)
clear_failure() {
    local script_name="$1"
    if [[ -f "$FAILURES_FILE" ]]; then
        grep -v "^${script_name}$" "$FAILURES_FILE" > "$FAILURES_FILE.tmp" || true
        mv "$FAILURES_FILE.tmp" "$FAILURES_FILE"
    fi
}

# Clean or preserve output based on mode
if [[ "$RETRY_MODE" == "1" ]]; then
    echo "Retry mode: re-running failed scripts and scripts with missing fixtures"
    if [[ -f "$FAILURES_FILE" ]]; then
        echo -e "${DIM}Known failures: $(wc -l < "$FAILURES_FILE" | tr -d ' ')${NC}"
    fi
elif [[ -n "$SINGLE_SCRIPT" ]]; then
    # Single script mode: only clean that script's output
    rm -rf "${RAW_OUTPUT:?}/${SINGLE_SCRIPT:?}"
else
    # Clean previous output
    rm -rf "$RAW_OUTPUT" "$FIXTURES_DIR"
fi
mkdir -p "$RAW_OUTPUT" "$FIXTURES_DIR"

TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

run_script() {
    local script="$1"
    local timeout="$2"
    local allow_failure="${3:-false}"
    local script_name
    script_name=$(basename "$script" .capsh)

    if [[ ! -f "$script" ]]; then
        return 0
    fi

    # Check if we should run this script
    if ! should_run_script "$script_name"; then
        return 0
    fi

    ((TOTAL++)) || true

    if run_capture "$script" "$RAW_OUTPUT" "$FIXTURES_DIR" "$timeout"; then
        ((PASSED++)) || true
        clear_failure "$script_name"
    else
        if [[ "$allow_failure" == "true" ]]; then
            echo -e "${YELLOW}Skipped script failed (expected): $script_name${NC}"
            ((SKIPPED++)) || true
        else
            ((FAILED++)) || true
            record_failure "$script_name"
        fi
    fi
}

# Run `capsh` scripts
echo ""
echo "=== Running capsh scripts ==="
for script in "$SCRIPT_DIR"/capsh/*.capsh; do
    [[ -f "$script" ]] || continue
    run_script "$script" "$KEYBOARD_TIMEOUT"
done

# Run tmux scripts
echo ""
echo "=== Running tmux scripts ==="
for script in "$SCRIPT_DIR"/tmux/*.sh; do
    [[ -f "$script" ]] || continue
    script_name=$(basename "$script" .sh)

    # Check if we should run this script
    if should_run_script "$script_name"; then
        ((TOTAL++)) || true
        if "$script"; then
            ((PASSED++)) || true
            clear_failure "$script_name"
        else
            ((FAILED++)) || true
            record_failure "$script_name"
        fi
    fi
done

# Run skipped scripts
if [[ "$RUN_SKIPPED" == "1" ]]; then
    echo ""
    echo -e "${YELLOW}=== Running skipped scripts (may fail) ===${NC}"
    for script in "$SCRIPT_DIR"/skipped/*.capsh; do
        [[ -f "$script" ]] || continue
        run_script "$script" "$THINKING_TIMEOUT" true
    done
else
    echo ""
    echo "=== Skipping skipped scripts ==="
fi

# Summary
echo ""
echo "=== Summary ==="
echo "Total: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${YELLOW}Skipped: $SKIPPED${NC}"
if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}Failed: $FAILED${NC}"
    echo -e "${DIM}Run with --retry to re-run only failed scripts${NC}"
    exit 1
else
    echo "Failed: $FAILED"
    # Clear failures file on full success
    rm -f "$FAILURES_FILE"
fi

echo ""
echo "Raw output: $RAW_OUTPUT"
echo "Fixtures: $FIXTURES_DIR"
