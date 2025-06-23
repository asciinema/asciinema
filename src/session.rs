use std::collections::HashMap;
use std::time::SystemTime;

use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use futures_util::future;
use futures_util::stream::StreamExt;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use tokio::io;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::error;

use crate::config::Key;
use crate::notifier::Notifier;
use crate::pty::{self, Pty};
use crate::tty::{Tty, TtySize, TtyTheme};
use crate::util::Utf8Decoder;

const BUF_SIZE: usize = 128 * 1024;

#[derive(Clone)]
pub enum Event {
    Output(u64, String),
    Input(u64, String),
    Resize(u64, TtySize),
    Marker(u64, String),
    Exit(u64, i32),
}

#[derive(Clone)]
pub struct Metadata {
    pub time: SystemTime,
    pub term: TermInfo,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: HashMap<String, String>,
}

#[derive(Clone)]
pub struct TermInfo {
    pub type_: Option<String>,
    pub version: Option<String>,
    pub size: TtySize,
    pub theme: Option<TtyTheme>,
}

struct Session<N: Notifier> {
    epoch: Instant,
    events_tx: mpsc::Sender<Event>,
    input_decoder: Utf8Decoder,
    keys: KeyBindings,
    notifier: N,
    output_decoder: Utf8Decoder,
    pause_time: Option<u64>,
    prefix_mode: bool,
    record_input: bool,
    time_offset: u64,
    tty_size: TtySize,
}

#[async_trait]
pub trait Output: Send {
    async fn event(&mut self, event: Event) -> io::Result<()>;
    async fn flush(&mut self) -> io::Result<()>;
}

pub async fn run<S: AsRef<str>, T: Tty + ?Sized, N: Notifier>(
    command: &[S],
    extra_env: &HashMap<String, String>,
    tty: &mut T,
    record_input: bool,
    outputs: Vec<Box<dyn Output>>,
    keys: KeyBindings,
    notifier: N,
) -> anyhow::Result<i32> {
    let epoch = Instant::now();
    let (events_tx, events_rx) = mpsc::channel::<Event>(1024);
    let winsize = tty.get_size();
    let pty = pty::spawn(command, winsize, extra_env)?;
    tokio::spawn(forward_events(events_rx, outputs));

    let mut session = Session {
        epoch,
        events_tx,
        input_decoder: Utf8Decoder::new(),
        keys,
        notifier,
        output_decoder: Utf8Decoder::new(),
        pause_time: None,
        prefix_mode: false,
        record_input,
        time_offset: 0,
        tty_size: winsize.into(),
    };

    session.run(pty, tty).await
}

async fn forward_events(mut events_rx: mpsc::Receiver<Event>, outputs: Vec<Box<dyn Output>>) {
    let mut outputs = outputs;

    while let Some(event) = events_rx.recv().await {
        let futs: Vec<_> = outputs
            .into_iter()
            .map(|output| forward_event(output, event.clone()))
            .collect();

        outputs = future::join_all(futs).await.into_iter().flatten().collect();
    }

    for mut output in outputs {
        if let Err(e) = output.flush().await {
            error!("output flush failed: {e:?}");
        }
    }
}

async fn forward_event(mut output: Box<dyn Output>, event: Event) -> Option<Box<dyn Output>> {
    match output.event(event).await {
        Ok(()) => Some(output),

        Err(e) => {
            error!("output event handler failed: {e:?}");
            None
        }
    }
}

