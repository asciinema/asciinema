use crate::format;
use crate::pty;
use std::collections::HashMap;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct Recorder {
    writer: Option<Box<dyn format::Writer + Send>>,
    start_time: Instant,
    append: bool,
    record_input: bool,
    idle_time_limit: Option<f32>,
    command: Option<String>,
    title: Option<String>,
    env: HashMap<String, String>,
    sender: mpsc::Sender<Message>,
    receiver: Option<mpsc::Receiver<Message>>,
    handle: Option<JoinHandle>,
}

enum Message {
    Output(u64, Vec<u8>),
    Input(u64, Vec<u8>),
    Resize(u64, (u16, u16)),
}

struct JoinHandle(Option<thread::JoinHandle<()>>);

impl Recorder {
    pub fn new(
        writer: Box<dyn format::Writer + Send>,
        append: bool,
        record_input: bool,
        idle_time_limit: Option<f32>,
        command: Option<String>,
        title: Option<String>,
        env: HashMap<String, String>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();

        Recorder {
            writer: Some(writer),
            start_time: Instant::now(),
            append,
            record_input,
            idle_time_limit,
            command,
            title,
            env,
            sender,
            receiver: Some(receiver),
            handle: None,
        }
    }

    fn elapsed_time(&self) -> u64 {
        self.start_time.elapsed().as_micros() as u64
    }
}

impl pty::Recorder for Recorder {
    fn start(&mut self, size: (u16, u16)) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut writer = self.writer.take().unwrap();
        let receiver = self.receiver.take().unwrap();

        if !self.append {
            let header = format::Header {
                cols: size.0,
                rows: size.1,
                timestamp,
                idle_time_limit: self.idle_time_limit,
                command: self.command.clone(),
                title: self.title.clone(),
                env: self.env.clone(),
            };

            writer.header(&header)?;
        }

        let handle = thread::spawn(move || {
            for msg in receiver {
                match msg {
                    Message::Output(time, data) => {
                        let _ = writer.output(time, &data);
                    }

                    Message::Input(time, data) => {
                        let _ = writer.input(time, &data);
                    }

                    Message::Resize(time, size) => {
                        let _ = writer.resize(time, size);
                    }
                }
            }
        });

        self.handle = Some(JoinHandle(Some(handle)));
        self.start_time = Instant::now();

        Ok(())
    }

    fn output(&mut self, data: &[u8]) {
        let msg = Message::Output(self.elapsed_time(), data.into());
        let _ = self.sender.send(msg);
        // TODO use notifier for error reporting
    }

    fn input(&mut self, data: &[u8]) {
        if self.record_input {
            let msg = Message::Input(self.elapsed_time(), data.into());
            let _ = self.sender.send(msg);
            // TODO use notifier for error reporting
        }
    }

    fn resize(&mut self, size: (u16, u16)) {
        let msg = Message::Resize(self.elapsed_time(), size);
        let _ = self.sender.send(msg);
        // TODO use notifier for error reporting
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        self.0.take().unwrap().join().expect("Thread panicked");
    }
}
