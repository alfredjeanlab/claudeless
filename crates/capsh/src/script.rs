//! Script DSL parser and executor.
//!
//! Parses commands like:
//!   wait "pattern"
//!   wait 2000
//!   send "hello\n"
//!   send <Up> <C-d>
//!   snapshot

use std::io::Read;

use anyhow::{anyhow, Result};
use nix::sys::signal::Signal;
use regex::Regex;

/// A parsed script command.
#[derive(Debug, Clone)]
pub enum Command {
    /// Wait for regex pattern in screen with optional timeout (ms).
    /// The bool indicates negation (true = wait until NOT matching).
    WaitPattern(Regex, bool, Option<u64>),
    /// Wait for milliseconds.
    WaitMs(u64),
    /// Send text/keys to PTY with optional inline delays.
    Send(Vec<SendPart>),
    /// Force snapshot with optional name.
    Snapshot(Option<String>),
    /// Send signal to child process.
    Kill(Signal),
}

/// Part of a send command: either bytes to send or a delay.
#[derive(Debug, Clone, PartialEq)]
pub enum SendPart {
    Bytes(Vec<u8>),
    Delay(u64),
}

/// Parse a script file into commands.
pub fn parse(source: &str) -> Result<Vec<Command>> {
    let mut commands = Vec::new();

    for (lineno, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let cmd = parse_line(line).map_err(|e| anyhow!("line {}: {}", lineno + 1, e))?;
        commands.push(cmd);
    }

    Ok(commands)
}

/// Load and parse script from stdin.
pub fn load_stdin() -> Result<Vec<Command>> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    parse(&buf)
}

fn parse_line(line: &str) -> Result<Command> {
    if let Some(rest) = line.strip_prefix("wait ") {
        let rest = rest.trim();
        // Check for negation
        let (negated, rest) = if let Some(r) = rest.strip_prefix('!') {
            (true, r.trim())
        } else {
            (false, rest)
        };
        // Check if it starts with a digit (duration) vs quote (pattern)
        if rest.starts_with('"') {
            // Pattern, possibly followed by a timeout
            let (pattern, timeout) = parse_wait_pattern_args(rest)?;
            let regex = Regex::new(&pattern)?;
            return Ok(Command::WaitPattern(regex, negated, timeout));
        } else if negated {
            return Err(anyhow!("wait ! requires a pattern"));
        } else {
            // Duration: 500, 500ms, 5s, 2m
            let ms = parse_duration_ms(rest)?;
            return Ok(Command::WaitMs(ms));
        }
    }

    if let Some(rest) = line.strip_prefix("send ") {
        let parts = parse_send_args(rest.trim())?;
        return Ok(Command::Send(parts));
    }

    if line == "snapshot" {
        return Ok(Command::Snapshot(None));
    }

    if let Some(rest) = line.strip_prefix("snapshot ") {
        let name = parse_quoted_string(rest.trim())?;
        return Ok(Command::Snapshot(Some(name)));
    }

    if let Some(rest) = line.strip_prefix("kill ") {
        let signal = parse_signal(rest.trim())?;
        return Ok(Command::Kill(signal));
    }

    Err(anyhow!("unknown command: {}", line))
}

fn parse_quoted_string(s: &str) -> Result<String> {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        Ok(s[1..s.len() - 1].to_string())
    } else {
        Err(anyhow!("expected quoted string"))
    }
}

/// Parse a duration string into milliseconds.
/// Supports: 500 (ms), 500ms, 5s, 2m
fn parse_duration_ms(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!("empty duration"));
    }

    // Find where digits end
    let num_end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());

    if num_end == 0 {
        return Err(anyhow!("invalid duration: {}", s));
    }

    let num: u64 = s[..num_end]
        .parse()
        .map_err(|_| anyhow!("invalid duration number: {}", s))?;
    let suffix = &s[num_end..];

    match suffix {
        "" | "ms" => Ok(num),
        "s" => Ok(num * 1000),
        "m" => Ok(num * 60 * 1000),
        _ => Err(anyhow!("unknown duration suffix: {}", suffix)),
    }
}

/// Parse wait pattern arguments: "pattern" [timeout]
fn parse_wait_pattern_args(s: &str) -> Result<(String, Option<u64>)> {
    let s = s.trim();
    if !s.starts_with('"') {
        return Err(anyhow!("expected quoted pattern"));
    }

    // Find the closing quote
    let mut in_escape = false;
    let mut end_quote = None;
    for (i, c) in s[1..].char_indices() {
        if in_escape {
            in_escape = false;
        } else if c == '\\' {
            in_escape = true;
        } else if c == '"' {
            end_quote = Some(i + 1); // +1 because we started at s[1..]
            break;
        }
    }

    let end = end_quote.ok_or_else(|| anyhow!("unterminated pattern string"))?;
    let pattern = s[1..end].to_string();
    let rest = s[end + 1..].trim();

    let timeout = if rest.is_empty() {
        None
    } else {
        Some(parse_duration_ms(rest)?)
    };

    Ok((pattern, timeout))
}

