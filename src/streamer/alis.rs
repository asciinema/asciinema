// This module implements ALiS (asciinema live stream) protocol,
// which is an application level protocol built on top of WebSocket binary messages,
// used by asciinema CLI, asciinema player and asciinema server.

// TODO document the protocol when it's final

use super::session;
use crate::leb128;
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
                let (bytes, time) = serialize_event(event, *prev_event_time);
                *prev_event_time = time;

                bytes
            })))
        });

    Ok(header.chain(events))
}

fn serialize_event(event: session::Event, prev_event_time: u64) -> (Vec<u8>, u64) {
    use session::Event::*;

    match event {
        Init(time, size, theme, init) => {
            let time_bytes = leb128::encode(time);
            let cols_bytes = leb128::encode(size.0);
            let rows_bytes = leb128::encode(size.1);
            let init_len = init.len() as u32;
            let init_len_bytes = leb128::encode(init_len);

            let mut msg = vec![0x01];
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

            (msg, time)
        }

        Output(time, text) => {
            let time_bytes = leb128::encode(time - prev_event_time);
            let text_len = text.len() as u32;
            let text_len_bytes = leb128::encode(text_len);

            let mut msg = vec![b'o'];
            msg.extend_from_slice(&time_bytes);
            msg.extend_from_slice(&text_len_bytes);
            msg.extend_from_slice(text.as_bytes());

            (msg, time)
        }

        Input(time, text) => {
            let time_bytes = leb128::encode(time - prev_event_time);
            let text_len = text.len() as u32;
            let text_len_bytes = leb128::encode(text_len);

            let mut msg = vec![b'i'];
            msg.extend_from_slice(&time_bytes);
            msg.extend_from_slice(&text_len_bytes);
            msg.extend_from_slice(text.as_bytes());

            (msg, time)
        }

        Resize(time, size) => {
            let time_bytes = leb128::encode(time - prev_event_time);
            let cols_bytes = leb128::encode(size.0);
            let rows_bytes = leb128::encode(size.1);

            let mut msg = vec![b'r'];
            msg.extend_from_slice(&time_bytes);
            msg.extend_from_slice(&cols_bytes);
            msg.extend_from_slice(&rows_bytes);

            (msg, time)
        }

        Marker(time, text) => {
            let time_bytes = leb128::encode(time - prev_event_time);
            let text_len = text.len() as u32;
            let text_len_bytes = leb128::encode(text_len);

            let mut msg = vec![b'm'];
            msg.extend_from_slice(&time_bytes);
            msg.extend_from_slice(&text_len_bytes);
            msg.extend_from_slice(text.as_bytes());

            (msg, time)
        }
    }
}
