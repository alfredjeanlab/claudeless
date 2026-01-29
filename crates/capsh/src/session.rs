//! Script execution session.
//!
//! Runs a PTY with a script, capturing frames and handling commands.

use std::path::PathBuf;

use anyhow::Result;
use tokio::time::{timeout, Duration};

use crate::pty::Pty;
use crate::recording::Recording;
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

    let mut recording = config
        .frames_dir
        .as_ref()
        .map(|dir| Recording::new(dir))
        .transpose()?;

    let pty = Pty::spawn(&config.command, config.cols, config.rows)?;
    let mut screen = Screen::new(config.cols, config.rows);
    let mut buf = [0u8; 4096];

    for cmd in config.script {
        // Drain pending PTY output before each command
        loop {
            match timeout(Duration::from_millis(10), pty.read(&mut buf)).await {
                Ok(Ok(0)) => {
                    // EOF
                    if let Some(ref mut rec) = recording {
                        rec.flush()?;
                    }
                    return pty.wait().await;
                }
                Ok(Ok(n)) => {
                    if let Some(ref mut rec) = recording {
                        rec.append_raw(&buf[..n])?;
                    }
                    screen.feed(&buf[..n]);
                    if let Some(ref dir) = config.frames_dir {
                        if screen.changed() {
                            let seq = screen.save_frame(dir)?;
                            if let Some(ref mut rec) = recording {
                                rec.log_frame(seq)?;
                            }
                        }
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => break, // Timeout, no more pending data
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
                                if screen.changed() {
                                    let seq = screen.save_frame(dir)?;
                                    if let Some(ref mut rec) = recording {
                                        rec.log_frame(seq)?;
                                    }
                                }
                            }
                        }
                        Ok(Err(e)) => return Err(e),
                        Err(_) => {} // Timeout, keep waiting
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
                    let seq = screen.save_frame(dir)?;
                    if let Some(ref mut rec) = recording {
                        rec.log_frame(seq)?;
                    }
                }
            }
        }
    }

    // Drain remaining output until EOF (child exits)
    loop {
        match timeout(Duration::from_millis(100), pty.read(&mut buf)).await {
            Ok(Ok(0)) => break, // EOF - child exited
            Ok(Ok(n)) => {
                if let Some(ref mut rec) = recording {
                    rec.append_raw(&buf[..n])?;
                }
                screen.feed(&buf[..n]);
                if let Some(ref dir) = config.frames_dir {
                    if screen.changed() {
                        let seq = screen.save_frame(dir)?;
                        if let Some(ref mut rec) = recording {
                            rec.log_frame(seq)?;
                        }
                    }
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {} // Timeout, keep waiting for EOF
        }
    }

    if let Some(ref mut rec) = recording {
        rec.flush()?;
    }
    pty.wait().await
}

/// Format send bytes for log (reverse of parse_send_args).
fn format_send_for_log(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            0x1b => out.push_str("<Esc>"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x01..=0x1a => {
                out.push_str("<C-");
                out.push((b'a' + b - 1) as char);
                out.push('>');
            }
            0x7f => out.push_str("<Backspace>"),
            _ => out.push(b as char),
        }
    }
    out
}
