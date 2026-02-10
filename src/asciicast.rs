mod util;
mod v1;
mod v2;
mod v3;

use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::tty::TtyTheme;
pub use v2::V2Encoder;
pub use v3::V3Encoder;

pub struct Asciicast<'a> {
    pub version: Version,
    pub header: Header,
    pub events: Box<dyn Iterator<Item = Result<Event>> + Send + 'a>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Version {
    One,
    Two,
    Three,
}

pub struct Header {
    pub term_cols: u16,
    pub term_rows: u16,
    pub term_type: Option<String>,
    pub term_version: Option<String>,
    pub term_theme: Option<TtyTheme>,
    pub timestamp: Option<u64>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

pub struct Event {
    pub time: Duration,
    pub data: EventData,
}

pub enum EventData {
    Output(String),
    Input(String),
    Resize(u16, u16),
    Marker(String),
    Exit(i32),
    Other(char, String),
}

pub trait Encoder {
    fn header(&mut self, header: &Header) -> Vec<u8>;
    fn event(&mut self, event: &Event) -> Vec<u8>;
}

impl PartialEq<u8> for Version {
    fn eq(&self, other: &u8) -> bool {
        matches!(
            (self, other),
            (Version::One, 1) | (Version::Two, 2) | (Version::Three, 3)
        )
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::One => write!(f, "1"),
            Version::Two => write!(f, "2"),
            Version::Three => write!(f, "3"),
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            term_cols: 80,
            term_rows: 24,
            term_type: None,
            term_version: None,
            term_theme: None,
            timestamp: None,
            idle_time_limit: None,
            command: None,
            title: None,
            env: None,
        }
    }
}

impl Encoder for V2Encoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        self.header(header)
    }

    fn event(&mut self, event: &Event) -> Vec<u8> {
        self.event(event)
    }
}

impl Encoder for V3Encoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        self.header(header)
    }

    fn event(&mut self, event: &Event) -> Vec<u8> {
        self.event(event)
    }
}

pub fn open_from_path<S: AsRef<Path>>(path: S) -> Result<Asciicast<'static>> {
    fs::File::open(&path)
        .map(io::BufReader::new)
        .map_err(|e| anyhow!(e))
        .and_then(open)
        .map_err(|e| anyhow!("can't open {}: {}", path.as_ref().to_string_lossy(), e))
}

pub fn open<'a, R: BufRead + Send + 'a>(reader: R) -> Result<Asciicast<'a>> {
    let mut lines = reader.lines();
    let first_line = lines.next().ok_or(anyhow!("empty file"))??;

    if let Ok(parser) = v3::open(&first_line) {
        Ok(parser.parse(lines))
    } else if let Ok(parser) = v2::open(&first_line) {
        Ok(parser.parse(lines))
    } else {
        let json = std::iter::once(Ok(first_line))
            .chain(lines)
            .collect::<io::Result<String>>()?;

        v1::load(json).map_err(|_| anyhow!("not a v1, v2, v3 asciicast file"))
    }
}

pub fn get_duration<S: AsRef<Path>>(path: S) -> Result<Duration> {
    let Asciicast { events, .. } = open_from_path(path)?;
    let time = events
        .last()
        .map_or(Ok(Duration::from_micros(0)), |e| e.map(|e| e.time))?;

    Ok(time)
}

impl Event {
    pub fn output(time: Duration, text: String) -> Self {
        Event {
            time,
            data: EventData::Output(text),
        }
    }

    pub fn input(time: Duration, text: String) -> Self {
        Event {
            time,
            data: EventData::Input(text),
        }
    }

    pub fn resize(time: Duration, size: (u16, u16)) -> Self {
        Event {
            time,
            data: EventData::Resize(size.0, size.1),
        }
    }

    pub fn marker(time: Duration, label: String) -> Self {
        Event {
            time,
            data: EventData::Marker(label),
        }
    }

    pub fn exit(time: Duration, status: i32) -> Self {
        Event {
            time,
            data: EventData::Exit(status),
        }
    }
}

pub fn limit_idle_time(
    events: impl Iterator<Item = Result<Event>> + Send,
    limit: f64,
) -> impl Iterator<Item = Result<Event>> + Send {
    let limit = Duration::from_micros((limit * 1_000_000.0) as u64);
    let mut prev_time = Duration::from_micros(0);
    let mut offset = Duration::from_micros(0);

    events.map(move |event| {
        event.map(|event| {
            let delay = event.time - prev_time;

            if delay > limit {
                offset += delay - limit;
            }

            prev_time = event.time;
            let time = event.time - offset;

            Event { time, ..event }
        })
    })
}

