use avt::util::TextCollector;

use crate::asciicast::{Event, EventData, Header};

pub struct TextEncoder {
    collector: Option<TextCollector>,
    pending_output: String,
    term_size: Option<(u16, u16)>,
}

impl TextEncoder {
    pub fn new() -> Self {
        TextEncoder {
            collector: None,
            pending_output: String::new(),
            term_size: None,
        }
    }

    fn init_collector(&self) -> TextCollector {
        let (cols, rows) = self
            .term_size
            .expect("text encoder header must be initialized");
        let vt = avt::Vt::builder()
            .size(cols as usize, rows as usize)
            .scrollback_limit(100)
            .build();

        TextCollector::new(vt)
    }

    fn encode_output(&mut self, data: &str) -> Vec<u8> {
        const CURSOR_HOME: &str = "\x1b[H";
        const CLEAR_SCREEN: &str = "\x1b[2J";
        const CLEAR_SCROLLBACK: &str = "\x1b[3J";
        const CLEAR_PREFIXES: [&str; 2] = ["\x1b[H\x1b[2J", "\x1b[2J"];

        let data = std::mem::take(&mut self.pending_output) + data;
        let mut bytes = Vec::new();
        let mut cursor = 0;

        while let Some(offset) = data[cursor..].find(CLEAR_SCREEN) {
            let clear_start = cursor + offset;
            let mut sequence_start = clear_start;
            if data[..clear_start].ends_with(CURSOR_HOME) {
                sequence_start -= CURSOR_HOME.len();
            }

            bytes.extend(text_lines_to_bytes(
                self.collector
                    .as_mut()
                    .unwrap()
                    .feed_str(&data[cursor..sequence_start]),
            ));
            bytes.extend(text_lines_to_bytes(
                self.collector.take().unwrap().flush().iter(),
            ));

            self.collector = Some(self.init_collector());

            cursor = clear_start + CLEAR_SCREEN.len();
            if data[cursor..].starts_with(CLEAR_SCROLLBACK) {
                cursor += CLEAR_SCROLLBACK.len();
            }
        }

        let pending_suffix_len = (1..=data[cursor..].len())
            .rev()
            .find(|len| {
                let suffix = &data[data.len() - len..];
                CLEAR_PREFIXES
                    .iter()
                    .any(|prefix| prefix.starts_with(suffix) && suffix.len() < prefix.len())
            })
            .unwrap_or(0);
        let feed_end = data.len() - pending_suffix_len;

        bytes.extend(text_lines_to_bytes(
            self.collector
                .as_mut()
                .unwrap()
                .feed_str(&data[cursor..feed_end]),
        ));
        self.pending_output = data[feed_end..].to_owned();

        bytes
    }
}

impl super::Encoder for TextEncoder {
    fn header(&mut self, header: &Header) -> Vec<u8> {
        self.term_size = Some((header.term_cols, header.term_rows));
        self.collector = Some(self.init_collector());

        Vec::new()
    }

    fn event(&mut self, event: Event) -> Vec<u8> {
        use EventData::*;

        match &event.data {
            Output(data) => self.encode_output(data),

            Resize(cols, rows) => {
                self.term_size = Some((*cols, *rows));
                text_lines_to_bytes(self.collector.as_mut().unwrap().resize(*cols, *rows))
            }

            _ => Vec::new(),
        }
    }

    fn flush(&mut self) -> Vec<u8> {
        let pending_output = std::mem::take(&mut self.pending_output);
        let mut bytes =
            text_lines_to_bytes(self.collector.as_mut().unwrap().feed_str(&pending_output));
        bytes.extend(text_lines_to_bytes(
            self.collector.take().unwrap().flush().iter(),
        ));

        bytes
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
    use std::time::Duration;

    use super::TextEncoder;
    use crate::asciicast::{Event, Header};
    use crate::encoder::Encoder;

    fn encode_output(events: &[&str]) -> Vec<u8> {
        let mut enc = TextEncoder::new();
        let mut output = Vec::new();
        let header = Header {
            term_cols: 80,
            term_rows: 24,
            ..Default::default()
        };

        assert!(enc.header(&header).is_empty());

        for (i, event) in events.iter().enumerate() {
            output.extend(enc.event(Event::output(
                Duration::from_micros(i as u64),
                (*event).to_owned(),
            )));
        }

        output.extend(enc.flush());

        output
    }

    #[test]
    fn encoder() {
        let mut enc = TextEncoder::new();

        let header = Header {
            term_cols: 3,
            term_rows: 1,
            ..Default::default()
        };

        assert!(enc.header(&header).is_empty());

        assert!(enc
            .event(Event::output(
                Duration::from_micros(0),
                "he\x1b[1mllo\r\n".to_owned()
            ))
            .is_empty());

        assert!(enc
            .event(Event::output(
                Duration::from_micros(1),
                "world\r\n".to_owned()
            ))
            .is_empty());

        assert_eq!(enc.flush(), "hello\nworld\n".as_bytes());
    }

    #[test]
    fn encoder_preserves_history_across_clear_screen() {
        let output = encode_output(&[
            "before clear\r\n",
            "\x1b[H\x1b[2J\x1b[3J",
            "after clear\r\n",
        ]);

        assert_eq!(output, "before clear\nafter clear\n".as_bytes());
    }

    #[test]
    fn encoder_preserves_history_across_ctrl_l_redraw() {
        let output = encode_output(&[
            "$ echo before clear\r\nbefore clear\r\n",
            "\x1b[H\x1b[2J$ echo after clear\r\nafter clear\r\n",
        ]);

        assert_eq!(
            output,
            "$ echo before clear\nbefore clear\n$ echo after clear\nafter clear\n".as_bytes()
        );
    }

    #[test]
    fn encoder_preserves_history_when_clear_sequence_spans_events() {
        let output = encode_output(&["before clear\r\n", "\x1b[H", "\x1b[2Jafter clear\r\n"]);

        assert_eq!(output, "before clear\nafter clear\n".as_bytes());
    }

    #[test]
    fn encoder_preserves_history_when_clear_sequence_splits_at_csi_prefix() {
        let output = encode_output(&["before clear\r\n", "\x1b[", "2Jafter clear\r\n"]);

        assert_eq!(output, "before clear\nafter clear\n".as_bytes());
    }

    #[test]
    fn encoder_preserves_history_when_clear_sequence_splits_inside_csi_argument() {
        let output = encode_output(&["before clear\r\n", "\x1b[2", "Jafter clear\r\n"]);

        assert_eq!(output, "before clear\nafter clear\n".as_bytes());
    }
}
