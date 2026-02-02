// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Script execution session.
//!
//! Runs a PTY with a script, capturing frames and handling commands.

use std::path::PathBuf;

use anyhow::Result;
use regex::Regex;
use tokio::time::{timeout, Duration};

use crate::pty::Pty;
use crate::recording::Recording;
use crate::screen::Screen;
use crate::script::{Command, MatchArm, SendPart};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

fn resolve_timeout(timeout_ms: Option<u64>) -> Duration {
    timeout_ms.map_or(DEFAULT_TIMEOUT, Duration::from_millis)
}

/// Result of a PTY read with timeout.
enum ReadResult {
    Eof,
    Data(usize),
    Error(anyhow::Error),
    Timeout,
}

async fn read_with_timeout(pty: &Pty, buf: &mut [u8], timeout_duration: Duration) -> ReadResult {
    match timeout(timeout_duration, pty.read(buf)).await {
        Ok(Ok(0)) => ReadResult::Eof,
        Ok(Ok(n)) => ReadResult::Data(n),
        Ok(Err(e)) => ReadResult::Error(e),
        Err(_) => ReadResult::Timeout,
    }
}

pub struct Config {
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    pub frames_dir: Option<PathBuf>,
    pub script: Vec<Command>,
}

/// Result of command execution.
enum ExecResult {
    Continue,
    Eof, // Child sent EOF, need to call wait()
    Error(anyhow::Error),
}

/// Result of a wait operation.
enum WaitResult {
    Matched,
    Timeout,
    Eof,
}

/// Execution context shared across command execution functions.
struct ExecutionContext<'a> {
    pty: &'a Pty,
    screen: &'a mut Screen,
    recording: &'a mut Option<Recording>,
    frames_dir: &'a Option<PathBuf>,
    buf: &'a mut [u8],
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

    let mut ctx = ExecutionContext {
        pty: &pty,
        screen: &mut screen,
        recording: &mut recording,
        frames_dir: &config.frames_dir,
        buf: &mut buf,
    };

    let result = execute_commands(&mut ctx, &config.script).await;

    match result {
        ExecResult::Continue => {
            // Drain remaining output until EOF
            loop {
                match read_with_timeout(&pty, &mut buf, Duration::from_millis(100)).await {
                    ReadResult::Eof => break,
                    ReadResult::Data(n) => {
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
                    ReadResult::Error(e) => return Err(e),
                    ReadResult::Timeout => {}
                }
            }
        }
        ExecResult::Eof => {}
        ExecResult::Error(e) => return Err(e),
    }

    let exit_code = pty.wait().await?;
    if let Some(ref mut rec) = recording {
        rec.log_exit(exit_code)?;
        rec.flush()?;
    }
    Ok(exit_code)
}

