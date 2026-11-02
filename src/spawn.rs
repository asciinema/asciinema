use std::io;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};

use nix::sys::signal::{kill as send_signal, Signal};
use nix::unistd::Pid;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdin, Command};
use tracing::warn;

/// Session details a companion command can't get from the asciicast header,
/// passed to it in the environment.
pub struct Context<'a> {
    /// Value of the `ASCIINEMA_SESSION` variable of the recorded command.
    pub session_id: &'a str,

    /// Path of the recording file, when the session is saved to one.
    pub output_file: Option<&'a str>,

    /// URL of the asciinema server this CLI is pointed at, when configured.
    pub server_url: Option<&'a str>,
}

/// A companion command spawned with [`shell`].
pub struct Child {
    child: tokio::process::Child,
    last_error_line: Arc<Mutex<Option<String>>>,
}

impl Child {
    /// The process ID, if the command has not been waited for.
    #[cfg(test)]
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Takes the command's standard input pipe, if it was spawned piped.
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }

    /// Waits for the command to finish.
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait().await
    }

    /// Checks whether the command has exited without waiting.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    /// The last non-empty line the command wrote to its standard error.
    pub fn last_error_line(&self) -> Option<String> {
        self.last_error_line.lock().unwrap().clone()
    }

    /// Takes the command's whole process group down, children included.
    /// A no-op once the command has been waited for.
    pub fn kill(&self) {
        if let Some(pid) = self.child.id() {
            let _ = send_signal(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);
        }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Spawns a command, via `/bin/sh -c`, as a companion of the recording
/// session.
///
/// The command runs in its very own process group!
///
/// This means that it survives ctrl+c and
/// gets a chance to finish, regardless of circumstance, and the returned [`Child`] takes the whole group
/// down when it's dropped without waiting!
pub fn shell(command: &str, context: &Context, stdin: Stdio, stdout: Stdio) -> io::Result<Child> {
    let mut child = Command::new("/bin/sh")
        .args(["-c", command])
        .env("ASCIINEMA_SESSION", context.session_id)
        .envs(context.output_file.map(|p| ("ASCIINEMA_OUTPUT_FILE", p)))
        .envs(context.server_url.map(|u| ("ASCIINEMA_SERVER_URL", u)))
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .process_group(0)
        .spawn()?;

    let stderr = child
        .stderr
        .take()
        .expect("stderr should be piped when spawning");

    let last_error_line = Arc::new(Mutex::new(None));
    tokio::spawn(log_errors(
        stderr,
        command.to_owned(),
        last_error_line.clone(),
    ));

    Ok(Child {
        child,
        last_error_line,
    })
}

async fn log_errors(
    stderr: ChildStderr,
    command: String,
    last_error_line: Arc<Mutex<Option<String>>>,
) {
    let mut lines = BufReader::new(stderr).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        warn!("{command}: {line}");
        *last_error_line.lock().unwrap() = Some(line);
    }
}
