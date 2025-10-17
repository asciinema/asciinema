use std::future;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use avt::Vt;
use futures_util::{stream, StreamExt};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::{io, time};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use crate::session::{self, Metadata};
use crate::tty::{TtySize, TtyTheme};

pub struct Stream {
    request_tx: mpsc::Sender<Request>,
    request_rx: mpsc::Receiver<Request>,
}

type Request = oneshot::Sender<Subscription>;

struct Subscription {
    init: Event,
    events_rx: broadcast::Receiver<Event>,
}

#[derive(Clone)]
pub enum Event {
    Init(u64, Duration, TtySize, Option<TtyTheme>, String),
    Output(u64, Duration, String),
    Input(u64, Duration, String),
    Resize(u64, Duration, TtySize),
    Marker(u64, Duration, String),
    Exit(u64, Duration, i32),
}

#[derive(Clone)]
pub struct Subscriber(mpsc::Sender<Request>);

pub struct LiveStream(mpsc::Sender<session::Event>);

impl Stream {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel(16);

        Stream {
            request_tx,
            request_rx,
        }
    }

    pub fn subscriber(&self) -> Subscriber {
        Subscriber(self.request_tx.clone())
    }

    pub async fn start(self, metadata: &Metadata) -> LiveStream {
        let (stream_tx, stream_rx) = mpsc::channel(1024);
        let request_rx = self.request_rx;

        tokio::spawn(run(
            metadata.term.size,
            metadata.term.theme.clone(),
            stream_rx,
            request_rx,
        ));

        LiveStream(stream_tx)
    }
}

async fn run(
    tty_size: TtySize,
    tty_theme: Option<TtyTheme>,
    mut stream_rx: mpsc::Receiver<session::Event>,
    mut request_rx: mpsc::Receiver<Request>,
) {
    let (broadcast_tx, _) = broadcast::channel(1024);
    let mut vt = build_vt(tty_size);
    let mut stream_time = Duration::from_micros(0);
    let mut last_event_id = 0;
    let mut last_event_time = Instant::now();

    loop {
        tokio::select! {
            event = stream_rx.recv() => {
                match event {
                    Some(event) => {
                        last_event_time = Instant::now();
                        last_event_id += 1;

                        match event {
                            session::Event::Output(time, text) => {
                                vt.feed_str(&text);
                                let _ = broadcast_tx.send(Event::Output(last_event_id, time, text));
                                stream_time = time;
                            }

                            session::Event::Input(time, text) => {
                                let _ = broadcast_tx.send(Event::Input(last_event_id, time, text));
                                stream_time = time;
                            }

                            session::Event::Resize(time, tty_size) => {
                                vt.resize(tty_size.0.into(), tty_size.1.into());
                                let _ = broadcast_tx.send(Event::Resize(last_event_id, time, tty_size));
                                stream_time = time;
                            }

                            session::Event::Marker(time, label) => {
                                let _ = broadcast_tx.send(Event::Marker(last_event_id, time, label));
                                stream_time = time;
                            }

                            session::Event::Exit(time, status) => {
                                let _ = broadcast_tx.send(Event::Exit(last_event_id, time, status));
                                stream_time = time;
                            }
                        }
                    }

                    None => break,
                }
            }

            request = request_rx.recv() => {
                match request {
                    Some(request) => {
                        let init = if last_event_id > 0 {
                            let elapsed_time = stream_time + last_event_time.elapsed();

                            Event::Init(
                                last_event_id,
                                elapsed_time,
                                vt.size().into(),
                                tty_theme.clone(),
                                vt.dump(),
                            )
                        } else {
                            Event::Init(
                                last_event_id,
                                stream_time,
                                vt.size().into(),
                                tty_theme.clone(),
                                "".to_owned(),
                            )
                        };

                        let events_rx = broadcast_tx.subscribe();
                        let _ = request.send(Subscription { init, events_rx });
                        info!("subscriber count: {}", broadcast_tx.receiver_count());
                    }

                    None => break,
                }
            }
        }
    }
}

impl Subscriber {
    pub async fn subscribe(
        &self,
    ) -> anyhow::Result<impl futures_util::Stream<Item = Result<Event, BroadcastStreamRecvError>>>
    {
        let (tx, rx) = oneshot::channel();
        self.0.send(tx).await?;
        let subscription = time::timeout(Duration::from_secs(5), rx).await??;
        let init = stream::once(future::ready(Ok(subscription.init)));
        let events = BroadcastStream::new(subscription.events_rx);

        Ok(init.chain(events))
    }
}

fn build_vt(tty_size: TtySize) -> Vt {
    Vt::builder()
        .size(tty_size.0 as usize, tty_size.1 as usize)
        .scrollback_limit(1000)
        .build()
}

#[async_trait]
impl session::Output for LiveStream {
    async fn event(&mut self, event: session::Event) -> io::Result<()> {
        self.0.send(event).await.map_err(io::Error::other)
    }

    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
