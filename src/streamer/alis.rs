// This module implements ALiS (asciinema live stream) protocol,
// which is an application level protocol built on top of WebSocket binary messages,
// used by asciinema CLI, asciinema player and asciinema server.

// TODO document the protocol when it's final

use super::session;
use anyhow::Result;
use futures_util::{stream, Stream, StreamExt};
use std::future;
use tokio::sync::mpsc;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

static MAGIC_STRING: &str = "ALiS\x01";

pub async fn stream(
    clients_tx: &mpsc::Sender<session::Client>,
) -> Result<impl Stream<Item = Result<Vec<u8>, BroadcastStreamRecvError>>> {
    let header = stream::once(future::ready(Ok(MAGIC_STRING.into())));

    let events = session::stream(clients_tx)
        .await?
        .scan(0u64, |prev_event_time, event| {
            future::ready(Some(event.map(|event| {
                let (bytes, time) = encode_event(event, *prev_event_time);
                *prev_event_time = time;

                bytes
            })))
        });

    Ok(header.chain(events))
}

fn encode_event(event: session::Event, prev_event_time: u64) -> (Vec<u8>, u64) {
    use session::Event::*;

    match event {
        Init(time, size, theme, init) => {
            let time_bytes = encode_rel_time(time);
            let (cols, rows): (u16, u16) = (size.0, size.1);
            let cols_bytes = cols.to_le_bytes();
            let rows_bytes = rows.to_le_bytes();
            let init_len = init.len() as u32;
            let init_len_bytes = init_len.to_le_bytes();

            let mut msg = vec![0x01]; // 1 byte
            msg.extend_from_slice(&time_bytes); // 2-9 bytes
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

            (msg, time)
        }

        Output(time, text) => {
            let time_bytes = encode_rel_time(time - prev_event_time);
            let text_len = text.len() as u32;
            let text_len_bytes = text_len.to_le_bytes();

            let mut msg = vec![b'o']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 2-9 bytes
            msg.extend_from_slice(&text_len_bytes); // 4 bytes
            msg.extend_from_slice(text.as_bytes()); // text_len bytes

            (msg, time)
        }

        Input(time, text) => {
            let time_bytes = encode_rel_time(time - prev_event_time);
            let text_len = text.len() as u32;
            let text_len_bytes = text_len.to_le_bytes();

            let mut msg = vec![b'i']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 2-9 bytes
            msg.extend_from_slice(&text_len_bytes); // 4 bytes
            msg.extend_from_slice(text.as_bytes()); // text_len bytes

            (msg, time)
        }

        Resize(time, size) => {
            let time_bytes = encode_rel_time(time - prev_event_time);
            let (cols, rows): (u16, u16) = (size.0, size.1);
            let cols_bytes = cols.to_le_bytes();
            let rows_bytes = rows.to_le_bytes();

            let mut msg = vec![b'r']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 2-9 bytes
            msg.extend_from_slice(&cols_bytes); // 2 bytes
            msg.extend_from_slice(&rows_bytes); // 2 bytes

            (msg, time)
        }

        Marker(time, text) => {
            let time_bytes = encode_rel_time(time - prev_event_time);
            let text_len = text.len() as u32;
            let text_len_bytes = text_len.to_le_bytes();

            let mut msg = vec![b'm']; // 1 byte
            msg.extend_from_slice(&time_bytes); // 2-9 bytes
            msg.extend_from_slice(&text_len_bytes); // 4 bytes
            msg.extend_from_slice(text.as_bytes()); // text_len bytes

            (msg, time)
        }
    }
}

fn encode_rel_time(rel_time: u64) -> Vec<u8> {
    let mut msg = vec![];

    if rel_time < 256 {
        msg.push(1);
        msg.push(rel_time as u8);
    } else if rel_time < 65_536 {
        msg.push(2);
        msg.extend_from_slice(&(rel_time as u16).to_le_bytes());
    } else if rel_time < 4_294_967_296 {
        msg.push(4);
        msg.extend_from_slice(&(rel_time as u32).to_le_bytes());
    } else {
        msg.push(8);
        msg.extend_from_slice(&rel_time.to_le_bytes());
    }

    msg
}
