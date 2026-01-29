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
    /// Conditional execution based on wait result.
    IfWait {
        pattern: Regex,
        negated: bool,
        timeout: Option<u64>,
        then_cmds: Vec<Command>,
        else_cmds: Vec<Command>,
    },
    /// Match first pattern that appears, execute corresponding commands.
    Match {
        timeout: Option<u64>,
        arms: Vec<MatchArm>,
        else_cmds: Vec<Command>,
    },
}

/// A match arm: pattern and commands to execute if matched.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Regex,
    pub commands: Vec<Command>,
}

/// Part of a send command: either bytes to send or a delay.
#[derive(Debug, Clone, PartialEq)]
pub enum SendPart {
    Bytes(Vec<u8>),
    Delay(u64),
}

/// Parse a script file into commands.
pub fn parse(source: &str) -> Result<Vec<Command>> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let mut iter = lines.iter().peekable();
    parse_block(&mut iter, false)
}

/// Parse a block of commands until end/else or EOF.
fn parse_block(
    iter: &mut std::iter::Peekable<std::slice::Iter<(usize, &str)>>,
    in_if: bool,
) -> Result<Vec<Command>> {
    let mut commands = Vec::new();

    while let Some(&(lineno, line)) = iter.peek() {
        // Check for block terminators
        if *line == "end" || *line == "else" || line.starts_with("else if ") {
            if !in_if {
                return Err(anyhow!("line {}: unexpected '{}'", lineno, line));
            }
            break;
        }

        iter.next(); // consume the line

        // Check for if statement
        if let Some(rest) = line.strip_prefix("if ") {
            let cmd = parse_if_statement(rest.trim(), iter, *lineno)?;
            commands.push(cmd);
        } else if let Some(rest) = line.strip_prefix("match ") {
            let cmd = parse_match_statement(rest.trim(), iter, *lineno)?;
            commands.push(cmd);
        } else if *line == "match" {
            let cmd = parse_match_statement("", iter, *lineno)?;
            commands.push(cmd);
        } else {
            let cmd = parse_line(line).map_err(|e| anyhow!("line {}: {}", lineno, e))?;
            commands.push(cmd);
        }
    }

    Ok(commands)
}

/// Parse an if statement with its then/else blocks.
fn parse_if_statement(
    condition: &str,
    iter: &mut std::iter::Peekable<std::slice::Iter<(usize, &str)>>,
    lineno: usize,
) -> Result<Command> {
    // Parse: if wait "pattern" [timeout]
    let rest = condition
        .strip_prefix("wait ")
        .ok_or_else(|| anyhow!("line {}: expected 'if wait'", lineno))?
        .trim();

    // Check for negation
    let (negated, rest) = if let Some(r) = rest.strip_prefix('!') {
        (true, r.trim())
    } else {
        (false, rest)
    };

    if !rest.starts_with('"') {
        return Err(anyhow!("line {}: expected pattern after 'if wait'", lineno));
    }

    let (pattern, timeout) = parse_wait_pattern_args(rest)?;
    let regex = Regex::new(&pattern)?;

    // Parse then block
    let then_cmds = parse_block(iter, true)?;

    // Check for else or else if
    let (else_cmds, had_else_if) = if let Some(&(ln, line)) = iter.peek() {
        if *line == "else" {
            iter.next(); // consume "else"
            (parse_block(iter, true)?, false)
        } else if let Some(rest) = line.strip_prefix("else if ") {
            iter.next(); // consume "else if ..."
            // Parse as nested if - it will consume the shared "end"
            let nested_if = parse_if_statement(rest.trim(), iter, *ln)?;
            (vec![nested_if], true)
        } else {
            (Vec::new(), false)
        }
    } else {
        (Vec::new(), false)
    };

    // Expect "end" only if we didn't have else if (else if consumes the shared end)
    if !had_else_if {
        match iter.next() {
            Some(&(_, "end")) => {}
            Some(&(ln, other)) => return Err(anyhow!("line {}: expected 'end', got '{}'", ln, other)),
            None => return Err(anyhow!("unexpected end of script, expected 'end'")),
        }
    }

    Ok(Command::IfWait {
        pattern: regex,
        negated,
        timeout,
        then_cmds,
        else_cmds,
    })
}

