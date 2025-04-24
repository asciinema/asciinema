use std::collections::HashMap;
use std::fmt;
use std::io;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Deserializer, Serialize};

use super::{util, Asciicast, Event, EventData, Header};
use crate::tty::TtyTheme;

#[derive(Deserialize)]
struct V3Header {
    version: u8,
    term: V3Term,
    timestamp: Option<u64>,
    idle_time_limit: Option<f64>,
    command: Option<String>,
    title: Option<String>,
    env: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct V3Term {
    cols: u16,
    rows: u16,
    #[serde(rename = "type")]
    type_: Option<String>,
    version: Option<String>,
    theme: Option<V3Theme>,
}

#[derive(Deserialize, Serialize, Clone)]
struct V3Theme {
    #[serde(deserialize_with = "deserialize_color")]
    fg: RGB8,
    #[serde(deserialize_with = "deserialize_color")]
    bg: RGB8,
    #[serde(deserialize_with = "deserialize_palette")]
    palette: V3Palette,
}

#[derive(Clone)]
struct RGB8(rgb::RGB8);

#[derive(Clone)]
struct V3Palette(Vec<RGB8>);

#[derive(Debug, Deserialize)]
struct V3Event {
    #[serde(deserialize_with = "util::deserialize_time")]
    time: u64,
    #[serde(deserialize_with = "deserialize_code")]
    code: V3EventCode,
    data: String,
}

#[derive(PartialEq, Debug)]
enum V3EventCode {
    Output,
    Input,
    Resize,
    Marker,
    Other(char),
}

pub struct Parser {
    header: V3Header,
    prev_time: u64,
}

pub fn open(header_line: &str) -> Result<Parser> {
    let header = serde_json::from_str::<V3Header>(header_line)?;

    if header.version != 3 {
        bail!("not an asciicast v3 file")
    }

    Ok(Parser {
        header,
        prev_time: 0,
    })
}

impl Parser {
    pub fn parse<'a, I: Iterator<Item = io::Result<String>> + 'a>(
        mut self,
        lines: I,
    ) -> Asciicast<'a> {
        let term_theme = self.header.term.theme.as_ref().map(|t| t.into());

        let header = Header {
            term_cols: self.header.term.cols,
            term_rows: self.header.term.rows,
            term_type: self.header.term.type_.clone(),
            term_version: self.header.term.version.clone(),
            term_theme,
            timestamp: self.header.timestamp,
            idle_time_limit: self.header.idle_time_limit,
            command: self.header.command.clone(),
            title: self.header.title.clone(),
            env: self.header.env.clone(),
        };

        let events = Box::new(lines.filter_map(move |line| self.parse_line(line)));

        Asciicast { header, events }
    }

    fn parse_line(&mut self, line: io::Result<String>) -> Option<Result<Event>> {
        match line {
            Ok(line) => {
                if line.is_empty() || line.starts_with("#") {
                    None
                } else {
                    Some(self.parse_event(line))
                }
            }

            Err(e) => Some(Err(e.into())),
        }
    }

    fn parse_event(&mut self, line: String) -> Result<Event> {
        let event = serde_json::from_str::<V3Event>(&line).context("asciicast v3 parse error")?;

        let data = match event.code {
            V3EventCode::Output => EventData::Output(event.data),
            V3EventCode::Input => EventData::Input(event.data),

            V3EventCode::Resize => match event.data.split_once('x') {
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

            V3EventCode::Marker => EventData::Marker(event.data),
            V3EventCode::Other(c) => EventData::Other(c, event.data),
        };

        let time = self.prev_time + event.time;
        self.prev_time = time;

        Ok(Event { time, data })
    }
}

fn deserialize_code<'de, D>(deserializer: D) -> Result<V3EventCode, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use V3EventCode::*;

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

pub struct V3Encoder {
    prev_time: u64,
}

impl V3Encoder {
    pub fn new() -> Self {
        Self { prev_time: 0 }
    }

