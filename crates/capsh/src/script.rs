//! Script DSL parser and executor.
//!
//! Parses commands like:
//!   wait "pattern"
//!   wait 2000
//!   send "hello\n"
//!   send <Up> <C-d>
//!   snapshot

use anyhow::{anyhow, Result};
use regex::Regex;

/// A parsed script command.
#[derive(Debug, Clone)]
pub enum Command {
    /// Wait for regex pattern in screen.
    WaitPattern(Regex),
    /// Wait for milliseconds.
    WaitMs(u64),
    /// Send text/keys to PTY.
    Send(Vec<u8>),
    /// Force snapshot.
    Snapshot,
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

fn parse_line(line: &str) -> Result<Command> {
    if let Some(rest) = line.strip_prefix("wait ") {
        let rest = rest.trim();
        // Check if it's a number (milliseconds)
        if let Ok(ms) = rest.parse::<u64>() {
            return Ok(Command::WaitMs(ms));
        }
        // Otherwise it's a pattern
        let pattern = parse_quoted_string(rest)?;
        let regex = Regex::new(&pattern)?;
        return Ok(Command::WaitPattern(regex));
    }

    if let Some(rest) = line.strip_prefix("send ") {
        let bytes = parse_send_args(rest.trim())?;
        return Ok(Command::Send(bytes));
    }

    if line == "snapshot" {
        return Ok(Command::Snapshot);
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

fn parse_send_args(s: &str) -> Result<Vec<u8>> {
    let mut result = Vec::new();
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
            result.extend(text.as_bytes());
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
            result.extend(parse_special_key(&key)?);
        } else if c.is_whitespace() {
            // Skip whitespace between tokens
        } else {
            return Err(anyhow!("unexpected character: {}", c));
        }
    }

    Ok(result)
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

        _ => return Err(anyhow!("unknown special key: <{}>", key)),
    };

    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wait_pattern() {
        let cmds = parse(r#"wait "Ready>""#).unwrap();
        assert!(matches!(cmds[0], Command::WaitPattern(_)));
    }

    #[test]
    fn parse_wait_ms() {
        let cmds = parse("wait 2000").unwrap();
        assert!(matches!(cmds[0], Command::WaitMs(2000)));
    }

    #[test]
    fn parse_send_text() {
        let cmds = parse(r#"send "hello\n""#).unwrap();
        if let Command::Send(bytes) = &cmds[0] {
            assert_eq!(bytes, b"hello\n");
        } else {
            panic!("expected Send");
        }
    }

    #[test]
    fn parse_send_special_keys() {
        let cmds = parse("send <Up> <C-d>").unwrap();
        if let Command::Send(bytes) = &cmds[0] {
            assert_eq!(bytes, b"\x1b[A\x04");
        } else {
            panic!("expected Send");
        }
    }
}
