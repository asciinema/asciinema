use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::asciicast;
use crate::encoder;
use crate::session;
use crate::tty;

pub struct FileWriter {
    pub writer: Box<dyn Write + Send>,
    pub encoder: Box<dyn encoder::Encoder + Send>,
    pub metadata: Metadata,
}

pub struct Metadata {
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl session::Output for FileWriter {
    fn start(
        &mut self,
        time: SystemTime,
        tty_size: tty::TtySize,
        theme: Option<tty::Theme>,
    ) -> io::Result<()> {
        let timestamp = time.duration_since(UNIX_EPOCH).unwrap().as_secs();

        let header = asciicast::Header {
            cols: tty_size.0,
            rows: tty_size.1,
            timestamp: Some(timestamp),
            theme,
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.as_ref().cloned(),
            title: self.metadata.title.as_ref().cloned(),
            env: self.metadata.env.as_ref().cloned(),
        };

        self.writer.write_all(&self.encoder.header(&header))
    }

    fn event(&mut self, event: session::Event) -> io::Result<()> {
        self.writer.write_all(&self.encoder.event(event.into()))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.write_all(&self.encoder.flush())
    }
}

impl From<session::Event> for asciicast::Event {
    fn from(event: session::Event) -> Self {
        match event {
            session::Event::Output(time, text) => asciicast::Event::output(time, text),
            session::Event::Input(time, text) => asciicast::Event::input(time, text),
            session::Event::Resize(time, tty_size) => {
                asciicast::Event::resize(time, tty_size.into())
            }
            session::Event::Marker(time, label) => asciicast::Event::marker(time, label),
        }
    }
}