fn parse_send_args(s: &str) -> Result<Vec<SendPart>> {
    let mut parts = Vec::new();
    let mut current_bytes = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            // Quoted string
            let mut text = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some('\\') => match chars.next() {
                        Some('n') => text.push('\n'),
                        Some('r') => text.push('\r'),
                        Some('t') => text.push('\t'),
                        Some('\\') => text.push('\\'),
                        Some('"') => text.push('"'),
                        Some(c) => return Err(anyhow!("unknown escape: \\{}", c)),
                        None => return Err(anyhow!("unterminated string")),
                    },
                    Some(c) => text.push(c),
                    None => return Err(anyhow!("unterminated string")),
                }
            }
            current_bytes.extend(text.as_bytes());
        } else if c == '<' {
            // Special key
            let mut key = String::new();
            loop {
                match chars.next() {
                    Some('>') => break,
                    Some(c) => key.push(c),
                    None => return Err(anyhow!("unterminated <key>")),
                }
            }
            current_bytes.extend(parse_special_key(&key)?);
        } else if c.is_ascii_digit() {
            // Duration = delay (e.g., 500, 500ms, 5s)
            // First, flush any accumulated bytes
            if !current_bytes.is_empty() {
                parts.push(SendPart::Bytes(std::mem::take(&mut current_bytes)));
            }
            // Parse the full duration (digits + optional suffix)
            let mut duration_str = String::new();
            duration_str.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_alphanumeric() {
                    duration_str.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let ms = parse_duration_ms(&duration_str)?;
            parts.push(SendPart::Delay(ms));
        } else if c.is_whitespace() {
            // Skip whitespace between tokens
        } else {
            return Err(anyhow!("unexpected character: {}", c));
        }
    }

    // Flush remaining bytes
    if !current_bytes.is_empty() {
        parts.push(SendPart::Bytes(current_bytes));
    }

    Ok(parts)
}

fn parse_signal(s: &str) -> Result<Signal> {
    // Try parsing as number first
    if let Ok(num) = s.parse::<i32>() {
        return Signal::try_from(num).map_err(|_| anyhow!("invalid signal number: {}", num));
    }

    // Normalize: strip SIG prefix if present, uppercase
    let name = s.strip_prefix("SIG").unwrap_or(s).to_uppercase();

    match name.as_str() {
        "HUP" => Ok(Signal::SIGHUP),
        "INT" => Ok(Signal::SIGINT),
        "QUIT" => Ok(Signal::SIGQUIT),
        "KILL" => Ok(Signal::SIGKILL),
        "TERM" => Ok(Signal::SIGTERM),
        "USR1" => Ok(Signal::SIGUSR1),
        "USR2" => Ok(Signal::SIGUSR2),
        "STOP" => Ok(Signal::SIGSTOP),
        "CONT" => Ok(Signal::SIGCONT),
        _ => Err(anyhow!("unknown signal: {}", s)),
    }
}

fn parse_special_key(key: &str) -> Result<Vec<u8>> {
    let bytes: &[u8] = match key {
        // Arrow keys
        "Up" => b"\x1b[A",
        "Down" => b"\x1b[B",
        "Right" => b"\x1b[C",
        "Left" => b"\x1b[D",

        // Common keys
        "Enter" => b"\r",
        "Tab" => b"\t",
        "Esc" => b"\x1b",
        "Backspace" => b"\x7f",
        "Space" => b" ",

        // Ctrl+letter
        _ if key.starts_with("C-") && key.len() == 3 => {
            let c = key.chars().nth(2).unwrap();
            if c.is_ascii_lowercase() {
                return Ok(vec![c as u8 - b'a' + 1]);
            } else if c.is_ascii_uppercase() {
                return Ok(vec![c as u8 - b'A' + 1]);
            } else {
                return Err(anyhow!("invalid Ctrl key: {}", key));
            }
        }

        // Meta/Alt+letter (sends ESC + letter)
        _ if (key.starts_with("M-") || key.starts_with("A-")) && key.len() == 3 => {
            let c = key.chars().nth(2).unwrap();
            if c.is_ascii_alphabetic() {
                return Ok(vec![0x1b, c.to_ascii_lowercase() as u8]);
            } else {
                return Err(anyhow!("invalid Meta/Alt key: {}", key));
            }
        }

        _ => return Err(anyhow!("unknown special key: <{}>", key)),
    };

    Ok(bytes.to_vec())
}

#[cfg(test)]
#[path = "script_tests.rs"]
mod tests;
