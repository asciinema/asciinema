use crate::asciicast::{Encoder, Event, Header};
use crate::tty;
use std::collections::HashMap;

pub struct AsciicastEncoder {
    inner: Encoder,
    append: bool,
    metadata: Metadata,
}

pub struct Metadata {
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub theme: Option<tty::Theme>,
}

impl AsciicastEncoder {
    pub fn new(append: bool, time_offset: u64, metadata: Metadata) -> Self {
        let inner = Encoder::new(time_offset);

        Self {
            inner,
            append,
            metadata,
        }
    }

    fn build_header(&self, timestamp: Option<u64>, tty_size: &tty::TtySize) -> Header {
        Header {
            cols: tty_size.0,
            rows: tty_size.1,
            timestamp,
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.clone(),
            title: self.metadata.title.clone(),
            env: self.metadata.env.clone(),
            theme: self.metadata.theme.clone(),
        }
    }
}

impl super::Encoder for AsciicastEncoder {
    fn start(&mut self, timestamp: Option<u64>, tty_size: tty::TtySize) -> Vec<u8> {
        if self.append {
            Vec::new()
        } else {
            self.inner.header(&self.build_header(timestamp, &tty_size))
        }
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        self.inner.event(&event)
    }

    fn finish(&mut self) -> Vec<u8> {
        Vec::new()
    }
}

impl From<&Header> for Metadata {
    fn from(header: &Header) -> Self {
        Metadata {
            idle_time_limit: header.idle_time_limit.as_ref().cloned(),
            command: header.command.as_ref().cloned(),
            title: header.title.as_ref().cloned(),
            env: header.env.as_ref().cloned(),
            theme: header.theme.as_ref().cloned(),
        }
    }
}
