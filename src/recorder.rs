use crate::pty;
use std::fs::{self, File};
use std::io::{self, Write};

pub struct Recorder {
    writer: Box<dyn FileWriter>,
    record_input: bool,
}

trait FileWriter {
    fn header(&mut self, size: (u16, u16)) -> io::Result<()>;
    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()>;
    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()>;
}

pub enum Format {
    Asciicast,
    Raw,
}

struct AsciicastWriter {
    file: File,
    append: bool,
}

struct RawWriter {
    file: File,
    append: bool,
}

pub fn new<S: Into<String>>(
    path: S,
    format: Format,
    append: bool,
    record_input: bool,
) -> io::Result<Recorder> {
    let path = path.into();

    let writer: Box<dyn FileWriter> = match format {
        Format::Asciicast => Box::new(AsciicastWriter::new(path, append)?),
        Format::Raw => Box::new(RawWriter::new(path, append)?),
    };

    Ok(Recorder {
        writer,
        record_input,
    })
}

impl AsciicastWriter {
    fn new(path: String, append: bool) -> io::Result<Self> {
        let file = File::create(path)?;

        Ok(Self { file, append })
    }
}

impl RawWriter {
    fn new(path: String, append: bool) -> io::Result<Self> {
        let file = fs::OpenOptions::new()
            .write(true)
            .append(append)
            .open(path)?;

        Ok(Self { file, append })
    }
}

impl FileWriter for AsciicastWriter {
    fn header(&mut self, size: (u16, u16)) -> io::Result<()> {
        todo!()
    }

    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        todo!()
    }

    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        todo!()
    }
}

impl FileWriter for RawWriter {
    fn header(&mut self, size: (u16, u16)) -> io::Result<()> {
        if self.append {
            Ok(())
        } else {
            write!(self.file, "\x1b[8;{};{}t", size.1, size.0)
        }
    }

    fn output(&mut self, _time: f64, data: &[u8]) -> io::Result<()> {
        self.file.write_all(data)
    }

    fn input(&mut self, _time: f64, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }
}

impl pty::Recorder for Recorder {
    fn start(&mut self, size: (u16, u16)) -> io::Result<()> {
        self.writer.header(size)
    }

    fn output(&mut self, data: &[u8]) {
        let _ = self.writer.output(0.0, data);
    }

    fn input(&mut self, data: &[u8]) {
        if self.record_input {
            let _ = self.writer.input(0.0, data);
        }
    }
}
