use anyhow::bail;
use serde::Deserialize;
use std::fmt::{self, Display};
use std::fs;
use std::io::BufRead;
use std::io::{self, Write};
use std::path::Path;

pub struct Writer<W: Write> {
    writer: W,
    time_offset: f64,
}

pub struct Header {
    pub terminal_size: (usize, usize),
    pub idle_time_limit: Option<f64>,
}

#[derive(Deserialize)]
pub struct V2Header {
    pub width: usize,
    pub height: usize,
    pub idle_time_limit: Option<f64>,
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
        write_header(&mut self.writer, header)
    }

    pub fn write_event(&mut self, mut event: Event) -> io::Result<()> {
        event.time += self.time_offset;

        write_event(&mut self.writer, &event)
    }
}

impl<W> super::Writer for Writer<W>
where
    W: Write,
{
    fn header(&mut self, size: (u16, u16)) -> io::Result<()> {
        let header = Header {
            terminal_size: (size.0 as usize, size.1 as usize),
            idle_time_limit: None,
        };

        self.write_header(&header)
    }

    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.write_event(Event::output(time, data))
    }

    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.write_event(Event::input(time, data))
    }
}

pub fn open<R: BufRead>(
    reader: R,
) -> anyhow::Result<(Header, impl Iterator<Item = anyhow::Result<Event>>)> {
    let mut lines = reader.lines();
    let first_line = lines.next().ok_or(anyhow::anyhow!("empty"))??;
    let v2_header: V2Header = serde_json::from_str(&first_line)?;
    let header: Header = v2_header.into();

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

pub fn write_header<W: Write>(sink: &mut W, header: &Header) -> io::Result<()> {
    writeln!(sink, "{}", serde_json::to_string(header)?)
}

pub fn write_event<W: Write>(sink: &mut W, event: &Event) -> io::Result<()> {
    writeln!(sink, "{}", serde_json::to_string(event)?)
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
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("version", &2)?;
        map.serialize_entry("width", &self.terminal_size.0)?;
        map.serialize_entry("height", &self.terminal_size.1)?;
        // TODO idle_time_limit
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

impl From<V2Header> for Header {
    fn from(v2: V2Header) -> Self {
        Self {
            terminal_size: (v2.width, v2.height),
            idle_time_limit: v2.idle_time_limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, EventCode, Header, Writer};
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

        assert_eq!(header.terminal_size, (75, 18));

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
            terminal_size: (80, 24),
            idle_time_limit: None,
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

        let asciicast = String::from_utf8(data).unwrap();

        assert_eq!(asciicast, "{\"version\":2,\"width\":80,\"height\":24}\n[1.0,\"o\",\"hello\\r\\n\"]\n[2.0,\"o\",\"world\"]\n");
    }
}
