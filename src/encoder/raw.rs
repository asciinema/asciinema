use crate::asciicast::{Event, EventData, Header};

pub struct RawEncoder;

impl RawEncoder {
    pub fn new() -> Self {
        RawEncoder
    }
}

impl super::Encoder for RawEncoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        format!("\x1b[8;{};{}t", header.term_rows, header.term_cols).into_bytes()
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        if let EventData::Output(data) = event.data {
            data.into_bytes()
        } else {
            Vec::new()
        }
    }

    fn flush(&mut self) -> Vec<u8> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RawEncoder;
    use crate::asciicast::{Event, Header};
    use crate::encoder::Encoder;

    #[test]
    fn encoder() {
        let mut enc = RawEncoder::new();

        let header = Header {
            term_cols: 100,
            term_rows: 50,
            ..Default::default()
        };

        assert_eq!(enc.header(&header), "\x1b[8;50;100t".as_bytes());

        assert_eq!(
            enc.event(Event::output(
                Duration::from_micros(0),
                "he\x1b[1mllo\r\n".to_owned()
            )),
            "he\x1b[1mllo\r\n".as_bytes()
        );

        assert_eq!(
            enc.event(Event::output(
                Duration::from_micros(1),
                "world\r\n".to_owned()
            )),
            "world\r\n".as_bytes()
        );

        assert!(enc
            .event(Event::input(Duration::from_micros(2), ".".to_owned()))
            .is_empty());
        assert!(enc
            .event(Event::resize(Duration::from_micros(3), (80, 24)))
            .is_empty());
        assert!(enc
            .event(Event::marker(Duration::from_micros(4), ".".to_owned()))
            .is_empty());
        assert!(enc.flush().is_empty());
    }
}