/// Execute a list of commands.
async fn execute_commands(ctx: &mut ExecutionContext<'_>, commands: &[Command]) -> ExecResult {
    for cmd in commands {
        // Drain pending PTY output before each command
        match drain_pty(ctx).await {
            ExecResult::Continue => {}
            other => return other,
        }

        match cmd {
            Command::WaitPattern(regex, negated, timeout_ms) => {
                let wait_timeout = resolve_timeout(*timeout_ms);
                match do_wait(ctx, regex, *negated, wait_timeout).await {
                    Ok(WaitResult::Matched) => {}
                    Ok(WaitResult::Timeout) => {
                        let msg = if *negated {
                            format!("timeout waiting for absence of: {}", regex)
                        } else {
                            format!("timeout waiting for: {}", regex)
                        };
                        return ExecResult::Error(anyhow::anyhow!("{}", msg));
                    }
                    Ok(WaitResult::Eof) => return ExecResult::Eof,
                    Err(e) => return ExecResult::Error(e),
                }
            }
            Command::WaitMs(ms) => {
                tokio::time::sleep(Duration::from_millis(*ms)).await;
            }
            Command::Send(ref parts) => {
                for part in parts {
                    match part {
                        SendPart::Bytes(bytes) => {
                            if let Some(ref mut rec) = ctx.recording {
                                if let Err(e) = rec.log_send(&format_send_for_log(bytes)) {
                                    return ExecResult::Error(e);
                                }
                            }
                            if let Err(e) = ctx.pty.write(bytes).await {
                                return ExecResult::Error(e);
                            }
                        }
                        SendPart::Delay(ms) => {
                            tokio::time::sleep(Duration::from_millis(*ms)).await;
                        }
                    }
                }
            }
            Command::Snapshot(ref name) => {
                if let Some(ref dir) = ctx.frames_dir {
                    match ctx.screen.save_frame(dir) {
                        Ok(seq) => {
                            if let Some(ref mut rec) = ctx.recording {
                                if let Err(e) = rec.log_snapshot(seq, name.as_deref()) {
                                    return ExecResult::Error(e);
                                }
                            }
                        }
                        Err(e) => return ExecResult::Error(e),
                    }
                }
            }
            Command::Kill(signal) => {
                if let Some(ref mut rec) = ctx.recording {
                    if let Err(e) = rec.log_kill(*signal) {
                        return ExecResult::Error(e);
                    }
                }
                if let Err(e) = ctx.pty.kill(*signal) {
                    return ExecResult::Error(e);
                }
            }
            Command::IfWait {
                pattern,
                negated,
                timeout: timeout_ms,
                then_cmds,
                else_cmds,
            } => {
                let wait_timeout = resolve_timeout(*timeout_ms);
                match do_wait(ctx, pattern, *negated, wait_timeout).await {
                    Ok(WaitResult::Matched) => {
                        match Box::pin(execute_commands(ctx, then_cmds)).await {
                            ExecResult::Continue => {}
                            other => return other,
                        }
                    }
                    Ok(WaitResult::Timeout) => {
                        match Box::pin(execute_commands(ctx, else_cmds)).await {
                            ExecResult::Continue => {}
                            other => return other,
                        }
                    }
                    Ok(WaitResult::Eof) => return ExecResult::Eof,
                    Err(e) => return ExecResult::Error(e),
                }
            }
            Command::Match {
                timeout: timeout_ms,
                arms,
                else_cmds,
            } => {
                let wait_timeout = resolve_timeout(*timeout_ms);
                match do_match(ctx, arms, wait_timeout).await {
                    Ok(Some(matched_cmds)) => {
                        match Box::pin(execute_commands(ctx, matched_cmds)).await {
                            ExecResult::Continue => {}
                            other => return other,
                        }
                    }
                    Ok(None) => {
                        // No match - execute else block
                        match Box::pin(execute_commands(ctx, else_cmds)).await {
                            ExecResult::Continue => {}
                            other => return other,
                        }
                    }
                    Err(e) => return ExecResult::Error(e),
                }
            }
        }
    }
    ExecResult::Continue
}

/// Drain pending PTY output.
async fn drain_pty(ctx: &mut ExecutionContext<'_>) -> ExecResult {
    loop {
        match read_with_timeout(ctx.pty, ctx.buf, Duration::from_millis(10)).await {
            ReadResult::Eof => return ExecResult::Eof,
            ReadResult::Data(n) => {
                if let Some(ref mut rec) = ctx.recording {
                    if let Err(e) = rec.append_raw(&ctx.buf[..n]) {
                        return ExecResult::Error(e);
                    }
                }
                ctx.screen.feed(&ctx.buf[..n]);
                if let Some(ref dir) = ctx.frames_dir {
                    if ctx.screen.changed() {
                        match ctx.screen.save_frame(dir) {
                            Ok(seq) => {
                                if let Some(ref mut rec) = ctx.recording {
                                    if let Err(e) = rec.log_frame(seq) {
                                        return ExecResult::Error(e);
                                    }
                                }
                            }
                            Err(e) => return ExecResult::Error(e),
                        }
                    }
                }
            }
            ReadResult::Error(e) => return ExecResult::Error(e),
            ReadResult::Timeout => break,
        }
    }
    ExecResult::Continue
}

