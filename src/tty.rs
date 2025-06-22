use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::OpenOptionsExt;

use async_trait::async_trait;
use nix::libc;
use nix::pty::Winsize;
use nix::sys::termios::{self, SetArg, Termios};
use rgb::RGB8;
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};
use tokio::time::{self, Duration};

const QUERY_READ_TIMEOUT: u64 = 500;
const COLORS_QUERY: &str = "\x1b]10;?\x07\x1b]11;?\x07\x1b]4;0;?\x07\x1b]4;1;?\x07\x1b]4;2;?\x07\x1b]4;3;?\x07\x1b]4;4;?\x07\x1b]4;5;?\x07\x1b]4;6;?\x07\x1b]4;7;?\x07\x1b]4;8;?\x07\x1b]4;9;?\x07\x1b]4;10;?\x07\x1b]4;11;?\x07\x1b]4;12;?\x07\x1b]4;13;?\x07\x1b]4;14;?\x07\x1b]4;15;?\x07";
const XTVERSION_QUERY: &str = "\x1b[>0q";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TtySize(pub u16, pub u16);

#[derive(Clone)]
pub struct TtyTheme {
    pub fg: RGB8,
    pub bg: RGB8,
    pub palette: Vec<RGB8>,
}

pub struct DevTty {
    file: AsyncFd<File>,
    settings: libc::termios,
}

pub struct NullTty;

pub struct FixedSizeTty<T> {
    inner: T,
    cols: Option<u16>,
    rows: Option<u16>,
}

#[async_trait(?Send)]
pub trait Tty {
    fn get_size(&self) -> Winsize;
    async fn get_theme(&mut self) -> Option<TtyTheme>;
    async fn get_version(&mut self) -> Option<String>;
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

impl DevTty {
    pub async fn open() -> anyhow::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/tty")?;

        let file = AsyncFd::new(file)?;
        let settings = make_raw(&file)?;

        Ok(Self { file, settings })
    }

    async fn query(&self, query: &str) -> anyhow::Result<Vec<u8>> {
        let mut query = query.to_string().into_bytes();
        query.extend_from_slice(b"\x1b[c");
        let mut query = &query[..];
        let mut response = Vec::new();
        let mut buf = [0u8; 1024];

        loop {
            tokio::select! {
                result = self.read(&mut buf) => {
                    let n = result?;
                    response.extend_from_slice(&buf[..n]);

                    if let Some(len) = complete_da_response_len(&response) {
                        response.truncate(len);
                        break;
                    }
                }

                result = self.write(query), if !query.is_empty() => {
                    let n = result?;
                    query = &query[n..];
                }

                _ = time::sleep(Duration::from_millis(QUERY_READ_TIMEOUT)) => {
                    break;
                }
            }
        }

        Ok(response)
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
        let response = self.query(COLORS_QUERY).await.ok()?;
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

    async fn get_version(&mut self) -> Option<String> {
        let response = self.query(XTVERSION_QUERY).await.ok()?;

        if let [b'\x1b', b'P', b'>', b'|', version @ .., b'\x1b', b'\\'] = &response[..] {
            Some(String::from_utf8_lossy(version).to_string())
        } else {
            None
        }
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

impl<T: Tty> FixedSizeTty<T> {
    pub fn new(inner: T, cols: Option<u16>, rows: Option<u16>) -> Self {
        Self { inner, cols, rows }
    }
}

#[async_trait(?Send)]
impl Tty for NullTty {
    fn get_size(&self) -> Winsize {
        Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }

    async fn get_theme(&mut self) -> Option<TtyTheme> {
        None
    }

    async fn get_version(&mut self) -> Option<String> {
        None
    }

    async fn read(&self, _buf: &mut [u8]) -> io::Result<usize> {
        std::future::pending().await
    }

    async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
}

#[async_trait(?Send)]
impl<T: Tty> Tty for FixedSizeTty<T> {
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

    async fn get_theme(&mut self) -> Option<TtyTheme> {
        self.inner.get_theme().await
    }

    async fn get_version(&mut self) -> Option<String> {
        self.inner.get_version().await
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

fn complete_da_response_len(response: &[u8]) -> Option<usize> {
    let mut reversed = response.iter().rev();
    let mut includes_da_response = false;
    let mut da_response_len = 0;

    if let Some(b'c') = reversed.next() {
        da_response_len += 1;

        for b in reversed {
            if *b == b'[' {
                includes_da_response = true;
                break;
            }

            if *b != b';' && *b != b'?' && !b.is_ascii_digit() {
                break;
            }

            da_response_len += 1;
        }
    }

    if includes_da_response {
        Some(response.len() - da_response_len - 2)
    } else {
        None
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

#[cfg(test)]
mod tests {
    use super::{FixedSizeTty, NullTty, Tty};

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
