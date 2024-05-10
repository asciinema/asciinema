use super::alis;
use super::session;
use anyhow::anyhow;
use anyhow::bail;
use core::future::{self, Future};
use futures_util::{stream, SinkExt, Stream, StreamExt};
use std::borrow::Cow;
use std::pin::Pin;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, timeout};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::IntervalStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info};

const PING_INTERVAL: u64 = 15;
const PING_TIMEOUT: u64 = 10;
const SEND_TIMEOUT: u64 = 10;
const MAX_RECONNECT_DELAY: u64 = 5000;

pub async fn forward(
    url: url::Url,
    clients_tx: mpsc::Sender<session::Client>,
    notifier_tx: std::sync::mpsc::Sender<String>,
    shutdown_token: tokio_util::sync::CancellationToken,
) {
    info!("forwarding to {url}");
    let mut reconnect_attempt = 0;
    let mut connection_count: u64 = 0;

    loop {
        let conn = connect_and_forward(&url, &clients_tx);
        tokio::pin!(conn);

        let result = tokio::select! {
            result = &mut conn => result,

            _ = sleep(Duration::from_secs(3)) => {
                if reconnect_attempt > 0 {
                    if connection_count == 0 {
                        let _ = notifier_tx.send("Connected to the server".to_string());
                    } else {
                        let _ = notifier_tx.send("Reconnected to the server".to_string());
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
                let _ = notifier_tx.send("Stream halted by the server".to_string());
                break;
            }

            Err(e) => {
                error!("connection error: {e}");

                if reconnect_attempt == 0 {
                    if connection_count == 0 {
                        let _ = notifier_tx
                            .send("Cannot connect to the server, retrying...".to_string());
                    } else {
                        let _ = notifier_tx
                            .send("Disconnected from the server, reconnecting...".to_string());
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

async fn connect_and_forward(
    url: &url::Url,
    clients_tx: &mpsc::Sender<session::Client>,
) -> anyhow::Result<bool> {
    let (ws, _) = tokio_tungstenite::connect_async_with_config(url, None, true).await?;
    info!("connected to the endpoint");
    let events = event_stream(clients_tx).await?;

    handle_socket(ws, events).await
}

async fn event_stream(
    clients_tx: &mpsc::Sender<session::Client>,
) -> anyhow::Result<impl Stream<Item = anyhow::Result<Message>>> {
    let stream = alis::stream(clients_tx)
        .await?
        .map(ws_result)
        .chain(stream::once(future::ready(Ok(close_message()))));

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
    (2_u64.pow(attempt as u32) * 500).min(MAX_RECONNECT_DELAY)
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
        reason: Cow::from("ended"),
    }))
}

fn ping_stream() -> impl Stream<Item = Message> {
    IntervalStream::new(interval(Duration::from_secs(PING_INTERVAL)))
        .skip(1)
        .map(|_| Message::Ping(vec![]))
}
