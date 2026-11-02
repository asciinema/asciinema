use std::io;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time;

use crate::asciicast;
use crate::encoder::{AsciicastV3Encoder, Encoder};
use crate::notifier::Notifier;
use crate::session::{self, Event, Metadata, Sink};
use crate::spawn::{self, Context};
use crate::status;

/// How many session events may pile up for a hook which doesn't read its
/// standard input fast enough.
const MAX_QUEUED_EVENTS: usize = 128;

/// How long a hook may keep running, after its standard input is closed,
/// before asciinema tells the user it's waiting for it.
const QUIET_WAIT_TIME: Duration = Duration::from_secs(2);

/// A command fed with the live asciicast v3 stream of a session.
#[derive(Debug, Clone, Deserialize)]
pub struct Hook(pub String);

impl Hook {
    /// Starts the hook, as a sink whose failure fails the session.
    pub fn start(
        &self,
        metadata: &Metadata,
        context: &Context,
        notifier: Box<dyn Notifier>,
    ) -> io::Result<Sink> {
        Ok(Sink::new(self.spawn(metadata, context, notifier)?).essential())
    }

    fn spawn(
        &self,
        metadata: &Metadata,
        context: &Context,
        notifier: Box<dyn Notifier>,
    ) -> io::Result<LiveHook> {
        let mut child = spawn::shell(&self.0, context, Stdio::piped(), Stdio::null())?;

        let stdin = child
            .take_stdin()
            .expect("hook stdin should be piped when spawning");

        let (events, events_rx) = mpsc::channel(MAX_QUEUED_EVENTS);
        let header = AsciicastV3Encoder::new(false).header(&asciicast::Header::from(metadata));

        Ok(LiveHook {
            command: self.0.clone(),
            events: Some(events),
            writer: Some(tokio::spawn(write_stream(stdin, header, events_rx))),
            child,
            notifier,
        })
    }
}

/// A running [`Hook`], fed by the session.
struct LiveHook {
    command: String,
    events: Option<mpsc::Sender<Event>>,
    writer: Option<JoinHandle<io::Result<()>>>,
    child: spawn::Child,
    notifier: Box<dyn Notifier>,
}

impl LiveHook {
    async fn shutdown(&mut self) -> io::Result<()> {
        self.events.take();

        let write_result = match self.writer.take() {
            Some(writer) => writer.await.unwrap_or_else(|e| Err(io::Error::other(e))),
            None => Ok(()),
        };

        self.wait().await.and(write_result)
    }

    async fn wait(&mut self) -> io::Result<()> {
        let status = match time::timeout(QUIET_WAIT_TIME, self.child.wait()).await {
            Ok(status) => status?,

            Err(_) => {
                status::info!("Waiting for hook to finish: {}", self.command);

                self.child.wait().await?
            }
        };

        if status.success() {
            return Ok(());
        }

        Err(self.status_error(status))
    }

    /// The error for a hook which exited with a non-zero status, including
    /// its last stderr line, when it logged one.
    fn status_error(&self, status: ExitStatus) -> io::Error {
        io::Error::other(match self.child.last_error_line().as_deref() {
            Some(line) => format!("hook `{}` {status}: {line}", self.command),
            None => format!("hook `{}` {status}", self.command),
        })
    }

    /// Shuts the hook down and reports why it stopped.
    async fn fail(&mut self) -> io::Error {
        // A hook which already exited on its own with eror, explains
        // a real broken pipe.
        let error = match self.child.try_wait() {
            Ok(Some(status)) if !status.success() => self.status_error(status),

            _ => io::Error::other(format!(
                "hook `{}` is not reading the session stream fast enough",
                self.command
            )),
        };

        // It already proved it's stuck, so don't wait for it to notice the
        // closed pipe
        self.child.kill();

        let _ = self.shutdown().await;
        let _ = self.notifier.notify(format!("Hook failed: {error}")).await;

        error
    }
}

#[async_trait]
impl session::Output for LiveHook {
    async fn event(&mut self, event: Event) -> io::Result<()> {
        let queued = self
            .events
            .as_ref()
            .is_some_and(|events| events.try_send(event).is_ok());

        if queued {
            Ok(())
        } else {
            Err(self.fail().await)
        }
    }

    async fn finish(&mut self) -> io::Result<()> {
        self.shutdown().await
    }
}

