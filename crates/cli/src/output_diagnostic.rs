// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Diagnostic output helpers for consistent error/warning formatting.
//!
//! Provides ANSI color support with automatic terminal detection.

use std::io::{self, IsTerminal, Write};

/// Print an error message to stderr.
///
/// Displays in red when stderr is a terminal, plain text otherwise.
pub fn print_error(msg: impl std::fmt::Display) {
    let is_tty = io::stderr().is_terminal();
    write_error(&mut io::stderr(), msg, is_tty);
}

/// Write an error message to a writer with explicit terminal flag.
fn write_error<W: Write>(writer: &mut W, msg: impl std::fmt::Display, is_terminal: bool) {
    if is_terminal {
        let _ = writeln!(writer, "\x1b[31mError: {}\x1b[0m", msg);
    } else {
        let _ = writeln!(writer, "Error: {}", msg);
    }
}

/// Print a warning message to stderr.
///
/// Displays in yellow when stderr is a terminal, plain text otherwise.
pub fn print_warning(msg: impl std::fmt::Display) {
    let is_tty = io::stderr().is_terminal();
    write_warning(&mut io::stderr(), msg, is_tty);
}

/// Write a warning message to a writer with explicit terminal flag.
fn write_warning<W: Write>(writer: &mut W, msg: impl std::fmt::Display, is_terminal: bool) {
    if is_terminal {
        let _ = writeln!(writer, "\x1b[33mWarning: {}\x1b[0m", msg);
    } else {
        let _ = writeln!(writer, "Warning: {}", msg);
    }
}

/// Print an MCP debug message to stderr.
///
/// Used for MCP-related status messages.
pub fn print_mcp(msg: impl std::fmt::Display) {
    let _ = writeln!(io::stderr(), "MCP: {}", msg);
}

/// Print an MCP error message to stderr.
///
/// For strict mode failures that require immediate exit.
pub fn print_mcp_error(msg: impl std::fmt::Display) {
    let _ = writeln!(io::stderr(), "MCP error: {}", msg);
}

/// Print an MCP warning message to stderr.
///
/// For non-fatal MCP issues in debug mode.
pub fn print_mcp_warning(msg: impl std::fmt::Display) {
    let _ = writeln!(io::stderr(), "MCP warning: {}", msg);
}

#[cfg(test)]
#[path = "output_diagnostic_tests.rs"]
mod tests;
