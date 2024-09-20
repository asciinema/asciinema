use crate::config::Key;
use crate::notifier::Notifier;
use crate::pty;
use crate::tty;
use crate::util;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub struct Recorder {
    output: Option<Box<dyn Output + Send>>,
    record_input: bool,
    keys: KeyBindings,
    notifier: Option<Box<dyn Notifier>>,
    sender: mpsc::Sender<Message>,
    receiver: Option<mpsc::Receiver<Message>>,
    handle: Option<util::JoinHandle>,
    start_time: Instant,
    pause_time: Option<u64>,
    prefix_mode: bool,
}

pub trait Output {
    fn start(&mut self, tty_size: &tty::TtySize) -> io::Result<()>;
    fn output(&mut self, time: u64, text: String) -> io::Result<()>;
    fn input(&mut self, time: u64, text: String) -> io::Result<()>;
    fn resize(&mut self, time: u64, size: (u16, u16)) -> io::Result<()>;
    fn marker(&mut self, time: u64) -> io::Result<()>;

    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

enum Message {
    Output(u64, Vec<u8>),
    Input(u64, Vec<u8>),
    Resize(u64, tty::TtySize),
    Marker(u64),
    Notification(String),
}

impl Recorder {
    pub fn new(
        output: Box<dyn Output + Send>,
        record_input: bool,
        keys: KeyBindings,
        notifier: Box<dyn Notifier>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();

        Recorder {
            output: Some(output),
            record_input,
            keys,
            notifier: Some(notifier),
            sender,
            receiver: Some(receiver),
            handle: None,
            start_time: Instant::now(),
            pause_time: None,
            prefix_mode: false,
        }
    }

    fn elapsed_time(&self) -> u64 {
        if let Some(pause_time) = self.pause_time {
            pause_time
        } else {
            self.start_time.elapsed().as_micros() as u64
        }
    }

    fn notify<S: ToString>(&self, text: S) {
        let msg = Message::Notification(text.to_string());

        self.sender
            .send(msg)
            .expect("notification send should succeed");
    }
}

impl pty::Handler for Recorder {
    fn start(&mut self, tty_size: tty::TtySize) {
        let mut output = self.output.take().unwrap();
        let _ = output.start(&tty_size);
        let receiver = self.receiver.take().unwrap();
        let mut notifier = self.notifier.take().unwrap();

        let handle = thread::spawn(move || {
            use Message::*;
            let mut last_tty_size = tty_size;
            let mut input_decoder = util::Utf8Decoder::new();
            let mut output_decoder = util::Utf8Decoder::new();

            for msg in receiver {
                match msg {
                    Output(time, data) => {
                        let text = output_decoder.feed(&data);

                        if !text.is_empty() {
                            let _ = output.output(time, text);
                        }
                    }

                    Input(time, data) => {
                        let text = input_decoder.feed(&data);

                        if !text.is_empty() {
                            let _ = output.input(time, text);
                        }
                    }

                    Resize(time, new_tty_size) => {
                        if new_tty_size != last_tty_size {
                            let _ = output.resize(time, new_tty_size.into());
                            last_tty_size = new_tty_size;
                        }
                    }

                    Marker(time) => {
                        let _ = output.marker(time);
                    }

                    Notification(text) => {
                        let _ = notifier.notify(text);
                    }
                }
            }

            let _ = output.finish();
        });

        self.handle = Some(util::JoinHandle::new(handle));
        self.start_time = Instant::now();
    }

    fn output(&mut self, data: &[u8]) -> bool {
        if self.pause_time.is_none() {
            let msg = Message::Output(self.elapsed_time(), data.into());
            self.sender.send(msg).expect("output send should succeed");
        }

        true
    }

    fn input(&mut self, data: &[u8]) -> bool {
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
                    self.start_time = Instant::now() - Duration::from_micros(pt);
                    self.pause_time = None;
                    self.notify("Resumed recording");
                } else {
                    self.pause_time = Some(self.elapsed_time());
                    self.notify("Paused recording");
                }

                return false;
            } else if add_marker_key.is_some_and(|key| data == key) {
                let msg = Message::Marker(self.elapsed_time());
                self.sender.send(msg).expect("marker send should succeed");
                self.notify("Marker added");
                return false;
            }
        }

        if self.record_input && self.pause_time.is_none() {
            let msg = Message::Input(self.elapsed_time(), data.into());
            self.sender.send(msg).expect("input send should succeed");
        }

        true
    }

    fn resize(&mut self, size: tty::TtySize) -> bool {
        let msg = Message::Resize(self.elapsed_time(), size);
        self.sender.send(msg).expect("resize send should succeed");

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
