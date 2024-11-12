mod util;
mod v1;
mod v2;
use crate::tty;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
pub use v2::Encoder;

pub struct Asciicast<'a> {
    pub header: Header,
    pub events: Box<dyn Iterator<Item = Result<Event>> + 'a>,
}

pub struct Header {
    pub cols: u16,
    pub rows: u16,
    pub timestamp: Option<u64>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub theme: Option<tty::Theme>,
}

pub struct Event {
    pub time: u64,
    pub data: EventData,
}

pub enum EventData {
    Output(String),
    Input(String),
    Resize(u16, u16),
    Marker(String),
    Other(char, String),
}

impl Default for Header {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            timestamp: None,
            idle_time_limit: None,
            command: None,
            title: None,
            env: None,
            theme: None,
        }
    }
}

pub fn open_from_path<S: AsRef<Path>>(path: S) -> Result<Asciicast<'static>> {
    fs::File::open(path)
        .map(io::BufReader::new)
        .map_err(|e| anyhow!(e))
        .and_then(open)
        .map_err(|e| anyhow!("can't open asciicast file: {e}"))
}

pub fn open<'a, R: BufRead + 'a>(reader: R) -> Result<Asciicast<'a>> {
    let mut lines = reader.lines();
    let first_line = lines.next().ok_or(anyhow!("empty file"))??;

    if let Ok(parser) = v2::open(&first_line) {
        Ok(parser.parse(lines))
    } else {
        let json = std::iter::once(Ok(first_line))
            .chain(lines)
            .collect::<io::Result<String>>()?;

        v1::load(json)
    }
}

pub fn get_duration<S: AsRef<Path>>(path: S) -> Result<u64> {
    let Asciicast { events, .. } = open_from_path(path)?;
    let time = events.last().map_or(Ok(0), |e| e.map(|e| e.time))?;

    Ok(time)
}

impl Event {
    pub fn output(time: u64, text: String) -> Self {
        Event {
            time,
            data: EventData::Output(text),
        }
    }

    pub fn input(time: u64, text: String) -> Self {
        Event {
            time,
            data: EventData::Input(text),
        }
    }

    pub fn resize(time: u64, size: (u16, u16)) -> Self {
        Event {
            time,
            data: EventData::Resize(size.0, size.1),
        }
    }

    pub fn marker(time: u64, label: String) -> Self {
        Event {
            time,
            data: EventData::Marker(label),
        }
    }
}

