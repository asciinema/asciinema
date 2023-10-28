use std::io::{self, Write};

pub struct Writer<W> {
    writer: W,
}

impl<W> Writer<W> {
    pub fn new(writer: W) -> Self {
        Writer { writer }
    }
}

impl<W: Write> super::Writer for Writer<W> {
    fn header(
        &mut self,
        size: (u16, u16),
        _timestamp: u64,
        _idle_time_limit: Option<f32>,
    ) -> io::Result<()> {
        write!(self.writer, "\x1b[8;{};{}t", size.1, size.0)
    }

    fn output(&mut self, _time: f64, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)
    }

    fn input(&mut self, _time: f64, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }
}
