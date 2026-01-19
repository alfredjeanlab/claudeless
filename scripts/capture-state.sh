#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
#
# Capture state directory files from real Claude CLI for fixture testing.
#
# This script runs real Claude CLI commands and captures the resulting
# ~/.claude state files for comparison testing against claudeless.
#
# Usage: ./capture-state.sh [output-dir]
#
# Requirements:
# - Real Claude CLI installed and authenticated
# - jq for JSON normalization
#
# Output structure:
#   fixtures/dotclaude/v{VERSION}/
#   ├── sessions-index.json    (synthetic, captured once)
#   ├── todo-write/
#   │   ├── session.jsonl
#   │   └── todo.json
#   └── plan-mode/
#       ├── session.jsonl
#       └── plan.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"

# Default output directory
OUTPUT_DIR="${1:-$CRATE_DIR/tests/fixtures/dotclaude}"

# Get Claude version for fixture directory naming
CLAUDE_VERSION=$(claude --version 2>/dev/null | grep -oE 'v?[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
CLAUDE_VERSION="${CLAUDE_VERSION#v}"  # Remove leading v if present

VERSION_DIR="$OUTPUT_DIR/v$CLAUDE_VERSION"

# Temporary working directories
TODO_TEMP_DIR=""
PLAN_TEMP_DIR=""

cleanup() {
    [[ -n "$TODO_TEMP_DIR" && -d "$TODO_TEMP_DIR" ]] && rm -rf "$TODO_TEMP_DIR"
    [[ -n "$PLAN_TEMP_DIR" && -d "$PLAN_TEMP_DIR" ]] && rm -rf "$PLAN_TEMP_DIR"
}

trap cleanup EXIT ERR INT

echo "=== Claude State Capture Script ==="
echo "Claude version: $CLAUDE_VERSION"
echo "Output directory: $VERSION_DIR"
echo ""

# Normalize path for project directory lookup
normalize_path() {
    local path="$1"
    echo "$path" | sed 's|/|-|g; s|\.|-|g'
}

# Normalize JSON files (replace UUIDs, timestamps, paths with placeholders)
normalize_json() {
    local input="$1"
    jq '
        walk(
            if type == "string" then
                if test("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"; "i") then
                    "<UUID>"
                elif test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}") then
                    "<TIMESTAMP>"
                elif test("^/tmp/") or test("^/var/folders/") or test("^/private/") then
                    "<TEMP_PATH>"
                elif test("^msg_") then
                    "<MESSAGE_ID>"
                elif test("^toolu_") then
                    "<TOOL_USE_ID>"
                elif test("^req_") then
                    "<REQUEST_ID>"
                else
                    .
                end
            elif type == "number" and . > 1700000000000 then
                "<MTIME>"
            else
                .
            end
        )
    ' "$input"
}

# Normalize JSONL files (line by line)
normalize_jsonl() {
    local input="$1"
    while IFS= read -r line; do
        echo "$line" | jq -c '
            walk(
                if type == "string" then
                    if test("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"; "i") then
                        "<UUID>"
                    elif test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}") then
                        "<TIMESTAMP>"
                    elif test("^/tmp/") or test("^/var/folders/") or test("^/private/var/folders/") then
                        "<TEMP_PATH>"
                    elif test("^msg_") then
                        "<MESSAGE_ID>"
                    elif test("^toolu_") then
                        "<TOOL_USE_ID>"
                    elif test("^req_") then
                        "<REQUEST_ID>"
                    else
                        .
                    end
                else
                    .
                end
            )
        '
    done < "$input"
}

# =============================================================================
# Step 1: Capture TodoWrite scenario
# =============================================================================
echo "=== Step 1: Capture TodoWrite scenario ==="

TODO_FIXTURE_DIR="$VERSION_DIR/todo-write"
mkdir -p "$TODO_FIXTURE_DIR"

TODO_TEMP_DIR=$(mktemp -d)
TODO_NORMALIZED_PATH=$(normalize_path "$TODO_TEMP_DIR")

echo "Working directory: $TODO_TEMP_DIR"
echo "Running: claude --model haiku -p 'Create a simple todo list...'"

cd "$TODO_TEMP_DIR"
claude --model haiku -p "Create a simple todo list with 3 items: buy groceries, walk the dog, read a book. Use the TodoWrite tool to create them." 2>&1 || true

