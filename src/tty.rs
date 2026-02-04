mod inspect;

use std::os::fd::AsFd;

use async_trait::async_trait;
use nix::libc;
use nix::pty::Winsize;
use nix::sys::termios::{self, SetArg};
use rgb::RGB8;
use tokio::io;

#[cfg(all(not(target_os = "macos"), not(feature = "macos-tty")))]
mod default;

#[cfg(any(target_os = "macos", feature = "macos-tty"))]
mod macos;

#[cfg(all(not(target_os = "macos"), not(feature = "macos-tty")))]
pub use default::DevTty;

#[cfg(any(target_os = "macos", feature = "macos-tty"))]
pub use macos::DevTty;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TtySize(pub u16, pub u16);

#[derive(Clone)]
pub struct TtyTheme {
    pub fg: RGB8,
    pub bg: RGB8,
    pub palette: Vec<RGB8>,
}

pub struct NullTty;

pub struct FixedSizeTty<T> {
    inner: T,
    cols: Option<u16>,
    rows: Option<u16>,
}

#[async_trait(?Send)]
pub trait RawTty {
    fn get_size(&self) -> Winsize;
    async fn read(&self, buf: &mut [u8]) -> io::Result<usize>;
    async fn write(&self, buf: &[u8]) -> io::Result<usize>;

    async fn write_all(&self, mut buf: &[u8]) -> io::Result<()> {
        while !buf.is_empty() {
            let n = self.write(buf).await?;
            buf = &buf[n..];
        }

        Ok(())
    }
}

impl Default for TtySize {
    fn default() -> Self {
        TtySize(80, 24)
    }
}

impl From<Winsize> for TtySize {
    fn from(winsize: Winsize) -> Self {
        TtySize(winsize.ws_col, winsize.ws_row)
    }
}

impl From<TtySize> for Winsize {
    fn from(tty_size: TtySize) -> Self {
        Winsize {
            ws_col: tty_size.0,
            ws_row: tty_size.1,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

impl From<(usize, usize)> for TtySize {
    fn from((cols, rows): (usize, usize)) -> Self {
        TtySize(cols as u16, rows as u16)
    }
}

impl From<TtySize> for (u16, u16) {
    fn from(tty_size: TtySize) -> Self {
        (tty_size.0, tty_size.1)
    }
}

impl<T: RawTty> FixedSizeTty<T> {
    pub fn new(inner: T, cols: Option<u16>, rows: Option<u16>) -> Self {
        Self { inner, cols, rows }
    }
}

#[async_trait(?Send)]
impl RawTty for NullTty {
    fn get_size(&self) -> Winsize {
        Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }

    async fn read(&self, _buf: &mut [u8]) -> io::Result<usize> {
        std::future::pending().await
    }

    async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
}

#[async_trait(?Send)]
impl<T: RawTty> RawTty for FixedSizeTty<T> {
    fn get_size(&self) -> Winsize {
        let mut winsize = self.inner.get_size();

        if let Some(cols) = self.cols {
            winsize.ws_col = cols;
        }

        if let Some(rows) = self.rows {
            winsize.ws_row = rows;
        }

        winsize
    }

    async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).await
    }

    async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).await
    }
}

fn make_raw<F: AsFd>(fd: F) -> anyhow::Result<libc::termios> {
    let termios = termios::tcgetattr(fd.as_fd())?;
    let mut raw_termios = termios.clone();
    termios::cfmakeraw(&mut raw_termios);
    termios::tcsetattr(fd.as_fd(), SetArg::TCSANOW, &raw_termios)?;

    Ok(termios.into())
}

pub(crate) use inspect::inspect;

#[cfg(test)]
mod tests {
    use super::{FixedSizeTty, NullTty, RawTty};

    #[test]
    fn fixed_size_tty_get_size() {
        let tty = FixedSizeTty::new(NullTty, Some(100), Some(50));
        let winsize = tty.get_size();
        assert!(winsize.ws_col == 100);
        assert!(winsize.ws_row == 50);

        let tty = FixedSizeTty::new(NullTty, Some(100), None);
        let winsize = tty.get_size();
        assert!(winsize.ws_col == 100);
        assert!(winsize.ws_row == 24);

        let tty = FixedSizeTty::new(NullTty, None, None);
        let winsize = tty.get_size();
        assert!(winsize.ws_col == 80);
        assert!(winsize.ws_row == 24);
    }
}
