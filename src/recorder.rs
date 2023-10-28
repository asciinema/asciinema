use crate::format;
use crate::pty;
use std::io;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct Recorder {
    writer: Box<dyn format::Writer>,
    start_time: Instant,
    append: bool,
    record_input: bool,
    idle_time_limit: Option<f32>,
    command: Option<String>,
    title: Option<String>,
}

impl Recorder {
    pub fn new(
        writer: Box<dyn format::Writer>,
        append: bool,
        record_input: bool,
        idle_time_limit: Option<f32>,
        command: Option<String>,
        title: Option<String>,
    ) -> Self {
        Recorder {
            writer,
            start_time: Instant::now(),
            append,
            record_input,
            idle_time_limit,
            command,
            title,
        }
    }

    fn elapsed_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

impl pty::Recorder for Recorder {
    fn start(&mut self, size: (u16, u16)) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.start_time = Instant::now();

        if !self.append {
            let header = format::Header {
                cols: size.0,
                rows: size.1,
                timestamp,
                idle_time_limit: self.idle_time_limit,
                command: self.command.clone(),
                title: self.title.clone(),
            };

            self.writer.header(&header)
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
