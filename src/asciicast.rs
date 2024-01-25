mod util;
mod v1;
mod v2;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
pub use v2::Writer;

pub struct Asciicast<'a> {
    pub header: Header,
    pub events: Box<dyn Iterator<Item = Result<Event>> + 'a>,
}

pub struct Header {
    pub version: u8,
    pub cols: u16,
    pub rows: u16,
    pub timestamp: Option<u64>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
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
    pub fn output(time: u64, data: &[u8]) -> Self {
        Event {
            time,
            data: EventData::Output(String::from_utf8_lossy(data).to_string()),
        }
    }

    pub fn input(time: u64, data: &[u8]) -> Self {
        Event {
            time,
            data: EventData::Input(String::from_utf8_lossy(data).to_string()),
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
    use super::{Asciicast, Event, EventData, Header, Writer};
    use anyhow::Result;
    use std::collections::HashMap;
    use std::io;

    #[test]
    fn open_v1_minimal() {
        let Asciicast { header, events } =
            super::open_from_path("tests/casts/minimal.json").unwrap();

        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(header.version, 1);
        assert_eq!((header.cols, header.rows), (100, 50));

        assert_eq!(events[0].time, 1230000);
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "hello"));
    }

    #[test]
    fn open_v1_full() {
        let Asciicast { header, events } = super::open_from_path("tests/casts/full.json").unwrap();
        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(header.version, 1);
        assert_eq!((header.cols, header.rows), (100, 50));

        assert_eq!(events[0].time, 1);
        assert!(matches!(events[0].data, EventData::Output(ref s) if s == "ż"));

        assert_eq!(events[1].time, 1000000);
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "ółć"));

        assert_eq!(events[2].time, 10500000);
        assert!(matches!(events[2].data, EventData::Output(ref s) if s == "\r\n"));
    }

    #[test]
    fn open_v2() {
        let Asciicast { header, events } = super::open_from_path("tests/casts/demo.cast").unwrap();
        let events = events.take(7).collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!((header.cols, header.rows), (75, 18));

        assert_eq!(events[1].time, 100989);
        assert!(matches!(events[1].data, EventData::Output(ref s) if s == "\u{1b}[?2004h"));

        assert_eq!(events[5].time, 1511526);
        assert!(matches!(events[5].data, EventData::Input(ref s) if s == "v"));

        assert_eq!(events[6].time, 1511937);
        assert!(matches!(events[6].data, EventData::Output(ref s) if s == "v"));
    }

    #[test]
    fn writer() {
        let mut data = Vec::new();

        {
            let mut fw = Writer::new(&mut data, 0);

            let header = Header {
                version: 2,
                cols: 80,
                rows: 24,
                timestamp: None,
                idle_time_limit: None,
                command: None,
                title: None,
                env: Default::default(),
            };

            fw.write_header(&header).unwrap();

            fw.write_event(&Event::output(1000001, "hello\r\n".as_bytes()))
                .unwrap();
        }

        {
            let mut fw = Writer::new(&mut data, 1000001);

            fw.write_event(&Event::output(1000001, "world".as_bytes()))
                .unwrap();

            fw.write_event(&Event::input(2000002, " ".as_bytes()))
                .unwrap();

            fw.write_event(&Event::resize(3000003, (100, 40))).unwrap();

            fw.write_event(&Event::output(4000004, "żółć".as_bytes()))
                .unwrap();
        }

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
    fn write_header() {
        let mut data = Vec::new();

        {
            let mut fw = Writer::new(io::Cursor::new(&mut data), 0);
            let mut env = HashMap::new();
            env.insert("SHELL".to_owned(), "/usr/bin/fish".to_owned());
            env.insert("TERM".to_owned(), "xterm256-color".to_owned());

            let header = Header {
                version: 2,
                cols: 80,
                rows: 24,
                timestamp: Some(1704719152),
                idle_time_limit: Some(1.5),
                command: Some("/bin/bash".to_owned()),
                title: Some("Demo".to_owned()),
                env: Some(env),
            };

            fw.write_header(&header).unwrap();
        }

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
            .map(|(time, output)| Ok(Event::output(time, output.as_bytes())));

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
        .map(|(time, output)| Ok(Event::output(time, output.as_bytes())));

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
