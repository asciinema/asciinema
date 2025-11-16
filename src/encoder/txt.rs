use std::time::Duration;

use avt::util::TextCollector;

// add chrono + regex + once_cell
use chrono::{DateTime, NaiveDateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::asciicast::{Event, EventData, Header};

static CSI_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").expect("valid regex"));
static OSC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\x1B\][^\x07\x1B]*(?:\x07|\x1B\\)").expect("valid regex"));

pub struct TextEncoder {
    collector: Option<TextCollector>,
    timestamp: bool,
    last_time: Option<Duration>,

    // used only when timestamp == true: simple visible-text buffer and naive ANSI strip
    buf: String,

    // base unix timestamp from the cast header (seconds). Used to produce absolute UTC times.
    base_ts: Option<i64>,
}

impl TextEncoder {
    pub fn new(timestamp: bool) -> Self {
        TextEncoder {
            collector: None,
            timestamp,
            last_time: None,
            buf: String::new(),
            base_ts: None,
        }
    }
}

// replace the previous strip_ansi with a regex-driven approach
fn strip_ansi(s: &str) -> String {
    // remove CSI and OSC sequences
    let s = CSI_RE.replace_all(s, "");
    let s = OSC_RE.replace_all(&s, "");
    // drop control characters except newline and carriage return and printable
    s.chars()
        .filter(|&c| c == '\r' || c == '\n' || (c >= ' ' && c != '\x7f'))
        .collect()
}

impl super::Encoder for TextEncoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        // capture base timestamp if available in header
        // header.timestamp is an Option<u64>; map to Option<i64>
        self.base_ts = header.timestamp.map(|t| t as i64);

        if self.timestamp {
            // don't create the TextCollector when timestamping; use simple buffer
            self.collector = None;
            self.buf.clear();
        } else {
            let vt = avt::Vt::builder()
                .size(header.term_cols as usize, header.term_rows as usize)
                .scrollback_limit(100)
                .build();

            self.collector = Some(TextCollector::new(vt));
        }

        Vec::new()
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        use EventData::*;

        // record last time for flush (used only for remaining partial lines)
        self.last_time = Some(event.time);

        match &event.data {
            Output(data) => {
                if self.timestamp {
                    // strip ANSI and append to visible buffer, split on '\n'
                    let visible = strip_ansi(data);
                    self.buf.push_str(&visible);

                    let mut out = Vec::new();

                    while let Some(pos) = self.buf.find('\n') {
                        let mut line = self.buf.drain(..=pos).collect::<String>();
                        // remove trailing '\n'
                        if line.ends_with('\n') {
                            line.pop();
                        }
                        // remove trailing '\r' if present
                        if line.ends_with('\r') {
                            line.pop();
                        }

                        // prefix timestamp (absolute UTC if base_ts present, otherwise relative seconds)
                        let ts_prefix = format_timestamp(self.base_ts, event.time);
                        out.extend_from_slice(ts_prefix.as_bytes());

                        out.extend_from_slice(line.as_bytes());
                        out.push(b'\n');
                    }

                    out
                } else {
                    // original behavior: let TextCollector render visible lines (no timestamps)
                    let lines = self
                        .collector
                        .as_mut()
                        .unwrap()
                        .feed_str(data)
                        .into_iter();
                    text_lines_to_bytes(lines, None)
                }
            }

            Resize(cols, rows) => {
                if self.timestamp {
                    // ignore resize in simple timestamping mode
                    Vec::new()
                } else {
                    let lines = self
                        .collector
                        .as_mut()
                        .unwrap()
                        .resize(*cols, *rows)
                        .into_iter();
                    text_lines_to_bytes(lines, None)
                }
            }

            _ => Vec::new(),
        }
    }

    fn flush(&mut self) -> Vec<u8> {
        if self.timestamp {
            // emit any leftover partial line with last_time (if any)
            let mut out = Vec::new();
            if !self.buf.is_empty() {
                let line = self.buf.trim_end_matches(&['\r', '\n'][..]).to_owned();
                let ts_prefix = format_timestamp(self.base_ts, self.last_time.unwrap_or_default());
                out.extend_from_slice(ts_prefix.as_bytes());
                out.extend_from_slice(line.as_bytes());
                out.push(b'\n');
                self.buf.clear();
            }
            out
        } else {
            // Only use last_time for the leftover partial lines emitted at flush.
            let ts = if self.timestamp { self.last_time } else { None };

            text_lines_to_bytes(self.collector.take().unwrap().flush().into_iter(), ts)
        }
    }
}

fn format_timestamp(base_ts: Option<i64>, dur: Duration) -> String {
    if let Some(base) = base_ts {
        // compute absolute timestamp = base + dur
        let secs = dur.as_secs() as i64;
        let nanos = dur.subsec_nanos();
        let abs_secs = base.saturating_add(secs);
        // clamp nanos to u32 for NaiveDateTime
        let ndt = NaiveDateTime::from_timestamp_opt(abs_secs, nanos).unwrap_or_else(|| {
            // fallback to epoch if conversion fails
            NaiveDateTime::from_timestamp(0, 0)
        });
        let dt: DateTime<Utc> = DateTime::from_utc(ndt, Utc);
        // RFC3339 with milliseconds
        format!("{} ", dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    } else {
        // relative seconds fallback
        format!("{:.3}s ", dur.as_secs_f64())
    }
}

fn text_lines_to_bytes<S: AsRef<str>>(lines: impl Iterator<Item = S>, ts: Option<Duration>) -> Vec<u8> {
    lines.fold(Vec::new(), |mut bytes, line| {
        if let Some(t) = ts {
            // rough timestamp in seconds with millisecond precision
            let prefix = format!("{:.3}s ", t.as_secs_f64());
            bytes.extend_from_slice(prefix.as_bytes());
        }

        bytes.extend_from_slice(line.as_ref().as_bytes());
        bytes.push(b'\n');

        bytes
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::TextEncoder;
    use crate::asciicast::{Event, Header};
    use crate::encoder::Encoder;

    #[test]
    fn encoder() {
        // default: no timestamps
        let mut enc = TextEncoder::new(false);

        let header = Header {
            term_cols: 3,
            term_rows: 1,
            ..Default::default()
        };

        assert!(enc.header(&header).is_empty());

        assert!(enc
            .event(Event::output(
                Duration::from_micros(0),
                "he\x1b[1mllo\r\n".to_owned()
            ))
            .is_empty());

        assert!(enc
            .event(Event::output(
                Duration::from_micros(1),
                "world\r\n".to_owned()
            ))
            .is_empty());

        assert_eq!(enc.flush(), "hello\nworld\n".as_bytes());
    }
}
