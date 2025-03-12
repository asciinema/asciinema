use std::future;
use std::io;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use avt::Vt;
use futures_util::{stream, StreamExt};
use tokio::runtime::Runtime;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use crate::session;
use crate::tty;

pub struct Stream {
    tx: mpsc::Sender<Request>,
    rx: mpsc::Receiver<Request>,
}

type Request = oneshot::Sender<Subscription>;

struct Subscription {
    init: Event,
    events_rx: broadcast::Receiver<Event>,
}

#[derive(Clone)]
pub struct Subscriber(mpsc::Sender<Request>);

pub struct Output(mpsc::UnboundedSender<Message>);

enum Message {
    Start(tty::TtySize, Option<tty::Theme>),
    SessionEvent(session::Event),
}

#[derive(Clone)]
pub enum Event {
    Init(u64, u64, tty::TtySize, Option<tty::Theme>, String),
    Output(u64, u64, String),
    Input(u64, u64, String),
    Resize(u64, u64, tty::TtySize),
    Marker(u64, u64, String),
}

impl Stream {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);

        Stream { tx, rx }
    }

    pub fn subscriber(&self) -> Subscriber {
        Subscriber(self.tx.clone())
    }

    pub fn start(self, runtime: &Runtime) -> Output {
        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        runtime.spawn(async move { run(stream_rx, self.rx).await });

        Output(stream_tx)
    }
}

async fn run(
    mut stream_rx: mpsc::UnboundedReceiver<Message>,
    mut request_rx: mpsc::Receiver<Request>,
) {
    let (broadcast_tx, _) = broadcast::channel(1024);
    let mut vt = build_vt(tty::TtySize::default());
    let mut stream_time = 0;
    let mut last_event_id = 0;
    let mut last_event_time = Instant::now();
    let mut tty_theme = None;

    loop {
        tokio::select! {
            event = stream_rx.recv() => {
                match event {
                    Some(Message::Start(tty_size_, tty_theme_)) => {
                        tty_theme = tty_theme_;
                        vt = build_vt(tty_size_);
                    }

                    Some(Message::SessionEvent(event)) => {
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
                        }
                    }

                    None => break,
                }
            }

            request = request_rx.recv() => {
                match request {
                    Some(request) => {
                        let elapsed_time = stream_time + last_event_time.elapsed().as_micros() as u64;

                        let init = Event::Init(
                            last_event_id,
                            elapsed_time,
                            vt.size().into(),
                            tty_theme.clone(),
                            vt.dump(),
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

fn build_vt(tty_size: tty::TtySize) -> Vt {
    Vt::builder()
        .size(tty_size.0 as usize, tty_size.1 as usize)
        .build()
}

impl session::Output for Output {
    fn start(
        &mut self,
        _time: SystemTime,
        tty_size: tty::TtySize,
        theme: Option<tty::Theme>,
    ) -> io::Result<()> {
        self.0
            .send(Message::Start(tty_size, theme))
            .expect("send should succeed");

        Ok(())
    }

    fn event(&mut self, event: session::Event) -> io::Result<()> {
        self.0
            .send(Message::SessionEvent(event))
            .expect("send should succeed");

        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
