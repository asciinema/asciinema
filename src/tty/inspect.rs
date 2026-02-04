use rgb::RGB8;
use tokio::time::{self, Duration};

use super::{RawTty, TtyTheme};

const INSPECT_QUERY: &str = concat!(
    "\x1b[>0q",                                                     // XTVERSION
    "\x1b]10;?\x07\x1b]11;?\x07",                                   // fg, bg
    "\x1b]4;0;?\x07\x1b]4;1;?\x07\x1b]4;2;?\x07\x1b]4;3;?\x07",     // palette 0-3
    "\x1b]4;4;?\x07\x1b]4;5;?\x07\x1b]4;6;?\x07\x1b]4;7;?\x07",     // palette 4-7
    "\x1b]4;8;?\x07\x1b]4;9;?\x07\x1b]4;10;?\x07\x1b]4;11;?\x07",   // palette 8-11
    "\x1b]4;12;?\x07\x1b]4;13;?\x07\x1b]4;14;?\x07\x1b]4;15;?\x07", // palette 12-15
    "\x1b[c",                                                       // DA (flush)
);

pub(crate) async fn inspect<T: RawTty + ?Sized>(tty: &T) -> (Option<String>, Option<TtyTheme>) {
    query(tty, INSPECT_QUERY, ReplyParser::new())
        .await
        .unwrap_or_default()
}

enum ParseResult {
    Pending,
    Done,
}

async fn query<T: RawTty + ?Sized>(
    tty: &T,
    query: &str,
    mut parser: ReplyParser,
) -> anyhow::Result<(Option<String>, Option<TtyTheme>)> {
    let mut query = query.as_bytes();
    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            result = tty.read(&mut buf) => {
                let n = result?;

                if let ParseResult::Done = parser.feed(&buf[..n]) {
                    break;
                }
            }

            result = tty.write(query), if !query.is_empty() => {
                let n = result?;
                query = &query[n..];
            }

            _ = time::sleep(Duration::from_millis(1000)) => {
                break;
            }
        }
    }

    Ok(parser.result())
}

struct ReplyParser {
    buf: Vec<u8>,
    fg: Option<RGB8>,
    bg: Option<RGB8>,
    palette: [Option<RGB8>; 16],
    version: Option<String>,
}

impl ReplyParser {
    fn new() -> Self {
        ReplyParser {
            buf: Vec::new(),
            fg: None,
            bg: None,
            palette: [None; 16],
            version: None,
        }
    }
}

impl ReplyParser {
    fn feed(&mut self, chunk: &[u8]) -> ParseResult {
        self.buf.extend_from_slice(chunk);
        let mut i = 0;

        while i < self.buf.len() {
            let buf = &self.buf[i..];

            if let Some(m) = match_seq_prefix(buf, b"\x1b]10;") {
                // OSC 10 (fg color) reply

                let PrefixMatch::Full(rest) = m else {
                    break;
                };

                let Some((end, terminator_len)) = find_osc_end(rest) else {
                    break;
                };

                self.fg = parse_rgb_color(&rest[..end]);
                i += 5 + end + terminator_len;
            } else if let Some(m) = match_seq_prefix(buf, b"\x1b]11;") {
                // OSC 11 (bg color) reply

                let PrefixMatch::Full(rest) = m else {
                    break;
                };

                let Some((end, terminator_len)) = find_osc_end(rest) else {
                    break;
                };

                self.bg = parse_rgb_color(&rest[..end]);
                i += 5 + end + terminator_len;
            } else if let Some(m) = match_seq_prefix(buf, b"\x1b]4;") {
                // OSC 4 (palette entry) reply

                let PrefixMatch::Full(rest) = m else {
                    break;
                };

                let Some((end, terminator_len)) = find_osc_end(rest) else {
                    break;
                };

                for (idx, color) in parse_palette_entries(&rest[..end]) {
                    self.palette[idx] = Some(color);
                }

                i += 4 + end + terminator_len;
            } else if let Some(m) = match_seq_prefix(buf, b"\x1bP") {
                // DCS reply

                let PrefixMatch::Full(rest) = m else {
                    break;
                };

                let Some((end, terminator_len)) = find_dcs_end(rest) else {
                    break;
                };

                // looking for XTVERSION function selector
                if let Some(version) = parse_dcs_reply(&rest[..end], b">|") {
                    self.version = Some(version);
                }

                i += 2 + end + terminator_len;
            } else if let Some(m) = match_seq_prefix(buf, b"\x1b[?") {
                // DEC private reply

                let PrefixMatch::Full(rest) = m else {
                    break;
                };

                let Some((end, terminator)) = find_dec_prv_final(rest) else {
                    break;
                };

                // check for DA
                if terminator == 'c' {
                    // We assume here that the reply order matches the query order. There's no spec
                    // guaranteeing orderly replies, but in practice the replies for the queries we
                    // use here are ordered on all tested terminals. Therefore, if we get a reply
                    // for DA (which is widely supported) it means all the replies for supported
                    // queries already came.
                    return ParseResult::Done;
                }

                i += 3 + end + 1;
            } else {
                i += 1;
            }
        }

        self.buf.drain(..i);

        ParseResult::Pending
    }