/// Perform a wait operation.
async fn do_wait(
    ctx: &mut ExecutionContext<'_>,
    pattern: &Regex,
    negated: bool,
    wait_timeout: Duration,
) -> Result<WaitResult> {
    let pattern_str = pattern.as_str();
    let deadline = tokio::time::Instant::now() + wait_timeout;

    let condition_met = |screen: &Screen| {
        if negated {
            !screen.matches(pattern)
        } else {
            screen.matches(pattern)
        }
    };

    while !condition_met(ctx.screen) {
        if tokio::time::Instant::now() > deadline {
            if let Some(ref mut rec) = ctx.recording {
                rec.log_wait_timeout(pattern_str)?;
            }
            return Ok(WaitResult::Timeout);
        }
        match read_with_timeout(ctx.pty, ctx.buf, Duration::from_millis(100)).await {
            ReadResult::Eof => {
                let met = condition_met(ctx.screen);
                if let Some(ref mut rec) = ctx.recording {
                    if met {
                        rec.log_wait_match(pattern_str)?;
                    } else {
                        rec.log_wait_eof(pattern_str)?;
                    }
                }
                return Ok(WaitResult::Eof);
            }
            ReadResult::Data(n) => {
                if let Some(ref mut rec) = ctx.recording {
                    rec.append_raw(&ctx.buf[..n])?;
                }
                ctx.screen.feed(&ctx.buf[..n]);
                if let Some(ref dir) = ctx.frames_dir {
                    if ctx.screen.changed() {
                        let seq = ctx.screen.save_frame(dir)?;
                        if let Some(ref mut rec) = ctx.recording {
                            rec.log_frame(seq)?;
                        }
                    }
                }
            }
            ReadResult::Error(e) => return Err(e),
            ReadResult::Timeout => {}
        }
    }

    if let Some(ref mut rec) = ctx.recording {
        rec.log_wait_match(pattern_str)?;
    }
    Ok(WaitResult::Matched)
}

/// Perform a match operation - wait for any of the patterns to match.
/// Returns Some(commands) if a pattern matched, None on timeout.
async fn do_match<'a>(
    ctx: &mut ExecutionContext<'_>,
    arms: &'a [MatchArm],
    wait_timeout: Duration,
) -> Result<Option<&'a Vec<Command>>> {
    let deadline = tokio::time::Instant::now() + wait_timeout;

    loop {
        // Check all patterns against current screen
        for arm in arms {
            if ctx.screen.matches(&arm.pattern) {
                if let Some(ref mut rec) = ctx.recording {
                    rec.log_wait_match(arm.pattern.as_str())?;
                }
                return Ok(Some(&arm.commands));
            }
        }

        // Check timeout
        if tokio::time::Instant::now() > deadline {
            if let Some(ref mut rec) = ctx.recording {
                // Log timeout for all patterns
                let patterns: Vec<_> = arms.iter().map(|a| a.pattern.as_str()).collect();
                rec.log_match_timeout(&patterns)?;
            }
            return Ok(None);
        }

        // Wait for more PTY output
        match read_with_timeout(ctx.pty, ctx.buf, Duration::from_millis(100)).await {
            ReadResult::Eof => {
                // EOF - check one more time then return None
                for arm in arms {
                    if ctx.screen.matches(&arm.pattern) {
                        if let Some(ref mut rec) = ctx.recording {
                            rec.log_wait_match(arm.pattern.as_str())?;
                        }
                        return Ok(Some(&arm.commands));
                    }
                }
                return Ok(None);
            }
            ReadResult::Data(n) => {
                if let Some(ref mut rec) = ctx.recording {
                    rec.append_raw(&ctx.buf[..n])?;
                }
                ctx.screen.feed(&ctx.buf[..n]);
                if let Some(ref dir) = ctx.frames_dir {
                    if ctx.screen.changed() {
                        let seq = ctx.screen.save_frame(dir)?;
                        if let Some(ref mut rec) = ctx.recording {
                            rec.log_frame(seq)?;
                        }
                    }
                }
            }
            ReadResult::Error(e) => return Err(e),
            ReadResult::Timeout => {} // Timeout on read, loop again
        }
    }
}

/// Format send bytes for log.
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
