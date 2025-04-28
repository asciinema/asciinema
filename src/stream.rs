use std::future;
use std::io;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use avt::Vt;
use futures_util::{stream, StreamExt};
use tokio::runtime::Handle;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use crate::session;
use crate::tty::TtySize;
use crate::tty::TtyTheme;

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
pub struct Subscriber(mpsc::Sender<Request>);

pub struct OutputStarter {
    handle: Handle,
    request_rx: mpsc::Receiver<Request>,
}

struct Output(mpsc::UnboundedSender<session::Event>);

#[derive(Clone)]
pub enum Event {
    Init(u64, u64, TtySize, Option<TtyTheme>, String),
    Output(u64, u64, String),
    Input(u64, u64, String),
    Resize(u64, u64, TtySize),
    Marker(u64, u64, String),
}

impl Stream {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel(1);

        Stream {
            request_tx,
            request_rx,
        }
    }

    pub fn subscriber(&self) -> Subscriber {
        Subscriber(self.request_tx.clone())
    }

    pub fn start(self, handle: Handle) -> OutputStarter {
        OutputStarter {
            handle,
            request_rx: self.request_rx,
        }
    }
}

async fn run(
    tty_size: TtySize,
    tty_theme: Option<TtyTheme>,
    mut stream_rx: mpsc::UnboundedReceiver<session::Event>,
    mut request_rx: mpsc::Receiver<Request>,
) {
    let (broadcast_tx, _) = broadcast::channel(1024);
    let mut vt = build_vt(tty_size);
    let mut stream_time = 0;
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
                            session::Event::Output(time, text, _) => {
                                vt.feed_str(&text);
                                let _ = broadcast_tx.send(Event::Output(last_event_id, time, text));
                                stream_time = time;
                            }

                            session::Event::Input(time, text, _) => {
                                let _ = broadcast_tx.send(Event::Input(last_event_id, time, text));
                                stream_time = time;
                            }

                            session::Event::Resize(time, tty_size, _) => {
                                vt.resize(tty_size.0.into(), tty_size.1.into());
                                let _ = broadcast_tx.send(Event::Resize(last_event_id, time, tty_size));
                                stream_time = time;
                            }

                            session::Event::Marker(time, label, _) => {
                                let _ = broadcast_tx.send(Event::Marker(last_event_id, time, label));
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
                        let elapsed_time = stream_time + last_event_time.elapsed().as_micros() as u64;

                        let vt_seed = if last_event_id > 0 {
                            vt.dump()
                        } else {
                            "".to_owned()
                        };

                        let init = Event::Init(
                            last_event_id,
                            elapsed_time,
                            vt.size().into(),
                            tty_theme.clone(),
                            vt_seed,
                        );

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
    ) -> Result<impl futures_util::Stream<Item = Result<Event, BroadcastStreamRecvError>>> {
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
        .build()
}

impl session::OutputStarter for OutputStarter {
    fn start(
        self: Box<Self>,
        _time: SystemTime,
        tty_size: TtySize,
        theme: Option<TtyTheme>,
    ) -> io::Result<Box<dyn session::Output>> {
        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        let request_rx = self.request_rx;

        self.handle
            .spawn(async move { run(tty_size, theme, stream_rx, request_rx).await });

        Ok(Box::new(Output(stream_tx)))
    }
}

impl session::Output for Output {
    fn event(&mut self, event: session::Event) -> io::Result<()> {
        self.0.send(event).map_err(io::Error::other)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
