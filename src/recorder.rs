use crate::format;
use crate::pty;
use std::io;
use std::time;

pub struct Recorder {
    writer: Box<dyn format::Writer>,
    append: bool,
    record_input: bool,
    start_time: time::Instant,
}

impl Recorder {
    pub fn new(writer: Box<dyn format::Writer>, append: bool, record_input: bool) -> Self {
        Recorder {
            writer,
            append,
            record_input,
            start_time: time::Instant::now(),
        }
    }

    fn elapsed_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

impl pty::Recorder for Recorder {
    fn start(&mut self, size: (u16, u16)) -> io::Result<()> {
        self.start_time = time::Instant::now();

        if !self.append {
            self.writer.header(size)
        } else {
            Ok(())
        }
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
