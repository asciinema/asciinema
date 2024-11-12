use crate::asciicast::{Encoder, Event, Header};

pub struct AsciicastEncoder {
    inner: Encoder,
    append: bool,
}

impl AsciicastEncoder {
    pub fn new(append: bool, time_offset: u64) -> Self {
        let inner = Encoder::new(time_offset);

        Self { inner, append }
    }
}

impl super::Encoder for AsciicastEncoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        if self.append {
            Vec::new()
        } else {
            self.inner.header(header)
        }
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        self.inner.event(&event)
    }

    fn flush(&mut self) -> Vec<u8> {
        Vec::new()
    }
}