# Find and copy project files
PROJECTS_DIR="$HOME/.claude/projects"
if [[ -d "$PROJECTS_DIR" ]]; then
    PROJECT_DIR=$(find "$PROJECTS_DIR" -maxdepth 1 -type d -name "*${TODO_NORMALIZED_PATH}*" 2>/dev/null | head -1)

    if [[ -n "$PROJECT_DIR" && -d "$PROJECT_DIR" ]]; then
        echo "Found project directory: $PROJECT_DIR"

        # Copy session JSONL file
        JSONL_FILE=$(find "$PROJECT_DIR" -maxdepth 1 -name "*.jsonl" -type f | head -1)
        if [[ -n "$JSONL_FILE" ]]; then
            normalize_jsonl "$JSONL_FILE" > "$TODO_FIXTURE_DIR/session.jsonl"
            echo "  Captured: session.jsonl"
        fi

        # Copy sessions-index.json to version root (synthetic, only need one)
        if [[ -f "$PROJECT_DIR/sessions-index.json" && ! -f "$VERSION_DIR/sessions-index.json" ]]; then
            normalize_json "$PROJECT_DIR/sessions-index.json" > "$VERSION_DIR/sessions-index.json"
            echo "  Captured: sessions-index.json (to version root)"
        fi
    else
        echo "Warning: Could not find project directory for $TODO_TEMP_DIR"
    fi
fi

# Copy todo files
TODOS_DIR="$HOME/.claude/todos"
if [[ -d "$TODOS_DIR" ]]; then
    TODO_FILE=$(find "$TODOS_DIR" -maxdepth 1 -name "*.json" -type f -newer "$TODO_TEMP_DIR" 2>/dev/null | head -1)
    if [[ -n "$TODO_FILE" ]]; then
        normalize_json "$TODO_FILE" > "$TODO_FIXTURE_DIR/todo.json"
        echo "  Captured: todo.json"
    fi
fi

echo ""

# =============================================================================
# Step 2: Capture Plan Mode scenario
# =============================================================================
echo "=== Step 2: Capture Plan Mode scenario ==="

PLAN_FIXTURE_DIR="$VERSION_DIR/plan-mode"
mkdir -p "$PLAN_FIXTURE_DIR"

PLAN_TEMP_DIR=$(mktemp -d)
PLAN_NORMALIZED_PATH=$(normalize_path "$PLAN_TEMP_DIR")

echo "Working directory: $PLAN_TEMP_DIR"
echo "Running: claude --model haiku --permission-mode plan -p 'Plan a simple feature...'"

cd "$PLAN_TEMP_DIR"
claude --model haiku --permission-mode plan -p "Plan a simple feature to add user authentication. Write the plan and exit." 2>&1 || true

# Find and copy project files (session.jsonl)
if [[ -d "$PROJECTS_DIR" ]]; then
    PROJECT_DIR=$(find "$PROJECTS_DIR" -maxdepth 1 -type d -name "*${PLAN_NORMALIZED_PATH}*" 2>/dev/null | head -1)

    if [[ -n "$PROJECT_DIR" && -d "$PROJECT_DIR" ]]; then
        echo "Found project directory: $PROJECT_DIR"

        # Copy session JSONL file
        JSONL_FILE=$(find "$PROJECT_DIR" -maxdepth 1 -name "*.jsonl" -type f | head -1)
        if [[ -n "$JSONL_FILE" ]]; then
            normalize_jsonl "$JSONL_FILE" > "$PLAN_FIXTURE_DIR/session.jsonl"
            echo "  Captured: session.jsonl"
        fi
    else
        echo "Warning: Could not find project directory for $PLAN_TEMP_DIR"
    fi
fi

# Copy plan files
PLANS_DIR="$HOME/.claude/plans"
if [[ -d "$PLANS_DIR" ]]; then
    PLAN_FILE=$(find "$PLANS_DIR" -maxdepth 1 -name "*.md" -type f -newer "$PLAN_TEMP_DIR" 2>/dev/null | head -1)
    if [[ -n "$PLAN_FILE" ]]; then
        cp "$PLAN_FILE" "$PLAN_FIXTURE_DIR/plan.md"
        echo "  Captured: plan.md"
    fi
fi

echo ""

# =============================================================================
# Summary
# =============================================================================
echo "=== Capture Complete ==="
echo ""
echo "Fixture structure:"
find "$VERSION_DIR" -type f | sort | sed "s|$VERSION_DIR/|  |"
echo ""
echo "Fixtures saved to: $VERSION_DIR"
