//! PTY (pseudo-terminal) handling.
//!
//! Creates a PTY pair, spawns the child process, and provides
//! async read/write to the master side.

use anyhow::Result;

/// A running PTY session with a child process.
pub struct Pty {
    // TODO: master fd, child pid
}

impl Pty {
    /// Spawn a command in a new PTY.
    pub fn spawn(_cmd: &[String], _cols: u16, _rows: u16) -> Result<Self> {
        todo!()
    }

    /// Write input to the PTY (sends to child's stdin).
    pub fn write(&mut self, _data: &[u8]) -> Result<()> {
        todo!()
    }

    /// Read output from the PTY (child's stdout/stderr).
    pub fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        todo!()
    }

    /// Resize the terminal.
    pub fn resize(&self, _cols: u16, _rows: u16) -> Result<()> {
        todo!()
    }
}
