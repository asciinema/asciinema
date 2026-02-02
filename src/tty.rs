use std::os::fd::AsFd;

use async_trait::async_trait;
use nix::libc;
use nix::pty::Winsize;
use nix::sys::termios::{self, SetArg};
use rgb::RGB8;
use tokio::io;
use tokio::time::{self, Duration};

const QUERY_READ_TIMEOUT: u64 = 1000;
const THEME_QUERY: &str = "\x1b]10;?\x07\x1b]11;?\x07\x1b]4;0;?\x07\x1b]4;1;?\x07\x1b]4;2;?\x07\x1b]4;3;?\x07\x1b]4;4;?\x07\x1b]4;5;?\x07\x1b]4;6;?\x07\x1b]4;7;?\x07\x1b]4;8;?\x07\x1b]4;9;?\x07\x1b]4;10;?\x07\x1b]4;11;?\x07\x1b]4;12;?\x07\x1b]4;13;?\x07\x1b]4;14;?\x07\x1b]4;15;?\x07";
const XTVERSION_QUERY: &str = "\x1b[>0q";

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

pub(crate) async fn query_theme<T: RawTty + ?Sized>(tty: &T) -> Option<TtyTheme> {
    parse_theme_response(&query(tty, THEME_QUERY).await.ok()?)
}

pub(crate) async fn query_version<T: RawTty + ?Sized>(tty: &T) -> Option<String> {
    parse_version_response(&query(tty, XTVERSION_QUERY).await.ok()?)
}

async fn query<T: RawTty + ?Sized>(tty: &T, query: &str) -> anyhow::Result<Vec<u8>> {
    let mut query = query.to_string().into_bytes();
    query.extend_from_slice(b"\x1b[c");
    let mut query = &query[..];
    let mut response = Vec::new();
    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            result = tty.read(&mut buf) => {
                let n = result?;
                response.extend_from_slice(&buf[..n]);

                if let Some(len) = complete_da_response_len(&response) {
                    response.truncate(len);
                    break;
                }
            }

            result = tty.write(query), if !query.is_empty() => {
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

fn parse_theme_response(response: &[u8]) -> Option<TtyTheme> {
    let mut fg = None;
    let mut bg = None;
    let mut palette: [Option<RGB8>; 16] = [None; 16];
    let response = String::from_utf8_lossy(response);
    let mut rest = &response[..];

    loop {
        let Some(seq_start) = rest.find("\x1b]") else {
            break;
        };

        let rest_after_start = &rest[seq_start + 2..];
        let bel_end = rest_after_start.find("\x07");
        let st_end = rest_after_start.find("\x1b\\");

        let Some(seq_end) = (match (bel_end, st_end) {
            (Some(bel), Some(st)) => Some(bel.min(st)),
            (Some(bel), None) => Some(bel),
            (None, Some(st)) => Some(st),
            (None, None) => None,
        }) else {
            break;
        };

        let reply = &rest_after_start[..seq_end];

        if rest_after_start[seq_end..].starts_with("\x07") {
            rest = &rest_after_start[seq_end + 1..];
        } else {
            rest = &rest_after_start[seq_end + 2..];
        };

        let mut params = reply.split(';');

        let Some(p1) = params.next() else {
            continue;
        };

        let Some(p2) = params.next() else {
            continue;
        };

        match p1 {
            "10" => {
                if let Some(c) = p2.strip_prefix("rgb:") {
                    fg = parse_color(c);
                }
            }

            "11" => {
                if let Some(c) = p2.strip_prefix("rgb:") {
                    bg = parse_color(c);
                }
            }

            "4" => {
                let Some(i) = p2.parse::<u8>().ok() else {
                    continue;
                };

                let Some(p3) = params.next() else {
                    continue;
                };

                if i < 16 {
                    if let Some(c) = p3.strip_prefix("rgb:") {
                        palette[i as usize] = parse_color(c);
                    }
                }
            }

            _ => {
                continue;
            }
        }
    }

    let fg = fg?;
    let bg = bg?;
    let palette = palette.into_iter().flatten().collect::<Vec<_>>();

    if palette.len() < 16 {
        return None;
    }

    Some(TtyTheme { fg, bg, palette })
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

fn parse_version_response(response: &[u8]) -> Option<String> {
    if let [b'\x1b', b'P', b'>', b'|', version @ .., b'\x1b', b'\\'] = response {
        Some(String::from_utf8_lossy(version).to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{FixedSizeTty, NullTty, RawTty};

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

    #[test]
    fn parse_theme_response_ok() {
        let response = concat!(
            "\x1b]4;1;rgb:3333/4444/5555\x07",
            "\x1b]11;rgb:7788/99aa/bbcc\x07",
            "\x1b]4;3;rgb:9999/aaaa/bbbb\x07",
            "\x1b]4;2;rgb:6666/7777/8888\x1b\\",
            "\x1b]4;4;rgb:cccc/dddd/eeee\x07",
            "\x1b]4;0;rgb:0000/1111/2222\x07",
            "\x1b]4;6;rgb:2222/3333/4444\x07",
            "\x1b]4;5;rgb:ffff/0000/1111\x07",
            "\x1b]4;7;rgb:5555/6666/7777\x07",
            "\x1b]4;10;rgb:eeee/ffff/0000\x07",
            "\x1b]4;8;rgb:8888/9999/aaaa\x07",
            "\x1b]4;9;rgb:bbbb/cccc/dddd\x07",
            "\x1b]4;11;rgb:1111/2222/3333\x07",
            "\x1b]4;14;rgb:aaaa/bbbb/cccc\x07",
            "\x1b]4;13;rgb:7777/8888/9999\x07",
            "\x1b]10;rgb:1122/3344/5566\x1b\\",
            "\x1b]4;15;rgb:dddd/eeee/ffff\x07",
            "\x1b]4;12;rgb:4444/5555/6666\x07",
        )
        .as_bytes();

        let theme = super::parse_theme_response(response).expect("theme");
        assert_eq!(theme.fg, RGB8::new(0x11, 0x33, 0x55));
        assert_eq!(theme.bg, RGB8::new(0x77, 0x99, 0xbb));
        assert_eq!(theme.palette.len(), 16);
        assert_eq!(theme.palette[0], RGB8::new(0x00, 0x11, 0x22));
        assert_eq!(theme.palette[15], RGB8::new(0xdd, 0xee, 0xff));
    }

    #[test]
    fn parse_theme_response_missing_colors() {
        let response = b"\x1b]10;rgb:1122/3344/5566\x07";
        assert!(super::parse_theme_response(response).is_none());
    }

    #[test]
    fn parse_version_response_ok() {
        let response = b"\x1bP>|xterm-395\x1b\\";
        let version = super::parse_version_response(response).expect("version");
        assert_eq!(version, "xterm-395");
    }

    #[test]
    fn parse_version_response_invalid() {
        let response = b"\x1bP>|xterm-395\x07";
        assert!(super::parse_version_response(response).is_none());
    }
}
