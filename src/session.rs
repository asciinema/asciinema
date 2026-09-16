use std::collections::HashMap;
use std::time::{Duration, SystemTime};

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
use crate::tty::{RawTty, TtySize, TtyTheme};
use crate::util::Utf8Decoder;

const BUF_SIZE: usize = 128 * 1024;

#[derive(Clone)]
pub enum Event {
    Output(Duration, String),
    Input(Duration, String),
    Resize(Duration, TtySize),
    Marker(Duration, String),
    Exit(Duration, i32),
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
    capture_input: bool,
    epoch: Instant,
    events_tx: mpsc::Sender<Event>,
    input_decoder: Utf8Decoder,
    keys: KeyBindings,
    notifier: N,
    output_decoder: Utf8Decoder,
    pause_time: Option<Duration>,
    prefix_mode: bool,
    time_offset: Duration,
    tty_size: TtySize,
}

#[async_trait]
pub trait Output: Send {
    async fn event(&mut self, event: Event) -> io::Result<()>;
    async fn finish(&mut self) -> io::Result<()>;
}

/// An [`Output`], plus how the session treats it.
///
/// Sinks are fed concurrently, in the order the events happen. A sink which
/// fails is dropped from the fan-out, and only an essential one fails the
/// session along with it.
pub struct Sink {
    output: Box<dyn Output>,
    essential: bool,
}

impl Sink {
    pub fn new(output: impl Output + 'static) -> Self {
        Self {
            output: Box::new(output),
            essential: false,
        }
    }

    /// Makes a failure of the output fail the session. Failures of ordinary
    /// sinks are only logged.
    pub fn essential(mut self) -> Self {
        self.essential = true;

        self
    }

    async fn event(&mut self, event: &Event) -> io::Result<()> {
        self.output.event(event.clone()).await
    }

    /// Reports a failure, keeping the first one which fails the session.
    fn failed(&self, error: io::Error, failure: &mut Option<io::Error>) {
        if self.essential {
            failure.get_or_insert(error);
        } else {
            error!("output failed: {error:?}");
        }
    }
}

/// Runs `command` in a pty, feeding its terminal output - and, when
/// `capture_input` is on, the input it gets from the keyboard - to every
/// sink.
pub async fn run<S: AsRef<str>, T: RawTty + ?Sized, N: Notifier>(
    command: &[S],
    extra_env: &HashMap<String, String>,
    tty: &mut T,
    capture_input: bool,
    sinks: Vec<Sink>,
    keys: KeyBindings,
    notifier: N,
) -> anyhow::Result<i32> {
    let epoch = Instant::now();
    let (events_tx, events_rx) = mpsc::channel::<Event>(1024);
    let winsize = tty.get_size();
    let pty = pty::spawn(command, winsize, extra_env)?;
    let forwarder = tokio::spawn(forward_events(events_rx, sinks));

    let session = Session {
        capture_input,
        epoch,
        events_tx,
        input_decoder: Utf8Decoder::new(),
        keys,
        notifier,
        output_decoder: Utf8Decoder::new(),
        pause_time: None,
        prefix_mode: false,
        time_offset: Duration::from_micros(0),
        tty_size: winsize.into(),
    };

    let result = session.run(pty, tty).await;

    // Wait for the sinks to be finished, so that the recording is flushed and
    // closed before a failure is reported.
    let sink_error = forwarder
        .await
        .map_err(|e| anyhow::anyhow!("event forwarder failed: {e}"))?;

    // The session's own failure takes precedence over a failed sink.
    let status = result?;

    match sink_error {
        Some(error) => Err(error.into()),
        None => Ok(status),
    }
}

/// Feeds session events to all sinks until the session ends, then finishes
/// them. Returns the first failure of an essential sink, if any.
async fn forward_events(
    mut events_rx: mpsc::Receiver<Event>,
    mut sinks: Vec<Sink>,
) -> Option<io::Error> {
    let mut failure = None;

    while let Some(event) = events_rx.recv().await {
        let results = future::join_all(sinks.iter_mut().map(|sink| sink.event(&event))).await;

        sinks = sinks
            .into_iter()
            .zip(results)
            .filter_map(|(sink, result)| match result {
                Ok(()) => Some(sink),

                Err(e) => {
                    sink.failed(e, &mut failure);
                    None
                }
            })
            .collect();
    }

    for mut sink in sinks {
        if let Err(e) = sink.output.finish().await {
            sink.failed(e, &mut failure);
        }
    }

    failure
}

