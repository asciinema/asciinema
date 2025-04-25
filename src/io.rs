use std::io;
use std::os::fd::RawFd;

use anyhow::Result;

pub fn set_non_blocking(fd: &RawFd) -> Result<(), io::Error> {
    use nix::fcntl::{fcntl, FcntlArg::*, OFlag};

    let flags = fcntl(*fd, F_GETFL)?;
    let mut oflags = OFlag::from_bits_truncate(flags);
    oflags |= OFlag::O_NONBLOCK;
    fcntl(*fd, F_SETFL(oflags))?;

    Ok(())
}
