use anyhow::Result;
use mio::unix::pipe;
use nix::{libc, pty};
use std::{
    fs, io,
    os::fd::{AsFd, AsRawFd, BorrowedFd},
};
use termion::raw::{IntoRawMode, RawTerminal};

pub trait Tty: io::Write + io::Read + AsFd {
    fn get_size(&self) -> pty::Winsize;
}

pub struct DevTty {
    file: RawTerminal<fs::File>,
}

impl DevTty {
    pub fn open() -> Result<Self> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into_raw_mode()?;

        crate::io::set_non_blocking(&file.as_raw_fd())?;

        Ok(Self { file })
    }
}

impl Tty for DevTty {
    fn get_size(&self) -> pty::Winsize {
        let mut winsize = pty::Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe { libc::ioctl(self.file.as_raw_fd(), libc::TIOCGWINSZ, &mut winsize) };

        winsize
    }
}

impl io::Read for DevTty {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl io::Write for DevTty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl AsFd for DevTty {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

pub struct DevNull {
    tx: pipe::Sender,
    _rx: pipe::Receiver,
}

impl DevNull {
    pub fn open() -> Result<Self> {
        let (tx, rx) = pipe::new()?;

        Ok(Self { tx, _rx: rx })
    }
}

impl Tty for DevNull {
    fn get_size(&self) -> pty::Winsize {
        pty::Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

impl io::Read for DevNull {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        panic!("read attempt from DevNull impl of Tty");
    }
}

impl io::Write for DevNull {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsFd for DevNull {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.tx.as_raw_fd()) }
    }
}
