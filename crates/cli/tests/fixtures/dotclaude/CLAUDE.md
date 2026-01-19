# Claude State Directory Fixtures

Captured from real Claude CLI for comparison testing.

**Behavior observed with:** claude --version 2.1.12 (Claude Code)

## Directory Structure

```
dotclaude/
├── README.md                    # This file
└── v{version}/                  # Per-version fixtures (e.g., v2.1.12)
    ├── sessions-index.json      # Normalized sessions index
    ├── sessions-index.raw.json  # Raw output (gitignored)
    ├── session.jsonl            # Normalized session file
    ├── session.raw.jsonl        # Raw output (gitignored)
    ├── todo.json                # Normalized todo file
    ├── todo.raw.json            # Raw output (gitignored)
    └── plan.md                  # Plan file (markdown)
```

## Captures

### sessions-index.json
The session index maintained per-project directory.
**Note:** Only created for projects with multiple sessions (not for single-prompt temp directories).
Captured from a real project with session history.

- `version`: Always 1
- `entries`: Array of session metadata
  - `sessionId`: UUID of the session
  - `fullPath`: Full path to session JSONL file
  - `fileMtime`: File modification time (ms since epoch)
  - `firstPrompt`: First user prompt (truncated with "…" if long)
  - `messageCount`: Number of messages in session
  - `created`: ISO8601 timestamp
  - `modified`: ISO8601 timestamp
  - `gitBranch`: Current git branch (or empty)
  - `projectPath`: Path to project directory
  - `isSidechain`: Boolean

### session.jsonl
Session transcript in JSONL format (one JSON object per line).

**Message types:**
- `queue-operation` - Session start marker (for `-p` mode) or input queuing
- `user` - User message
- `assistant` - Assistant response
- `file-history-snapshot` - File state capture (interactive sessions)
- `summary` - Conversation summary (resumed sessions)
- `progress` - Progress updates
- `system` - System messages

**Session start patterns:**
- `-p` (print) mode: starts with `queue-operation` (operation: "dequeue")
- Interactive mode: starts with `file-history-snapshot`
- Resumed sessions: starts with `summary`

**Common fields:**
- User message: `type`, `uuid`, `parentUuid`, `sessionId`, `timestamp`, `cwd`, `version`, `gitBranch`, `isSidechain`, `userType`, `message.role`, `message.content`
- Assistant message: same fields plus `requestId`, `message.model`, `message.id`, `message.usage`

### todo.json
Todo list file in JSON format:
- File naming: `{sessionId}-agent-{sessionId}.json`
- Content: Array of todo items
  - `content`: Task description
  - `status`: "pending" | "in_progress" | "completed"
  - `activeForm`: Present tense form of task

### plan.md
Plan file in markdown format:
- File naming: `{adjective}-{verb}-{noun}.md` (three lowercase words)
- Content: Markdown with plan structure

## Normalization

For deterministic comparison, certain fields are normalized:
- UUIDs: Replaced with `<UUID>`
- Timestamps: Replaced with `<TIMESTAMP>`
- Mtimes: Replaced with `<MTIME>`
- Temp paths: Replaced with `<TEMP_PATH>`

## Capture Method

Use the capture-state.sh script:
```bash
./crates/claudeless/scripts/capture-state.sh
```

This will:
1. Run real Claude with test prompts
2. Capture the resulting state files
3. Normalize for comparison
4. Save to fixtures directory

## Comparison Method

Use the compare-state.sh script:
```bash
./crates/claudeless/scripts/compare-state.sh
```

This will:
1. Run both real Claude and the simulator
2. Normalize the output
3. Report differences
