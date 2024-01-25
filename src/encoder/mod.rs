mod asciicast;
mod raw;
mod txt;

pub use asciicast::AsciicastEncoder;
pub use asciicast::Metadata;
pub use raw::RawEncoder;
pub use txt::TxtEncoder;

use crate::asciicast::Event;
use crate::recorder;
use crate::tty;
use anyhow::Result;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait Encoder {
    fn start(&mut self, timestamp: Option<u64>, tty_size: &tty::TtySize) -> io::Result<()>;
    fn event(&mut self, event: &Event) -> io::Result<()>;

    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub trait EncoderExt {
    fn encode(&mut self, recording: crate::asciicast::Asciicast) -> Result<()>;
}

impl<E: Encoder + ?Sized> EncoderExt for E {
    fn encode(&mut self, recording: crate::asciicast::Asciicast) -> Result<()> {
        let tty_size = tty::TtySize(recording.header.cols, recording.header.rows);
        self.start(recording.header.timestamp, &tty_size)?;

        for event in recording.events {
            self.event(&event?)?;
        }

        self.finish()?;

        Ok(())
    }
}

impl<E: Encoder> recorder::Output for E {
    fn start(&mut self, tty_size: &tty::TtySize) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.start(Some(timestamp), tty_size)
    }

    fn output(&mut self, time: u64, data: &[u8]) -> io::Result<()> {
        self.event(&Event::output(time, data))
    }

    fn input(&mut self, time: u64, data: &[u8]) -> io::Result<()> {
        self.event(&Event::input(time, data))
    }

    fn resize(&mut self, time: u64, size: (u16, u16)) -> io::Result<()> {
        self.event(&Event::resize(time, size))
    }

    fn marker(&mut self, time: u64) -> io::Result<()> {
        self.event(&Event::marker(time, "".to_owned()))
    }

    fn finish(&mut self) -> io::Result<()> {
        self.finish()
    }
}
