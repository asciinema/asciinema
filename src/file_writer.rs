use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::asciicast;
use crate::encoder;
use crate::notifier::Notifier;
use crate::session;
use crate::tty::{TtySize, TtyTheme};

pub struct FileWriterStarter {
    pub writer: Box<dyn Write + Send>,
    pub encoder: Box<dyn encoder::Encoder + Send>,
    pub metadata: Metadata,
    pub notifier: Box<dyn Notifier>,
}

pub struct FileWriter {
    pub writer: Box<dyn Write + Send>,
    pub encoder: Box<dyn encoder::Encoder + Send>,
    pub notifier: Box<dyn Notifier>,
}

pub struct Metadata {
    pub term_type: Option<String>,
    pub term_version: Option<String>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl session::OutputStarter for FileWriterStarter {
    fn start(
        mut self: Box<Self>,
        time: SystemTime,
        tty_size: TtySize,
        theme: Option<TtyTheme>,
        child_pid: u32,
    ) -> io::Result<Box<dyn session::Output>> {
        let timestamp = time.duration_since(UNIX_EPOCH).unwrap().as_secs();

        let header = asciicast::Header {
            term_cols: tty_size.0,
            term_rows: tty_size.1,
            term_type: self.metadata.term_type,
            term_version: self.metadata.term_version,
            term_theme: theme,
            timestamp: Some(timestamp),
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.as_ref().cloned(),
            title: self.metadata.title.as_ref().cloned(),
            env: self.metadata.env.as_ref().cloned(),
            child_pid: Some(child_pid),
        };

        if let Err(e) = self.writer.write_all(&self.encoder.header(&header)) {
            let _ = self
                .notifier
                .notify("Write error, session won't be recorded".to_owned());

            return Err(e);
        }

        Ok(Box::new(FileWriter {
            writer: self.writer,
            encoder: self.encoder,
            notifier: self.notifier,
        }))
    }
}

impl session::Output for FileWriter {
    fn event(&mut self, event: session::Event) -> io::Result<()> {
        match self.writer.write_all(&self.encoder.event(event.into())) {
            Ok(_) => Ok(()),

            Err(e) => {
                let _ = self
                    .notifier
                    .notify("Write error, recording suspended".to_owned());

                Err(e)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.write_all(&self.encoder.flush())
    }
}

impl From<session::Event> for asciicast::Event {
    fn from(event: session::Event) -> Self {
        match event {
            session::Event::Output(time, text, pid) => asciicast::Event::output(time, text, pid),
            session::Event::Input(time, text, pid) => asciicast::Event::input(time, text, pid),
            session::Event::Resize(time, tty_size, pid) =>
                asciicast::Event::resize(time, tty_size.into(), pid),
            session::Event::Marker(time, label, pid) => asciicast::Event::marker(time, label, pid),
        }
    }
}
