use anyhow::bail;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs;
use std::io::BufRead;
use std::io::{self, Write};
use std::path::Path;

pub struct Writer<W: Write> {
    writer: W,
    time_offset: f64,
}

#[derive(Deserialize)]
pub struct Header {
    width: u16,
    height: u16,
    timestamp: u64,
    idle_time_limit: Option<f32>,
    command: Option<String>,
    title: Option<String>,
    env: HashMap<String, String>,
}

pub struct Event {
    pub time: f64,
    pub code: EventCode,
    pub data: String,
}

#[derive(PartialEq, Eq, Debug)]
pub enum EventCode {
    Output,
    Input,
    Resize,
    Marker,
    Other(char),
}

impl<W> Writer<W>
where
    W: Write,
{
    pub fn new(writer: W, time_offset: f64) -> Self {
        Self {
            writer,
            time_offset,
        }
    }

    pub fn write_header(&mut self, header: &Header) -> io::Result<()> {
        writeln!(self.writer, "{}", serde_json::to_string(&header)?)
    }

    pub fn write_event(&mut self, mut event: Event) -> io::Result<()> {
        event.time += self.time_offset;

        writeln!(self.writer, "{}", serde_json::to_string(&event)?)
    }
}

impl<W> super::Writer for Writer<W>
where
    W: Write,
{
    fn header(&mut self, header: &super::Header) -> io::Result<()> {
        self.write_header(&header.into())
    }

    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.write_event(Event::output(time, data))
    }

    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.write_event(Event::input(time, data))
    }

    fn resize(&mut self, time: f64, size: (u16, u16)) -> io::Result<()> {
        self.write_event(Event::resize(time, size))
    }
}

pub fn open<R: BufRead>(
    reader: R,
) -> anyhow::Result<(super::Header, impl Iterator<Item = anyhow::Result<Event>>)> {
    let mut lines = reader.lines();
    let first_line = lines.next().ok_or(anyhow::anyhow!("empty"))??;
    let header: Header = serde_json::from_str(&first_line)?;
    let header: super::Header = (&header).into();

    let events = lines
        .filter(|l| l.as_ref().map_or(true, |l| !l.is_empty()))
        .enumerate()
        .map(|(i, l)| l.map(|l| parse_event(l, i + 2))?);

    Ok((header, events))
}

fn parse_event(line: String, i: usize) -> anyhow::Result<Event> {
    use EventCode::*;

    let value: serde_json::Value = serde_json::from_str(&line)?;

    let time = value[0]
        .as_f64()
        .ok_or(anyhow::anyhow!("line {}: invalid event time", i))?;

    let code = match value[1].as_str() {
        Some("o") => Output,
        Some("i") => Input,
        Some("r") => Resize,
        Some("m") => Marker,
        Some(s) if !s.is_empty() => Other(s.chars().next().unwrap()),
        Some(_) => bail!("line {}: missing event code", i),
        None => bail!("line {}: event code must be a string", i),
    };

    let data = match value[2].as_str() {
        Some(data) => data.to_owned(),
        None => bail!("line {}: event data must be a string", i),
    };

    Ok(Event { time, code, data })
}

pub fn get_duration<S: AsRef<Path>>(path: S) -> anyhow::Result<f64> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let (_header, events) = open(reader)?;
    let time = events.last().map_or(Ok(0.0), |e| e.map(|e| e.time))?;

    Ok(time)
}

impl Event {
    pub fn output(time: f64, data: &[u8]) -> Self {
        Event {
            time,
            code: EventCode::Output,
            data: String::from_utf8_lossy(data).to_string(),
        }
    }

    pub fn input(time: f64, data: &[u8]) -> Self {
        Event {
            time,
            code: EventCode::Input,
            data: String::from_utf8_lossy(data).to_string(),
        }
    }

    pub fn resize(time: f64, size: (u16, u16)) -> Self {
        Event {
            time,
            code: EventCode::Resize,
            data: format!("{}x{}", size.0, size.1),
        }
    }
}

impl Display for EventCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        use EventCode::*;

        match self {
            Output => f.write_str("o"),
            Input => f.write_str("i"),
            Resize => f.write_str("r"),
            Marker => f.write_str("m"),
            Other(t) => f.write_str(&t.to_string()),
        }
    }
}

impl serde::Serialize for Header {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut len = 4;

        if self.idle_time_limit.is_some() {
            len += 1;
        }

