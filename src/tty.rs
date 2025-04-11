use std::fs;
use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

use anyhow::Result;
use nix::{
    errno::Errno,
    libc, pty,
    sys::{
        select::{select, FdSet},
        time::TimeVal,
    },
    unistd,
};
use rgb::RGB8;
use termion::raw::{IntoRawMode, RawTerminal};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TtySize(pub u16, pub u16);

impl Default for TtySize {
    fn default() -> Self {
        TtySize(80, 24)
    }
}

impl From<pty::Winsize> for TtySize {
    fn from(winsize: pty::Winsize) -> Self {
        TtySize(winsize.ws_col, winsize.ws_row)
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

pub trait Tty: io::Write + io::Read + AsFd {
    fn get_size(&self) -> pty::Winsize;
    fn get_theme(&self) -> Option<TtyTheme>;
    fn get_version(&self) -> Option<String>;
}

#[derive(Clone)]
pub struct TtyTheme {
    pub fg: RGB8,
    pub bg: RGB8,
    pub palette: Vec<RGB8>,
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

    fn query(&self, query: &str) -> Result<Vec<u8>> {
        let mut query = query.to_string().into_bytes();
        query.extend_from_slice(b"\x1b[c");
        let mut query = &query[..];
        let mut response = Vec::new();
        let mut buf = [0u8; 1024];
        let fd = self.as_fd().as_raw_fd();

        loop {
            let mut timeout = TimeVal::new(0, 100_000);
            let mut rfds = FdSet::new();
            let mut wfds = FdSet::new();
            rfds.insert(self);

            if !query.is_empty() {
                wfds.insert(self);
            }

            match select(None, &mut rfds, &mut wfds, None, &mut timeout) {
                Ok(0) => break,

                Ok(_) => {
                    if rfds.contains(self) {
                        let n = unistd::read(fd, &mut buf)?;
                        response.extend_from_slice(&buf[..n]);
                        let mut reversed = response.iter().rev();
                        let mut got_da_response = false;
                        let mut da_len = 0;

                        if let Some(b'c') = reversed.next() {
                            da_len += 1;

                            for b in reversed {
                                if *b == b'[' {
                                    got_da_response = true;
                                    break;
                                }

                                if *b != b';' && *b != b'?' && !b.is_ascii_digit() {
                                    break;
                                }

                                da_len += 1;
                            }
                        }

                        if got_da_response {
                            response.truncate(response.len() - da_len - 2);
                            break;
                        }
                    }

                    if wfds.contains(self) {
                        let n = unistd::write(fd, query)?;
                        query = &query[n..];
                    }
                }

                Err(e) => {
                    if e == Errno::EINTR {
                        continue;
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(response)
    }
}

fn parse_color(rgb: &str) -> Option<RGB8> {
    let mut components = rgb.split('/');
    let r_hex = components.next()?;
    let g_hex = components.next()?;
    let b_hex = components.next()?;

    if r_hex.len() < 2 || g_hex.len() < 2 || b_hex.len() < 2 {
        return None;
    }

    let r = u8::from_str_radix(&r_hex[..2], 16).ok()?;
    let g = u8::from_str_radix(&g_hex[..2], 16).ok()?;
    let b = u8::from_str_radix(&b_hex[..2], 16).ok()?;

    Some(RGB8::new(r, g, b))
}

static COLORS_QUERY: &str = "\x1b]10;?\x07\x1b]11;?\x07\x1b]4;0;?\x07\x1b]4;1;?\x07\x1b]4;2;?\x07\x1b]4;3;?\x07\x1b]4;4;?\x07\x1b]4;5;?\x07\x1b]4;6;?\x07\x1b]4;7;?\x07\x1b]4;8;?\x07\x1b]4;9;?\x07\x1b]4;10;?\x07\x1b]4;11;?\x07\x1b]4;12;?\x07\x1b]4;13;?\x07\x1b]4;14;?\x07\x1b]4;15;?\x07";

static XTVERSION_QUERY: &str = "\x1b[>0q";

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

    fn get_theme(&self) -> Option<TtyTheme> {
        let response = self.query(COLORS_QUERY).ok()?;
        let response = String::from_utf8_lossy(response.as_slice());
        let mut colors = response.match_indices("rgb:");
        let (idx, _) = colors.next()?;
        let fg = parse_color(&response[idx + 4..])?;
        let (idx, _) = colors.next()?;
        let bg = parse_color(&response[idx + 4..])?;
        let mut palette = Vec::new();

        for _ in 0..16 {
            let (idx, _) = colors.next()?;
            let color = parse_color(&response[idx + 4..])?;
            palette.push(color);
        }

        Some(TtyTheme { fg, bg, palette })
    }

    fn get_version(&self) -> Option<String> {
        let response = self.query(XTVERSION_QUERY).ok()?;

        if let [b'\x1b', b'P', b'>', b'|', version @ .., b'\x1b', b'\\'] = &response[..] {
            Some(String::from_utf8_lossy(version).to_string())
        } else {
            None
        }
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

pub struct NullTty {
    tx: OwnedFd,
    _rx: OwnedFd,
}

impl NullTty {
    pub fn open() -> Result<Self> {
        let (rx, tx) = unistd::pipe()?;
        let rx = unsafe { OwnedFd::from_raw_fd(rx) };
        let tx = unsafe { OwnedFd::from_raw_fd(tx) };

        Ok(Self { tx, _rx: rx })
    }
}

impl Tty for NullTty {
    fn get_size(&self) -> pty::Winsize {
        pty::Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }

    fn get_theme(&self) -> Option<TtyTheme> {
        None
    }

    fn get_version(&self) -> Option<String> {
        None
    }
}

impl io::Read for NullTty {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        panic!("read attempt from NullTty");
    }
}

impl io::Write for NullTty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsFd for NullTty {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.tx.as_fd()
    }
}

pub struct FixedSizeTty {
    inner: Box<dyn Tty>,
    cols: Option<u16>,
    rows: Option<u16>,
}

impl FixedSizeTty {
    pub fn new<T: Tty + 'static>(inner: T, cols: Option<u16>, rows: Option<u16>) -> Self {
        Self {
            inner: Box::new(inner),
            cols,
            rows,
        }
    }
}

impl Tty for FixedSizeTty {
    fn get_size(&self) -> pty::Winsize {
        let mut winsize = self.inner.get_size();

        if let Some(cols) = self.cols {
            winsize.ws_col = cols;
        }

        if let Some(rows) = self.rows {
            winsize.ws_row = rows;
        }

        winsize
    }

    fn get_theme(&self) -> Option<TtyTheme> {
        self.inner.get_theme()
    }

    fn get_version(&self) -> Option<String> {
        self.inner.get_version()
    }
}

impl AsFd for FixedSizeTty {
    fn as_fd(&self) -> BorrowedFd<'_> {
        return self.inner.as_fd();
    }
}

impl io::Read for FixedSizeTty {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl io::Write for FixedSizeTty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::{FixedSizeTty, Tty};
    use crate::tty::NullTty;
    use rgb::RGB8;

    #[test]
    fn parse_color() {
        use super::parse_color as parse;
        let color = Some(RGB8::new(0xaa, 0xbb, 0xcc));

        assert_eq!(parse("aa11/bb22/cc33"), color);
        assert_eq!(parse("aa11/bb22/cc33\x07"), color);
        assert_eq!(parse("aa11/bb22/cc33\x1b\\"), color);
        assert_eq!(parse("aa11/bb22/cc33.."), color);
        assert_eq!(parse("aa1/bb2/cc3"), color);
        assert_eq!(parse("aa1/bb2/cc3\x07"), color);
        assert_eq!(parse("aa1/bb2/cc3\x1b\\"), color);
        assert_eq!(parse("aa1/bb2/cc3.."), color);
        assert_eq!(parse("aa/bb/cc"), color);
        assert_eq!(parse("aa/bb/cc\x07"), color);
        assert_eq!(parse("aa/bb/cc\x1b\\"), color);
        assert_eq!(parse("aa/bb/cc.."), color);
        assert_eq!(parse("aa11/bb22"), None);
        assert_eq!(parse("xxxx/yyyy/zzzz"), None);
        assert_eq!(parse("xxx/yyy/zzz"), None);
        assert_eq!(parse("xx/yy/zz"), None);
        assert_eq!(parse("foo"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn fixed_size_tty() {
        let tty = FixedSizeTty::new(NullTty::open().unwrap(), Some(100), Some(50));

        let winsize = tty.get_size();

        assert!(winsize.ws_col == 100);
        assert!(winsize.ws_row == 50);
    }
}
