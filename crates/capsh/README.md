# capsh

**Headless terminal capture with a scripting DSL.**

> **Attribution**: This project is heavily inspired by [ht (Headless Terminal)](https://github.com/andyk/ht) by Andy Konwinski.
> We use the same terminal emulation library ([avt](https://github.com/asciinema/avt)) and learned from ht's architecture.
> If you need a full-featured headless terminal with WebSocket support, use ht instead.

## What capsh does differently

- **Automatic frame capture**: Saves rendered terminal state on every change
- **Frame diffing**: Only writes when plain text content changes
- **Dual output**: Both plain text and ANSI-colored frames
- **Recording log**: Timing information in `recording.jsonl`
- **Raw capture**: Complete PTY output in `raw.bin`
- **Built-in scripting DSL**: `wait`, `send`, `snapshot` commands
- **No server**: Pure CLI tool, frames go to disk

## Usage

Script is read from stdin:

```bash
# Basic: run vi and quit
capsh -- vi <<'EOF'
wait "~"
send ":q\n"
EOF

# Capture frames to directory
capsh --frames ./output -- vi <<'EOF'
wait "~"
send "ihello world"
send <Esc>
snapshot
send ":q!\n"
EOF

# From file
capsh --frames ./output -- vi < test.capsh
```

## Script DSL

```
wait "pattern"     # Wait for regex match in current frame (30s timeout)
wait 2000          # Wait milliseconds
send "text"        # Send literal text
send <Up>          # Send special key
send <C-d>         # Send Ctrl+key
snapshot           # Force save frame even if unchanged
```

### Special Keys

| Syntax | Key |
|--------|-----|
| `<Up>` `<Down>` `<Left>` `<Right>` | Arrow keys |
| `<Enter>` | Enter/Return |
| `<Tab>` | Tab |
| `<Esc>` | Escape |
| `<Space>` | Space |
| `<Backspace>` | Backspace |
| `<C-a>` ... `<C-z>` | Ctrl+letter |

### Escape Sequences

| Sequence | Meaning |
|----------|---------|
| `\n` | Newline |
| `\r` | Carriage return |
| `\t` | Tab |
| `\\` | Backslash |
| `\"` | Quote |

## Output Format

When `--frames` is specified:

```diagram
./output/
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

```jsonl
{"ms":0,"frame":"000001"}
{"ms":50,"frame":"000002"}
{"ms":100,"send":"ihello"}
{"ms":150,"frame":"000003"}
```

## License

MIT
