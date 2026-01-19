#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
#
# Compare state directory files between real Claude CLI and claudeless.
#
# This script runs both real Claude and the simulator with the same prompts,
# then compares the resulting state files to identify format differences.
#
# Usage: ./compare-state.sh [--real-only | --sim-only]
#
# Requirements:
# - Real Claude CLI installed and authenticated (unless --sim-only)
# - claudeless binary built (unless --real-only)
# - jq for JSON normalization
# - diff for comparison

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"

# Parse args
REAL_ONLY=false
SIM_ONLY=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --real-only) REAL_ONLY=true; shift ;;
        --sim-only) SIM_ONLY=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Build claudeless if needed
if [[ "$REAL_ONLY" != "true" ]]; then
    echo "Building claudeless..."
    cargo build --package claudeless --release 2>/dev/null
    CLAUDELESS="$CRATE_DIR/../../target/release/claudeless"
    if [[ ! -f "$CLAUDELESS" ]]; then
        CLAUDELESS="$CRATE_DIR/../../target/debug/claudeless"
    fi
    if [[ ! -f "$CLAUDELESS" ]]; then
        echo "Error: claudeless binary not found. Run 'cargo build --package claudeless'"
        exit 1
    fi
fi

# Temp directories
REAL_TEMP=""
SIM_TEMP=""
REAL_STATE=""
SIM_STATE=""

cleanup() {
    [[ -n "$REAL_TEMP" && -d "$REAL_TEMP" ]] && rm -rf "$REAL_TEMP"
    [[ -n "$SIM_TEMP" && -d "$SIM_TEMP" ]] && rm -rf "$SIM_TEMP"
    [[ -n "$REAL_STATE" && -d "$REAL_STATE" ]] && rm -rf "$REAL_STATE"
    [[ -n "$SIM_STATE" && -d "$SIM_STATE" ]] && rm -rf "$SIM_STATE"
}

trap cleanup EXIT ERR INT

echo "=== Claude State Comparison Script ==="
echo ""

# Create temp directories
REAL_TEMP=$(mktemp -d)
SIM_TEMP=$(mktemp -d)
REAL_STATE=$(mktemp -d)
SIM_STATE=$(mktemp -d)

TEST_PROMPT="Say hello and nothing else"

# Normalize path for lookups
normalize_path() {
    echo "$1" | sed 's|/|-|g; s|\.|-|g'
}

# Create a minimal scenario for the simulator
create_scenario() {
    local dir="$1"
    cat > "$dir/scenario.json" << 'EOF'
{
    "default_response": "Hello!"
}
EOF
}

