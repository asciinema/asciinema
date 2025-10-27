use core::future::{self, Future};
use std::pin::Pin;
use std::time::Duration;

use anyhow::{anyhow, bail};
use axum::http::Uri;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, Stream, StreamExt};
use rand::Rng;
use tokio::net::TcpStream;
use tokio::time;
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
use crate::stream::{Event, Subscriber};

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
) -> anyhow::Result<()> {
    info!("forwarding to {url}");
    let mut reconnect_attempt = 0;
    let mut connection_count: u64 = 0;

    loop {
        let session_stream = subscriber.subscribe().await?;
        let conn = connect_and_forward(&url, session_stream);
        tokio::pin!(conn);

        let result = tokio::select! {
            result = &mut conn => result,

            _ = time::sleep(Duration::from_secs(3)) => {
                if reconnect_attempt > 0 {
                    if connection_count == 0 {
                        let _ = notifier.notify("Connected to the server".to_string()).await;
                    } else {
                        let _ = notifier.notify("Reconnected to the server".to_string()).await;
                    }
                }

                connection_count += 1;
                reconnect_attempt = 0;

                conn.await
            }
        };

        match result {
            Ok(true) => {
                break;
            }

            Ok(false) => {
                let _ = notifier
                    .notify("Stream halted by the server".to_string())
                    .await;

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
                        .notify("The server version is too old, forwarding failed".to_string())
                        .await;

                    break;
                }

                if let Some(tungstenite::error::Error::Http(response)) =
                    e.downcast_ref::<tungstenite::error::Error>()
                {
                    if response.status().as_u16() == 400 {
                        // This happens when the server doesn't support our protocol (version).
                        // This applies to asciinema-server versions newer than v20241103.

                        let _ = notifier
                            .notify(
                                "CLI not compatible with the server, forwarding failed".to_string(),
                            )
                            .await;

                        break;
                    }
                }

                error!("connection error: {e}");

                if reconnect_attempt == 0 {
                    if connection_count == 0 {
                        let _ = notifier
                            .notify("Cannot connect to the server, retrying...".to_string())
                            .await;
                    } else {
                        let _ = notifier
                            .notify("Disconnected from the server, reconnecting...".to_string())
                            .await;
                    }
                }
            }
        }

        let delay = exponential_delay(reconnect_attempt);
        reconnect_attempt = (reconnect_attempt + 1).min(10);
        info!("reconnecting in {delay} ms");

        tokio::select! {
            _ = time::sleep(Duration::from_millis(delay)) => (),
            _ = shutdown_token.cancelled() => break
        }
    }

    Ok(())
}

async fn connect_and_forward(
    url: &url::Url,
    session_stream: impl Stream<Item = Result<Event, BroadcastStreamRecvError>> + Unpin,
) -> anyhow::Result<bool> {
    let request = build_request(url)?;
    let (ws, _) = tokio_tungstenite::connect_async_with_config(request, None, true).await?;
    info!("connected to the endpoint");

    handle_socket(ws, get_alis_stream(session_stream)).await
}

fn build_request(url: &url::Url) -> anyhow::Result<ClientRequestBuilder> {
    let uri: Uri = url.to_string().parse()?;

    Ok(ClientRequestBuilder::new(uri)
        .with_sub_protocol("v1.alis")
        .with_header("user-agent", api::build_user_agent()))
}

fn get_alis_stream(
    stream: impl Stream<Item = Result<Event, BroadcastStreamRecvError>>,
) -> impl Stream<Item = anyhow::Result<Message>> {
    alis::stream(stream)
        .map(ws_result)
        .chain(futures_util::stream::once(future::ready(Ok(
            close_message(),
        ))))
}

async fn handle_socket<T>(
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    alis_messages: T,
) -> anyhow::Result<bool>
where
    T: Stream<Item = anyhow::Result<Message>> + Unpin,
{
    let (mut sink, mut stream) = ws.split();
    let mut alis_messages = alis_messages;
    let mut pings = ping_stream();
    let mut ping_timeout: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(future::pending());

    loop {
        tokio::select! {
            message = alis_messages.next() => {
                match message {
                    Some(message) => {
                        send_with_timeout(&mut sink, message?).await??;
                    },

                    None => {
                        return Ok(true);
                    }
                }
            },

            ping = pings.next() => {
                send_with_timeout(&mut sink, ping.unwrap()).await??;
                ping_timeout = Box::pin(time::sleep(Duration::from_secs(PING_TIMEOUT)));
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

async fn send_with_timeout(
    sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    message: Message,
) -> anyhow::Result<Result<(), tungstenite::Error>> {
    time::timeout(Duration::from_secs(SEND_TIMEOUT), sink.send(message))
        .await
        .map_err(|_| anyhow!("send timeout"))
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
    IntervalStream::new(time::interval(Duration::from_secs(PING_INTERVAL)))
        .skip(1)
        .map(|_| Message::Ping(vec![].into()))
}
