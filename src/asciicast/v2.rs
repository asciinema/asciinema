use super::{util, Asciicast, Event, EventData, Header};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Deserialize)]
struct V2Header {
    version: u8,
    width: u16,
    height: u16,
    timestamp: Option<u64>,
    idle_time_limit: Option<f64>,
    command: Option<String>,
    title: Option<String>,
    env: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct V2Event {
    #[serde(deserialize_with = "util::deserialize_time")]
    time: u64,
    #[serde(deserialize_with = "deserialize_code")]
    code: V2EventCode,
    data: String,
}

#[derive(PartialEq, Debug)]
enum V2EventCode {
    Output,
    Input,
    Resize,
    Marker,
    Other(char),
}

pub struct Parser(V2Header);

pub fn open(header_line: &str) -> Result<Parser> {
    let header = serde_json::from_str::<V2Header>(header_line)?;

    if header.version != 2 {
        bail!("unsupported asciicast version")
    }

    Ok(Parser(header))
}

impl Parser {
    pub fn parse<'a, I: Iterator<Item = io::Result<String>> + 'a>(
        &self,
        lines: I,
    ) -> Asciicast<'a> {
        let header = Header {
            version: 2,
            cols: self.0.width,
            rows: self.0.height,
            timestamp: self.0.timestamp,
            idle_time_limit: self.0.idle_time_limit,
            command: self.0.command.clone(),
            title: self.0.title.clone(),
            env: self.0.env.clone(),
        };

        let events = Box::new(lines.filter_map(parse_line));

        Asciicast { header, events }
    }
}

fn parse_line(line: io::Result<String>) -> Option<Result<Event>> {
    match line {
        Ok(line) => {
            if line.is_empty() {
                None
            } else {
                Some(parse_event(line))
            }
        }

        Err(e) => Some(Err(e.into())),
    }
}

fn parse_event(line: String) -> Result<Event> {
    let event = serde_json::from_str::<V2Event>(&line)?;

    let data = match event.code {
        V2EventCode::Output => EventData::Output(event.data),
        V2EventCode::Input => EventData::Input(event.data),

        V2EventCode::Resize => match event.data.split_once('x') {
            Some((cols, rows)) => {
                let cols: u16 = cols
                    .parse()
                    .map_err(|e| anyhow!("invalid cols value in resize event: {e}"))?;

                let rows: u16 = rows
                    .parse()
                    .map_err(|e| anyhow!("invalid rows value in resize event: {e}"))?;

                EventData::Resize(cols, rows)
            }

            None => {
                bail!("invalid size value in resize event");
            }
        },

        V2EventCode::Marker => EventData::Marker(event.data),
        V2EventCode::Other(c) => EventData::Other(c, event.data),
    };

    Ok(Event {
        time: event.time,
        data,
    })
}

fn deserialize_code<'de, D>(deserializer: D) -> Result<V2EventCode, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use V2EventCode::*;

    let value: &str = Deserialize::deserialize(deserializer)?;

    match value {
        "o" => Ok(Output),
        "i" => Ok(Input),
        "r" => Ok(Resize),
        "m" => Ok(Marker),
        "" => Err(Error::custom("missing event code")),
        s => Ok(Other(s.chars().next().unwrap())),
    }
}

pub struct Writer<W: Write> {
    writer: io::LineWriter<W>,
    time_offset: u64,
}

impl<W> Writer<W>
where
    W: Write,
{
    pub fn new(writer: W, time_offset: u64) -> Self {
        Self {
            writer: io::LineWriter::new(writer),
            time_offset,
        }
    }

    pub fn write_header(&mut self, header: &Header) -> io::Result<()> {
        let header: V2Header = header.into();

        writeln!(self.writer, "{}", serde_json::to_string(&header)?)
    }

    pub fn write_event(&mut self, event: &Event) -> io::Result<()> {
        writeln!(self.writer, "{}", self.serialize_event(event)?)
    }

    fn serialize_event(&self, event: &Event) -> Result<String, serde_json::Error> {
        use EventData::*;

        let (code, data) = match &event.data {
            Output(data) => ('o', serde_json::to_string(data)?),
            Input(data) => ('i', serde_json::to_string(data)?),
            Resize(cols, rows) => ('r', serde_json::to_string(&format!("{cols}x{rows}"))?),
            Marker(data) => ('m', serde_json::to_string(data)?),
            Other(code, data) => (*code, serde_json::to_string(data)?),
        };

        Ok(format!(
            "[{}, {}, {}]",
            format_time(event.time + self.time_offset).trim_end_matches('0'),
            serde_json::to_string(&code)?,
            data,
        ))
    }
}

fn format_time(time: u64) -> String {
    format!("{}.{:0>6}", time / 1_000_000, time % 1_000_000)
}

impl serde::Serialize for V2Header {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut len = 4;

        if self.timestamp.is_some() {
            len += 1;
        }

        if self.idle_time_limit.is_some() {
            len += 1;
        }

        if self.command.is_some() {
            len += 1;
        }

        if self.title.is_some() {
            len += 1;
        }

        if self.env.as_ref().is_some_and(|env| !env.is_empty()) {
            len += 1;
        }

        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("version", &2)?;
        map.serialize_entry("width", &self.width)?;
        map.serialize_entry("height", &self.height)?;

        if let Some(timestamp) = self.timestamp {
            map.serialize_entry("timestamp", &timestamp)?;
        }

        if let Some(limit) = self.idle_time_limit {
            map.serialize_entry("idle_time_limit", &limit)?;
        }

        if let Some(command) = &self.command {
            map.serialize_entry("command", &command)?;
        }

        if let Some(title) = &self.title {
            map.serialize_entry("title", &title)?;
        }

        if let Some(env) = &self.env {
            if !env.is_empty() {
                map.serialize_entry("env", &env)?;
            }
        }

        map.end()
    }
}

impl From<&Header> for V2Header {
    fn from(header: &Header) -> Self {
        V2Header {
            version: 2,
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
