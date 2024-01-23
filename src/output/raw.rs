use crate::recorder;
use crate::tty;
use std::io::{self, Write};

pub struct Raw<W> {
    writer: W,
    append: bool,
}

impl<W> Raw<W> {
    pub fn new(writer: W, append: bool) -> Self {
        Raw { writer, append }
    }
}

impl<W: Write> recorder::Output for Raw<W> {
    fn start(&mut self, _timestamp: u64, tty_size: &tty::TtySize) -> io::Result<()> {
        if self.append {
            Ok(())
        } else {
            let (cols, rows): (u16, u16) = (*tty_size).into();

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
