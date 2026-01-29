# capsh

Headless terminal capture with scripting DSL.

## Overview

capsh spawns a command in a PTY, executes a script of wait/send commands, and captures terminal frames. Useful for:

- Testing TUI applications
- Recording terminal sessions for documentation
- Automating terminal interactions

## Usage

```bash
capsh [OPTIONS] -- <command>...
```

Script is read from stdin. Frames written to `--frames` directory if specified.

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--frames <dir>` | none | Directory for frame capture |
| `--cols <n>` | 80 | Terminal width |
| `--rows <n>` | 24 | Terminal height |

### Examples

```bash
# Basic: open vi and quit
capsh -- vi <<'EOF'
wait "~"
send ":q\n"
EOF

# Capture frames
capsh --frames /tmp/out -- vi <<'EOF'
wait "~"
send "ihello world"
send <Esc>
snapshot
send ":q!\n"
EOF

# From file
capsh --frames /tmp/out -- vi < script.capsh
```

## Script DSL

### wait

Wait for a condition before continuing.

```
wait "pattern"         # Wait for regex to match screen (30s default timeout)
wait "pattern" 5s      # Wait with custom timeout
wait !"pattern"        # Wait until pattern does NOT match (negated)
wait !"Loading" 10s    # Negated with timeout
wait 500               # Wait 500 milliseconds
wait 2s                # Wait 2 seconds
wait 1m                # Wait 1 minute
```

Pattern matching uses Rust regex syntax against the plain text screen content. Use `!` prefix for negated waits (wait until pattern disappears). Durations support suffixes: `ms` (milliseconds), `s` (seconds), `m` (minutes). Plain numbers are milliseconds.

### send

Send text or keys to the terminal, with optional inline delays.

```
send "hello"              # Send literal text
send "line\n"             # Send with newline escape
send <Enter>              # Send Enter key
send <Esc>                # Send Escape key
send "text" <Enter>       # Mixed text and keys
send <C-c>                # Send Ctrl+C
send "hello" 150 <Enter>  # Send "hello", wait 150ms, then Enter
send <Esc> 50 ":wq\n"     # Escape, wait 50ms, then :wq
```

Inline delays (numbers in milliseconds) pause between sends without a separate `wait` command.

#### Escape sequences

| Sequence | Meaning |
|----------|---------|
| `\n` | Newline |
| `\r` | Carriage return |
| `\t` | Tab |
| `\\` | Backslash |
| `\"` | Quote |

#### Special keys

| Key | Description |
|-----|-------------|
| `<Enter>` | Enter/Return |
| `<Tab>` | Tab |
| `<Esc>` | Escape |
| `<Space>` | Space |
| `<Backspace>` | Backspace |
| `<Up>` | Arrow up |
| `<Down>` | Arrow down |
| `<Left>` | Arrow left |
| `<Right>` | Arrow right |
| `<C-a>` ... `<C-z>` | Ctrl + letter |
| `<M-a>` ... `<M-z>` | Meta/Option + letter |
| `<A-a>` ... `<A-z>` | Alt + letter (same as Meta) |

### snapshot

Force capture a frame, even if screen hasn't changed.

```
snapshot              # Unnamed snapshot
snapshot "name"       # Named snapshot (name recorded in jsonl)
```

Named snapshots are useful for identifying specific points in the recording.

### kill

Send a signal to the child process.

```
kill SIGTERM          # Send SIGTERM
kill TERM             # Same (SIG prefix optional)
kill 15               # Same (by number)
kill SIGKILL          # Force kill
kill 9                # Same
```

Supported signals: HUP, INT, QUIT, KILL, TERM, USR1, USR2, STOP, CONT.

### if / else if / else / end

Conditional execution based on wait result.

```
if wait "pattern" [timeout]
    # commands if pattern matches
else if wait "other" [timeout]
    # commands if other matches
else
    # commands if all conditions timeout (optional)
end
```

