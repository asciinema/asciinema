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
                let time_bytes = leb128::encode(time - self.0);
                let text_len = text.len() as u32;
                let text_len_bytes = leb128::encode(text_len);

                let mut msg = vec![b'o'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&text_len_bytes);
                msg.extend_from_slice(text.as_bytes());

                self.0 = time;

                msg
            }

            Input(id, time, text) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(time - self.0);
                let text_len = text.len() as u32;
                let text_len_bytes = leb128::encode(text_len);

                let mut msg = vec![b'i'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&text_len_bytes);
                msg.extend_from_slice(text.as_bytes());

                self.0 = time;

                msg
            }

            Resize(id, time, size) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(time - self.0);
                let cols_bytes = leb128::encode(size.0);
                let rows_bytes = leb128::encode(size.1);

                let mut msg = vec![b'r'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&cols_bytes);
                msg.extend_from_slice(&rows_bytes);

                self.0 = time;

                msg
            }

            Marker(id, time, text) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(time - self.0);
                let text_len = text.len() as u32;
                let text_len_bytes = leb128::encode(text_len);

                let mut msg = vec![b'm'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&text_len_bytes);
                msg.extend_from_slice(text.as_bytes());

                self.0 = time;

                msg
            }

            Exit(id, time, status) => {
                let id_bytes = leb128::encode(id);
                let time_bytes = leb128::encode(time - self.0);
                let status_bytes = leb128::encode(status.max(0) as u64);

                let mut msg = vec![b'x'];
                msg.extend_from_slice(&id_bytes);
                msg.extend_from_slice(&time_bytes);
                msg.extend_from_slice(&status_bytes);

                self.0 = time;

                msg
            }
        }
    }
}
