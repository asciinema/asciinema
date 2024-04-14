use super::alis;
use super::session;
use anyhow::bail;
use futures_util::{future, stream, SinkExt, Stream, StreamExt};
use std::borrow::Cow;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::IntervalStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info};

const WS_PING_INTERVAL: u64 = 15;
const MAX_RECONNECT_DELAY: u64 = 5000;

pub async fn forward(
    url: url::Url,
    clients_tx: mpsc::Sender<session::Client>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut reconnect_attempt = 0;
    info!("forwarding to {url}");

    loop {
        let time = Instant::now();

        match connect_and_forward(&url, &clients_tx).await {
            Ok(true) => break,
            Ok(false) => (),
            Err(e) => error!("connection error: {e}"),
        }

        if time.elapsed().as_secs_f32() > 1.0 {
            reconnect_attempt = 0;
        }

        let delay = exponential_delay(reconnect_attempt);
        reconnect_attempt = (reconnect_attempt + 1).min(10);
        info!("reconnecting in {delay}");

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(delay)) => (),
            _ = shutdown_rx.recv() => break
        }
    }

    info!("shutting down");
}

async fn connect_and_forward(
    url: &url::Url,
    clients_tx: &mpsc::Sender<session::Client>,
) -> anyhow::Result<bool> {
    let (ws, _) = tokio_tungstenite::connect_async(url).await?;
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
    let (mut sink, stream) = ws.split();
    let mut stream = stream.fuse();
    let mut events = events.fuse();
    let mut pings = ping_stream().fuse();

    loop {
        futures_util::select! {
            event = events.next() => {
                match event {
                    Some(event) => sink.send(event?).await?,

                    None => {
                        info!("event stream ended");
                        return Ok(true);
                    }
                }
            },

            ping = pings.next() => sink.send(ping.unwrap()).await?,

            message = stream.next() => {
                match message {
                    Some(Ok(Message::Close(close_frame))) => {
                        info!("server closed the connection");
                        return Ok(handle_close_frame(close_frame));
                    },

                    Some(Ok(msg)) => debug!("unexpected message from the server: {msg}"),
                    Some(Err(e)) => bail!(e),
                    None => bail!("SplitStream closed")
                }
            }
        }
    }
}

fn handle_close_frame(frame: Option<CloseFrame>) -> bool {
    match frame {
        Some(CloseFrame { code, reason }) => {
            info!("close reason: {code} ({reason})");

            match code {
                CloseCode::Normal => true,
                CloseCode::Library(code) if code < 4100 => true,
                _ => false,
            }
        }

        None => {
            info!("close reason: none");
            true
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
    let interval = tokio::time::interval(Duration::from_secs(WS_PING_INTERVAL));

    IntervalStream::new(interval)
        .skip(1)
        .map(|_| Message::Ping(vec![]))
}
