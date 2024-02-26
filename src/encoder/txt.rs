use crate::asciicast::{Event, EventData};
use crate::tty;
use avt::util::{TextCollector, TextCollectorOutput};
use std::io::{self, Write};

pub struct TextEncoder<W: Write> {
    writer: Option<W>,
    collector: Option<TextCollector<TextWriter<W>>>,
}

impl<W: Write> TextEncoder<W> {
    pub fn new(writer: W) -> Self {
        TextEncoder {
            writer: Some(writer),
            collector: None,
        }
    }
}

impl<W: Write> super::Encoder for TextEncoder<W> {
    fn start(&mut self, _timestamp: Option<u64>, tty_size: &tty::TtySize) -> io::Result<()> {
        let vt = avt::Vt::builder()
            .size(tty_size.0 as usize, tty_size.1 as usize)
            .resizable(true)
            .scrollback_limit(100)
            .build();

        self.collector = Some(TextCollector::new(
            vt,
            TextWriter(self.writer.take().unwrap()),
        ));

        Ok(())
    }

    fn event(&mut self, event: &Event) -> io::Result<()> {
        use EventData::*;

        match &event.data {
            Output(data) => self.collector.as_mut().unwrap().feed_str(data),
            Resize(cols, rows) => self.collector.as_mut().unwrap().resize(*cols, *rows),
            _ => Ok(()),
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        self.collector.as_mut().unwrap().flush()
    }
}

struct TextWriter<W: Write>(W);

impl<W: Write> TextCollectorOutput for TextWriter<W> {
    type Error = io::Error;

    fn push(&mut self, line: String) -> Result<(), Self::Error> {
        self.0.write_all(line.as_bytes())?;
        self.0.write_all(b"\n")
    }
}

#[cfg(test)]
mod tests {
    use super::TextEncoder;
    use crate::asciicast::Event;
    use crate::encoder::Encoder;
    use crate::tty::TtySize;

    #[test]
    fn encoder_impl() -> anyhow::Result<()> {
        let mut out: Vec<u8> = Vec::new();
        let mut enc = TextEncoder::new(&mut out);

        enc.start(None, &TtySize(3, 1))?;
        enc.event(&Event::output(0, "he\x1b[1mllo\r\n".to_owned()))?;
        enc.event(&Event::output(1, "world\r\n".to_owned()))?;
        enc.finish()?;

        assert_eq!(out, b"hello\nworld\n");

        Ok(())
    }
}
