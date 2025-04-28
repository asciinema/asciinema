use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use tracing::error;

use crate::config::Key;
use crate::notifier::Notifier;
use crate::pty;
use crate::tty::{TtySize, TtyTheme};
use crate::util::{JoinHandle, Utf8Decoder};

pub struct SessionStarter<N> {
    starters: Vec<Box<dyn OutputStarter>>,
    record_input: bool,
    keys: KeyBindings,
    notifier: N,
}

pub trait OutputStarter {
    fn start(
        self: Box<Self>,
        time: SystemTime,
        tty_size: TtySize,
        theme: Option<TtyTheme>,
    ) -> io::Result<Box<dyn Output>>;
}

pub trait Output: Send {
    fn event(&mut self, event: Event) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

#[derive(Clone)]
pub enum Event {
    Output(u64, String, Option<u32>),
    Input(u64, String, Option<u32>),
    Resize(u64, TtySize, Option<u32>),
    Marker(u64, String, Option<u32>),
}

impl<N: Notifier> SessionStarter<N> {
    pub fn new(
        starters: Vec<Box<dyn OutputStarter>>,
        record_input: bool,
        keys: KeyBindings,
        notifier: N,
    ) -> Self {
        SessionStarter {
            starters,
            record_input,
            keys,
            notifier,
        }
    }
}

impl<N: Notifier> pty::HandlerStarter<Session<N>> for SessionStarter<N> {
    fn start(self, tty_size: TtySize, tty_theme: Option<TtyTheme>, child_pid: u32) -> Session<N> {
        let time = SystemTime::now();
        let mut outputs = Vec::new();

        for starter in self.starters {
            match starter.start(time, tty_size, tty_theme.clone()) {
                Ok(output) => {
                    outputs.push(output);
                }

                Err(e) => {
                    error!("output startup failed: {e:?}");
                }
            }
        }

        let (sender, receiver) = mpsc::channel::<Event>();

        let handle = thread::spawn(move || {
            for event in receiver {
                outputs.retain_mut(|output| match output.event(event.clone()) {
                    Ok(_) => true,

                    Err(e) => {
                        error!("output event handler failed: {e:?}");

                        false
                    }
                });
            }

            for mut output in outputs {
                match output.flush() {
                    Ok(_) => {}

                    Err(e) => {
                        error!("output flush failed: {e:?}");
                    }
                }
            }
        });

        Session {
            notifier: self.notifier,
            input_decoder: Utf8Decoder::new(),
            output_decoder: Utf8Decoder::new(),
            record_input: self.record_input,
            keys: self.keys,
            tty_size,
            sender,
            time_offset: 0,
            pause_time: None,
            prefix_mode: false,
            _handle: JoinHandle::new(handle),
            child_pid,
        }
    }
}

pub struct Session<N> {
    notifier: N,
    input_decoder: Utf8Decoder,
    output_decoder: Utf8Decoder,
    tty_size: TtySize,
    record_input: bool,
    keys: KeyBindings,
    sender: mpsc::Sender<Event>,
    time_offset: u64,
    pause_time: Option<u64>,
    prefix_mode: bool,
    _handle: JoinHandle,
    child_pid: u32,
}

impl<N: Notifier> Session<N> {
    fn elapsed_time(&self, time: Duration) -> u64 {
        if let Some(pause_time) = self.pause_time {
            pause_time
        } else {
            time.as_micros() as u64 - self.time_offset
        }
    }

    fn notify<S: ToString>(&mut self, text: S) {
        self.notifier
            .notify(text.to_string())
            .expect("notification should succeed");
    }
}

impl<N: Notifier> pty::Handler for Session<N> {
    fn output(&mut self, time: Duration, data: &[u8]) -> bool {
        if self.pause_time.is_none() {
            let text = self.output_decoder.feed(data);

            if !text.is_empty() {
                let msg = Event::Output(self.elapsed_time(time), text, Some(self.child_pid));
                self.sender.send(msg).expect("output send should succeed");
            }
        }

        true
    }

    fn input(&mut self, time: Duration, data: &[u8]) -> bool {
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
                    self.time_offset += self.elapsed_time(time) - pt;
                    self.notify("Resumed recording");
                } else {
                    self.pause_time = Some(self.elapsed_time(time));
                    self.notify("Paused recording");
                }

                return false;
            } else if add_marker_key.is_some_and(|key| data == key) {
                let msg = Event::Marker(self.elapsed_time(time), "".to_owned(), Some(self.child_pid));
                self.sender.send(msg).expect("marker send should succeed");
                self.notify("Marker added");
                return false;
            }
        }

        if self.record_input && self.pause_time.is_none() {
            let text = self.input_decoder.feed(data);

            if !text.is_empty() {
                let msg = Event::Input(self.elapsed_time(time), text, Some(self.child_pid));
                self.sender.send(msg).expect("input send should succeed");
            }
        }

        true
    }

    fn resize(&mut self, time: Duration, tty_size: TtySize) -> bool {
        if tty_size != self.tty_size {
            let msg = Event::Resize(self.elapsed_time(time), tty_size, Some(self.child_pid));
            self.sender.send(msg).expect("resize send should succeed");

            self.tty_size = tty_size;
        }

        true
    }

    fn stop(self) -> Self {
        self
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
