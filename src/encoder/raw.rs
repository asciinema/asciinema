use crate::asciicast::{Event, EventData};
use crate::tty;

pub struct RawEncoder {
    append: bool,
}

impl RawEncoder {
    pub fn new(append: bool) -> Self {
        RawEncoder { append }
    }
}

impl super::Encoder for RawEncoder {
    fn start(&mut self, _timestamp: Option<u64>, tty_size: tty::TtySize) -> Vec<u8> {
        if self.append {
            Vec::new()
        } else {
            format!("\x1b[8;{};{}t", tty_size.1, tty_size.0).into_bytes()
        }
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        if let EventData::Output(data) = event.data {
            data.into_bytes()
        } else {
            Vec::new()
        }
    }

    fn finish(&mut self) -> Vec<u8> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::RawEncoder;
    use crate::asciicast::Event;
    use crate::encoder::Encoder;
    use crate::tty::TtySize;

    #[test]
    fn encoder() {
        let mut enc = RawEncoder::new(false);

        assert_eq!(
            enc.start(None, TtySize(100, 50)),
            "\x1b[8;50;100t".as_bytes()
        );

        assert_eq!(
            enc.event(Event::output(0, "he\x1b[1mllo\r\n".to_owned())),
            "he\x1b[1mllo\r\n".as_bytes()
        );

        assert_eq!(
            enc.event(Event::output(1, "world\r\n".to_owned())),
            "world\r\n".as_bytes()
        );

        assert!(enc.event(Event::input(2, ".".to_owned())).is_empty());
        assert!(enc.event(Event::resize(3, (80, 24))).is_empty());
        assert!(enc.event(Event::marker(4, ".".to_owned())).is_empty());
        assert!(enc.finish().is_empty());
    }
}
