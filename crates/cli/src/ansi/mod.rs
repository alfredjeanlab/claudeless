// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! ANSI escape sequence parsing module.
//!
//! Provides utilities for parsing, extracting, and manipulating ANSI escape sequences
//! in terminal output. Used primarily for TUI snapshot testing with color comparison.

mod parser;

pub use parser::{extract_sequences, parse_ansi, strip_ansi, AnsiSequence, AnsiSpan};