echo "=== Step 1: Run real Claude ==="
if [[ "$SIM_ONLY" != "true" ]]; then
    echo "Working directory: $REAL_TEMP"
    echo "Running: claude --model haiku -p '$TEST_PROMPT'"

    cd "$REAL_TEMP"
    claude --model haiku -p "$TEST_PROMPT" 2>&1 || true

    # Find and copy state files
    REAL_NORMALIZED=$(normalize_path "$REAL_TEMP")
    PROJECTS_DIR="$HOME/.claude/projects"

    if [[ -d "$PROJECTS_DIR" ]]; then
        PROJECT_DIR=$(find "$PROJECTS_DIR" -maxdepth 1 -type d -name "*${REAL_NORMALIZED}*" 2>/dev/null | head -1)
        if [[ -n "$PROJECT_DIR" && -d "$PROJECT_DIR" ]]; then
            echo "  Found project: $PROJECT_DIR"
            cp -r "$PROJECT_DIR"/* "$REAL_STATE/" 2>/dev/null || true
        fi
    fi
    echo "  Real Claude state captured"
else
    echo "  Skipped (--sim-only)"
fi

echo ""
echo "=== Step 2: Run claudeless ==="
if [[ "$REAL_ONLY" != "true" ]]; then
    echo "Working directory: $SIM_TEMP"
    echo "State directory: $SIM_STATE"

    create_scenario "$SIM_TEMP"

    "$CLAUDELESS" \
        --scenario "$SIM_TEMP/scenario.json" \
        -p "$TEST_PROMPT" 2>&1 || true

    # Move projects contents to state dir for comparison
    if [[ -d "$SIM_STATE/projects" ]]; then
        # Find the project dir and copy its contents
        for proj in "$SIM_STATE/projects"/*; do
            if [[ -d "$proj" ]]; then
                cp -r "$proj"/* "$SIM_STATE/" 2>/dev/null || true
                break
            fi
        done
    fi

    echo "  Simulator state captured"
else
    echo "  Skipped (--real-only)"
fi

echo ""
echo "=== Step 3: Normalize state files ==="

normalize_json() {
    local file="$1"
    local out="$2"

    if [[ ! -f "$file" ]]; then
        return
    fi

    jq -S '
        walk(
            if type == "string" then
                if test("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"; "i") then
                    "<UUID>"
                elif test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}") then
                    "<TIMESTAMP>"
                elif test("^/tmp/") or test("^/var/folders/") or test("^/private/") then
                    "<TEMP_PATH>"
                else
                    .
                end
            elif type == "number" and . > 1700000000000 then
                "<MTIME>"
            else
                .
            end
        )
    ' "$file" > "$out"
}

normalize_jsonl() {
    local file="$1"
    local out="$2"

    if [[ ! -f "$file" ]]; then
        return
    fi

    while IFS= read -r line; do
        echo "$line" | jq -c '
            walk(
                if type == "string" then
                    if test("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"; "i") then
                        "<UUID>"
                    elif test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}") then
                        "<TIMESTAMP>"
                    elif test("^/tmp/") or test("^/var/folders/") or test("^/private/") then
                        "<TEMP_PATH>"
                    else
                        .
                    end
                else
                    .
                end
            )
        '
    done < "$file" | jq -s '.' > "$out"
}

# Normalize real Claude files
for f in "$REAL_STATE"/*.json; do
    [[ -f "$f" ]] && normalize_json "$f" "${f%.json}.normalized.json"
done
for f in "$REAL_STATE"/*.jsonl; do
    [[ -f "$f" ]] && normalize_jsonl "$f" "${f%.jsonl}.normalized.json"
done

# Normalize simulator files
for f in "$SIM_STATE"/*.json; do
    [[ -f "$f" ]] && normalize_json "$f" "${f%.json}.normalized.json"
done
for f in "$SIM_STATE"/*.jsonl; do
    [[ -f "$f" ]] && normalize_jsonl "$f" "${f%.jsonl}.normalized.json"
done

echo "  Normalization complete"

echo ""
echo "=== Step 4: Compare state files ==="

DIFF_FOUND=false

compare_files() {
    local name="$1"
    local real_file="$2"
    local sim_file="$3"

    echo ""
    echo "--- $name ---"

    if [[ ! -f "$real_file" && ! -f "$sim_file" ]]; then
        echo "  Both missing (OK if not expected)"
        return
    fi

    if [[ ! -f "$real_file" ]]; then
        echo "  MISSING: Real Claude did not create this file"
        echo "  Simulator created: $sim_file"
        DIFF_FOUND=true
        return
    fi

    if [[ ! -f "$sim_file" ]]; then
        echo "  MISSING: Simulator did not create this file"
        echo "  Real Claude created: $real_file"
        DIFF_FOUND=true
        return
    fi

    if diff -q "$real_file" "$sim_file" > /dev/null 2>&1; then
        echo "  MATCH"
    else
        echo "  DIFFERS:"
        diff -u "$real_file" "$sim_file" | head -50 || true
        DIFF_FOUND=true
    fi
}

# Compare sessions-index.json
compare_files "sessions-index.json" \
    "$REAL_STATE/sessions-index.normalized.json" \
    "$SIM_STATE/sessions-index.normalized.json"

# Compare session files (first .jsonl found)
REAL_SESSION=$(find "$REAL_STATE" -name "*.normalized.json" -type f | grep -v sessions-index | head -1)
SIM_SESSION=$(find "$SIM_STATE" -name "*.normalized.json" -type f | grep -v sessions-index | head -1)

if [[ -n "$REAL_SESSION" || -n "$SIM_SESSION" ]]; then
    compare_files "session.jsonl" "${REAL_SESSION:-/nonexistent}" "${SIM_SESSION:-/nonexistent}"
fi

echo ""
echo "=== Comparison Complete ==="

if [[ "$DIFF_FOUND" == "true" ]]; then
    echo ""
    echo "RESULT: Differences found between real Claude and simulator"
    echo ""
    echo "Real Claude state files:"
    ls -la "$REAL_STATE"/ 2>/dev/null || echo "  (none)"
    echo ""
    echo "Simulator state files:"
    ls -la "$SIM_STATE"/ 2>/dev/null || echo "  (none)"
    exit 1
else
    echo ""
    echo "RESULT: State files match!"
    exit 0
fi
