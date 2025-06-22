use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::OpenOptionsExt;

use async_trait::async_trait;
use nix::pty::Winsize;
use nix::sys::termios::{self, SetArg, Termios};
use nix::libc;
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};

use super::{TtySize, Tty, TtyTheme};

pub struct DevTty {
    file: AsyncFd<File>,
    settings: libc::termios,
}

impl DevTty {
    pub async fn open() -> anyhow::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/tty")?;

        let file = AsyncFd::new(file)?;
        let settings = super::make_raw(&file)?;

        Ok(Self { file, settings })
    }

    pub async fn resize(&mut self, size: TtySize) -> io::Result<()> {
        let xtwinops_seq = format!("\x1b[8;{};{}t", size.1, size.0);
        self.write_all(xtwinops_seq.as_bytes()).await?;

        Ok(())
    }
}

impl Drop for DevTty {
    fn drop(&mut self) {
        let termios = Termios::from(self.settings);
        let _ = termios::tcsetattr(self.file.as_fd(), SetArg::TCSANOW, &termios);
    }
}

#[async_trait(?Send)]
impl Tty for DevTty {
    fn get_size(&self) -> Winsize {
        let mut winsize = Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe { libc::ioctl(self.file.as_raw_fd(), libc::TIOCGWINSZ, &mut winsize) };

        winsize
    }

    async fn get_theme(&mut self) -> Option<TtyTheme> {
        super::get_theme(self).await
    }

    async fn get_version(&mut self) -> Option<String> {
        super::get_version(self).await
    }

    async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.file
            .async_io(Interest::READABLE, |mut file| file.read(buf))
            .await
    }

    async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.file
            .async_io(Interest::WRITABLE, |mut file| file.write(buf))
            .await
    }
}
