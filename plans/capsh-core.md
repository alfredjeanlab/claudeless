# capsh-core: PTY + Frame Capture + Script Execution

Headless terminal capture with scripting DSL. No interactive mode.

## Architecture

```
script ──► session.rs ──► pty.rs ──► child process
                │              │
                ▼              ▼
           screen.rs ◄─── PTY output
                │
                ▼
          frames/*.txt
```

## Files

### 1. `src/nbio.rs` (new) - Non-blocking I/O helpers

~40 lines.

```rust
use std::os::fd::AsRawFd;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::errno::Errno;

pub fn set_non_blocking<F: AsRawFd>(fd: &F) -> nix::Result<()> {
    let flags = fcntl(fd.as_raw_fd(), FcntlArg::F_GETFL)?;
    let flags = OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK;
    fcntl(fd.as_raw_fd(), FcntlArg::F_SETFL(flags))?;
    Ok(())
}

/// Read, handling EAGAIN/EWOULDBLOCK. Returns None if would block.
pub fn read<F: AsRawFd>(fd: &F, buf: &mut [u8]) -> nix::Result<Option<usize>> {
    match nix::unistd::read(fd.as_raw_fd(), buf) {
        Ok(n) => Ok(Some(n)),
        Err(Errno::EAGAIN | Errno::EWOULDBLOCK) => Ok(None),
        Err(Errno::EIO) => Ok(Some(0)),  // PTY closed
        Err(e) => Err(e),
    }
}

/// Write, handling EAGAIN/EWOULDBLOCK. Returns None if would block.
pub fn write<F: AsRawFd>(fd: &F, buf: &[u8]) -> nix::Result<Option<usize>> {
    match nix::unistd::write(fd.as_raw_fd(), buf) {
        Ok(n) => Ok(Some(n)),
        Err(Errno::EAGAIN | Errno::EWOULDBLOCK) => Ok(None),
        Err(e) => Err(e),
    }
}
```

### 2. `src/pty.rs` - Full PTY implementation

~100 lines.

```rust
use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use anyhow::{anyhow, Result};
use nix::pty::{forkpty, Winsize};
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{ForkResult, Pid};
use tokio::io::unix::AsyncFd;

pub struct Pty {
    master_fd: AsyncFd<OwnedFd>,
    child_pid: Pid,
}

impl Pty {
    pub fn spawn(command: &str, cols: u16, rows: u16) -> Result<Self> {
        let winsize = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let result = unsafe { forkpty(&winsize, None)? };

        match result.fork_result {
            ForkResult::Child => {
                // Restore SIGPIPE default (nix masks it)
                unsafe { signal(Signal::SIGPIPE, SigHandler::SigDfl).ok() };
                std::env::set_var("TERM", "xterm-256color");

                let shell = CString::new("/bin/sh").unwrap();
                let args = [
                    CString::new("/bin/sh").unwrap(),
                    CString::new("-c").unwrap(),
                    CString::new(command).unwrap(),
                ];
                nix::unistd::execvp(&shell, &args)?;
                unreachable!()
            }
            ForkResult::Parent { child } => {
                let master = result.master;
                crate::nbio::set_non_blocking(&master)?;

                let owned: OwnedFd = unsafe { OwnedFd::from_raw_fd(master.into_raw_fd()) };
                let async_fd = AsyncFd::new(owned)?;

                Ok(Self {
                    master_fd: async_fd,
                    child_pid: child,
                })
            }
        }
    }

    pub async fn read(&self, buf: &mut [u8]) -> Result<usize> {
        loop {
            let mut guard = self.master_fd.readable().await?;
            match crate::nbio::read(self.master_fd.get_ref(), buf)? {
                Some(n) => return Ok(n),
                None => guard.clear_ready(),
            }
        }
    }

    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let mut written = 0;
        while written < data.len() {
            let mut guard = self.master_fd.writable().await?;
            match crate::nbio::write(self.master_fd.get_ref(), &data[written..])? {
                Some(n) => written += n,
                None => guard.clear_ready(),
            }
        }
        Ok(())
    }

    pub async fn wait(self) -> Result<i32> {
        nix::sys::signal::kill(self.child_pid, Signal::SIGHUP).ok();

        let pid = self.child_pid;
        let status = tokio::task::spawn_blocking(move || waitpid(pid, None)).await??;

        match status {
            WaitStatus::Exited(_, code) => Ok(code),
            WaitStatus::Signaled(_, sig, _) => Ok(128 + sig as i32),
            _ => Ok(1),
        }
    }
}
```

### 3. `src/screen.rs` - Add matches() and force_save()

Add to existing file:

```rust
impl Screen {
    /// Check if pattern matches anywhere in current screen.
    pub fn matches(&self, pattern: &regex::Regex) -> bool {
        pattern.is_match(&self.render())
    }

    /// Force save regardless of diff (for snapshot command).
    pub fn force_save(&mut self, dir: &Path) -> Result<u64> {
        let frame = self.render();
        self.frame_seq += 1;
        let seq = self.frame_seq;

        let path = dir.join(format!("{:06}.txt", seq));
        std::fs::write(&path, &frame)?;

        let latest = dir.join("latest.txt");
        let _ = std::fs::remove_file(&latest);
        #[cfg(unix)]
        std::os::unix::fs::symlink(format!("{:06}.txt", seq), &latest)?;

        self.last_frame = Some(frame);
        Ok(seq)
    }
}
```

