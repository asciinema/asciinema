use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::UNIX_EPOCH;

use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::asciicast;
use crate::encoder::Encoder;
use crate::notifier::Notifier;
use crate::output_writer::OutputWriter;
use crate::session::{self, Metadata};

pub struct FileOutput {
    writer: Box<dyn OutputWriter>,
    encoder: Box<dyn Encoder + Send>,
    notifier: Box<dyn Notifier>,
    metadata: Metadata,
}

pub struct LiveFileOutput {
    commands: Option<mpsc::Sender<Command>>,
    worker: Option<thread::JoinHandle<()>>,
    notifier: Box<dyn Notifier>,
}

enum Command {
    Event(session::Event, oneshot::Sender<io::Result<()>>),
    Finish(oneshot::Sender<io::Result<()>>),
}

impl FileOutput {
    pub fn new(
        writer: Box<dyn OutputWriter>,
        encoder: Box<dyn Encoder + Send>,
        notifier: Box<dyn Notifier>,
        metadata: Metadata,
    ) -> Self {
        Self {
            writer,
            encoder,
            notifier,
            metadata,
        }
    }

    pub async fn start(self) -> io::Result<LiveFileOutput> {
        let header = make_header(&self.metadata);
        let (commands_tx, commands_rx) = mpsc::channel();
        let (started_tx, started_rx) = oneshot::channel();

        let worker = thread::Builder::new()
            .name("asciinema-file-output".to_owned())
            .spawn(move || {
                run_worker(self.writer, self.encoder, header, commands_rx, started_tx)
            })?;

        let mut output = LiveFileOutput {
            commands: Some(commands_tx),
            worker: Some(worker),
            notifier: self.notifier,
        };

        match started_rx.await {
            Ok(Ok(())) => Ok(output),

            Ok(Err(e)) => {
                output.join_worker()?;

                let _ = output
                    .notifier
                    .notify("Write error, session won't be recorded".to_owned())
                    .await;

                Err(e)
            }

            Err(_) => {
                output.join_worker()?;
                Err(worker_failed())
            }
        }
    }
}

#[async_trait]
impl session::Output for LiveFileOutput {
    async fn event(&mut self, event: session::Event) -> io::Result<()> {
        let result = match &self.commands {
            Some(commands) => send_command(commands, |result| Command::Event(event, result)).await,
            None => Err(worker_failed()),
        };

        if let Err(e) = result {
            self.commands.take();
            self.join_worker()?;

            let _ = self
                .notifier
                .notify("Write error, recording suspended".to_owned())
                .await;

            Err(e)
        } else {
            Ok(())
        }
    }

    async fn finish(&mut self) -> io::Result<()> {
        let Some(commands) = self.commands.take() else {
            return Ok(());
        };

        let result = send_command(&commands, Command::Finish).await;
        let join_result = self.join_worker();

        result.and(join_result)
    }
}

impl LiveFileOutput {
    fn join_worker(&mut self) -> io::Result<()> {
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| worker_failed())?;
        }

        Ok(())
    }
}

fn run_worker(
    mut writer: Box<dyn OutputWriter>,
    mut encoder: Box<dyn Encoder + Send>,
    header: asciicast::Header,
    commands: mpsc::Receiver<Command>,
    started: oneshot::Sender<io::Result<()>>,
) {
    let start_result = writer
        .write_all(&encoder.header(&header))
        .and_then(|()| writer.flush());

    if let Err(e) = start_result {
        let _ = writer.finish();
        let _ = started.send(Err(e));
        return;
    }

    if started.send(Ok(())).is_err() {
        let _ = writer.finish();
        return;
    }

    while let Ok(command) = commands.recv() {
        match command {
            Command::Event(event, result) => {
                if let Err(e) = writer.write_all(&encoder.event(event.into())) {
                    let _ = writer.finish();
                    let _ = result.send(Err(e));
                    return;
                }

                let _ = result.send(Ok(()));
            }

            Command::Finish(result) => {
                let write_result = writer.write_all(&encoder.finish());

                let finish_result = match write_result {
                    Ok(()) => writer.finish(),

                    Err(e) => {
                        let _ = writer.finish();
                        Err(e)
                    }
                };

                let _ = result.send(finish_result);

                return;
            }
        }
    }

    let _ = writer.write_all(&encoder.finish());
    let _ = writer.finish();
}

async fn send_command(
    commands: &mpsc::Sender<Command>,
    make_command: impl FnOnce(oneshot::Sender<io::Result<()>>) -> Command,
) -> io::Result<()> {
    let (result_tx, result_rx) = oneshot::channel();

    commands
        .send(make_command(result_tx))
        .map_err(|_| worker_failed())?;

    result_rx.await.unwrap_or_else(|_| Err(worker_failed()))
}

