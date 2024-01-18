use crate::recorder;
use std::io::{self, Write};

pub struct Writer<W> {
    writer: W,
    append: bool,
}

impl<W> Writer<W> {
    pub fn new(writer: W, append: bool) -> Self {
        Writer { writer, append }
    }
}

impl<W: Write> recorder::EventWriter for Writer<W> {
    fn start(&mut self, header: &recorder::Header) -> io::Result<()> {
        if self.append {
            Ok(())
        } else {
            let (cols, rows) = header.tty_size;
            write!(self.writer, "\x1b[8;{rows};{cols}t")
        }
    }

    fn output(&mut self, _time: u64, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)
    }

    fn input(&mut self, _time: u64, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn resize(&mut self, _time: u64, _size: (u16, u16)) -> io::Result<()> {
        Ok(())
    }

    fn marker(&mut self, _time: u64) -> io::Result<()> {
        Ok(())
    }
}