impl<N: Notifier> Session<N> {
    async fn run<T: Tty + ?Sized>(&mut self, pty: Pty, tty: &mut T) -> anyhow::Result<i32> {
        let mut signals =
            Signals::new([SIGWINCH, SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGALRM, SIGCHLD])?;
        let mut output_buf = [0u8; BUF_SIZE];
        let mut input_buf = [0u8; BUF_SIZE];
        let mut input = BytesMut::with_capacity(BUF_SIZE);
        let mut output = BytesMut::with_capacity(BUF_SIZE);
        let mut wait_status = None;

        loop {
            tokio::select! {
                result = pty.read(&mut output_buf) => {
                    let n = result?;

                    if n > 0 {
                        self.handle_output(&output_buf[..n]).await;
                        output.extend_from_slice(&output_buf[0..n]);
                    } else {
                        break;
                    }
                }

                result = pty.write(&input), if !input.is_empty() => {
                    let n = result?;
                    input.advance(n);
                }

                result = tty.read(&mut input_buf) => {
                    let n = result?;

                    if n > 0 {
                        if self.handle_input(&input_buf[..n]).await {
                            input.extend_from_slice(&input_buf[..n]);
                        }
                    } else {
                        break;
                    }
                }

                result = tty.write(&output), if !output.is_empty() => {
                    let n = result?;
                    output.advance(n);
                }

                Some(signal) = signals.next() => {
                    match signal {
                        SIGWINCH => {
                            let winsize = tty.get_size();
                            pty.resize(winsize);
                            self.handle_resize(winsize.into()).await;
                        }

                        SIGINT | SIGTERM | SIGQUIT | SIGHUP => {
                            pty.kill();
                        }

                        SIGCHLD => {
                            if let Ok(status) = pty.wait(Some(WaitPidFlag::WNOHANG)).await {
                                if status != WaitStatus::StillAlive {
                                    wait_status = Some(status);
                                    break;
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
        }

        if !output.is_empty() {
            self.handle_output(&output).await;
            let _ = tty.write_all(&output).await;
        }

        let wait_status = match wait_status {
            Some(ws) => ws,
            None => pty.wait(None).await?,
        };

        let status = match wait_status {
            WaitStatus::Exited(_pid, status) => status,
            WaitStatus::Signaled(_pid, signal, ..) => 128 + signal as i32,
            _ => 1,
        };

        self.handle_exit(status).await;

        Ok(status)
    }

    async fn handle_output(&mut self, data: &[u8]) {
        if self.pause_time.is_none() {
            let text = self.output_decoder.feed(data);

            if !text.is_empty() {
                let event = Event::Output(self.elapsed_time(), text);
                self.send_session_event(event).await;
            }
        }
    }

    async fn handle_input(&mut self, data: &[u8]) -> bool {
        let prefix_key = self.keys.prefix.as_ref();
        let pause_key = self.keys.pause.as_ref();
        let add_marker_key = self.keys.add_marker.as_ref();

        if !self.prefix_mode && prefix_key.is_some_and(|key| data == key) {
            self.prefix_mode = true;
            return false;
        }

        if self.prefix_mode || prefix_key.is_none() {
            self.prefix_mode = false;

            if pause_key.is_some_and(|key| data == key) {
                if let Some(pt) = self.pause_time {
                    self.pause_time = None;
                    self.time_offset += self.elapsed_time() - pt;
                    self.notify("Resumed recording").await;
                } else {
                    self.pause_time = Some(self.elapsed_time());
                    self.notify("Paused recording").await;
                }

                return false;
            } else if add_marker_key.is_some_and(|key| data == key) {
                let event = Event::Marker(self.elapsed_time(), "".to_owned());
                self.send_session_event(event).await;
                self.notify("Marker added").await;
                return false;
            }
        }

        if self.record_input && self.pause_time.is_none() {
            let text = self.input_decoder.feed(data);

            if !text.is_empty() {
                let event = Event::Input(self.elapsed_time(), text);
                self.send_session_event(event).await;
            }
        }

        true
    }

    async fn handle_resize(&mut self, tty_size: TtySize) {
        if tty_size != self.tty_size {
            let event = Event::Resize(self.elapsed_time(), tty_size);
            self.send_session_event(event).await;
            self.tty_size = tty_size;
        }
    }

    async fn handle_exit(&mut self, status: i32) {
        let event = Event::Exit(self.elapsed_time(), status);
        self.send_session_event(event).await;
    }

    fn elapsed_time(&self) -> u64 {
        if let Some(pause_time) = self.pause_time {
            pause_time
        } else {
            self.epoch.elapsed().as_micros() as u64 - self.time_offset
        }
    }

    async fn send_session_event(&mut self, event: Event) {
        self.events_tx
            .send(event)
            .await
            .expect("session event send should succeed");
    }

    async fn notify<S: ToString>(&mut self, text: S) {
        self.notifier
            .notify(text.to_string())
            .await
            .expect("notification should succeed");
    }
}

pub struct KeyBindings {
    pub prefix: Key,
    pub pause: Key,
    pub add_marker: Key,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            prefix: None,
            pause: Some(vec![0x1c]), // ^\
            add_marker: None,
        }
    }
}
