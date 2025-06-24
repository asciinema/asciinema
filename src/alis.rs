// This module implements ALiS (asciinema live stream) protocol,
// which is an application level protocol built on top of WebSocket binary messages,
// used by asciinema CLI, asciinema player and asciinema server.

// TODO document the protocol when it's final

use std::future;

use futures_util::{stream, Stream, StreamExt};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use crate::leb128;
use crate::stream::Event;

static MAGIC_STRING: &str = "ALiS\x01";

struct EventSerializer(u64);

pub fn stream<S: Stream<Item = Result<Event, BroadcastStreamRecvError>>>(
    stream: S,
) -> impl Stream<Item = Result<Vec<u8>, BroadcastStreamRecvError>> {
    let header = stream::once(future::ready(Ok(MAGIC_STRING.into())));
    let mut serializer = EventSerializer(0);
    let events = stream.map(move |event| event.map(|event| serializer.serialize_event(event)));

    header.chain(events)
}

impl EventSerializer {
    fn serialize_event(&mut self, event: Event) -> Vec<u8> {
        use Event::*;

        match event {
            Init(last_id, time, size, theme, init) => {
                let last_id_bytes = leb128::encode(last_id);
                let time_bytes = leb128::encode(time);
                let cols_bytes = leb128::encode(size.0);
                let rows_bytes = leb128::encode(size.1);
                let init_len = init.len() as u32;
                let init_len_bytes = leb128::encode(init_len);

                let mut msg = vec![0x01];
                msg.extend_from_slice(&last_id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&cols_bytes);
                msg.extend_from_slice(&rows_bytes);

                match theme {
                    Some(theme) => {
                        msg.push(16);
                        msg.push(theme.fg.r);
                        msg.push(theme.fg.g);
                        msg.push(theme.fg.b);
                        msg.push(theme.bg.r);
                        msg.push(theme.bg.g);
                        msg.push(theme.bg.b);

                        for color in &theme.palette {
                            msg.push(color.r);
                            msg.push(color.g);
                            msg.push(color.b);
                        }
                    }

                    None => {
                        msg.push(0);
                    }
                }

                msg.extend_from_slice(&init_len_bytes);
                msg.extend_from_slice(init.as_bytes());

                self.0 = time;

                msg
            }

            Output(id, time, text) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(self.rel_time(time));
                let text_len = text.len() as u32;
                let text_len_bytes = leb128::encode(text_len);

                let mut msg = vec![b'o'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&text_len_bytes);
                msg.extend_from_slice(text.as_bytes());

                msg
            }

            Input(id, time, text) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(self.rel_time(time));
                let text_len = text.len() as u32;
                let text_len_bytes = leb128::encode(text_len);

                let mut msg = vec![b'i'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&text_len_bytes);
                msg.extend_from_slice(text.as_bytes());

                msg
            }

            Resize(id, time, size) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(self.rel_time(time));
                let cols_bytes = leb128::encode(size.0);
                let rows_bytes = leb128::encode(size.1);

                let mut msg = vec![b'r'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&cols_bytes);
                msg.extend_from_slice(&rows_bytes);

                msg
            }

            Marker(id, time, text) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(self.rel_time(time));
                let text_len = text.len() as u32;
                let text_len_bytes = leb128::encode(text_len);

                let mut msg = vec![b'm'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&text_len_bytes);
                msg.extend_from_slice(text.as_bytes());

                msg
            }

            Exit(id, time, status) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(self.rel_time(time));
                let status_bytes = leb128::encode(status.max(0) as u64);

                let mut msg = vec![b'x'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&status_bytes);

                msg
            }
        }
    }

    fn rel_time(&mut self, time: u64) -> u64 {
        let time = time.max(self.0);
        let rel_time = time - self.0;
        self.0 = time;

        rel_time
    }
}

#[cfg(test)]
mod tests {
    use rgb::RGB8;

    use super::*;
    use crate::tty::{TtySize, TtyTheme};

