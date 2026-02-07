use std::collections::HashMap;
use std::env;
use std::ffi::{CString, NulError};
use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use anyhow::{bail, Context};
use nix::errno::Errno;
use nix::pty::{ForkptyResult, Winsize};
use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::stat;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{self, Pid};
use nix::{libc, pty};
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};
use tokio::task;

use crate::fd::FdExt;

pub struct Pty {
    child: Option<Pid>,
    master: AsyncFd<OwnedFd>,
}

impl Pty {
    pub async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        self.master
            .async_io(Interest::READABLE, |fd| match unistd::read(fd, buffer) {
                Ok(n) => Ok(n),
                Err(Errno::EIO) => Ok(0),
                Err(e) => Err(e.into()),
            })
            .await
    }

    pub async fn write(&self, buffer: &[u8]) -> io::Result<usize> {
        self.master
            .async_io(Interest::WRITABLE, |fd| match unistd::write(fd, buffer) {
                Ok(n) => Ok(n),
                Err(Errno::EIO) => Ok(0),
                Err(e) => Err(e.into()),
            })
            .await
    }

    pub fn resize(&self, winsize: Winsize) {
        unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &winsize) };
    }

    pub fn kill(&self) {
        // Only kill if we have a child process (spawned mode, not attached mode)
        if let Some(child) = self.child {
            // Any errors occurred when killing the child are ignored.
            let _ = signal::kill(child, Signal::SIGTERM);
        }
    }

    pub async fn wait(&self, options: Option<WaitPidFlag>) -> io::Result<WaitStatus> {
        if let Some(pid) = self.child {
            task::spawn_blocking(move || Ok(wait::waitpid(pid, options)?)).await?
        } else {
            // In attached mode, we don't have a child process to wait for.
            // Return a synthetic "exited with 0" status.
            Ok(WaitStatus::Exited(Pid::from_raw(0), 0))
        }
    }

    /// Returns true if this PTY is attached to an existing process (vs spawned a new one)
    pub fn is_attached(&self) -> bool {
        self.child.is_none()
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Some(child) = self.child {
            self.kill();
            let _ = wait::waitpid(child, None);
        }
        // In attached mode, we don't kill or wait - the shell continues running
    }
}

pub fn spawn<S: AsRef<str>>(
    command: &[S],
    winsize: Winsize,
    extra_env: &HashMap<String, String>,
) -> anyhow::Result<Pty> {
    let result = unsafe { pty::forkpty(Some(&winsize), None) }?;

    match result {
        ForkptyResult::Parent { child, master } => {
            master.set_nonblocking()?;
            let master = AsyncFd::new(master)?;

            Ok(Pty {
                child: Some(child),
                master,
            })
        }

        ForkptyResult::Child => {
            handle_child(command, extra_env)?;
            unreachable!();
        }
    }
}

/// Attach to an existing PTY device for recording/streaming.
///
/// This opens the PTY device at the given path (e.g., /dev/pts/5 on Linux or
/// /dev/ttys005 on macOS) in read-only mode to capture output. The original
/// shell continues to run normally.
pub fn attach(pty_path: &str, winsize: Winsize) -> anyhow::Result<Pty> {
    let path = Path::new(pty_path);

    // Verify the path exists
    if !path.exists() {
        bail!("PTY device does not exist: {}", pty_path);
    }

    // Verify it's a character device (PTY)
    let file_stat = stat::stat(path).context("failed to stat PTY device")?;
    let mode = file_stat.st_mode;
    if !stat::SFlag::from_bits_truncate(mode).contains(stat::SFlag::S_IFCHR) {
        bail!("path is not a character device: {}", pty_path);
    }

    // Open the PTY device
    let file = File::options()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
        .context("failed to open PTY device")?;

    let fd: OwnedFd = file.into();
    let master = AsyncFd::new(fd)?;

    // Set the window size
    unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ, &winsize) };

    Ok(Pty {
        child: None,
        master,
    })
}

/// Resolve a process ID to its controlling PTY device path.
///
/// On Linux, this reads /proc/<pid>/fd/0 to find the terminal.
/// On macOS, this uses lsof to find the controlling terminal.
pub fn resolve_pty_from_pid(pid: i32) -> anyhow::Result<String> {
    #[cfg(target_os = "linux")]
    {
        resolve_pty_from_pid_linux(pid)
    }

    #[cfg(target_os = "macos")]
    {
        resolve_pty_from_pid_macos(pid)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        bail!("--pid is not supported on this platform");
    }
}

