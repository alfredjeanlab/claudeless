//! Recording state for timing, JSONL log, and raw PTY dump.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use nix::sys::signal::Signal;

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
        self.jsonl.flush()?;
        Ok(())
    }

    pub fn log_snapshot(&mut self, seq: u64, name: Option<&str>) -> Result<()> {
        match name {
            Some(n) => {
                let escaped = Self::escape_json(n);
                writeln!(
                    self.jsonl,
                    r#"{{"ms":{},"snapshot":"{:06}","name":"{}"}}"#,
                    self.elapsed_ms(),
                    seq,
                    escaped
                )?;
            }
            None => {
                writeln!(
                    self.jsonl,
                    r#"{{"ms":{},"snapshot":"{:06}"}}"#,
                    self.elapsed_ms(),
                    seq
                )?;
            }
        }
        self.jsonl.flush()?;
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
        self.jsonl.flush()?;
        Ok(())
    }

    pub fn log_exit(&mut self, code: i32) -> Result<()> {
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"exit":{}}}"#,
            self.elapsed_ms(),
            code
        )?;
        self.jsonl.flush()?;
        Ok(())
    }

    pub fn log_kill(&mut self, signal: Signal) -> Result<()> {
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"kill":"{}"}}"#,
            self.elapsed_ms(),
            signal
        )?;
        self.jsonl.flush()?;
        Ok(())
    }

    /// Log when a wait pattern was found on screen.
    pub fn log_wait_match(&mut self, pattern: &str) -> Result<()> {
        let escaped = Self::escape_json(pattern);
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"wait_match":"{}"}}"#,
            self.elapsed_ms(),
            escaped
        )?;
        self.jsonl.flush()?;
        Ok(())
    }

    /// Log when a wait timed out without the pattern matching.
    pub fn log_wait_timeout(&mut self, pattern: &str) -> Result<()> {
        let escaped = Self::escape_json(pattern);
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"wait_timeout":"{}"}}"#,
            self.elapsed_ms(),
            escaped
        )?;
        self.jsonl.flush()?;
        Ok(())
    }

    /// Log when a wait ended due to EOF without the pattern matching.
    pub fn log_wait_eof(&mut self, pattern: &str) -> Result<()> {
        let escaped = Self::escape_json(pattern);
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"wait_eof":"{}"}}"#,
            self.elapsed_ms(),
            escaped
        )?;
        self.jsonl.flush()?;
        Ok(())
    }

    /// Log when a match statement timed out without any pattern matching.
    pub fn log_match_timeout(&mut self, patterns: &[&str]) -> Result<()> {
        let escaped: Vec<_> = patterns.iter().map(|p| Self::escape_json(p)).collect();
        let json_array = escaped
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            self.jsonl,
            r#"{{"ms":{},"match_timeout":[{}]}}"#,
            self.elapsed_ms(),
            json_array
        )?;
        self.jsonl.flush()?;
        Ok(())
    }

    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
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

#[cfg(test)]
#[path = "recording_tests.rs"]
mod tests;
