// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Non-blocking I/O helpers for PTY operations.

use nix::errno::Errno;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::os::fd::{AsFd, AsRawFd};

pub fn set_non_blocking<F: AsRawFd>(fd: &F) -> nix::Result<()> {
    let flags = fcntl(fd.as_raw_fd(), FcntlArg::F_GETFL)?;
    let flags = OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK;
    fcntl(fd.as_raw_fd(), FcntlArg::F_SETFL(flags))?;
    Ok(())
}

/// Read, handling EAGAIN/EWOULDBLOCK. Returns None if would block.
pub fn read<F: AsRawFd>(fd: &F, buf: &mut [u8]) -> nix::Result<Option<usize>> {
    match nix::unistd::read(fd.as_raw_fd(), buf) {
        Ok(n) => Ok(Some(n)),
        Err(Errno::EAGAIN) => Ok(None),
        Err(Errno::EIO) => Ok(Some(0)), // PTY closed
        Err(e) => Err(e),
    }
}

/// Write, handling EAGAIN/EWOULDBLOCK. Returns None if would block.
pub fn write<F: AsFd>(fd: &F, buf: &[u8]) -> nix::Result<Option<usize>> {
    match nix::unistd::write(fd, buf) {
        Ok(n) => Ok(Some(n)),
        Err(Errno::EAGAIN) => Ok(None),
        Err(e) => Err(e),
    }
}