pub fn limit_idle_time(
    events: impl Iterator<Item = Result<Event>>,
    limit: f64,
) -> impl Iterator<Item = Result<Event>> {
    let limit = (limit * 1_000_000.0) as u64;
    let mut prev_time = 0;
    let mut offset = 0;

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
    events: impl Iterator<Item = Result<Event>>,
    speed: f64,
) -> impl Iterator<Item = Result<Event>> {
    events.map(move |event| {
        event.map(|event| {
            let time = ((event.time as f64) / speed) as u64;

            Event { time, ..event }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{Asciicast, Encoder, Event, EventData, Header};
    use crate::tty;
    use anyhow::Result;
    use rgb::RGB8;
    use std::collections::HashMap;

    #[test]
    fn open_v1_minimal() {
        let Asciicast { header, events } =
            super::open_from_path("tests/casts/minimal.json").unwrap();

        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!((header.cols, header.rows), (100, 50));
        assert!(header.theme.is_none());

        assert_eq!(events[0].time, 1230000);
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "hello"));
    }

    #[test]
    fn open_v1_full() {
        let Asciicast { header, events } = super::open_from_path("tests/casts/full.json").unwrap();
        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!((header.cols, header.rows), (100, 50));

        assert_eq!(events[0].time, 1);
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "ż"));

        assert_eq!(events[1].time, 1000000);
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "ółć"));

        assert_eq!(events[2].time, 10500000);
        assert!(matches!(events[2].data, EventData::Output(ref s) if s == "\r\n"));
    }

    #[test]
    fn open_v2_minimal() {
        let Asciicast { header, events } =
            super::open_from_path("tests/casts/minimal.cast").unwrap();
        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!((header.cols, header.rows), (100, 50));
        assert!(header.theme.is_none());

        assert_eq!(events[0].time, 1230000);
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "hello"));
    }

    #[test]
    fn open_v2_full() {
        let Asciicast { header, events } = super::open_from_path("tests/casts/full.cast").unwrap();
        let events = events.take(5).collect::<Result<Vec<Event>>>().unwrap();
        let theme = header.theme.unwrap();

        assert_eq!((header.cols, header.rows), (100, 50));
        assert_eq!(theme.fg, RGB8::new(0, 0, 0));
        assert_eq!(theme.bg, RGB8::new(0xff, 0xff, 0xff));
        assert_eq!(theme.palette[0], RGB8::new(0x24, 0x1f, 0x31));

        assert_eq!(events[0].time, 1);
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "ż"));

        assert_eq!(events[1].time, 1_000_000);
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "ółć"));

        assert_eq!(events[2].time, 2_300_000);
        assert!(matches!(events[2].data, EventData::Input(ref s) if s == "\n"));

        assert_eq!(events[3].time, 5_600_001);
        assert!(
            matches!(events[3].data, EventData::Resize(ref cols, ref rows) if *cols == 80 && *rows == 40)
        );

        assert_eq!(events[4].time, 10_500_000);
        assert!(matches!(events[4].data, EventData::Output(ref s) if s == "\r\n"));
    }

    #[test]
    fn encoder() {
        let mut data = Vec::new();

        let header = Header {
            cols: 80,
            rows: 24,
            timestamp: None,
            idle_time_limit: None,
            command: None,
            title: None,
            env: Default::default(),
            theme: None,
        };

        let mut enc = Encoder::new(0);
        data.extend(enc.header(&header));
        data.extend(enc.event(&Event::output(1000001, "hello\r\n".to_owned())));

        let mut enc = Encoder::new(1000001);
        data.extend(enc.event(&Event::output(1000001, "world".to_owned())));
        data.extend(enc.event(&Event::input(2000002, " ".to_owned())));
        data.extend(enc.event(&Event::resize(3000003, (100, 40))));
        data.extend(enc.event(&Event::output(4000004, "żółć".to_owned())));

        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert!(lines[0]["timestamp"].is_null());
        assert_eq!(lines[1][0], 1.000001);
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
        let mut enc = Encoder::new(0);
        let mut env = HashMap::new();
        env.insert("SHELL".to_owned(), "/usr/bin/fish".to_owned());
        env.insert("TERM".to_owned(), "xterm256-color".to_owned());

        let theme = tty::Theme {
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
            cols: 80,
            rows: 24,
            timestamp: Some(1704719152),
            idle_time_limit: Some(1.5),
            command: Some("/bin/bash".to_owned()),
            title: Some("Demo".to_owned()),
            env: Some(env),
            theme: Some(theme),
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
        let events = [(0u64, "foo"), (20, "bar"), (50, "baz")]
            .map(|(time, output)| Ok(Event::output(time, output.to_owned())));

        let output = output(super::accelerate(events.into_iter(), 2.0));

        assert_eq!(output[0], (0, "foo".to_owned()));
        assert_eq!(output[1], (10, "bar".to_owned()));
        assert_eq!(output[2], (25, "baz".to_owned()));
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
        .map(|(time, output)| Ok(Event::output(time, output.to_owned())));

        let events = output(super::limit_idle_time(events.into_iter(), 2.0));

        assert_eq!(events[0], (0, "foo".to_owned()));
        assert_eq!(events[1], (1_000_000, "bar".to_owned()));
        assert_eq!(events[2], (3_000_000, "baz".to_owned()));
        assert_eq!(events[3], (3_500_000, "qux".to_owned()));
        assert_eq!(events[4], (5_500_000, "quux".to_owned()));
    }

    fn output(events: impl Iterator<Item = Result<Event>>) -> Vec<(u64, String)> {
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
