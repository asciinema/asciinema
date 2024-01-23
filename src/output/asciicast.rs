use crate::asciicast::{Event, Header, Writer};
use crate::recorder;
use crate::tty;
use std::collections::HashMap;
use std::io::{self, Write};

pub struct Asciicast<W: Write> {
    writer: Writer<W>,
    append: bool,
    metadata: Metadata,
}

pub struct Metadata {
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: HashMap<String, String>,
}

impl<W> Asciicast<W>
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

    fn build_header(&self, timestamp: u64, tty_size: &tty::TtySize) -> Header {
        let (width, height) = (*tty_size).into();

        Header {
            version: 2,
            width,
            height,
            timestamp: Some(timestamp),
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.clone(),
            title: self.metadata.title.clone(),
            env: Some(self.metadata.env.clone()),
        }
    }
}

impl<W> recorder::Output for Asciicast<W>
where
    W: Write,
{
    fn start(&mut self, timestamp: u64, tty_size: &tty::TtySize) -> io::Result<()> {
        if self.append {
            Ok(())
        } else {
            let header = self.build_header(timestamp, tty_size);

            self.writer.write_header(&header)
        }
    }

    fn output(&mut self, time: u64, data: &[u8]) -> io::Result<()> {
        self.writer.write_event(Event::output(time, data))
    }

    fn input(&mut self, time: u64, data: &[u8]) -> io::Result<()> {
        self.writer.write_event(Event::input(time, data))
    }

    fn resize(&mut self, time: u64, size: (u16, u16)) -> io::Result<()> {
        self.writer.write_event(Event::resize(time, size))
    }

    fn marker(&mut self, time: u64) -> io::Result<()> {
        self.writer.write_event(Event::marker(time))
    }
}
