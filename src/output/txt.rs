use crate::recorder;
use crate::tty;
use std::io::{self, Write};

pub struct Txt<W> {
    writer: W,
    vt: Option<avt::Vt>,
}

impl<W> Txt<W> {
    pub fn new(writer: W) -> Self {
        Txt { writer, vt: None }
    }
}

impl<W: Write> recorder::Output for Txt<W> {
    fn start(&mut self, _timestamp: u64, tty_size: &tty::TtySize) -> io::Result<()> {
        let (cols, rows) = (*tty_size).into();

        let vt = avt::Vt::builder()
            .size(cols as usize, rows as usize)
            .resizable(true)
            .build();

        self.vt = Some(vt);

        Ok(())
    }

    fn output(&mut self, _time: u64, data: &[u8]) -> io::Result<()> {
        let data = String::from_utf8_lossy(data).to_string();
        self.vt.as_mut().unwrap().feed_str(&data);

        Ok(())
    }

    fn input(&mut self, _time: u64, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn resize(&mut self, _time: u64, (cols, rows): (u16, u16)) -> io::Result<()> {
        self.vt
            .as_mut()
            .unwrap()
            .feed_str(&format!("\x1b[8;{rows};{cols}t"));

        Ok(())
    }

    fn marker(&mut self, _time: u64) -> io::Result<()> {
        Ok(())
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
    use super::Txt;
    use crate::recorder::Output;
    use crate::tty::TtySize;

    #[test]
    fn x() {
        let mut output: Vec<u8> = Vec::new();
        let mut txt = Txt::new(&mut output);

        txt.start(1706111685, &TtySize(3, 1)).unwrap();
        txt.output(0, b"he\x1b[1mllo\r\n").unwrap();
        txt.output(1, b"world\r\n").unwrap();
        txt.finish().unwrap();

        assert_eq!(output, b"hello\nworld\n");
    }
}