fn make_header(metadata: &Metadata) -> asciicast::Header {
    let timestamp = metadata
        .time
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());

    asciicast::Header {
        term_cols: metadata.term.size.0,
        term_rows: metadata.term.size.1,
        term_type: metadata.term.type_.clone(),
        term_version: metadata.term.version.clone(),
        term_theme: metadata.term.theme.clone(),
        timestamp,
        idle_time_limit: metadata.idle_time_limit,
        command: metadata.command.clone(),
        title: metadata.title.clone(),
        env: Some(metadata.env.clone()),
    }
}

fn worker_failed() -> io::Error {
    io::Error::other("file output worker failed")
}

impl From<session::Event> for asciicast::Event {
    fn from(event: session::Event) -> Self {
        match event {
            session::Event::Output(time, text) => asciicast::Event::output(time, text),
            session::Event::Input(time, text) => asciicast::Event::input(time, text),
            session::Event::Resize(time, tty_size) => {
                asciicast::Event::resize(time, tty_size.into())
            }
            session::Event::Marker(time, label) => asciicast::Event::marker(time, label),
            session::Event::Exit(time, status) => asciicast::Event::exit(time, status),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::time::{Duration, SystemTime};

    use tempfile::tempdir;

    use super::*;
    use crate::encoder::{AsciicastV2Encoder, AsciicastV3Encoder, Encoder};
    use crate::notifier::NullNotifier;
    use crate::output_writer;
    use crate::session::{Output, TermInfo};
    use crate::tty::TtySize;

    #[test]
    fn writes_appended_zstd_frames() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recording.cast.zst");

        write_recording(&path, false, true, "first", Duration::from_secs(1));
        write_recording(&path, true, true, "second", Duration::from_secs(2));

        assert!(asciicast::is_zstd(&path).unwrap());

        let cast = asciicast::open_from_path(&path).unwrap();
        let events = cast.events.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events.len(), 3);
        assert_eq!(events.last().unwrap().time, Duration::from_secs(3));
        assert_eq!(
            asciicast::get_duration(path).unwrap(),
            Duration::from_secs(3)
        );
    }

    #[test]
    fn writes_plain_recording() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recording.cast");

        write_recording(&path, false, false, "output", Duration::from_secs(1));

        assert!(!asciicast::is_zstd(&path).unwrap());
        assert_eq!(asciicast::open_from_path(path).unwrap().events.count(), 1);
    }

    #[test]
    fn writes_appended_zstd_v2_frames() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recording.cast.zst");

        write_v2_recording(&path, false, "first", Duration::from_secs(1));
        write_v2_recording(&path, true, "second", Duration::from_secs(2));

        let cast = asciicast::open_from_path(&path).unwrap();
        assert_eq!(cast.version, asciicast::Version::Two);
        assert_eq!(
            cast.events.last().unwrap().unwrap().time,
            Duration::from_secs(3)
        );
    }

    fn write_recording(
        path: &std::path::Path,
        append: bool,
        compressed: bool,
        text: &str,
        time: Duration,
    ) {
        let file = fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create_new(!append)
            .open(path)
            .unwrap();
        write_with_encoder(
            file,
            compressed,
            Box::new(AsciicastV3Encoder::new(append)),
            text,
            time,
        );
    }

    fn write_v2_recording(path: &std::path::Path, append: bool, text: &str, time: Duration) {
        let time_offset = if append {
            asciicast::get_duration(path).unwrap()
        } else {
            Duration::ZERO
        };
        let file = fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create_new(!append)
            .open(path)
            .unwrap();

        write_with_encoder(
            file,
            true,
            Box::new(AsciicastV2Encoder::new(append, time_offset)),
            text,
            time,
        );
    }

    fn write_with_encoder(
        file: fs::File,
        compressed: bool,
        encoder: Box<dyn Encoder + Send>,
        text: &str,
        time: Duration,
    ) {
        let writer = output_writer::new(file, compressed).unwrap();
        let metadata = Metadata {
            time: SystemTime::now(),
            term: TermInfo {
                type_: None,
                version: None,
                size: TtySize(80, 24),
                theme: None,
            },
            idle_time_limit: None,
            command: None,
            title: None,
            env: HashMap::new(),
        };
        let file_output = FileOutput::new(writer, encoder, Box::new(NullNotifier), metadata);
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async {
            let mut output = file_output.start().await.unwrap();
            output
                .event(session::Event::Output(time, text.to_owned()))
                .await
                .unwrap();
            output.finish().await.unwrap();
        });
    }
}
