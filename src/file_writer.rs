use std::io::{self, Write};
use std::time::UNIX_EPOCH;

use crate::asciicast;
use crate::encoder;
use crate::notifier::Notifier;
use crate::session::{self, Metadata};

pub struct FileWriter {
    writer: Box<dyn Write + Send>,
    encoder: Box<dyn encoder::Encoder + Send>,
    notifier: Box<dyn Notifier>,
    metadata: Metadata,
}

pub struct LiveFileWriter {
    writer: Box<dyn Write + Send>,
    encoder: Box<dyn encoder::Encoder + Send>,
    notifier: Box<dyn Notifier>,
}

impl FileWriter {
    pub fn new(
        writer: Box<dyn Write + Send>,
        encoder: Box<dyn encoder::Encoder + Send>,
        notifier: Box<dyn Notifier>,
        metadata: Metadata,
    ) -> Self {
        FileWriter {
            writer,
            encoder,
            notifier,
            metadata,
        }
    }

    pub fn start(mut self) -> io::Result<LiveFileWriter> {
        let timestamp = self
            .metadata
            .time
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let header = asciicast::Header {
            term_cols: self.metadata.term.size.0,
            term_rows: self.metadata.term.size.1,
            term_type: self.metadata.term.type_.clone(),
            term_version: self.metadata.term.version.clone(),
            term_theme: self.metadata.term.theme.clone(),
            timestamp: Some(timestamp),
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.as_ref().cloned(),
            title: self.metadata.title.as_ref().cloned(),
            env: Some(self.metadata.env.clone()),
        };

        if let Err(e) = self.writer.write_all(&self.encoder.header(&header)) {
            let _ = self
                .notifier
                .notify("Write error, session won't be recorded".to_owned());

            return Err(e);
        }

        Ok(LiveFileWriter {
            writer: self.writer,
            encoder: self.encoder,
            notifier: self.notifier,
        })
    }
}

impl session::Output for LiveFileWriter {
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
            session::Event::Output(time, text) => asciicast::Event::output(time, text),
            session::Event::Input(time, text) => asciicast::Event::input(time, text),
            session::Event::Resize(time, tty_size) => {
                asciicast::Event::resize(time, tty_size.into())
            }
            session::Event::Marker(time, label) => asciicast::Event::marker(time, label),
            session::Event::Exit(time, status) => asciicast::Event::exit(time, status),
        }
    }
}
