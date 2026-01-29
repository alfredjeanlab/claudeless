//! Script execution session.
//!
//! Runs a PTY with a script, capturing frames and handling commands.

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
                Ok(Ok(0)) => return pty.wait().await, // EOF
                Ok(Ok(n)) => {
                    screen.feed(&buf[..n]);
                    if let Some(ref dir) = config.frames_dir {
                        screen.save_if_changed(dir)?;
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
                            screen.feed(&buf[..n]);
                            if let Some(ref dir) = config.frames_dir {
                                screen.save_if_changed(dir)?;
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

    // Drain remaining output until EOF (child exits)
    loop {
        match timeout(Duration::from_millis(100), pty.read(&mut buf)).await {
            Ok(Ok(0)) => break, // EOF - child exited
            Ok(Ok(n)) => {
                screen.feed(&buf[..n]);
                if let Some(ref dir) = config.frames_dir {
                    screen.save_if_changed(dir)?;
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {} // Timeout, keep waiting for EOF
        }
    }

    pty.wait().await
}