impl<N: Notifier> Session<N> {
    async fn run<T: RawTty + ?Sized>(mut self, pty: Pty, tty: &mut T) -> anyhow::Result<i32> {
        let mut signals =
            Signals::new([SIGWINCH, SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGALRM, SIGCHLD])?;

        // Kept on the heap so the session future, and the futures holding it,
        // stay small enough for the default thread stack.
        let mut output_buf = vec![0u8; BUF_SIZE];
        let mut input_buf = vec![0u8; BUF_SIZE];
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

        while let Ok(n) = pty.read(&mut output_buf).await {
            if n > 0 {
                self.handle_output(&output_buf[..n]).await;
                output.extend_from_slice(&output_buf[0..n]);
            } else {
                break;
            }
        }

        if !output.is_empty() {
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

        if self.capture_input && self.pause_time.is_none() {
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

    fn elapsed_time(&self) -> Duration {
        if let Some(pause_time) = self.pause_time {
            pause_time
        } else {
            self.epoch.elapsed() - self.time_offset
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

#[cfg(test)]
mod tests {
    use std::future::pending;
    use std::sync::{Arc, Mutex};

    use nix::pty::Winsize;
    use tempfile::tempdir;

    use super::*;
    use crate::encoder::AsciicastV3Encoder;
    use crate::file_output::FileOutput;
    use crate::hook::Hook;
    use crate::notifier::NullNotifier;
    use crate::output_writer;
    use crate::spawn::Context;

    /// How a [`TestOutput`] fails, when it does.
    #[derive(Clone, Copy, PartialEq)]
    enum Fails {
        Events,
        Finish,
    }

    /// An output which remembers what it was fed, and can be made to fail.
    #[derive(Default)]
    struct TestOutput {
        events: Arc<Mutex<Vec<Event>>>,
        fail: Option<Fails>,
    }

    #[async_trait]
    impl Output for TestOutput {
        async fn event(&mut self, event: Event) -> io::Result<()> {
            if self.fail == Some(Fails::Events) {
                return Err(io::Error::other("event failed"));
            }
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        async fn finish(&mut self) -> io::Result<()> {
            if self.fail == Some(Fails::Finish) {
                Err(io::Error::other("finish failed"))
            } else {
                Ok(())
            }
        }
    }

    /// A tty which yields one burst of input, and then nothing ever again.
    struct TestTty(Mutex<Option<Vec<u8>>>);

    #[async_trait(?Send)]
    impl RawTty for TestTty {
        fn get_size(&self) -> Winsize {
            Winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }
        }

        async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
            if let Some(input) = self.0.lock().unwrap().take() {
                buf[..input.len()].copy_from_slice(&input);
                return Ok(input.len());
            }
            pending().await
        }

        async fn write(&self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
    }

    fn test_sink() -> (Sink, Arc<Mutex<Vec<Event>>>) {
        let output = TestOutput::default();
        let events = output.events.clone();
        (Sink::new(output), events)
    }

    /// Extracts inputs or outputs from the recorded test events
    fn extract_texts(events: &Mutex<Vec<Event>>, is_input: bool) -> Vec<String> {
        events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match (is_input, e) {
                (true, Event::Input(_, text)) => Some(text.clone()),
                (false, Event::Output(_, text)) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Runs a session which reads a single line from the tty.
    async fn read_line(input: &str, capture_input: bool, sinks: Vec<Sink>) -> i32 {
        run(
            &["sh", "-c", "read value"],
            &HashMap::new(),
            &mut TestTty(Mutex::new(Some(input.as_bytes().to_vec()))),
            capture_input,
            sinks,
            KeyBindings::default(),
            NullNotifier,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_sink_routing_and_fault_tolerance() {
        // Scenario A: Input filtering logic across multiple sinks
        for capture_input in [true, false] {
            let (healthy1, events1) = test_sink();
            let (healthy2, events2) = test_sink();

            assert_eq!(
                read_line("hi\n", capture_input, vec![healthy1, healthy2]).await,
                0
            );

            let expected: Vec<&str> = if capture_input { vec!["hi\n"] } else { vec![] };
            assert_eq!(extract_texts(&events1, true), expected);
            assert_eq!(extract_texts(&events2, true), expected);
        }

        // Scenario B: Sink failure logic (Essential vs Non-essential & Event vs Finish failures)
        for fail_stage in [Fails::Events, Fails::Finish] {
            for essential in [true, false] {
                let (healthy, events) = test_sink();
                let failing_output = TestOutput {
                    fail: Some(fail_stage),
                    ..Default::default()
                };

                let failing_sink = if essential {
                    Sink::new(failing_output).essential()
                } else {
                    Sink::new(failing_output)
                };

                let (events_tx, events_rx) = mpsc::channel(2);
                events_tx
                    .send(Event::Output(Duration::from_secs(1), "a".to_owned()))
                    .await
                    .unwrap();
                events_tx
                    .send(Event::Output(Duration::from_secs(1), "b".to_owned()))
                    .await
                    .unwrap();
                drop(events_tx);

                let failure = forward_events(events_rx, vec![failing_sink, healthy]).await;

                // Only essential failures crash the session.
                assert_eq!(failure.is_some(), essential);
                // Regardless of the other sink failing, the healthy one gets everything.
                assert_eq!(extract_texts(&events, false), vec!["a", "b"]);
            }
        }
    }

    #[tokio::test]
    async fn test_live_hook_and_recording_parity() {
        // Verifies that hooks mirror the recording file exactly,
        // validating both metadata inclusion and the conditional rendering of input events.
        for capture_input in [true, false] {
            let dir = tempdir().unwrap();
            let hook_path = dir.path().join("hook.cast");
            let recording_path = dir.path().join("recording.cast");

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

            let hook = Hook(format!("cat > {}", hook_path.display()))
                .start(
                    &metadata,
                    &Context {
                        session_id: "test-session",
                        output_file: recording_path.to_str(),
                        server_url: None,
                    },
                    Box::new(NullNotifier),
                )
                .unwrap();

            let file = std::fs::File::create(&recording_path).unwrap();
            let recording = Sink::new(
                FileOutput::new(
                    output_writer::new(file, false).unwrap(),
                    Box::new(AsciicastV3Encoder::new(false)),
                    Box::new(NullNotifier),
                    metadata,
                )
                .start()
                .await
                .unwrap(),
            );

            read_line("hunter2\n", capture_input, vec![hook, recording]).await;

            let hook_stream = std::fs::read_to_string(&hook_path).unwrap();
            let recording_stream = std::fs::read_to_string(&recording_path).unwrap();

            // The hook must see exactly the bytes of the recording file.
            assert_eq!(hook_stream, recording_stream);

            // Ensure capturing behavior translates to the payload.
            if capture_input {
                assert!(hook_stream.contains(r#""i", "hunter2\n""#), "{hook_stream}");
                assert!(hook_stream.lines().last().unwrap().contains(r#""x""#));
            } else {
                assert!(!hook_stream.contains(r#""i", "#), "{hook_stream}");
            }
        }
    }
}