#[cfg(target_os = "linux")]
fn resolve_pty_from_pid_linux(pid: i32) -> anyhow::Result<String> {
    use std::fs;

    // Try to read the symlink for fd 0 (stdin) of the process
    let fd_path = format!("/proc/{}/fd/0", pid);
    let target = fs::read_link(&fd_path)
        .with_context(|| format!("failed to read {}", fd_path))?;

    let target_str = target
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("PTY path is not valid UTF-8"))?;

    // Verify it looks like a PTY device
    if !target_str.starts_with("/dev/pts/") && !target_str.starts_with("/dev/tty") {
        bail!(
            "process {} does not have a PTY as stdin (found: {})",
            pid,
            target_str
        );
    }

    Ok(target_str.to_owned())
}

#[cfg(target_os = "macos")]
fn resolve_pty_from_pid_macos(pid: i32) -> anyhow::Result<String> {
    use std::process::Command;

    // Use lsof to find the controlling terminal
    let output = Command::new("lsof")
        .args(["-p", &pid.to_string(), "-a", "-d", "0"])
        .output()
        .context("failed to run lsof")?;

    if !output.status.success() {
        bail!(
            "lsof failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse lsof output to find the device
    // Example line: bash    12345 user    0u   CHR  16,5      0t0    1234 /dev/ttys005
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            let device_path = parts[8];
            if device_path.starts_with("/dev/ttys") || device_path.starts_with("/dev/pty") {
                return Ok(device_path.to_owned());
            }
        }
    }

    bail!("could not find PTY for process {}", pid)
}

fn handle_child<S: AsRef<str>>(
    command: &[S],
    extra_env: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let command = command
        .iter()
        .map(|s| CString::new(s.as_ref()))
        .collect::<Result<Vec<CString>, NulError>>()?;

    for (k, v) in extra_env {
        env::set_var(k, v);
    }

    unsafe { signal::signal(Signal::SIGPIPE, SigHandler::SigDfl) }?;
    unistd::execvp(&command[0], &command)?;
    unsafe { libc::_exit(1) }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::Pty;
    use crate::tty::TtySize;

    async fn spawn<S: AsRef<str>>(command: &[S], extra_env: &HashMap<String, String>) -> Pty {
        super::spawn(command, TtySize::default().into(), extra_env).unwrap()
    }

    async fn read_output(pty: Pty) -> Vec<String> {
        let mut buf = [0u8; 1024];
        let mut output = Vec::new();

        while let Ok(n) = pty.read(&mut buf).await {
            if n == 0 {
                break;
            }

            output.push(String::from_utf8_lossy(&buf[..n]).to_string());
        }

        output
    }

    #[tokio::test]
    async fn spawn_basic() {
        let code = r#"
import sys;
import time;
sys.stdout.write('foo');
sys.stdout.flush();
time.sleep(0.1);
sys.stdout.write('bar');
"#;

        let pty = spawn(&["python3", "-c", code], &HashMap::new()).await;
        let output = read_output(pty).await;

        assert_eq!(output, vec!["foo", "bar"]);
    }

    #[tokio::test]
    async fn spawn_no_output() {
        let pty = spawn(&["true"], &HashMap::new()).await;
        let output = read_output(pty).await;

        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn spawn_quick() {
        let pty = spawn(&["printf", "hello world\n"], &HashMap::new()).await;
        let output = read_output(pty).await.join("");

        assert_eq!(output, "hello world\r\n");
    }

    #[tokio::test]
    async fn spawn_extra_env() {
        let mut extra_env = HashMap::new();
        extra_env.insert("ASCIINEMA_TEST_FOO".to_owned(), "bar".to_owned());

        let pty = spawn(&["sh", "-c", "echo -n $ASCIINEMA_TEST_FOO"], &extra_env).await;
        let output = read_output(pty).await;

        assert_eq!(output, vec!["bar"]);
    }

    #[tokio::test]
    async fn spawn_echo_input() {
        let pty = spawn(&["cat"], &HashMap::new()).await;
        pty.write(b"foo").await.unwrap();
        pty.write(b"bar").await.unwrap();
        pty.kill();
        let output = read_output(pty).await.join("");

        assert_eq!(output, "foobar");
    }
}
