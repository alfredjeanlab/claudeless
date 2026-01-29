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
wait "pattern"    # Wait for regex to match screen (30s timeout)
wait 500          # Wait 500 milliseconds
```

Pattern matching uses Rust regex syntax against the plain text screen content.

### send

Send text or keys to the terminal.

```
send "hello"              # Send literal text
send "line\n"             # Send with newline escape
send <Enter>              # Send Enter key
send <Esc>                # Send Escape key
send "text" <Enter>       # Mixed text and keys
send <C-c>                # Send Ctrl+C
```

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

### snapshot

Force capture a frame, even if screen hasn't changed.

```
snapshot
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
{"ms":200,"send":"<Esc>"}
```

- `ms`: Milliseconds since session start
- `frame`: Frame number (find files as `NNNNNN.txt` and `NNNNNN.ansi.txt`)
- `send`: Input sent to terminal

### raw.bin

Complete raw PTY output. Can be replayed through a terminal emulator for full fidelity reconstruction.

### Frame deduplication

Frames are deduplicated based on plain text content. A new frame is only saved when the text changes. Use `snapshot` to force a save.

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
