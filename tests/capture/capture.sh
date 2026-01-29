#!/bin/bash
# Master script for capturing TUI fixtures from Claude CLI
#
# Usage: capture-all.sh [--skip-experimental] [--skip-requires-config]
#
# Environment variables:
#   SKIP_REQUIRES_CONFIG=1  Skip scripts requiring special configuration
#   RUN_EXPERIMENTAL=1      Run experimental scripts (may fail)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Parse arguments
SKIP_REQUIRES_CONFIG="${SKIP_REQUIRES_CONFIG:-0}"
RUN_EXPERIMENTAL="${RUN_EXPERIMENTAL:-0}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-requires-config)
            SKIP_REQUIRES_CONFIG=1
            shift
            ;;
        --skip-experimental)
            RUN_EXPERIMENTAL=0
            shift
            ;;
        --run-experimental)
            RUN_EXPERIMENTAL=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Capture TUI fixtures from real Claude CLI using capsh scripts."
            echo ""
            echo "Options:"
            echo "  --skip-requires-config  Skip scripts needing special configuration"
            echo "  --skip-experimental     Skip experimental scripts (default)"
            echo "  --run-experimental      Run experimental scripts (may fail)"
            echo "  -h, --help              Show this help"
            echo ""
            echo "Environment variables:"
            echo "  SKIP_REQUIRES_CONFIG=1  Same as --skip-requires-config"
            echo "  RUN_EXPERIMENTAL=1      Same as --run-experimental"
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

echo -e "${GREEN}Claude CLI version: $VERSION${NC}"

# Raw output goes in git-ignored directory
RAW_OUTPUT="$SCRIPT_DIR/output/v${VERSION}"
# Fixtures go in tests/fixtures/
FIXTURES_DIR="$(dirname "$SCRIPT_DIR")/fixtures/v${VERSION}"

# Clean previous output
rm -rf "$RAW_OUTPUT" "$FIXTURES_DIR"
mkdir -p "$RAW_OUTPUT" "$FIXTURES_DIR"

TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

run_script() {
    local script="$1"
    local timeout="$2"
    local allow_failure="${3:-false}"

    ((TOTAL++)) || true

    if [[ ! -f "$script" ]]; then
        echo -e "${YELLOW}Skipping: $script (not found)${NC}"
        ((SKIPPED++)) || true
        return 0
    fi

    if run_capture "$script" "$RAW_OUTPUT" "$FIXTURES_DIR" "$timeout"; then
        ((PASSED++)) || true
    else
        if [[ "$allow_failure" == "true" ]]; then
            echo -e "${YELLOW}Experimental script failed (expected): $(basename "$script")${NC}"
            ((SKIPPED++)) || true
        else
            ((FAILED++)) || true
        fi
    fi
}

# Run reliable scripts (keyboard interactions only)
echo ""
echo -e "${GREEN}=== Running reliable scripts ===${NC}"
for script in "$SCRIPT_DIR"/reliable/*.capsh; do
    [[ -f "$script" ]] || continue
    run_script "$script" "$KEYBOARD_TIMEOUT"
done

# Run requires-config scripts (optional)
if [[ "$SKIP_REQUIRES_CONFIG" != "1" ]]; then
    echo ""
    echo -e "${GREEN}=== Running requires-config scripts ===${NC}"
    for script in "$SCRIPT_DIR"/requires-config/*.capsh; do
        [[ -f "$script" ]] || continue
        run_script "$script" "$KEYBOARD_TIMEOUT"
    done
else
    echo ""
    echo -e "${YELLOW}=== Skipping requires-config scripts ===${NC}"
fi

# Run experimental scripts (may involve API calls)
if [[ "$RUN_EXPERIMENTAL" == "1" ]]; then
    echo ""
    echo -e "${YELLOW}=== Running experimental scripts (may fail) ===${NC}"
    for script in "$SCRIPT_DIR"/experimental/*.capsh; do
        [[ -f "$script" ]] || continue
        run_script "$script" "$THINKING_TIMEOUT" true
    done
else
    echo ""
    echo -e "${YELLOW}=== Skipping experimental scripts ===${NC}"
fi

# Summary
echo ""
echo "=== Summary ==="
echo "Total: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${YELLOW}Skipped: $SKIPPED${NC}"
if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}Failed: $FAILED${NC}"
    exit 1
else
    echo "Failed: $FAILED"
fi

echo ""
echo "Raw output: $RAW_OUTPUT"
echo "Fixtures: $FIXTURES_DIR"
