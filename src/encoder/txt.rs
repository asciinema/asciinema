use crate::asciicast::{Event, EventData};
use crate::tty;
use std::io::{self, Write};

pub struct TxtEncoder<W> {
    writer: W,
    vt: Option<avt::Vt>,
}

impl<W> TxtEncoder<W> {
    pub fn new(writer: W) -> Self {
        TxtEncoder { writer, vt: None }
    }
}

impl<W: Write> super::Encoder for TxtEncoder<W> {
    fn start(&mut self, _timestamp: Option<u64>, tty_size: &tty::TtySize) -> io::Result<()> {
        let vt = avt::Vt::builder()
            .size(tty_size.0 as usize, tty_size.1 as usize)
            .resizable(true)
            .build();

        self.vt = Some(vt);

        Ok(())
    }

    fn event(&mut self, event: &Event) -> io::Result<()> {
        match &event.data {
            EventData::Output(data) => {
                self.vt.as_mut().unwrap().feed_str(data);

                Ok(())
            }

            EventData::Resize(cols, rows) => {
                self.vt
                    .as_mut()
                    .unwrap()
                    .feed_str(&format!("\x1b[8;{rows};{cols}t"));

                Ok(())
            }

            _ => Ok(()),
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        let mut text = self.vt.as_ref().unwrap().text();

        while !text.is_empty() && text[text.len() - 1].is_empty() {
            text.truncate(text.len() - 1);
        }

        for line in text {
            self.writer.write_all(line.as_bytes())?;
            self.writer.write_all(b"\n")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::TxtEncoder;
    use crate::asciicast::Event;
    use crate::encoder::Encoder;
    use crate::tty::TtySize;

    #[test]
    fn encoder_impl() {
        let mut out: Vec<u8> = Vec::new();
        let mut enc = TxtEncoder::new(&mut out);

        enc.start(None, &TtySize(3, 1)).unwrap();
        enc.event(&Event::output(0, b"he\x1b[1mllo\r\n")).unwrap();
        enc.event(&Event::output(1, b"world\r\n")).unwrap();
        enc.finish().unwrap();

        assert_eq!(out, b"hello\nworld\n");
    }
}
