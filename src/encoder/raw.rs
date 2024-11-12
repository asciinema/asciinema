use crate::asciicast::{Event, EventData, Header};

pub struct RawEncoder {
    append: bool,
}

impl RawEncoder {
    pub fn new(append: bool) -> Self {
        RawEncoder { append }
    }
}

impl super::Encoder for RawEncoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        if self.append {
            Vec::new()
        } else {
            format!("\x1b[8;{};{}t", header.rows, header.cols).into_bytes()
        }
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
    use super::RawEncoder;
    use crate::asciicast::{Event, Header};
    use crate::encoder::Encoder;

    #[test]
    fn encoder() {
        let mut enc = RawEncoder::new(false);

        let header = Header {
            cols: 100,
            rows: 50,
            ..Default::default()
        };

        assert_eq!(enc.header(&header), "\x1b[8;50;100t".as_bytes());

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
        assert!(enc.flush().is_empty());
    }
}