/// Parse a match statement with pattern arms.
fn parse_match_statement(
    args: &str,
    iter: &mut std::iter::Peekable<std::slice::Iter<(usize, &str)>>,
    lineno: usize,
) -> Result<Command> {
    // Parse optional timeout
    let timeout = if args.is_empty() {
        None
    } else {
        Some(parse_duration_ms(args)?)
    };

    let mut arms = Vec::new();

    // Parse match arms until else or end
    while let Some(&(ln, line)) = iter.peek() {
        if *line == "end" || *line == "else" {
            break;
        }

        iter.next(); // consume the line

        // Parse: "pattern" -> [command] with optional block
        let arm = parse_match_arm(line, *ln, iter)?;
        arms.push(arm);
    }

    if arms.is_empty() {
        return Err(anyhow!(
            "line {}: match requires at least one pattern arm",
            lineno
        ));
    }

    // Check for else
    let else_cmds = if let Some(&(_, "else")) = iter.peek() {
        iter.next(); // consume "else"
        parse_match_else_block(iter)?
    } else {
        Vec::new()
    };

    // Expect "end"
    match iter.next() {
        Some(&(_, "end")) => {}
        Some(&(ln, other)) => {
            return Err(anyhow!("line {}: expected 'end', got '{}'", ln, other))
        }
        None => return Err(anyhow!("unexpected end of script, expected 'end'")),
    }

    Ok(Command::Match {
        timeout,
        arms,
        else_cmds,
    })
}

/// Parse a match arm: "pattern" -> command(s) or "pattern" -> followed by block
fn parse_match_arm(
    line: &str,
    lineno: usize,
    iter: &mut std::iter::Peekable<std::slice::Iter<(usize, &str)>>,
) -> Result<MatchArm> {
    // Find the -> separator
    let arrow_pos = line
        .find("->")
        .ok_or_else(|| anyhow!("line {}: expected '\"pattern\" -> command'", lineno))?;

    let pattern_part = line[..arrow_pos].trim();
    let command_part = line[arrow_pos + 2..].trim();

    // Parse the pattern (quoted string)
    if !pattern_part.starts_with('"') || !pattern_part.ends_with('"') || pattern_part.len() < 2 {
        return Err(anyhow!(
            "line {}: match arm pattern must be quoted",
            lineno
        ));
    }

    let pattern_str = &pattern_part[1..pattern_part.len() - 1];
    let pattern = Regex::new(pattern_str)
        .map_err(|e| anyhow!("line {}: invalid regex '{}': {}", lineno, pattern_str, e))?;

    // Parse the command(s) after ->
    let commands = if command_part.is_empty() {
        // Empty after -> means block: read lines until next pattern, else, or end
        parse_match_arm_block(iter)?
    } else {
        vec![parse_line(command_part).map_err(|e| anyhow!("line {}: {}", lineno, e))?]
    };

    Ok(MatchArm { pattern, commands })
}

/// Parse commands in a match arm block until next pattern, else, or end.
fn parse_match_arm_block(
    iter: &mut std::iter::Peekable<std::slice::Iter<(usize, &str)>>,
) -> Result<Vec<Command>> {
    let mut commands = Vec::new();

    while let Some(&(lineno, line)) = iter.peek() {
        // Stop at: next pattern arm, else, or end
        if *line == "end" || *line == "else" || line.starts_with('"') {
            break;
        }

        iter.next(); // consume the line
        let cmd = parse_line(line).map_err(|e| anyhow!("line {}: {}", lineno, e))?;
        commands.push(cmd);
    }

    Ok(commands)
}

/// Parse commands after else in a match block (until end).
fn parse_match_else_block(
    iter: &mut std::iter::Peekable<std::slice::Iter<(usize, &str)>>,
) -> Result<Vec<Command>> {
    let mut commands = Vec::new();

    while let Some(&(lineno, line)) = iter.peek() {
        if *line == "end" {
            break;
        }

        iter.next(); // consume the line
        let cmd = parse_line(line).map_err(|e| anyhow!("line {}: {}", lineno, e))?;
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
