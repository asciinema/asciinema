use crate::asciicast::{Event, Header, Writer};
use crate::tty;
use std::collections::HashMap;
use std::io::{self, Write};

pub struct AsciicastEncoder<W: Write> {
    writer: Writer<W>,
    append: bool,
    metadata: Metadata,
}

pub struct Metadata {
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl<W> AsciicastEncoder<W>
where
    W: Write,
{
    pub fn new(writer: W, append: bool, time_offset: u64, metadata: Metadata) -> Self {
        Self {
            writer: Writer::new(writer, time_offset),
            append,
            metadata,
        }
    }

    fn build_header(&self, timestamp: Option<u64>, tty_size: &tty::TtySize) -> Header {
        Header {
            version: 2,
            cols: tty_size.0,
            rows: tty_size.1,
            timestamp,
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.clone(),
            title: self.metadata.title.clone(),
            env: self.metadata.env.clone(),
        }
    }
}

impl<W> super::Encoder for AsciicastEncoder<W>
where
    W: Write,
{
    fn start(&mut self, timestamp: Option<u64>, tty_size: &tty::TtySize) -> io::Result<()> {
        if self.append {
            Ok(())
        } else {
            let header = self.build_header(timestamp, tty_size);

            self.writer.write_header(&header)
        }
    }

    fn event(&mut self, event: &Event) -> io::Result<()> {
        self.writer.write_event(event)
    }
}

impl From<&Header> for Metadata {
    fn from(header: &Header) -> Self {
        Metadata {
            idle_time_limit: header.idle_time_limit.as_ref().cloned(),
            command: header.command.as_ref().cloned(),
            title: header.title.as_ref().cloned(),
            env: header.env.as_ref().cloned(),
        }
    }
}
