# capsh-recording: Full Recording with Timing

Extends capsh with complete session recording: every frame (with ANSI), timing log, and raw PTY dump.

## Current State

Core implementation complete:
- `pty.rs` - PTY spawn/read/write
- `nbio.rs` - non-blocking I/O helpers
- `screen.rs` - terminal emulation via avt, plain text render
- `script.rs` - DSL parser with wait/send/snapshot
- `session.rs` - script execution loop
- `main.rs` - CLI wiring

## New Output Format

When `--frames` is specified:

```
frames/
├── 000001.txt       # plain text
├── 000001.ansi.txt  # with ANSI escape codes
├── 000002.txt
├── 000002.ansi.txt
├── recording.jsonl  # timing + events
├── raw.bin          # raw PTY byte stream
└── latest.txt       # symlink to latest plain frame
```

### recording.jsonl

```jsonl
{"ms":0,"frame":"000001"}
{"ms":50,"frame":"000002"}
{"ms":100,"send":"ihello"}
{"ms":150,"frame":"000003"}
```

Events:
- `{"ms":N,"frame":"NNNNNN"}` - frame captured at time N
- `{"ms":N,"send":"..."}` - input sent at time N (with escapes like `<Esc>`, `<C-c>`)

### raw.bin

Append-only file of all bytes read from PTY master. Future replay can pipe through terminal emulator.

## Changes

### 1. `src/screen.rs` - Add ANSI rendering

```rust
/// Render current screen with ANSI escape codes.
pub fn render_ansi(&self) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    for (i, line) in self.vt.view().enumerate() {
        if i > 0 {
            out.push('\n');
        }

        let mut last_pen = avt::Pen::default();
        for cell in line.cells() {
            let pen = cell.pen();
            if pen != &last_pen {
                // Reset and apply new style
                out.push_str("\x1b[0m");
                out.push_str(&pen.dump());
                last_pen = pen.clone();
            }
            out.push(cell.char());
        }
        // Reset at end of line
        out.push_str("\x1b[0m");
    }

    out
}
```

### 2. `src/recording.rs` (new) - Recording state

~60 lines.

```rust
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::Result;

pub struct Recording {
    start: Instant,
    jsonl: BufWriter<File>,
    raw: BufWriter<File>,
}

impl Recording {
    pub fn new(dir: &Path) -> Result<Self> {
        let jsonl = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join("recording.jsonl"))?;

        let raw = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join("raw.bin"))?;

        Ok(Self {
            start: Instant::now(),
            jsonl: BufWriter::new(jsonl),
            raw: BufWriter::new(raw),
        })
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn log_frame(&mut self, seq: u64) -> Result<()> {
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"frame":"{:06}"}}"#,
            self.elapsed_ms(),
            seq
        )?;
        Ok(())
    }

    pub fn log_send(&mut self, input: &str) -> Result<()> {
        // Escape for JSON
        let escaped = input
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        writeln!(
            self.jsonl,
            r#"{{"ms":{},"send":"{}"}}"#,
            self.elapsed_ms(),
            escaped
        )?;
        Ok(())
    }

    pub fn append_raw(&mut self, data: &[u8]) -> Result<()> {
        self.raw.write_all(data)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.jsonl.flush()?;
        self.raw.flush()?;
        Ok(())
    }
}
```

### 3. `src/screen.rs` - Update save methods

Replace `save_if_changed` and `force_save` to write both formats:

```rust
/// Save frame (both plain and ANSI), returns frame number.
pub fn save_frame(&mut self, dir: &Path) -> Result<u64> {
    self.frame_seq += 1;
    let seq = self.frame_seq;

    // Plain text
    let plain = self.render();
    std::fs::write(dir.join(format!("{:06}.txt", seq)), &plain)?;

    // ANSI
    let ansi = self.render_ansi();
    std::fs::write(dir.join(format!("{:06}.ansi.txt", seq)), &ansi)?;

    // Update latest symlink
    let latest = dir.join("latest.txt");
    let _ = std::fs::remove_file(&latest);
    #[cfg(unix)]
    std::os::unix::fs::symlink(format!("{:06}.txt", seq), &latest)?;

    self.last_frame = Some(plain);
    Ok(seq)
}

/// Check if screen changed since last save.
pub fn changed(&self) -> bool {
    match &self.last_frame {
        Some(last) => &self.render() != last,
        None => true,
    }
}
```

Remove `save_if_changed` and `force_save` - session.rs will handle the logic.

### 4. `src/session.rs` - Integrate recording