        if self.command.is_some() {
            len += 1;
        }

        if self.title.is_some() {
            len += 1;
        }

        if !self.env.is_empty() {
            len += 1;
        }

        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("version", &2)?;
        map.serialize_entry("width", &self.width)?;
        map.serialize_entry("height", &self.height)?;
        map.serialize_entry("timestamp", &self.timestamp)?;

        if let Some(limit) = self.idle_time_limit {
            map.serialize_entry("idle_time_limit", &limit)?;
        }

        if let Some(command) = &self.command {
            map.serialize_entry("command", &command)?;
        }

        if let Some(title) = &self.title {
            map.serialize_entry("title", &title)?;
        }

        if !self.env.is_empty() {
            map.serialize_entry("env", &self.env)?;
        }

        map.end()
    }
}

impl serde::Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&self.time)?;
        tup.serialize_element(&self.code.to_string())?;
        tup.serialize_element(&self.data)?;
        tup.end()
    }
}

impl From<&Header> for super::Header {
    fn from(header: &Header) -> Self {
        Self {
            cols: header.width,
            rows: header.height,
            timestamp: header.timestamp,
            idle_time_limit: header.idle_time_limit,
            command: header.command.clone(),
            title: header.title.clone(),
            env: header.env.clone(),
        }
    }
}

impl From<&super::Header> for Header {
    fn from(header: &super::Header) -> Self {
        Self {
            width: header.cols,
            height: header.rows,
            timestamp: header.timestamp,
            idle_time_limit: header.idle_time_limit,
            command: header.command.clone(),
            title: header.title.clone(),
            env: header.env.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, EventCode, Header, Writer};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io;

    #[test]
    fn open() {
        let file = File::open("tests/demo.cast").unwrap();
        let (header, events) = super::open(io::BufReader::new(file)).unwrap();

        let events = events
            .take(7)
            .collect::<anyhow::Result<Vec<Event>>>()
            .unwrap();

        assert_eq!((header.cols, header.rows), (75, 18));

        assert_eq!(events[1].time, 0.100989);
        assert_eq!(events[1].code, EventCode::Output);
        assert_eq!(events[1].data, "\u{1b}[?2004h");

        assert_eq!(events[5].time, 1.511526);
        assert_eq!(events[5].code, EventCode::Input);
        assert_eq!(events[5].data, "v");

        assert_eq!(events[6].time, 1.511937);
        assert_eq!(events[6].code, EventCode::Output);
        assert_eq!(events[6].data, "v");
    }

    #[test]
    fn writer() {
        let mut data = Vec::new();

        let cursor = io::Cursor::new(&mut data);
        let mut fw = Writer::new(cursor, 0.0);

        let header = Header {
            width: 80,
            height: 24,
            timestamp: 1,
            idle_time_limit: None,
            command: None,
            title: None,
            env: Default::default(),
        };

        fw.write_header(&header).unwrap();

        fw.write_event(Event {
            time: 1.0,
            code: EventCode::Output,
            data: "hello\r\n".to_owned(),
        })
        .unwrap();

        let data_len = data.len() as u64;
        let mut cursor = io::Cursor::new(&mut data);
        cursor.set_position(data_len);
        let mut fw = Writer::new(cursor, 1.0);

        fw.write_event(Event {
            time: 1.0,
            code: EventCode::Output,
            data: "world".to_owned(),
        })
        .unwrap();

        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert_eq!(lines[0]["timestamp"], 1);
        assert_eq!(lines[1][0], 1.0);
        assert_eq!(lines[1][1], "o");
        assert_eq!(lines[1][2], "hello\r\n");
        assert_eq!(lines[2][0], 2.0);
        assert_eq!(lines[2][1], "o");
        assert_eq!(lines[2][2], "world");
    }

    #[test]
    fn write_header() {
        let mut data = Vec::new();
        let mut fw = Writer::new(io::Cursor::new(&mut data), 0.0);

        let mut env = HashMap::new();
        env.insert("SHELL".to_owned(), "/usr/bin/fish".to_owned());
        env.insert("TERM".to_owned(), "xterm256-color".to_owned());

        let header = Header {
            width: 80,
            height: 24,
            timestamp: 1,
            idle_time_limit: Some(1.5),
            command: Some("/bin/bash".to_owned()),
            title: Some("Demo".to_owned()),
            env,
        };

        fw.write_header(&header).unwrap();

        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert_eq!(lines[0]["timestamp"], 1);
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
}
