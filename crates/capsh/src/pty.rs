//! PTY (pseudo-terminal) handling.
//!
//! Creates a PTY pair, spawns the child process, and provides
//! async read/write to the master side.

use std::ffi::CString;
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};

use anyhow::Result;
use nix::pty::{forkpty, Winsize};
use nix::sys::signal::{signal, SigHandler, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{ForkResult, Pid};
use tokio::io::unix::AsyncFd;

/// A running PTY session with a child process.
pub struct Pty {
    master_fd: AsyncFd<OwnedFd>,
    child_pid: Pid,
}

impl Pty {
    /// Spawn a command in a new PTY.
    pub fn spawn(command: &str, cols: u16, rows: u16) -> Result<Self> {
        let winsize = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        // SAFETY: forkpty is safe to call; it creates a new PTY and forks.
        // In the child, we immediately exec, so no shared state issues.
        let result = unsafe { forkpty(&winsize, None)? };

        match result.fork_result {
            ForkResult::Child => {
                // SAFETY: Restoring SIGPIPE to default is safe in the child process
                // before exec. The child has no other threads at this point.
                unsafe { signal(Signal::SIGPIPE, SigHandler::SigDfl).ok() };
                // Use vt100 which supports basic escape sequences but not alternate screen
                std::env::set_var("TERM", "vt100");

                let shell = CString::new("/bin/sh").unwrap();
                let args = [
                    CString::new("/bin/sh").unwrap(),
                    CString::new("-c").unwrap(),
                    CString::new(command).unwrap(),
                ];
                nix::unistd::execvp(&shell, &args)?;
                unreachable!()
            }
            ForkResult::Parent { child } => {
                let master = result.master;
                crate::nbio::set_non_blocking(&master)?;

                // SAFETY: We own the master fd from forkpty and transfer ownership to OwnedFd.
                // The fd is valid and not used elsewhere after this point.
                let owned: OwnedFd = unsafe { OwnedFd::from_raw_fd(master.into_raw_fd()) };
                let async_fd = AsyncFd::new(owned)?;

                Ok(Self {
                    master_fd: async_fd,
                    child_pid: child,
                })
            }
        }
    }

    /// Read output from the PTY (child's stdout/stderr).
    pub async fn read(&self, buf: &mut [u8]) -> Result<usize> {
        loop {
            let mut guard = self.master_fd.readable().await?;
            match crate::nbio::read(self.master_fd.get_ref(), buf)? {
                Some(n) => return Ok(n),
                None => guard.clear_ready(),
            }
        }
    }

    /// Write input to the PTY (sends to child's stdin).
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let mut written = 0;
        while written < data.len() {
            let mut guard = self.master_fd.writable().await?;
            match crate::nbio::write(self.master_fd.get_ref(), &data[written..])? {
                Some(n) => written += n,
                None => guard.clear_ready(),
            }
        }
        Ok(())
    }

    /// Send a signal to the child process.
    pub fn kill(&self, signal: Signal) -> Result<()> {
        nix::sys::signal::kill(self.child_pid, signal)?;
        Ok(())
    }

    /// Wait for the child process to exit and return exit code.
    pub async fn wait(self) -> Result<i32> {
        nix::sys::signal::kill(self.child_pid, Signal::SIGHUP).ok();

        let pid = self.child_pid;
        let status = tokio::task::spawn_blocking(move || waitpid(pid, None)).await??;

        match status {
            WaitStatus::Exited(_, code) => Ok(code),
            WaitStatus::Signaled(_, sig, _) => Ok(128 + sig as i32),
            _ => Ok(1),
        }
    }
}