```rust
use crate::recording::Recording;

pub async fn run(config: Config) -> Result<i32> {
    let mut recording = config.frames_dir.as_ref()
        .map(|dir| Recording::new(dir))
        .transpose()?;

    if let Some(ref dir) = config.frames_dir {
        std::fs::create_dir_all(dir)?;
    }

    let pty = Pty::spawn(&config.command, config.cols, config.rows)?;
    let mut screen = Screen::new(config.cols, config.rows);
    let mut buf = [0u8; 4096];

    // Helper to save frame if changed (dedup on plain text)
    let maybe_save_frame = |screen: &mut Screen, recording: &mut Option<Recording>, dir: &Path| -> Result<()> {
        if screen.changed() {
            let seq = screen.save_frame(dir)?;
            if let Some(ref mut rec) = recording {
                rec.log_frame(seq)?;
            }
        }
        Ok(())
    };

    // Helper to force save frame (for snapshot command)
    let force_save_frame = |screen: &mut Screen, recording: &mut Option<Recording>, dir: &Path| -> Result<()> {
        let seq = screen.save_frame(dir)?;
        if let Some(ref mut rec) = recording {
            rec.log_frame(seq)?;
        }
        Ok(())
    };

    for cmd in config.script {
        // Drain pending PTY output before each command
        loop {
            match timeout(Duration::from_millis(10), pty.read(&mut buf)).await {
                Ok(Ok(0)) => {
                    if let Some(ref mut rec) = recording { rec.flush()?; }
                    return pty.wait().await;
                }
                Ok(Ok(n)) => {
                    if let Some(ref mut rec) = recording {
                        rec.append_raw(&buf[..n])?;
                    }
                    screen.feed(&buf[..n]);
                    if let Some(ref dir) = config.frames_dir {
                        maybe_save_frame(&mut screen, &mut recording, dir)?;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => break,
            }
        }

        match cmd {
            Command::WaitPattern(regex) => {
                let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
                while !screen.matches(&regex) {
                    if tokio::time::Instant::now() > deadline {
                        return Err(anyhow::anyhow!("timeout waiting for: {}", regex));
                    }
                    match timeout(Duration::from_millis(100), pty.read(&mut buf)).await {
                        Ok(Ok(0)) => return Err(anyhow::anyhow!("EOF waiting for: {}", regex)),
                        Ok(Ok(n)) => {
                            if let Some(ref mut rec) = recording {
                                rec.append_raw(&buf[..n])?;
                            }
                            screen.feed(&buf[..n]);
                            if let Some(ref dir) = config.frames_dir {
                                maybe_save_frame(&mut screen, &mut recording, dir)?;
                            }
                        }
                        Ok(Err(e)) => return Err(e),
                        Err(_) => {}
                    }
                }
            }
            Command::WaitMs(ms) => {
                tokio::time::sleep(Duration::from_millis(ms)).await;
            }
            Command::Send(ref bytes) => {
                if let Some(ref mut rec) = recording {
                    rec.log_send(&format_send_for_log(bytes))?;
                }
                pty.write(bytes).await?;
            }
            Command::Snapshot => {
                if let Some(ref dir) = config.frames_dir {
                    force_save_frame(&mut screen, &mut recording, dir)?;
                }
            }
        }
    }

    // Drain remaining output
    loop {
        match timeout(Duration::from_millis(100), pty.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                if let Some(ref mut rec) = recording {
                    rec.append_raw(&buf[..n])?;
                }
                screen.feed(&buf[..n]);
                if let Some(ref dir) = config.frames_dir {
                    maybe_save_frame(&mut screen, &mut recording, dir)?;
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {}
        }
    }

    if let Some(ref mut rec) = recording { rec.flush()?; }
    pty.wait().await
}

/// Format send bytes for log (reverse of parse_send_args).
fn format_send_for_log(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            0x1b => out.push_str("<Esc>"),
            0x01..=0x1a => {
                out.push_str("<C-");
                out.push((b'a' + b - 1) as char);
                out.push('>');
            }
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x7f => out.push_str("<Backspace>"),
            _ => out.push(b as char),
        }
    }
    out
}
```

### 5. `src/main.rs` - Add recording module

```rust
mod recording;
```

## Implementation Order

1. `recording.rs` - new recording state struct
2. `screen.rs` - add `render_ansi()`, replace save methods
3. `session.rs` - integrate recording, log sends, append raw
4. `main.rs` - add module
5. Test with vi

## Verification

```bash
# Record a session
capsh --frames /tmp/out -- vi <<'EOF'
wait "~"
send "ihello world"
send <Esc>
wait 100
send ":q!\n"
EOF

# Check outputs
ls /tmp/out/
# 000001.txt 000001.ansi.txt 000002.txt ... recording.jsonl raw.bin latest.txt

# View with colors
cat /tmp/out/000003.ansi.txt

# Check timing log
cat /tmp/out/recording.jsonl
# {"ms":0,"frame":"000001"}
# {"ms":50,"frame":"000002"}
# {"ms":100,"send":"ihello world"}
# ...

# Check raw size
ls -la /tmp/out/raw.bin
```

## Notes

- Frames deduped on plain text content (ANSI-only changes don't trigger new frame)
- `snapshot` command forces a frame save even if unchanged
- `recording.jsonl` stays small: just frame refs and sends
- `raw.bin` enables future replay through any terminal emulator
- ANSI files can be `cat`ed for quick color preview