The `else` and `else if` blocks are optional. Unlike regular `wait`, `if wait` does not error on timeout - it executes the else branch instead.

**Example:**
```
if wait "Ctrl-C again" 2s
    snapshot "with_hint"
else
    snapshot "no_hint"
end
```

**Chained conditions:**
```
if wait "Ready" 2s
    snapshot "ready"
else if wait "Loading" 2s
    snapshot "loading"
else if wait "Error" 2s
    snapshot "error"
else
    snapshot "unknown"
end
```

Supports negation:
```
if wait !"Loading" 5s
    snapshot "loaded"
end
```

### match

Wait for the first of multiple patterns to appear, then execute corresponding commands.

```
match [timeout]
    "pattern1" -> command
    "pattern2" ->
        command1
        command2
    ...
else
    # commands if no pattern matches (optional)
end
```

Unlike chained `if wait` statements, `match` checks all patterns against the current screen simultaneously. The first pattern that matches wins.

**Inline command:**
```
match 3s
    "Sonnet" -> snapshot "model_sonnet"
    "Opus" -> snapshot "model_opus"
    "Haiku" -> snapshot "model_haiku"
else
    snapshot "unknown_model"
end
```

**Block commands:**
```
match 3s
    "Sonnet" ->
        snapshot "model_sonnet"
        send "selected sonnet\n"
    "Opus" ->
        snapshot "model_opus"
        send "selected opus\n"
    "Haiku" -> snapshot "model_haiku"
end
```

### Comments

Lines starting with `#` are ignored.

```
# This is a comment
wait "prompt"
send "command\n"
```

## Output Format

When `--frames` is specified, capsh creates:

```diagram
frames/
├── 000001.txt       # Plain text frame
├── 000001.ansi.txt  # Frame with ANSI colors
├── 000002.txt
├── 000002.ansi.txt
├── ...
├── recording.jsonl  # Timing and event log
├── raw.bin          # Raw PTY byte stream
└── latest.txt       # Symlink to latest plain frame
```

### recording.jsonl

JSON Lines file with timing information:

```jsonl
{"ms":0,"frame":"000001"}
{"ms":50,"frame":"000002"}
{"ms":100,"send":"ihello"}
{"ms":150,"frame":"000003"}
{"ms":200,"snapshot":"000003"}
{"ms":250,"snapshot":"000004","name":"after-edit"}
{"ms":300,"wait_match":"pattern"}
{"ms":350,"kill":"SIGTERM"}
{"ms":400,"exit":0}
```

#### Event types

| Event | Description |
|-------|-------------|
| `frame` | Automatic frame capture (screen content changed) |
| `snapshot` | Explicit snapshot command; may include `name` |
| `send` | Input sent to terminal |
| `wait_match` | Wait pattern matched successfully |
| `wait_timeout` | Wait timed out without matching |
| `wait_eof` | Wait ended due to EOF without matching |
| `match_timeout` | Match timed out; value is array of patterns |
| `kill` | Signal sent to child process |
| `exit` | Process exit code |

All events include `ms` (milliseconds since session start). Frame and snapshot values reference files as `NNNNNN.txt` and `NNNNNN.ansi.txt`.

### raw.bin

Complete raw PTY output. Can be replayed through a terminal emulator for full fidelity reconstruction.

### Frame deduplication

Frames are deduplicated based on plain text content. A new frame file is only created when the text changes. If a `snapshot` is taken when content is unchanged, it references the existing frame number in the recording rather than creating a duplicate file.

## Exit Code

capsh exits with the exit code of the spawned command, or non-zero on script errors (timeout, EOF before pattern match).

## Tips

### Waiting for prompts

```
wait "\\$"      # Wait for shell prompt
wait ">>>"      # Wait for Python REPL
wait "❯"        # Wait for custom prompt
```

### Ctrl+C to interrupt

```
send <C-c>
wait "\\$"            # Wait for shell prompt to return
```

### Small delays for state changes

```
send <Esc>
wait 50               # Small delay for mode switch
send ":q\n"
```