    pub fn header(&mut self, header: &Header) -> Vec<u8> {
        let header: V3Header = header.into();
        let mut data = serde_json::to_string(&header).unwrap().into_bytes();
        data.push(b'\n');

        data
    }

    pub fn event(&mut self, event: &Event) -> Vec<u8> {
        let mut data = self.serialize_event(event).unwrap().into_bytes();
        data.push(b'\n');

        data
    }

    fn serialize_event(&mut self, event: &Event) -> Result<String, serde_json::Error> {
        use EventData::*;

        let (code, data) = match &event.data {
            Output(data) => ('o', serde_json::to_string(data)?),
            Input(data) => ('i', serde_json::to_string(data)?),
            Resize(cols, rows) => ('r', serde_json::to_string(&format!("{cols}x{rows}"))?),
            Marker(data) => ('m', serde_json::to_string(data)?),
            Other(code, data) => (*code, serde_json::to_string(data)?),
        };

        let time = event.time - self.prev_time;
        self.prev_time = event.time;

        Ok(format!(
            "[{}, {}, {}]",
            format_time(time),
            serde_json::to_string(&code)?,
            data,
        ))
    }
}

fn format_time(time: u64) -> String {
    let mut formatted_time = format!("{}.{:0>6}", time / 1_000_000, time % 1_000_000);
    let dot_idx = formatted_time.find('.').unwrap();

    for idx in (dot_idx + 2..=formatted_time.len() - 1).rev() {
        if formatted_time.as_bytes()[idx] != b'0' {
            break;
        }

        formatted_time.truncate(idx);
    }

    formatted_time
}

impl serde::Serialize for V3Header {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut len = 2;

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
        map.serialize_entry("version", &3)?;
        map.serialize_entry("term", &self.term)?;

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

impl serde::Serialize for V3Term {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut len = 2;

        if self.type_.is_some() {
            len += 1;
        }

        if self.version.is_some() {
            len += 1;
        }

        if self.theme.is_some() {
            len += 1;
        }

        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("cols", &self.cols)?;
        map.serialize_entry("rows", &self.rows)?;

        if let Some(type_) = &self.type_ {
            map.serialize_entry("type", &type_)?;
        }

        if let Some(version) = &self.version {
            map.serialize_entry("version", &version)?;
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

fn deserialize_palette<'de, D>(deserializer: D) -> Result<V3Palette, D::Error>
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

    Ok(V3Palette(colors))
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

impl serde::Serialize for V3Palette {
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

impl From<&Header> for V3Header {
    fn from(header: &Header) -> Self {
        V3Header {
            version: 3,
            term: V3Term {
                cols: header.term_cols,
                rows: header.term_rows,
                type_: header.term_type.clone(),
                version: header.term_version.clone(),
                theme: header.term_theme.as_ref().map(|t| t.into()),
            },
            timestamp: header.timestamp,
            idle_time_limit: header.idle_time_limit,
            command: header.command.clone(),
            title: header.title.clone(),
            env: header.env.clone(),
        }
    }
}

impl From<&TtyTheme> for V3Theme {
    fn from(theme: &TtyTheme) -> Self {
        let palette = theme.palette.iter().copied().map(RGB8).collect();

        V3Theme {
            fg: RGB8(theme.fg),
            bg: RGB8(theme.bg),
            palette: V3Palette(palette),
        }
    }
}

impl From<&V3Theme> for TtyTheme {
    fn from(theme: &V3Theme) -> Self {
        let palette = theme.palette.0.iter().map(|c| c.0).collect();

        TtyTheme {
            fg: theme.fg.0,
            bg: theme.bg.0,
            palette,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn format_time() {
        assert_eq!(super::format_time(0), "0.0");
        assert_eq!(super::format_time(1000001), "1.000001");
        assert_eq!(super::format_time(12300000), "12.3");
        assert_eq!(super::format_time(12000003), "12.000003");
    }
}
