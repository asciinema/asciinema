use super::alis;
use super::session;
use futures_util::future;
use futures_util::stream;
use futures_util::Sink;
use futures_util::{sink, SinkExt, Stream, StreamExt};
use std::borrow::Cow;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::IntervalStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info};

const WS_PING_INTERVAL: u64 = 15;
const MAX_RECONNECT_DELAY: u64 = 5000;

pub async fn forward(
    clients_tx: mpsc::Sender<session::Client>,
    url: url::Url,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let mut reconnect_attempt = 0;

    info!("forwarding to {url}");

    loop {
        let time = Instant::now();

        match forward_once(&clients_tx, &url).await {
            Ok(_) => return Ok(()),
            Err(e) => debug!("{e:?}"),
        }

        if time.elapsed().as_secs_f32() > 1.0 {
            reconnect_attempt = 0;
        }

        let delay = exponential_delay(reconnect_attempt);
        reconnect_attempt = (reconnect_attempt + 1).min(10);
        info!("connection error, reconnecting in {delay}");

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(delay)) => (),

            _ = shutdown_rx.recv() => {
                info!("shutting down");
                break;
            }
        }
    }

    Ok(())
}

async fn forward_once(
    clients_tx: &mpsc::Sender<session::Client>,
    url: &url::Url,
) -> anyhow::Result<()> {
    let (ws, _) = tokio_tungstenite::connect_async(url).await?;
    info!("connected to the endpoint");
    let (sink, stream) = ws.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));
    let events = event_stream(clients_tx).await?;
    let result = forward_with_pings(events, sink).await;
    drainer.abort();

    result
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

async fn forward_with_pings<T, U>(events: T, mut sink: U) -> anyhow::Result<()>
where
    T: Stream<Item = anyhow::Result<Message>> + Unpin,
    U: Sink<Message> + Unpin,
    <U>::Error: Into<anyhow::Error>,
{
    let mut events = events.fuse();
    let mut pings = ping_stream().fuse();

    loop {
        futures_util::select! {
            event = events.next() => {
                match event {
                    Some(event) => {
                        sink.send(event?).await.map_err(|e| e.into())?;
                    }

                    None => return Ok(())
                }
            },

            ping = pings.next() => {
                sink.send(ping.unwrap()).await.map_err(|e| e.into())?;
            }
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