    #[test]
    fn test_serialize_init_with_theme_and_seed() {
        let mut serializer = EventSerializer(0);

        let theme = TtyTheme {
            fg: rgb(255, 255, 255),
            bg: rgb(0, 0, 0),
            palette: vec![
                rgb(0, 0, 0),       // Black
                rgb(128, 0, 0),     // Dark Red
                rgb(0, 128, 0),     // Dark Green
                rgb(128, 128, 0),   // Dark Yellow
                rgb(0, 0, 128),     // Dark Blue
                rgb(128, 0, 128),   // Dark Magenta
                rgb(0, 128, 128),   // Dark Cyan
                rgb(192, 192, 192), // Light Gray
                rgb(128, 128, 128), // Dark Gray
                rgb(255, 0, 0),     // Bright Red
                rgb(0, 255, 0),     // Bright Green
                rgb(255, 255, 0),   // Bright Yellow
                rgb(0, 0, 255),     // Bright Blue
                rgb(255, 0, 255),   // Bright Magenta
                rgb(0, 255, 255),   // Bright Cyan
                rgb(255, 255, 255), // White
            ],
        };

        let event = Event::Init(
            42,
            1000,
            TtySize(180, 24),
            Some(theme),
            "terminal seed".to_string(),
        );

        let bytes = serializer.serialize_event(event);

        let mut expected = vec![
            0x01, // Init event type
            0x2A, // id (42) in LEB128
            0xE8, 0x07, // time (1000) in LEB128
            0xB4, 0x01, // cols (180) in LEB128
            0x18, // rows (24) in LEB128
            16,   // theme - 16 colors
            255, 255, 255, // foreground RGB
            0, 0, 0, // background RGB
        ];

        // Add palette colors (16 colors * 3 bytes each)
        expected.extend_from_slice(&[
            0, 0, 0, // Black
            128, 0, 0, // Dark Red
            0, 128, 0, // Dark Green
            128, 128, 0, // Dark Yellow
            0, 0, 128, // Dark Blue
            128, 0, 128, // Dark Magenta
            0, 128, 128, // Dark Cyan
            192, 192, 192, // Light Gray
            128, 128, 128, // Dark Gray
            255, 0, 0, // Bright Red
            0, 255, 0, // Bright Green
            255, 255, 0, // Bright Yellow
            0, 0, 255, // Bright Blue
            255, 0, 255, // Bright Magenta
            0, 255, 255, // Bright Cyan
            255, 255, 255, // White
        ]);

        expected.push(0x0D); // init string length (13)
        expected.extend_from_slice(b"terminal seed"); // init string

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 1000);
    }

    #[test]
    fn test_serialize_init_without_theme_nor_seed() {
        let mut serializer = EventSerializer(0);
        let event = Event::Init(1, 500, TtySize(120, 130), None, "".to_string());
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            0x01, // Init event type
            0x01, // id (1) in LEB128
            0xF4, 0x03, // relative time (500) in LEB128
            0x78, // cols (120) in LEB128
            0x82, 0x01, // rows (130) in LEB128
            0x00, // no theme flag
            0x00, // init string length (0) in LEB128
        ];

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 500);
    }

    #[test]
    fn test_serialize_output() {
        let mut serializer = EventSerializer(1000);
        let event = Event::Output(5, 1200, "Hello 世界 🌍".to_string());
        let bytes = serializer.serialize_event(event);

        let mut expected = vec![
            b'o', // Output event type
            0x05, // id (5) in LEB128
            0xC8, 0x01, // relative time (200) in LEB128
            0x11, // text length in bytes
        ];

        expected.extend_from_slice("Hello 世界 🌍".as_bytes()); // text bytes

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 1200); // Time updated to 1200
    }

    #[test]
    fn test_serialize_input() {
        let mut serializer = EventSerializer(500);
        let event = Event::Input(1000000, 750, "x".to_string());
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            b'i', // Input event type
            0xC0, 0x84, 0x3D, // id (1000000) in LEB128
            0xFA, 0x01, // relative time (250) in LEB128
            0x01, // text length (1) in LEB128
            b'x', // text
        ];

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 750);
    }

    #[test]
    fn test_serialize_resize() {
        let mut serializer = EventSerializer(2000);
        let event = Event::Resize(15, 2100, TtySize(180, 50));
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            b'r', // Resize event type
            0x0F, // id (15) in LEB128
            0x64, // relative time (100) in LEB128
            0xB4, 0x01, // cols (180) in LEB128
            0x32, // rows (50) in LEB128
        ];

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 2100);
    }

    #[test]
    fn test_serialize_marker_with_label() {
        let mut serializer = EventSerializer(3000);
        let event = Event::Marker(20, 3500, "checkpoint".to_string());
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            b'm', // Marker event type
            0x14, // id (20) in LEB128
            0xF4, 0x03, // relative time (500) in LEB128
            0x0A, // label length (10) in LEB128
        ];
        let mut expected = expected;
        expected.extend_from_slice(b"checkpoint"); // label bytes

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 3500);
    }

    #[test]
    fn test_serialize_marker_without_label() {
        let mut serializer = EventSerializer(3000);
        let event = Event::Marker(2, 3300, "".to_string());
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            b'm', // Marker event type
            0x02, // id (2) in LEB128
            0xAC, 0x02, // relative time (300) in LEB128
            0x00, // label length (0)
        ];

        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_serialize_exit_positive_status() {
        let mut serializer = EventSerializer(4000);
        let event = Event::Exit(25, 4200, 0);
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            b'x', // Exit event type
            0x19, // id (25) in LEB128
            0xC8, 0x01, // relative time (200) in LEB128
            0x00, // status (0) in LEB128
        ];

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 4200);
    }

    #[test]
    fn test_serialize_exit_negative_status() {
        let mut serializer = EventSerializer(5000);
        let event = Event::Exit(30, 5300, -1);
        let bytes = serializer.serialize_event(event);

        let expected = vec![
            b'x', // Exit event type
            0x1E, // id (30) in LEB128
            0xAC, 0x02, // relative time (300) in LEB128
            0x00, // status (clamped to 0) in LEB128
        ];

        assert_eq!(bytes, expected);
        assert_eq!(serializer.0, 5300);
    }

    #[test]
    fn test_subsequent_event_lower_time() {
        let mut serializer = EventSerializer(1000);

        // First event at time 1000
        let event1 = Event::Output(1, 1000, "first".to_string());
        let bytes1 = serializer.serialize_event(event1);

        // Verify first event uses time 0 (1000 - 1000)
        assert_eq!(bytes1[2], 0x00); // relative time should be 0
        assert_eq!(serializer.0, 1000);

        // Second event with lower timestamp (wraparound risk case)
        let event2 = Event::Output(2, 500, "second".to_string());
        let bytes2 = serializer.serialize_event(event2);

        assert_eq!(bytes2[2], 0x00); // relative time should be 0
        assert_eq!(serializer.0, 1000); // Time should remain 1000 (not decrease)
    }

    fn rgb(r: u8, g: u8, b: u8) -> RGB8 {
        RGB8::new(r, g, b)
    }
}
