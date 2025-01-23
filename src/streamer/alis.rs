// This module implements ALiS (asciinema live stream) protocol,
// which is an application level protocol built on top of WebSocket binary messages,
// used by asciinema CLI, asciinema player and asciinema server.

// TODO document the protocol when it's final

use super::session;
use anyhow::Result;
use futures_util::{stream, Stream, StreamExt, TryStreamExt};
use std::future;
use tokio::sync::mpsc;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

static MAGIC_STRING: &str = "ALiS\x01";

pub async fn stream(
    clients_tx: &mpsc::Sender<session::Client>,
) -> Result<impl Stream<Item = Result<Vec<u8>, BroadcastStreamRecvError>>> {
    let header = stream::once(future::ready(Ok(MAGIC_STRING.into())));
    let events = session::stream(clients_tx).await?.map_ok(encode_event);

    Ok(header.chain(events))
}

fn encode_event(event: session::Event) -> Vec<u8> {
    use session::Event::*;

    match event {
        Init(time, size, theme, init) => {
            let time_bytes = time.to_le_bytes();
            let (cols, rows): (u16, u16) = (size.0, size.1);
            let cols_bytes = cols.to_le_bytes();
            let rows_bytes = rows.to_le_bytes();
            let init_len = init.len() as u32;
            let init_len_bytes = init_len.to_le_bytes();

            let mut msg = vec![0x01]; // 1 byte
            msg.extend_from_slice(&time_bytes); // 8 bytes
            msg.extend_from_slice(&cols_bytes); // 2 bytes
            msg.extend_from_slice(&rows_bytes); // 2 bytes

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

            msg.extend_from_slice(&init_len_bytes); // 4 bytes
            msg.extend_from_slice(init.as_bytes()); // init_len bytes

            msg
        }

        Output(time, text) => {
            let time_bytes = time.to_le_bytes();
            let text_len = text.len() as u32;
            let text_len_bytes = text_len.to_le_bytes();

            let mut msg = vec![b'o']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 8 bytes
            msg.extend_from_slice(&text_len_bytes); // 4 bytes
            msg.extend_from_slice(text.as_bytes()); // text_len bytes

            msg
        }

        Input(time, text) => {
            let time_bytes = time.to_le_bytes();
            let text_len = text.len() as u32;
            let text_len_bytes = text_len.to_le_bytes();

            let mut msg = vec![b'i']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 8 bytes
            msg.extend_from_slice(&text_len_bytes); // 4 bytes
            msg.extend_from_slice(text.as_bytes()); // text_len bytes

            msg
        }

        Resize(time, size) => {
            let time_bytes = time.to_le_bytes();
            let (cols, rows): (u16, u16) = (size.0, size.1);
            let cols_bytes = cols.to_le_bytes();
            let rows_bytes = rows.to_le_bytes();

            let mut msg = vec![b'r']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 8 bytes
            msg.extend_from_slice(&cols_bytes); // 2 bytes
            msg.extend_from_slice(&rows_bytes); // 2 bytes

            msg
        }

        Marker(time, text) => {
            let time_bytes = time.to_le_bytes();
            let text_len = text.len() as u32;
            let text_len_bytes = text_len.to_le_bytes();

            let mut msg = vec![b'm']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 8 bytes
            msg.extend_from_slice(&text_len_bytes); // 4 bytes
            msg.extend_from_slice(text.as_bytes()); // text_len bytes

            msg
        }
    }
}
