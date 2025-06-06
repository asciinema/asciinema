use core::future::{self, Future};
use std::pin::Pin;
use std::time::Duration;

use anyhow::{anyhow, bail};
use axum::http::Uri;
use futures_util::{SinkExt, Stream, StreamExt};
use rand::Rng;
use tokio::net::TcpStream;
use tokio::time::{interval, sleep, timeout};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::IntervalStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{self, ClientRequestBuilder, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info};

use crate::alis;
use crate::api;
use crate::notifier::Notifier;
use crate::stream::Subscriber;

const PING_INTERVAL: u64 = 15;
const PING_TIMEOUT: u64 = 10;
const SEND_TIMEOUT: u64 = 10;
const RECONNECT_DELAY_BASE: u64 = 500;
const RECONNECT_DELAY_CAP: u64 = 10_000;

pub async fn forward<N: Notifier>(
    url: url::Url,
    subscriber: Subscriber,
    mut notifier: N,
    shutdown_token: tokio_util::sync::CancellationToken,
) {
    info!("forwarding to {url}");
    let mut reconnect_attempt = 0;
    let mut connection_count: u64 = 0;

    loop {
        let conn = connect_and_forward(&url, &subscriber);
        tokio::pin!(conn);

        let result = tokio::select! {
            result = &mut conn => result,

            _ = sleep(Duration::from_secs(3)) => {
                if reconnect_attempt > 0 {
                    if connection_count == 0 {
                        let _ = notifier.notify("Connected to the server".to_string());
                    } else {
                        let _ = notifier.notify("Reconnected to the server".to_string());
                    }
                }

                connection_count += 1;
                reconnect_attempt = 0;

                conn.await
            }
        };

        match result {
            Ok(true) => break,

            Ok(false) => {
                let _ = notifier.notify("Stream halted by the server".to_string());
                break;
            }

            Err(e) => {
                if let Some(tungstenite::error::Error::Protocol(
                    tungstenite::error::ProtocolError::SecWebSocketSubProtocolError(_),
                )) = e.downcast_ref::<tungstenite::error::Error>()
                {
                    // This happens when the server accepts the websocket connection
                    // but doesn't properly perform the protocol negotiation.
                    // This applies to asciinema-server v20241103 and earlier.

                    let _ = notifier
                        .notify("The server version is too old, forwarding failed".to_string());

                    break;
                }

                if let Some(tungstenite::error::Error::Http(response)) =
                    e.downcast_ref::<tungstenite::error::Error>()
                {
                    if response.status().as_u16() == 400 {
                        // This happens when the server doesn't support our protocol (version).
                        // This applies to asciinema-server versions newer than v20241103.

                        let _ = notifier.notify(
                            "CLI not compatible with the server, forwarding failed".to_string(),
                        );

                        break;
                    }
                }

                error!("connection error: {e}");

                if reconnect_attempt == 0 {
                    if connection_count == 0 {
                        let _ = notifier
                            .notify("Cannot connect to the server, retrying...".to_string());
                    } else {
                        let _ = notifier
                            .notify("Disconnected from the server, reconnecting...".to_string());
                    }
                }
            }
        }

        let delay = exponential_delay(reconnect_attempt);
        reconnect_attempt = (reconnect_attempt + 1).min(10);
        info!("reconnecting in {delay}");

        tokio::select! {
            _ = sleep(Duration::from_millis(delay)) => (),
            _ = shutdown_token.cancelled() => break
        }
    }
}

async fn connect_and_forward(url: &url::Url, subscriber: &Subscriber) -> anyhow::Result<bool> {
    let uri: Uri = url.to_string().parse()?;

    let builder = ClientRequestBuilder::new(uri)
        .with_sub_protocol("v1.alis")
        .with_header("user-agent", api::build_user_agent());

    let (ws, _) = tokio_tungstenite::connect_async_with_config(builder, None, true).await?;
    info!("connected to the endpoint");
    let events = event_stream(subscriber).await?;

    handle_socket(ws, events).await
}

async fn event_stream(
    subscriber: &Subscriber,
) -> anyhow::Result<impl Stream<Item = anyhow::Result<Message>>> {
    let stream = subscriber.subscribe().await?;

    let stream = alis::stream(stream)
        .await
        .map(ws_result)
        .chain(futures_util::stream::once(future::ready(Ok(
            close_message(),
        ))));

    Ok(stream)
}

async fn handle_socket<T>(
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    events: T,
) -> anyhow::Result<bool>
where
    T: Stream<Item = anyhow::Result<Message>> + Unpin,
{
    let (mut sink, mut stream) = ws.split();
    let mut events = events;
    let mut pings = ping_stream();
    let mut ping_timeout: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(future::pending());

    loop {
        tokio::select! {
            event = events.next() => {
                match event {
                    Some(event) => {
                        timeout(Duration::from_secs(SEND_TIMEOUT), sink.send(event?)).await.map_err(|_| anyhow!("send timeout"))??;
                    },

                    None => return Ok(true)
                }
            },

            ping = pings.next() => {
                timeout(Duration::from_secs(SEND_TIMEOUT), sink.send(ping.unwrap())).await.map_err(|_| anyhow!("send timeout"))??;
                ping_timeout = Box::pin(sleep(Duration::from_secs(PING_TIMEOUT)));
            }

            _ = &mut ping_timeout => bail!("ping timeout"),

            message = stream.next() => {
                match message {
                    Some(Ok(Message::Close(close_frame))) => {
                        info!("server closed the connection");
                        handle_close_frame(close_frame)?;
                        return Ok(false);
                    },

                    Some(Ok(Message::Ping(_))) => (),

                    Some(Ok(Message::Pong(_))) => {
                        ping_timeout = Box::pin(future::pending());
                    },

                    Some(Ok(msg)) => debug!("unexpected message from the server: {msg:?}"),
                    Some(Err(e)) => bail!(e),
                    None => bail!("SplitStream closed")
                }
            }
        }
    }
}

fn handle_close_frame(frame: Option<CloseFrame>) -> anyhow::Result<()> {
    match frame {
        Some(CloseFrame { code, reason }) => {
            info!("close reason: {code} ({reason})");

            match code {
                CloseCode::Normal => Ok(()),
                CloseCode::Library(code) if code < 4100 => Ok(()),
                c => bail!("unclean close: {c}"),
            }
        }

        None => {
            info!("close reason: none");
            Ok(())
        }
    }
}

fn exponential_delay(attempt: usize) -> u64 {
    let mut rng = rand::rng();
    let base = (RECONNECT_DELAY_BASE * 2_u64.pow(attempt as u32)).min(RECONNECT_DELAY_CAP);

    rng.random_range(..base)
}

fn ws_result(m: Result<Vec<u8>, BroadcastStreamRecvError>) -> anyhow::Result<Message> {
    match m {
        Ok(bytes) => Ok(Message::binary(bytes)),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

fn close_message() -> Message {
    Message::Close(Some(CloseFrame {
        code: CloseCode::Normal,
        reason: "ended".into(),
    }))
}

fn ping_stream() -> impl Stream<Item = Message> {
    IntervalStream::new(interval(Duration::from_secs(PING_INTERVAL)))
        .skip(1)
        .map(|_| Message::Ping(vec![].into()))
}
