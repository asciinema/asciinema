use crate::asciicast::{Event, EventData, Header};
use avt::util::TextCollector;

pub struct TextEncoder {
    collector: Option<TextCollector>,
}

impl TextEncoder {
    pub fn new() -> Self {
        TextEncoder { collector: None }
    }
}

impl super::Encoder for TextEncoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        let vt = avt::Vt::builder()
            .size(header.cols as usize, header.rows as usize)
            .scrollback_limit(100)
            .build();

        self.collector = Some(TextCollector::new(vt));

        Vec::new()
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        use EventData::*;

        match &event.data {
            Output(data) => text_lines_to_bytes(self.collector.as_mut().unwrap().feed_str(data)),

            Resize(cols, rows) => {
                text_lines_to_bytes(self.collector.as_mut().unwrap().resize(*cols, *rows))
            }

            _ => Vec::new(),
        }
    }

    fn flush(&mut self) -> Vec<u8> {
        text_lines_to_bytes(self.collector.take().unwrap().flush().iter())
    }
}

fn text_lines_to_bytes<S: AsRef<str>>(lines: impl Iterator<Item = S>) -> Vec<u8> {
    lines.fold(Vec::new(), |mut bytes, line| {
        bytes.extend_from_slice(line.as_ref().as_bytes());
        bytes.push(b'\n');

        bytes
    })
}

#[cfg(test)]
mod tests {
    use super::TextEncoder;
    use crate::asciicast::{Event, Header};
    use crate::encoder::Encoder;

    #[test]
    fn encoder() {
        let mut enc = TextEncoder::new();

        let header = Header {
            cols: 3,
            rows: 1,
            ..Default::default()
        };

        assert!(enc.header(&header).is_empty());

        assert!(enc
            .event(Event::output(0, "he\x1b[1mllo\r\n".to_owned()))
            .is_empty());

        assert!(enc
            .event(Event::output(1, "world\r\n".to_owned()))
            .is_empty());

        assert_eq!(enc.flush(), "hello\nworld\n".as_bytes());
    }
}
