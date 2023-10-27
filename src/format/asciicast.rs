use crate::asciicast;
use std::io::{self, Write};

pub struct Writer<W: Write> {
    inner: asciicast::Writer<W>,
}

impl<W> Writer<W>
where
    W: Write,
{
    pub fn new(writer: W, time_offset: f64) -> Self {
        Writer {
            inner: asciicast::Writer::new(writer, time_offset),
        }
    }
}

impl<W> super::Writer for Writer<W>
where
    W: Write,
{
    fn header(&mut self, size: (u16, u16)) -> io::Result<()> {
        let header = asciicast::Header {
            terminal_size: (size.0 as usize, size.1 as usize),
            idle_time_limit: None,
        };

        self.inner.write_header(&header)
    }

    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.inner.write_event(asciicast::Event::output(time, data))
    }

    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.inner.write_event(asciicast::Event::input(time, data))
    }
}
