use crate::config::Key;
use crate::notifier::Notifier;
use crate::pty;
use crate::tty;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct Recorder {
    output: Option<Box<dyn Output + Send>>,
    start_time: Instant,
    pause_time: Option<u64>,
    record_input: bool,
    keys: KeyBindings,
    notifier: Option<Box<dyn Notifier>>,
    sender: mpsc::Sender<Message>,
    receiver: Option<mpsc::Receiver<Message>>,
    handle: Option<JoinHandle>,
    prefix_mode: bool,
}

pub trait Output {
    fn start(&mut self, timestamp: u64, tty_size: &tty::TtySize) -> io::Result<()>;
    fn output(&mut self, time: u64, data: &[u8]) -> io::Result<()>;
    fn input(&mut self, time: u64, data: &[u8]) -> io::Result<()>;
    fn resize(&mut self, time: u64, size: (u16, u16)) -> io::Result<()>;
    fn marker(&mut self, time: u64) -> io::Result<()>;
}

enum Message {
    Output(u64, Vec<u8>),
    Input(u64, Vec<u8>),
    Resize(u64, tty::TtySize),
    Marker(u64),
    Notification(String),
}

struct JoinHandle(Option<thread::JoinHandle<()>>);

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
            start_time: Instant::now(),
            pause_time: None,
            record_input,
            keys,
            notifier: Some(notifier),
            sender,
            receiver: Some(receiver),
            handle: None,
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

impl pty::Recorder for Recorder {
    fn start(&mut self, tty_size: tty::TtySize) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut output = self.output.take().unwrap();
        output.start(timestamp, &tty_size)?;
        let receiver = self.receiver.take().unwrap();
        let mut notifier = self.notifier.take().unwrap();

        let handle = thread::spawn(move || {
            use Message::*;
            let mut last_tty_size = tty_size;

            for msg in receiver {
                match msg {
                    Output(time, data) => {
                        let _ = output.output(time, &data);
                    }

                    Input(time, data) => {
                        let _ = output.input(time, &data);
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
        });

        self.handle = Some(JoinHandle(Some(handle)));
        self.start_time = Instant::now();

        Ok(())
    }

    fn output(&mut self, data: &[u8]) {
        if self.pause_time.is_some() {
            return;
        }

        let msg = Message::Output(self.elapsed_time(), data.into());
        self.sender.send(msg).expect("output send should succeed");
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
                return false;
            }
        }

        if self.record_input && self.pause_time.is_none() {
            // TODO ignore OSC responses
            let msg = Message::Input(self.elapsed_time(), data.into());
            self.sender.send(msg).expect("input send should succeed");
        }

        true
    }

    fn resize(&mut self, size: tty::TtySize) {
        let msg = Message::Resize(self.elapsed_time(), size);
        self.sender.send(msg).expect("resize send should succeed");
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        self.0
            .take()
            .unwrap()
            .join()
            .expect("worker thread should finish cleanly");
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