pub fn accelerate(
    events: impl Iterator<Item = Result<Event>> + Send,
    speed: f64,
) -> impl Iterator<Item = Result<Event>> + Send {
    events.map(move |event| {
        event.map(|event| {
            let time = event.time.div_f64(speed);

            Event { time, ..event }
        })
    })
}

pub fn encoder(version: Version) -> Option<Box<dyn Encoder>> {
    match version {
        Version::One => None,
        Version::Two => Some(Box::new(V2Encoder::new(Duration::from_micros(0)))),
        Version::Three => Some(Box::new(V3Encoder::new())),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use anyhow::Result;
    use rgb::RGB8;

    use super::{Asciicast, Event, EventData, Header, V2Encoder};
    use crate::tty::TtyTheme;

    #[test]
    fn open_v1_minimal() {
        let Asciicast {
            version,
            header,
            events,
        } = super::open_from_path("tests/casts/minimal-v1.json").unwrap();

        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(version, 1);
        assert_eq!((header.term_cols, header.term_rows), (100, 50));
        assert!(header.term_theme.is_none());

        assert_eq!(events[0].time, Duration::from_micros(1230000));
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "hello"));
    }

    #[test]
    fn open_v1_full() {
        let Asciicast {
            version,
            header,
            events,
        } = super::open_from_path("tests/casts/full-v1.json").unwrap();
        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(version, 1);
        assert_eq!((header.term_cols, header.term_rows), (100, 50));

        let mut expected_env = HashMap::new();
        expected_env.insert("SHELL".to_owned(), "/bin/bash".to_owned());
        expected_env.insert("TERM".to_owned(), "xterm-256color".to_owned());
        assert_eq!(header.env.unwrap(), expected_env);

        assert_eq!(events[0].time, Duration::from_micros(1));
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "ż"));

        assert_eq!(events[1].time, Duration::from_micros(10000001));
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "ółć"));

        assert_eq!(events[2].time, Duration::from_micros(10500001));
        assert!(matches!(events[2].data, EventData::Output(ref s) if s == "\r\n"));
    }

    #[test]
    fn open_v1_with_nulls_in_header() {
        let Asciicast {
            version, header, ..
        } = super::open_from_path("tests/casts/nulls-v1.json").unwrap();
        assert_eq!(version, 1);

        let mut expected_env = HashMap::new();
        expected_env.insert("SHELL".to_owned(), "/bin/bash".to_owned());
        assert_eq!(header.env.unwrap(), expected_env);
    }

    #[test]
    fn open_v2_minimal() {
        let Asciicast {
            version,
            header,
            events,
        } = super::open_from_path("tests/casts/minimal-v2.cast").unwrap();

        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(version, 2);
        assert_eq!((header.term_cols, header.term_rows), (100, 50));
        assert!(header.term_theme.is_none());

        assert_eq!(events[0].time, Duration::from_micros(1230000));
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "hello"));
    }

    #[test]
    fn open_v2_full() {
        let Asciicast {
            version,
            header,
            events,
        } = super::open_from_path("tests/casts/full-v2.cast").unwrap();
        let events = events.take(5).collect::<Result<Vec<Event>>>().unwrap();
        let theme = header.term_theme.unwrap();

        assert_eq!(version, 2);
        assert_eq!((header.term_cols, header.term_rows), (100, 50));
        assert_eq!(header.timestamp, Some(1509091818));
        assert_eq!(theme.fg, RGB8::new(0, 0, 0));
        assert_eq!(theme.bg, RGB8::new(0xff, 0xff, 0xff));
        assert_eq!(theme.palette[0], RGB8::new(0x24, 0x1f, 0x31));

        let mut expected_env = HashMap::new();
        expected_env.insert("SHELL".to_owned(), "/bin/bash".to_owned());
        expected_env.insert("TERM".to_owned(), "xterm-256color".to_owned());
        assert_eq!(header.env.unwrap(), expected_env);

        assert_eq!(events[0].time, Duration::from_micros(1));
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "ż"));

        assert_eq!(events[1].time, Duration::from_micros(1_000_000));
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "ółć"));

        assert_eq!(events[2].time, Duration::from_micros(2_300_000));
        assert!(matches!(events[2].data, EventData::Input(ref s) if s == "\n"));

        assert_eq!(events[3].time, Duration::from_micros(5_600_001));

        assert!(
            matches!(events[3].data, EventData::Resize(ref cols, ref rows) if *cols == 80 && *rows == 40)
        );

        assert_eq!(events[4].time, Duration::from_micros(10_500_000));
        assert!(matches!(events[4].data, EventData::Output(ref s) if s == "\r\n"));
    }

    #[test]
    fn open_v2_with_nulls_in_header() {
        let Asciicast {
            version, header, ..
        } = super::open_from_path("tests/casts/nulls-v2.cast").unwrap();
        assert_eq!(version, 2);

        let mut expected_env = HashMap::new();
        expected_env.insert("SHELL".to_owned(), "/bin/bash".to_owned());
        assert_eq!(header.env.unwrap(), expected_env);
    }

    #[test]
    fn open_v3_minimal() {
        let Asciicast {
            version,
            header,
            events,
        } = super::open_from_path("tests/casts/minimal-v3.cast").unwrap();

        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(version, 3);
        assert_eq!((header.term_cols, header.term_rows), (100, 50));
        assert!(header.term_theme.is_none());

        assert_eq!(events[0].time, Duration::from_micros(1230000));
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "hello"));
    }

    #[test]
    fn open_v3_full() {
        let Asciicast {
            version,
            header,
            events,
        } = super::open_from_path("tests/casts/full-v3.cast").unwrap();
        let events = events.take(5).collect::<Result<Vec<Event>>>().unwrap();
        let theme = header.term_theme.unwrap();

        assert_eq!(version, 3);
        assert_eq!((header.term_cols, header.term_rows), (100, 50));
        assert_eq!(header.timestamp, Some(1509091818));
        assert_eq!(theme.fg, RGB8::new(0, 0, 0));
        assert_eq!(theme.bg, RGB8::new(0xff, 0xff, 0xff));
        assert_eq!(theme.palette[0], RGB8::new(0x24, 0x1f, 0x31));

        assert_eq!(events[0].time, Duration::from_micros(1));
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "ż"));

        assert_eq!(events[1].time, Duration::from_micros(1_000_001));
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "ółć"));

        assert_eq!(events[2].time, Duration::from_micros(1_300_001));
        assert!(matches!(events[2].data, EventData::Input(ref s) if s == "\n"));

        assert_eq!(events[3].time, Duration::from_micros(2_900_002));

        assert!(
            matches!(events[3].data, EventData::Resize(ref cols, ref rows) if *cols == 80 && *rows == 40)
        );

        assert_eq!(events[4].time, Duration::from_micros(13_400_002));
        assert!(matches!(events[4].data, EventData::Output(ref s) if s == "\r\n"));
    }

    #[test]
    fn encoder() {
        let mut data = Vec::new();
        let header = Header::default();
        let mut enc = V2Encoder::new(Duration::from_micros(0));
        data.extend(enc.header(&header));
        data.extend(enc.event(&Event::output(
            Duration::from_micros(1000000),
            "hello\r\n".to_owned(),
        )));

        let mut enc = V2Encoder::new(Duration::from_micros(1000001));
        data.extend(enc.event(&Event::output(
            Duration::from_micros(1000001),
            "world".to_owned(),
        )));
        data.extend(enc.event(&Event::input(
            Duration::from_micros(2000002),
            " ".to_owned(),
        )));
        data.extend(enc.event(&Event::resize(Duration::from_micros(3000003), (100, 40))));
        data.extend(enc.event(&Event::output(
            Duration::from_micros(4000004),
            "żółć".to_owned(),
        )));

        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert!(lines[0]["timestamp"].is_null());
        assert_eq!(lines[1][0], 1.000000);
        assert_eq!(lines[1][1], "o");
        assert_eq!(lines[1][2], "hello\r\n");
        assert_eq!(lines[2][0], 2.000002);
        assert_eq!(lines[2][1], "o");
        assert_eq!(lines[2][2], "world");
        assert_eq!(lines[3][0], 3.000003);
        assert_eq!(lines[3][1], "i");
        assert_eq!(lines[3][2], " ");
        assert_eq!(lines[4][0], 4.000004);
        assert_eq!(lines[4][1], "r");
        assert_eq!(lines[4][2], "100x40");
        assert_eq!(lines[5][0], 5.000005);
        assert_eq!(lines[5][1], "o");
        assert_eq!(lines[5][2], "żółć");
    }

    #[test]
    fn header_encoding() {
        let mut enc = V2Encoder::new(Duration::from_micros(0));
        let mut env = HashMap::new();
        env.insert("SHELL".to_owned(), "/usr/bin/fish".to_owned());
        env.insert("TERM".to_owned(), "xterm256-color".to_owned());

        let tty_theme = TtyTheme {
            fg: RGB8::new(0, 1, 2),
            bg: RGB8::new(0, 100, 200),
            palette: vec![
                RGB8::new(0, 0, 0),
                RGB8::new(10, 11, 12),
                RGB8::new(20, 21, 22),
                RGB8::new(30, 31, 32),
                RGB8::new(40, 41, 42),
                RGB8::new(50, 51, 52),
                RGB8::new(60, 61, 62),
                RGB8::new(70, 71, 72),
                RGB8::new(80, 81, 82),
                RGB8::new(90, 91, 92),
                RGB8::new(100, 101, 102),
                RGB8::new(110, 111, 112),
                RGB8::new(120, 121, 122),
                RGB8::new(130, 131, 132),
                RGB8::new(140, 141, 142),
                RGB8::new(150, 151, 152),
            ],
        };

        let header = Header {
            timestamp: Some(1704719152),
            idle_time_limit: Some(1.5),
            command: Some("/bin/bash".to_owned()),
            title: Some("Demo".to_owned()),
            env: Some(env),
            term_theme: Some(tty_theme),
            ..Default::default()
        };

        let data = enc.header(&header);
        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert_eq!(lines[0]["timestamp"], 1704719152);
        assert_eq!(lines[0]["idle_time_limit"], 1.5);
        assert_eq!(lines[0]["command"], "/bin/bash");
        assert_eq!(lines[0]["title"], "Demo");
        assert_eq!(lines[0]["env"].as_object().unwrap().len(), 2);
        assert_eq!(lines[0]["env"]["SHELL"], "/usr/bin/fish");
        assert_eq!(lines[0]["env"]["TERM"], "xterm256-color");
        assert_eq!(lines[0]["theme"]["fg"], "#000102");
        assert_eq!(lines[0]["theme"]["bg"], "#0064c8");
        assert_eq!(lines[0]["theme"]["palette"], "#000000:#0a0b0c:#141516:#1e1f20:#28292a:#323334:#3c3d3e:#464748:#505152:#5a5b5c:#646566:#6e6f70:#78797a:#828384:#8c8d8e:#969798");
    }

    fn parse(json: Vec<u8>) -> Vec<serde_json::Value> {
        String::from_utf8(json)
            .unwrap()
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<serde_json::Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn accelerate() {
        let events = [(0u64, "foo"), (20, "bar"), (50, "baz")].map(|(time, output)| {
            Ok(Event::output(
                Duration::from_micros(time),
                output.to_owned(),
            ))
        });

        let output = output(super::accelerate(events.into_iter(), 2.0));

        assert_eq!(output[0], (Duration::from_micros(0), "foo".to_owned()));
        assert_eq!(output[1], (Duration::from_micros(10), "bar".to_owned()));
        assert_eq!(output[2], (Duration::from_micros(25), "baz".to_owned()));
    }

    #[test]
    fn limit_idle_time() {
        let events = [
            (0, "foo"),
            (1_000_000, "bar"),
            (3_500_000, "baz"),
            (4_000_000, "qux"),
            (7_500_000, "quux"),
        ]
        .map(|(time, output)| {
            Ok(Event::output(
                Duration::from_micros(time),
                output.to_owned(),
            ))
        });

        let events = output(super::limit_idle_time(events.into_iter(), 2.0));

        assert_eq!(events[0], (Duration::from_micros(0), "foo".to_owned()));
        assert_eq!(
            events[1],
            (Duration::from_micros(1_000_000), "bar".to_owned())
        );
        assert_eq!(
            events[2],
            (Duration::from_micros(3_000_000), "baz".to_owned())
        );
        assert_eq!(
            events[3],
            (Duration::from_micros(3_500_000), "qux".to_owned())
        );
        assert_eq!(
            events[4],
            (Duration::from_micros(5_500_000), "quux".to_owned())
        );
    }

    fn output(events: impl Iterator<Item = Result<Event>>) -> Vec<(Duration, String)> {
        events
            .filter_map(|r| {
                if let Ok(Event {
                    time,
                    data: EventData::Output(data),
                }) = r
                {
                    Some((time, data))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}