/// Encodes the session stream and feeds it to the hook's standard input until
/// the session closes the queue.
async fn write_stream(
    mut stdin: ChildStdin,
    header: Vec<u8>,
    mut events: mpsc::Receiver<Event>,
) -> io::Result<()> {
    let mut encoder = AsciicastV3Encoder::new(false);

    stdin.write_all(&header).await?;

    while let Some(event) = events.recv().await {
        stdin.write_all(&encoder.event(event.into())).await?;
    }

    stdin.shutdown().await
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    use tempfile::tempdir;

    use super::*;
    use crate::notifier::NullNotifier;
    use crate::session::Output;
    use crate::tty::TtySize;

    const CONTEXT: Context<'static> = Context {
        session_id: "test-session",
        output_file: Some("demo.cast"),
        server_url: Some("https://asciinema.example.com"),
    };

    fn start_hook(command: &str) -> LiveHook {
        start_hook_in(command, &CONTEXT)
    }

    fn start_hook_in(command: &str, context: &Context) -> LiveHook {
        let metadata = Metadata {
            time: std::time::SystemTime::now(),
            term: crate::session::TermInfo {
                type_: None,
                version: None,
                size: TtySize(80, 24),
                theme: None,
            },
            idle_time_limit: None,
            command: None,
            title: None,
            env: std::collections::HashMap::new(),
        };

        Hook(command.to_owned())
            .spawn(&metadata, context, Box::new(NullNotifier))
            .unwrap()
    }

    fn output(size: usize) -> session::Event {
        session::Event::Output(Duration::from_secs(1), "x".repeat(size))
    }

    /// Feeds an event to a hook until it fails, returning the error.
    async fn fail_on(hook: &mut LiveHook, event: session::Event) -> String {
        for _ in 0..1024 {
            if let Err(e) = hook.event(event.clone()).await {
                return e.to_string();
            }
            time::sleep(Duration::from_millis(1)).await;
        }
        panic!("the hook should have failed");
    }

    /// Waits for a process to disappear, polling because signals land asynchronously.
    async fn assert_gone(pid: u32) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while kill(Pid::from_raw(pid as i32), None).is_ok() {
            assert!(Instant::now() < deadline, "the process should be gone");
            time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn test_successful_execution_and_telemetry() {
        // Consolidates: Event ordering, Environment passing, and Stdout stalling prevention.
        for (context, expected_env) in [
            (
                CONTEXT,
                "test-session|demo.cast|https://asciinema.example.com",
            ),
            (
                Context {
                    session_id: "test-session",
                    output_file: None,
                    server_url: None,
                },
                "test-session||",
            ),
        ] {
            let dir = tempdir().unwrap();
            let env_path = dir.path().join("env");
            let cast_path = dir.path().join("hook.cast");

            // 1. Dumps env vars. 2. Generates heavy stdout (must not block). 3. Captures stdin.
            let script = format!(
                "printf '%s' \"$ASCIINEMA_SESSION|${{ASCIINEMA_OUTPUT_FILE-}}|${{ASCIINEMA_SERVER_URL-}}\" > {}; \
                 dd if=/dev/zero bs=1024 count=1024 2>/dev/null; \
                 cat > {}",
                env_path.display(),
                cast_path.display()
            );

            let mut hook = start_hook_in(&script, &context);

            for event in [
                session::Event::Output(Duration::from_secs(1), "hello".to_owned()),
                session::Event::Marker(Duration::from_secs(2), "note".to_owned()),
                session::Event::Resize(Duration::from_secs(3), TtySize(100, 30)),
                session::Event::Exit(Duration::from_secs(4), 0),
            ] {
                hook.event(event).await.unwrap();
            }

            // Ensure wait completion isn't blocked by standard output.
            time::timeout(Duration::from_secs(10), hook.finish())
                .await
                .expect("hook stalled on standard output")
                .unwrap();

            // Verify Environment payload
            assert_eq!(std::fs::read_to_string(&env_path).unwrap(), expected_env);

            // Verify Asciicast stream formatting and completeness
            let stream = std::fs::read_to_string(&cast_path).unwrap();
            let lines: Vec<&str> = stream.lines().collect();

            assert!(lines[0].contains(r#""version":3"#), "{}", lines[0]);
            assert_eq!(
                &lines[1..],
                &[
                    r#"[1.000, "o", "hello"]"#,
                    r#"[1.000, "m", "note"]"#,
                    r#"[1.000, "r", "100x30"]"#,
                    r#"[1.000, "x", "0"]"#,
                ]
            );
        }
    }

    #[tokio::test]
    async fn test_error_reporting_and_broken_pipes() {
        // Consolidates: Failed hook status, stderr capture, and broken pipe / unread stream handling.
        let command = "echo 'renderer blew up' >&2; exit 7";

        // Scenario A: Fails abruptly, input closed.
        let mut hook = start_hook(command);
        time::sleep(Duration::from_millis(100)).await;
        let error = hook.finish().await.unwrap_err().to_string();
        assert!(
            error.contains("exit status: 7") && error.contains("renderer blew up"),
            "{error}"
        );

        // Scenario B: Fails abruptly, session keeps writing to closed pipe.
        let mut hook = start_hook(command);
        time::sleep(Duration::from_millis(100)).await;
        let error = fail_on(&mut hook, output(1024)).await;
        assert!(
            error.contains("exit status: 7") && error.contains("renderer blew up"),
            "{error}"
        );

        // Scenario C: Exits cleanly, but stops reading early (broken pipe variant).
        let mut hook = start_hook("exit 0");
        let error = fail_on(&mut hook, output(1024)).await;
        assert!(error.contains("not reading"), "{error}");
    }

    #[tokio::test]
    async fn test_lifecycle_stalls_and_cleanup() {
        // Consolidates: Long-running hooks, stall protection (killing), and `Drop` group termination.
        let dir = tempdir().unwrap();

        // Scenario A: Waits beyond QUIET_WAIT_TIME for a well-behaved but slow hook.
        let marker = dir.path().join("done");
        let mut hook = start_hook(&format!("sleep 3; touch {}", marker.display()));
        let started = Instant::now();
        hook.finish().await.unwrap();
        assert!(marker.exists());
        assert!(
            started.elapsed() >= QUIET_WAIT_TIME,
            "should not abandon hook once quiet"
        );

        // Scenario B: Kills a hook that never reads its input and causes a backlog.
        let mut hook = start_hook("exec sleep 30");
        let started = Instant::now();
        let error = fail_on(&mut hook, output(64 * 1024)).await;
        assert!(error.contains("not reading"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(
            hook.child.id().is_none(),
            "the hook should have been reaped"
        );

        // Scenario C: Dropping a hook aggressively tears down the process group.
        let hook = start_hook("sleep 30 & wait");
        let pid = hook.child.id().expect("the hook should be running");
        drop(hook);
        assert_gone(pid).await;
    }
}