### 4. `src/script.rs` - Add stdin loading

Add to existing file:

```rust
use std::io::Read;

/// Load and parse script from stdin.
pub fn load_stdin() -> Result<Vec<Command>> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    parse(&buf)
}
```

### 5. `src/session.rs` (new) - Script execution loop

~80 lines.

```rust
use std::path::PathBuf;
use anyhow::Result;
use tokio::time::{timeout, Duration};

use crate::pty::Pty;
use crate::screen::Screen;
use crate::script::Command;

pub struct Config {
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    pub frames_dir: Option<PathBuf>,
    pub script: Vec<Command>,
}

pub async fn run(config: Config) -> Result<i32> {
    if let Some(ref dir) = config.frames_dir {
        std::fs::create_dir_all(dir)?;
    }

    let pty = Pty::spawn(&config.command, config.cols, config.rows)?;
    let mut screen = Screen::new(config.cols, config.rows);
    let mut buf = [0u8; 4096];

    for cmd in config.script {
        // Drain pending PTY output before each command
        loop {
            match timeout(Duration::from_millis(10), pty.read(&mut buf)).await {
                Ok(Ok(0)) => return pty.wait().await,  // EOF
                Ok(Ok(n)) => {
                    screen.feed(&buf[..n]);
                    if let Some(ref dir) = config.frames_dir {
                        screen.save_if_changed(dir)?;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => break,  // Timeout, no more pending data
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
                            screen.feed(&buf[..n]);
                            if let Some(ref dir) = config.frames_dir {
                                screen.save_if_changed(dir)?;
                            }
                        }
                        Ok(Err(e)) => return Err(e),
                        Err(_) => {}  // Timeout, keep waiting
                    }
                }
            }
            Command::WaitMs(ms) => {
                tokio::time::sleep(Duration::from_millis(ms)).await;
            }
            Command::Send(bytes) => {
                pty.write(&bytes).await?;
            }
            Command::Snapshot => {
                if let Some(ref dir) = config.frames_dir {
                    screen.force_save(dir)?;
                }
            }
        }
    }

    pty.wait().await
}
```

### 6. `src/main.rs` - Wire up

```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod nbio;
mod pty;
mod screen;
mod script;
mod session;

#[derive(Parser, Debug)]
#[command(name = "capsh", about = "Headless terminal capture with scripting DSL")]
struct Args {
    /// Directory to save frame snapshots
    #[arg(long)]
    frames: Option<PathBuf>,

    /// Terminal width
    #[arg(long, default_value = "80")]
    cols: u16,

    /// Terminal height
    #[arg(long, default_value = "24")]
    rows: u16,

    /// Command to run
    #[arg(last = true, required = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config = session::Config {
        command: args.command.join(" "),
        cols: args.cols,
        rows: args.rows,
        frames_dir: args.frames,
        script: script::load_stdin()?,
    };

    let exit_code = session::run(config).await?;
    std::process::exit(exit_code);
}
```

## Verification

Using `vi` for predictable TUI behavior:

```bash
# Test 1: Basic vi open/close
capsh -- vi <<'EOF'
wait "~"
send ":q\n"
EOF
# Should exit cleanly

# Test 2: Insert text and snapshot
capsh --frames /tmp/out -- vi <<'EOF'
wait "~"
send "ihello world"
send <Esc>
snapshot
send ":q!\n"
EOF
cat /tmp/out/latest.txt
# Should show "hello world" on first line, tildes below

# Test 3: Multiple operations
capsh --frames /tmp/out -- vi <<'EOF'
wait "~"
send "iline one"
send <Esc>
send "oline two"
send <Esc>
snapshot
send ":q!\n"
EOF
grep -q "line one" /tmp/out/latest.txt && echo "line one: OK"
grep -q "line two" /tmp/out/latest.txt && echo "line two: OK"

# Test 4: Cursor movement
capsh --frames /tmp/out -- vi <<'EOF'
wait "~"
send "iABC"
send <Esc>
send "0"
send "iX"
send <Esc>
snapshot
send ":q!\n"
EOF
# Should show "XABC" (X inserted at beginning)

# Test 5: Script from file
capsh --frames /tmp/out -- vi < /tmp/test.capsh

# Test 6: Timeout (should fail)
timeout 5 capsh -- vi <<'EOF'
wait "this will never appear"
EOF
# Should timeout and fail
```

## Implementation Order

1. `nbio.rs` - non-blocking helpers
2. `pty.rs` - spawn + read/write
3. `screen.rs` - add `matches()` and `force_save()`
4. `script.rs` - add `load_stdin()`
5. `session.rs` - script execution loop
6. `main.rs` - wire up
7. Test verification cases
