use crate::asciicast;
use crate::pty;
use std::fs::{self, File};
use std::io::{self, Write};
use std::time;

pub struct Recorder {
    writer: Box<dyn FileWriter>,
    record_input: bool,
    start_time: time::Instant,
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

struct RawWriter {
    file: File,
    append: bool,
}

pub fn new<S: Into<String>>(
    path: S,
    format: Format,
    append: bool,
    record_input: bool,
) -> anyhow::Result<Recorder> {
    let path = path.into();

    let writer: Box<dyn FileWriter> = match format {
        Format::Asciicast => Box::new(asciicast::Writer::new(path, append)?),
        Format::Raw => Box::new(RawWriter::new(path, append)?),
    };

    Ok(Recorder {
        writer,
        record_input,
        start_time: time::Instant::now(),
    })
}

impl Recorder {
    fn elapsed_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

impl pty::Recorder for Recorder {
    fn start(&mut self, size: (u16, u16)) -> io::Result<()> {
        self.start_time = time::Instant::now();
        self.writer.header(size)
    }

    fn output(&mut self, data: &[u8]) {
        let _ = self.writer.output(self.elapsed_time(), data);
        // TODO use notifier for error reporting
    }

    fn input(&mut self, data: &[u8]) {
        if self.record_input {
            let _ = self.writer.input(self.elapsed_time(), data);
            // TODO use notifier for error reporting
        }
    }
}

impl FileWriter for asciicast::Writer {
    fn header(&mut self, size: (u16, u16)) -> io::Result<()> {
        let header = asciicast::Header {
            terminal_size: (size.0 as usize, size.1 as usize),
            idle_time_limit: None,
        };

        self.write_header(&header)
    }

    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.write_event(asciicast::Event::output(time, data))
    }

    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()> {
        self.write_event(asciicast::Event::input(time, data))
    }
}

impl RawWriter {
    fn new(path: String, append: bool) -> io::Result<Self> {
        let mut opts = fs::OpenOptions::new();

        if append {
            opts.append(true);
        } else {
            opts.create_new(true).write(true);
        }

        let file = opts.open(path)?;

        Ok(Self { file, append })
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