    fn result(self) -> (Option<String>, Option<TtyTheme>) {
        let theme = self.build_theme();

        (self.version, theme)
    }

    fn build_theme(&self) -> Option<TtyTheme> {
        let fg = self.fg?;
        let bg = self.bg?;
        let palette = self.palette.iter().flatten().cloned().collect::<Vec<_>>();

        if palette.len() < 16 {
            return None;
        }

        Some(TtyTheme { fg, bg, palette })
    }
}

enum PrefixMatch<'a> {
    Full(&'a [u8]),
    Partial,
}

fn match_seq_prefix<'a>(a: &'a [u8], b: &[u8]) -> Option<PrefixMatch<'a>> {
    if let Some(rest) = a.strip_prefix(b) {
        Some(PrefixMatch::Full(rest))
    } else if b.starts_with(a) {
        Some(PrefixMatch::Partial)
    } else {
        None
    }
}

fn find_osc_end(buf: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;

    while i < buf.len() {
        if buf[i] == 0x07 {
            return Some((i, 1));
        }

        if buf[i] == 0x1b && i + 1 < buf.len() && buf[i + 1] == b'\\' {
            return Some((i, 2));
        }

        i += 1;
    }

    None
}

fn find_dcs_end(buf: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;

    while i + 1 < buf.len() {
        if buf[i] == 0x1b && buf[i + 1] == b'\\' {
            return Some((i, 2));
        }

        i += 1;
    }

    None
}

fn find_dec_prv_final(buf: &[u8]) -> Option<(usize, char)> {
    for (i, byte) in buf.iter().enumerate() {
        if (0x40..=0x7e).contains(byte) {
            return Some((i, *byte as char));
        }

        if !((0x20..=0x3f).contains(byte)) {
            return None;
        }
    }

    None
}

fn parse_dcs_reply(reply: &[u8], prefix: &[u8]) -> Option<String> {
    reply
        .strip_prefix(prefix)
        .map(|value| String::from_utf8_lossy(value).to_string())
}

fn parse_palette_entries(reply: &[u8]) -> Vec<(usize, RGB8)> {
    let mut params = reply.split(|b| *b == b';');
    let mut entries = Vec::new();

    while let Some(idx_bytes) = params.next() {
        let Ok(idx_str) = std::str::from_utf8(idx_bytes) else {
            break;
        };

        let Ok(idx) = idx_str.parse::<u8>() else {
            break;
        };

        let Some(color_bytes) = params.next() else {
            break;
        };

        if idx < 16 {
            if let Some(c) = parse_rgb_color(color_bytes) {
                entries.push((idx as usize, c));
            }
        }
    }

    entries
}

fn parse_rgb_color(rgb: &[u8]) -> Option<RGB8> {
    let rgb = rgb.strip_prefix(b"rgb:")?;
    let mut components = rgb.split(|b| *b == b'/');
    let r_hex = components.next()?;
    let g_hex = components.next()?;
    let b_hex = components.next()?;
    let r = parse_hex_byte(r_hex)?;
    let g = parse_hex_byte(g_hex)?;
    let b = parse_hex_byte(b_hex)?;

    Some(RGB8::new(r, g, b))
}

