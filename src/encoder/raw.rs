use crate::asciicast::{Event, EventData};
use crate::tty;
use std::io::{self, Write};

pub struct RawEncoder<W> {
    writer: W,
    append: bool,
}

impl<W> RawEncoder<W> {
    pub fn new(writer: W, append: bool) -> Self {
        RawEncoder { writer, append }
    }
}

impl<W: Write> super::Encoder for RawEncoder<W> {
    fn start(&mut self, _timestamp: Option<u64>, tty_size: &tty::TtySize) -> io::Result<()> {
        if self.append {
            Ok(())
        } else {
            write!(self.writer, "\x1b[8;{};{}t", tty_size.1, tty_size.0)
        }
    }

    fn event(&mut self, event: &Event) -> io::Result<()> {
        if let EventData::Output(data) = &event.data {
            self.writer.write_all(data.as_bytes())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RawEncoder;
    use crate::asciicast::Event;
    use crate::encoder::Encoder;
    use crate::tty::TtySize;

    #[test]
    fn encoder_impl() -> anyhow::Result<()> {
        let mut out: Vec<u8> = Vec::new();
        let mut enc = RawEncoder::new(&mut out, false);

        enc.start(None, &TtySize(100, 50))?;
        enc.event(&Event::output(0, "he\x1b[1mllo\r\n".to_owned()))?;
        enc.event(&Event::output(1, "world\r\n".to_owned()))?;
        enc.event(&Event::input(2, ".".to_owned()))?;
        enc.event(&Event::resize(3, (80, 24)))?;
        enc.event(&Event::marker(4, ".".to_owned()))?;
        enc.finish()?;

        assert_eq!(out, b"\x1b[8;50;100the\x1b[1mllo\r\nworld\r\n");

        Ok(())
    }
}
