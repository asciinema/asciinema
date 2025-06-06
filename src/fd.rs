use std::io;
use std::os::fd::AsFd;

use nix::fcntl::{self, FcntlArg::*, OFlag};

pub trait FdExt: AsFd {
    fn set_nonblocking(&self) -> io::Result<()> {
        let flags = fcntl::fcntl(self.as_fd(), F_GETFL)?;
        let mut oflags = OFlag::from_bits_truncate(flags);
        oflags |= OFlag::O_NONBLOCK;
        fcntl::fcntl(self.as_fd(), F_SETFL(oflags))?;

        Ok(())
    }
}

impl<T: AsFd> FdExt for T {}
