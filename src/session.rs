use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::config::Key;
use crate::notifier::Notifier;
use crate::pty;
use crate::tty;
use crate::util::{JoinHandle, Utf8Decoder};

pub struct Session<N> {
    outputs: Vec<Box<dyn Output + Send>>,
    input_decoder: Utf8Decoder,
    output_decoder: Utf8Decoder,
    tty_size: tty::TtySize,
    record_input: bool,
    keys: KeyBindings,
    notifier: N,
    sender: mpsc::Sender<Event>,
    receiver: Option<Receiver<Event>>,
    handle: Option<JoinHandle>,
    time_offset: u64,
    pause_time: Option<u64>,
    prefix_mode: bool,
}

pub trait Output {
    fn start(
        &mut self,
        time: SystemTime,
        tty_size: tty::TtySize,
        theme: Option<tty::Theme>,
    ) -> io::Result<()>;
    fn event(&mut self, event: Event) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

#[derive(Clone)]
pub enum Event {
    Output(u64, String),
    Input(u64, String),
    Resize(u64, tty::TtySize),
    Marker(u64, String),
}

impl<N: Notifier> Session<N> {
    pub fn new(
        outputs: Vec<Box<dyn Output + Send>>,
        record_input: bool,
        keys: KeyBindings,
        notifier: N,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();

        Session {
            outputs,
            input_decoder: Utf8Decoder::new(),
            output_decoder: Utf8Decoder::new(),
            tty_size: tty::TtySize::default(),
            record_input,
            keys,
            notifier,
            sender,
            receiver: Some(receiver),
            handle: None,
            time_offset: 0,
            pause_time: None,
            prefix_mode: false,
        }
    }

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
    fn start(&mut self, tty_size: tty::TtySize, tty_theme: Option<tty::Theme>) {
        let mut outputs = std::mem::take(&mut self.outputs);
        let time = SystemTime::now();
        let receiver = self.receiver.take().unwrap();

        let handle = thread::spawn(move || {
            outputs.retain_mut(|output| output.start(time, tty_size, tty_theme.clone()).is_ok());

            for event in receiver {
                outputs.retain_mut(|output| output.event(event.clone()).is_ok())
            }

            for mut output in outputs {
                let _ = output.flush();
            }
        });

        self.handle = Some(JoinHandle::new(handle));
    }

    fn output(&mut self, time: Duration, data: &[u8]) -> bool {
        if self.pause_time.is_none() {
            let text = self.output_decoder.feed(data);

            if !text.is_empty() {
                let msg = Event::Output(self.elapsed_time(time), text);
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
                let msg = Event::Marker(self.elapsed_time(time), "".to_owned());
                self.sender.send(msg).expect("marker send should succeed");
                self.notify("Marker added");
                return false;
            }
        }

        if self.record_input && self.pause_time.is_none() {
            let text = self.input_decoder.feed(data);

            if !text.is_empty() {
                let msg = Event::Input(self.elapsed_time(time), text);
                self.sender.send(msg).expect("input send should succeed");
            }
        }

        true
    }

    fn resize(&mut self, time: Duration, tty_size: tty::TtySize) -> bool {
        if tty_size != self.tty_size {
            let msg = Event::Resize(self.elapsed_time(time), tty_size);
            self.sender.send(msg).expect("resize send should succeed");

            self.tty_size = tty_size;
        }

        true
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
