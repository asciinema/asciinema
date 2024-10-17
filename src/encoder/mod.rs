mod asciicast;
mod raw;
mod txt;

pub use asciicast::AsciicastEncoder;
pub use asciicast::Metadata;
pub use raw::RawEncoder;
pub use txt::TextEncoder;

use crate::asciicast::Event;
use crate::tty;
use anyhow::Result;
use std::fs::File;
use std::io::Write;

pub trait Encoder {
    fn start(&mut self, timestamp: Option<u64>, tty_size: tty::TtySize) -> Vec<u8>;
    fn event(&mut self, event: Event) -> Vec<u8>;
    fn finish(&mut self) -> Vec<u8>;
}

pub trait EncoderExt {
    fn encode_to_file(&mut self, cast: crate::asciicast::Asciicast, file: &mut File) -> Result<()>;
}

impl<E: Encoder + ?Sized> EncoderExt for E {
    fn encode_to_file(&mut self, cast: crate::asciicast::Asciicast, file: &mut File) -> Result<()> {
        let tty_size = tty::TtySize(cast.header.cols, cast.header.rows);
        file.write_all(&self.start(cast.header.timestamp, tty_size))?;

        for event in cast.events {
            file.write_all(&self.event(event?))?;
        }

        file.write_all(&self.finish())?;

        Ok(())
    }
}
