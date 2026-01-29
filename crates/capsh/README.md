# capsh

**Headless terminal capture with a scripting DSL.**

> **Attribution**: This project is heavily inspired by [ht (Headless Terminal)](https://github.com/andyk/ht) by Andy Konwinski.
> We use the same terminal emulation library ([avt](https://github.com/asciinema/avt)) and learned from ht's architecture.
> If you need a full-featured headless terminal with WebSocket support, use ht instead.

## What capsh does differently

- **Automatic frame capture**: Saves rendered terminal state on every change (not on-demand)
- **Frame diffing**: Only writes when output actually changes
- **Built-in scripting DSL**: `wait`, `send`, `snapshot` commands without JSON
- **No server**: Pure CLI tool, frames go to disk

## Usage

```bash
# Basic: capture frames to directory
capsh --frames ./output -- bash

# With script file
capsh --frames ./output --script test.capsh -- claude

# Inline script
capsh --frames ./output --script - -- claude <<'EOF'
wait "Ready>"
send "hello\n"
wait ">>>"
send "<C-d>"
EOF
```

## Script DSL

```
wait "pattern"     # Wait for regex match in current frame
wait 2000          # Wait milliseconds
send "text"        # Send literal text
send <Up>          # Send special key (arrow keys, etc.)
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
| `<C-x>` | Ctrl+x (any letter) |
| `<Backspace>` | Backspace |

## Frame Output

Frames are saved as `{seq:06}.txt` in the output directory:

```
./output/
  000001.txt
  000002.txt
  ...
  latest.txt  -> symlink to most recent
```

## License

MIT
