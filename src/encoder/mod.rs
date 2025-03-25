mod asciicast;
mod raw;
mod txt;

use std::fs::File;
use std::io::Write;

use anyhow::Result;

use crate::asciicast::Event;
use crate::asciicast::Header;
pub use asciicast::AsciicastEncoder;
pub use raw::RawEncoder;
pub use txt::TextEncoder;

pub trait Encoder {
    fn header(&mut self, header: &Header) -> Vec<u8>;
    fn event(&mut self, event: Event) -> Vec<u8>;
    fn flush(&mut self) -> Vec<u8>;
}

pub trait EncoderExt {
    fn encode_to_file(&mut self, cast: crate::asciicast::Asciicast, file: &mut File) -> Result<()>;
}

impl<E: Encoder + ?Sized> EncoderExt for E {
    fn encode_to_file(&mut self, cast: crate::asciicast::Asciicast, file: &mut File) -> Result<()> {
        file.write_all(&self.header(&cast.header))?;

        for event in cast.events {
            file.write_all(&self.event(event?))?;
        }

        file.write_all(&self.flush())?;

        Ok(())
    }
}
