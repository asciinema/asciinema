use crate::tty;
use anyhow::Result;
use futures_util::{stream, Stream, StreamExt};
use std::{
    future,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};

pub struct Session {
    vt: avt::Vt,
    broadcast_tx: broadcast::Sender<Event>,
    stream_time: u64,
    last_event_time: Instant,
    theme: Option<tty::Theme>,
}

#[derive(Clone)]
pub enum Event {
    Init(u64, tty::TtySize, Option<tty::Theme>, String),
    Stdout(u64, String),
    Resize(u64, tty::TtySize),
}

pub struct Client(oneshot::Sender<Subscription>);

pub struct Subscription {
    init: Event,
    broadcast_rx: broadcast::Receiver<Event>,
}

impl Session {
    pub fn new(tty_size: tty::TtySize, theme: Option<tty::Theme>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);

        Self {
            vt: build_vt(tty_size),
            broadcast_tx,
            stream_time: 0,
            last_event_time: Instant::now(),
            theme,
        }
    }

    pub fn output(&mut self, time: u64, data: String) {
        self.vt.feed_str(&data);
        let _ = self.broadcast_tx.send(Event::Stdout(time, data));
        self.stream_time = time;
        self.last_event_time = Instant::now();
    }

    pub fn input(&mut self, time: u64, _data: String) {
        self.stream_time = time;
        self.last_event_time = Instant::now();
    }

    pub fn resize(&mut self, time: u64, tty_size: tty::TtySize) {
        if tty_size != self.vt.size().into() {
            resize_vt(&mut self.vt, &tty_size);
            let _ = self.broadcast_tx.send(Event::Resize(time, tty_size));
            self.stream_time = time;
            self.last_event_time = Instant::now();
        }
    }

    pub fn subscribe(&self) -> Subscription {
        let init = Event::Init(
            self.elapsed_time(),
            self.vt.size().into(),
            self.theme.clone(),
            self.vt.dump(),
        );

        let broadcast_rx = self.broadcast_tx.subscribe();

        Subscription { init, broadcast_rx }
    }

    pub fn subscriber_count(&self) -> usize {
        self.broadcast_tx.receiver_count()
    }

    fn elapsed_time(&self) -> u64 {
        self.stream_time + self.last_event_time.elapsed().as_micros() as u64
    }
}

fn build_vt(tty_size: tty::TtySize) -> avt::Vt {
    avt::Vt::builder()
        .size(tty_size.0 as usize, tty_size.1 as usize)
        .resizable(true)
        .build()
}

fn resize_vt(vt: &mut avt::Vt, tty_size: &tty::TtySize) {
    vt.feed_str(&format!("\x1b[8;{};{}t", tty_size.1, tty_size.0));
}

impl Client {
    pub fn accept(self, subscription: Subscription) {
        let _ = self.0.send(subscription);
    }
}

pub async fn stream(
    clients_tx: &mpsc::Sender<Client>,
) -> Result<impl Stream<Item = Result<Event, BroadcastStreamRecvError>>> {
    let (sub_tx, sub_rx) = oneshot::channel();
    clients_tx.send(Client(sub_tx)).await?;
    let sub = tokio::time::timeout(Duration::from_secs(5), sub_rx).await??;
    let init = stream::once(future::ready(Ok(sub.init)));
    let events = BroadcastStream::new(sub.broadcast_rx);

    Ok(init.chain(events))
}