fn parse_hex_byte(bytes: &[u8]) -> Option<u8> {
    if bytes.len() < 2 {
        return None;
    }

    let hi = hex_value(bytes[0])?;
    let lo = hex_value(bytes[1])?;

    Some((hi << 4) | lo)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use rgb::RGB8;

    use super::{ParseResult, ReplyParser};

    const PALETTE_RESP: &[u8] = concat!(
        "\x1b]4;0;rgb:0000/1111/2222\x07",
        "\x1b]4;1;rgb:3333/4444/5555\x07",
        "\x1b]4;2;rgb:6666/7777/8888\x07",
        "\x1b]4;3;rgb:9999/aaaa/bbbb\x07",
        "\x1b]4;4;rgb:cccc/dddd/eeee\x07",
        "\x1b]4;5;rgb:ffff/0000/1111\x07",
        "\x1b]4;6;rgb:2222/3333/4444\x07",
        "\x1b]4;7;rgb:5555/6666/7777\x07",
        "\x1b]4;8;rgb:8888/9999/aaaa\x07",
        "\x1b]4;9;rgb:bbbb/cccc/dddd\x07",
        "\x1b]4;10;rgb:eeee/ffff/0000\x07",
        "\x1b]4;11;rgb:1111/2222/3333\x07",
        "\x1b]4;12;rgb:4444/5555/6666\x07",
        "\x1b]4;13;rgb:7777/8888/9999\x07",
        "\x1b]4;14;rgb:aaaa/bbbb/cccc\x07",
        "\x1b]4;15;rgb:dddd/eeee/ffff\x07",
    )
    .as_bytes();

    const FG_RESP: &[u8] = b"\x1b]10;rgb:1122/3344/5566\x07";
    const BG_RESP: &[u8] = b"\x1b]11;rgb:7788/99aa/bbcc\x07";
    const DA_RESP: &[u8] = b"\x1b[?1;2c";
    const XTVERSION_RESP: &[u8] = b"\x1bP>|xterm-395\x1b\\";

    fn feed_chunks(chunks: &[&[u8]]) -> (Option<String>, Option<super::TtyTheme>, bool) {
        let mut parser = ReplyParser::new();

        for chunk in chunks {
            if let ParseResult::Done = parser.feed(chunk) {
                let (version, theme) = parser.result();
                return (version, theme, true);
            }
        }

        let (version, theme) = parser.result();

        (version, theme, false)
    }

    #[test]
    fn parse_rgb_color() {
        use super::parse_rgb_color as parse;
        let color = Some(RGB8::new(0xaa, 0xbb, 0xcc));

        assert_eq!(parse(b"rgb:aa11/bb22/cc33"), color);
        assert_eq!(parse(b"rgb:aa11/bb22/cc33\x07"), color);
        assert_eq!(parse(b"rgb:aa11/bb22/cc33\x1b\\"), color);
        assert_eq!(parse(b"rgb:aa11/bb22/cc33.."), color);
        assert_eq!(parse(b"rgb:aa1/bb2/cc3"), color);
        assert_eq!(parse(b"rgb:aa1/bb2/cc3\x07"), color);
        assert_eq!(parse(b"rgb:aa1/bb2/cc3\x1b\\"), color);
        assert_eq!(parse(b"rgb:aa1/bb2/cc3.."), color);
        assert_eq!(parse(b"rgb:aa/bb/cc"), color);
        assert_eq!(parse(b"rgb:aa/bb/cc\x07"), color);
        assert_eq!(parse(b"rgb:aa/bb/cc\x1b\\"), color);
        assert_eq!(parse(b"rgb:aa/bb/cc.."), color);
        assert_eq!(parse(b"rgb:aa11/bb22"), None);
        assert_eq!(parse(b"rgb:xxxx/yyyy/zzzz"), None);
        assert_eq!(parse(b"rgb:xxx/yyy/zzz"), None);
        assert_eq!(parse(b"rgb:xx/yy/zz"), None);
        assert_eq!(parse(b"foo"), None);
        assert_eq!(parse(b""), None);
    }

    #[test]
    fn parse_palette_entries() {
        use super::parse_palette_entries as parse;

        // Valid entries
        let entries = parse(b"0;rgb:0000/1111/2222;1;rgb:3333/4444/5555");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], (0, RGB8::new(0x00, 0x11, 0x22)));
        assert_eq!(entries[1], (1, RGB8::new(0x33, 0x44, 0x55)));

        // Index >= 16 is ignored
        let entries = parse(b"16;rgb:3333/4444/5555");
        assert_eq!(entries.len(), 0);

        // Invalid index stops parsing
        let entries = parse(b"0;rgb:0000/1111/2222;xx;rgb:ffff/eeee/dddd");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], (0, RGB8::new(0x00, 0x11, 0x22)));

        // Empty input
        let entries = parse(b"");
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn parser_version_only() {
        // Just XTVERSION response, no theme
        let (version, theme, done) = feed_chunks(&[XTVERSION_RESP]);

        assert!(!done); // not done because no DA response
        assert!(theme.is_none());
        assert_eq!(version, Some("xterm-395".to_string()));

        // XTVERSION response with DA
        let (version, theme, done) = feed_chunks(&[XTVERSION_RESP, DA_RESP]);
        assert!(done);
        assert!(theme.is_none());
        assert_eq!(version, Some("xterm-395".to_string()));

        // No XTVERSION response at all
        let (version, theme, done) = feed_chunks(&[b""]);
        assert!(!done);
        assert!(theme.is_none());
        assert!(version.is_none());
    }

    #[test]
    fn parser_theme_only() {
        // Theme with out-of-order palette and mixed terminators (BEL and ST)
        let (version, theme, done) = feed_chunks(&[
            b"\x1b]4;1;rgb:3333/4444/5555\x07",   // index 1 first
            b"\x1b]4;0;rgb:0000/1111/2222\x1b\\", // index 0 with ST terminator
            b"\x1b]4;2;rgb:6666/7777/8888\x07",
            b"\x1b]4;3;rgb:9999/aaaa/bbbb\x07",
            b"\x1b]4;4;rgb:cccc/dddd/eeee\x07",
            b"\x1b]4;5;rgb:ffff/0000/1111\x07",
            b"\x1b]4;6;rgb:2222/3333/4444\x07",
            b"\x1b]4;7;rgb:5555/6666/7777\x07",
            b"\x1b]4;8;rgb:8888/9999/aaaa\x07",
            b"\x1b]4;9;rgb:bbbb/cccc/dddd\x07",
            b"\x1b]4;10;rgb:eeee/ffff/0000\x07",
            b"\x1b]4;11;rgb:1111/2222/3333\x07",
            b"\x1b]4;12;rgb:4444/5555/6666\x07",
            b"\x1b]4;13;rgb:7777/8888/9999\x07",
            b"\x1b]4;14;rgb:aaaa/bbbb/cccc\x07",
            b"\x1b]4;15;rgb:dddd/eeee/ffff\x07",
            b"\x1b]10;rgb:1122/3344/5566\x1b\\", // fg with ST
            BG_RESP,
            DA_RESP,
        ]);

        let theme = theme.expect("theme should be present");

        assert!(done);
        assert!(version.is_none());
        assert_eq!(theme.fg, RGB8::new(0x11, 0x33, 0x55));
        assert_eq!(theme.bg, RGB8::new(0x77, 0x99, 0xbb));
        assert_eq!(theme.palette.len(), 16);
        assert_eq!(theme.palette[0], RGB8::new(0x00, 0x11, 0x22));
        assert_eq!(theme.palette[1], RGB8::new(0x33, 0x44, 0x55));
        assert_eq!(theme.palette[15], RGB8::new(0xdd, 0xee, 0xff));
    }

    #[test]
    fn parser_version_and_theme() {
        // The happy path: both version and theme in one response
        let (version, theme, done) = feed_chunks(&[
            b"\x1bP>|foot(1.22.0)\x1b\\",
            PALETTE_RESP,
            FG_RESP,
            BG_RESP,
            DA_RESP,
        ]);

        let theme = theme.expect("theme should be present");

        assert!(done);
        assert_eq!(version, Some("foot(1.22.0)".to_string()));
        assert_eq!(theme.fg, RGB8::new(0x11, 0x33, 0x55));
        assert_eq!(theme.bg, RGB8::new(0x77, 0x99, 0xbb));
        assert_eq!(theme.palette.len(), 16);
    }

    #[test]
    fn parser_packed_palette() {
        // OSC 4 with multiple colors per response
        let (version, theme, done) = feed_chunks(&[
            b"\x1b]4;0;rgb:0000/1111/2222;1;rgb:3333/4444/5555;2;rgb:6666/7777/8888\x07",
            b"\x1b]4;3;rgb:9999/aaaa/bbbb;4;rgb:cccc/dddd/eeee;5;rgb:ffff/0000/1111\x07",
            b"\x1b]4;6;rgb:2222/3333/4444;7;rgb:5555/6666/7777;8;rgb:8888/9999/aaaa\x07",
            b"\x1b]4;9;rgb:bbbb/cccc/dddd;10;rgb:eeee/ffff/0000;11;rgb:1111/2222/3333\x07",
            b"\x1b]4;12;rgb:4444/5555/6666;13;rgb:7777/8888/9999;14;rgb:aaaa/bbbb/cccc;15;rgb:dddd/eeee/ffff\x07",
            FG_RESP,
            BG_RESP,
            DA_RESP,
        ]);

        let theme = theme.expect("theme should be present");

        assert!(done);
        assert!(version.is_none());
        assert_eq!(theme.fg, RGB8::new(0x11, 0x33, 0x55));
        assert_eq!(theme.bg, RGB8::new(0x77, 0x99, 0xbb));
        assert_eq!(theme.palette.len(), 16);
        assert_eq!(theme.palette[0], RGB8::new(0x00, 0x11, 0x22));
        assert_eq!(theme.palette[15], RGB8::new(0xdd, 0xee, 0xff));
    }

    #[test]
    fn parser_chunked_response() {
        // Response split across multiple feed() calls at awkward boundaries
        let chunks = [
            b"\x1bP>|xterm-".as_slice(), // version split mid-string
            b"395\x1b".as_slice(),       // escape without backslash
            b"\\\x1b]10;rgb:1122/3344/5566\x1b".as_slice(), // OSC 10 with partial ST
            b"\\\x1b]4;0;rgb:0000/1111/2222;".as_slice(), // packed palette, split mid-entry
            b"1;rgb:3333/4444/5555;2;rgb:6666/7777/8888\x07".as_slice(),
            b"\x1b]4;3;rgb:9999/aaaa/bbbb\x07".as_slice(),
            b"\x1b]4;4;rgb:cccc/dd".as_slice(), // color split mid-value
            b"dd/eeee\x07".as_slice(),
            b"\x1b]4;5;rgb:ffff/0000/1111\x07".as_slice(),
            b"\x1b]4;6;rgb:2222/3333/4444\x07".as_slice(),
            b"\x1b]4;7;rgb:5555/6666/7777\x07".as_slice(),
            b"\x1b]4;8;rgb:8888/9999/aaaa\x07".as_slice(),
            b"\x1b]4;9;rgb:bbbb/cccc/dddd\x07".as_slice(),
            b"\x1b]4;10;rgb:eeee/ffff/0000\x07".as_slice(),
            b"\x1b]4;11;rgb:1111/2222/3333\x07".as_slice(),
            b"\x1b]4;12;rgb:4444/5555/6666\x07".as_slice(),
            b"\x1b]4;13;rgb:7777/8888/9999\x07".as_slice(),
            b"\x1b]4;14;rgb:aaaa/bbbb/cccc\x07".as_slice(),
            b"\x1b]4;15;rgb:dddd/eeee/ffff\x07".as_slice(),
            b"\x1b]11;rgb:7788/99aa/bbcc\x07".as_slice(),
            DA_RESP,
        ];

        let (version, theme, done) = feed_chunks(&chunks);
        let theme = theme.expect("theme should be present");

        assert!(done);
        assert_eq!(version, Some("xterm-395".to_string()));
        assert_eq!(theme.fg, RGB8::new(0x11, 0x33, 0x55));
        assert_eq!(theme.bg, RGB8::new(0x77, 0x99, 0xbb));
        assert_eq!(theme.palette.len(), 16);
    }

    #[test]
    fn parser_garbage_ignored() {
        // Unknown sequences between valid ones should be skipped
        let (version, theme, done) = feed_chunks(&[
            b"\x1b[?25h",            // DECSET (ignored)
            b"\x1bP>|ghostty\x1b\\", // version
            b"\x1b[>0;1;2c",         // DA2 response (ignored)
            b"random garbage",       // plain text (ignored)
            PALETTE_RESP,
            FG_RESP,
            BG_RESP,
            DA_RESP,
        ]);

        let theme = theme.expect("theme should be present");

        assert!(done);
        assert_eq!(version, Some("ghostty".to_string()));
        assert_eq!(theme.fg, RGB8::new(0x11, 0x33, 0x55));
        assert_eq!(theme.bg, RGB8::new(0x77, 0x99, 0xbb));
        assert_eq!(theme.palette.len(), 16);
    }

    #[test]
    fn parser_incomplete_theme() {
        // Missing fg -> theme is None
        let (_version, theme, done) = feed_chunks(&[
            PALETTE_RESP,
            BG_RESP, // only bg, no fg
            DA_RESP,
        ]);

        assert!(done);
        assert!(theme.is_none());

        // Missing palette -> theme is None
        let (_version, theme, done) = feed_chunks(&[FG_RESP, BG_RESP, DA_RESP]);
        assert!(done);
        assert!(theme.is_none());

        // Partial palette (only 8 colors) -> theme is None
        let partial_palette = concat!(
            "\x1b]4;0;rgb:0000/1111/2222\x07",
            "\x1b]4;1;rgb:3333/4444/5555\x07",
            "\x1b]4;2;rgb:6666/7777/8888\x07",
            "\x1b]4;3;rgb:9999/aaaa/bbbb\x07",
            "\x1b]4;4;rgb:cccc/dddd/eeee\x07",
            "\x1b]4;5;rgb:ffff/0000/1111\x07",
            "\x1b]4;6;rgb:2222/3333/4444\x07",
            "\x1b]4;7;rgb:5555/6666/7777\x07",
        )
        .as_bytes();

        let (_version, theme, done) = feed_chunks(&[partial_palette, FG_RESP, BG_RESP, DA_RESP]);

        assert!(done);
        assert!(theme.is_none());
    }
}
