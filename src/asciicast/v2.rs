use super::{util, Asciicast, Event, EventData, Header};
use crate::tty;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io;

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
    theme: Option<V2Theme>,
}

#[derive(Deserialize, Serialize, Clone)]
struct V2Theme {
    #[serde(deserialize_with = "deserialize_color")]
    fg: RGB8,
    #[serde(deserialize_with = "deserialize_color")]
    bg: RGB8,
    #[serde(deserialize_with = "deserialize_palette")]
    palette: V2Palette,
}

#[derive(Clone)]
struct RGB8(rgb::RGB8);

#[derive(Clone)]
struct V2Palette(Vec<RGB8>);

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
            cols: self.0.width,
            rows: self.0.height,
            timestamp: self.0.timestamp,
            idle_time_limit: self.0.idle_time_limit,
            command: self.0.command.clone(),
            title: self.0.title.clone(),
            env: self.0.env.clone(),
            theme: self.0.theme.as_ref().map(|t| t.into()),
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

pub struct Encoder {
    time_offset: u64,
}

impl Encoder {
    pub fn new(time_offset: u64) -> Self {
        Self { time_offset }
    }

    pub fn header(&mut self, header: &Header) -> Vec<u8> {
        let header: V2Header = header.into();
        let mut data = serde_json::to_string(&header).unwrap().into_bytes();
        data.push(b'\n');

        data
    }

    pub fn event(&mut self, event: &Event) -> Vec<u8> {
        let mut data = self.serialize_event(event).unwrap().into_bytes();
        data.push(b'\n');

        data
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

        if self.theme.is_some() {
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

        if let Some(theme) = &self.theme {
            map.serialize_entry("theme", &theme)?;
        }

        map.end()
    }
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<RGB8, D::Error>
where
    D: Deserializer<'de>,
{
    let value: &str = Deserialize::deserialize(deserializer)?;
    parse_hex_color(value).ok_or(serde::de::Error::custom("invalid hex triplet"))
}

fn parse_hex_color(rgb: &str) -> Option<RGB8> {
    if rgb.len() != 7 {
        return None;
    }

    let r = u8::from_str_radix(&rgb[1..3], 16).ok()?;
    let g = u8::from_str_radix(&rgb[3..5], 16).ok()?;
    let b = u8::from_str_radix(&rgb[5..7], 16).ok()?;

    Some(RGB8(rgb::RGB8::new(r, g, b)))
}

fn deserialize_palette<'de, D>(deserializer: D) -> Result<V2Palette, D::Error>
where
    D: Deserializer<'de>,
{
    let value: &str = Deserialize::deserialize(deserializer)?;
    let mut colors: Vec<RGB8> = value.split(':').filter_map(parse_hex_color).collect();
    let len = colors.len();

    if len == 8 {
        colors.extend_from_within(..);
    } else if len != 16 {
        return Err(serde::de::Error::custom("expected 8 or 16 hex triplets"));
    }

    Ok(V2Palette(colors))
}

impl serde::Serialize for RGB8 {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for RGB8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "#{:0>2x}{:0>2x}{:0>2x}", self.0.r, self.0.g, self.0.b)
    }
}

impl serde::Serialize for V2Palette {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let palette = self
            .0
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(":");

        serializer.serialize_str(&palette)
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
            theme: header.theme.as_ref().map(|t| t.into()),
        }
    }
}

impl From<&tty::Theme> for V2Theme {
    fn from(theme: &tty::Theme) -> Self {
        let palette = theme.palette.iter().copied().map(RGB8).collect();

        V2Theme {
            fg: RGB8(theme.fg),
            bg: RGB8(theme.bg),
            palette: V2Palette(palette),
        }
    }
}

impl From<&V2Theme> for tty::Theme {
    fn from(theme: &V2Theme) -> Self {
        let palette = theme.palette.0.iter().map(|c| c.0).collect();

        tty::Theme {
            fg: theme.fg.0,
            bg: theme.bg.0,
            palette,
        }
    }
}
