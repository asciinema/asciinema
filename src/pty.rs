use std::collections::HashMap;
use std::env;
use std::ffi::{CString, NulError};
use std::os::fd::OwnedFd;
use std::os::unix::io::AsRawFd;

use nix::errno::Errno;
use nix::pty::{ForkptyResult, Winsize};
use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{self, Pid};
use nix::{libc, pty};
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};
use tokio::task;

use crate::fd::FdExt;

pub struct Pty {
    child: Pid,
    master: AsyncFd<OwnedFd>,
}

pub async fn spawn<S: AsRef<str>>(
    command: &[S],
    winsize: Winsize,
    extra_env: &HashMap<String, String>,
) -> anyhow::Result<Pty> {
    let result = unsafe { pty::forkpty(Some(&winsize), None) }?;

    match result {
        ForkptyResult::Parent { child, master } => {
            master.set_nonblocking()?;
            let master = AsyncFd::new(master)?;

            Ok(Pty { child, master })
        }

        ForkptyResult::Child => {
            handle_child(command, extra_env)?;
            unreachable!();
        }
    }
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
        // Any errors occurred when killing the child are ignored.
        let _ = signal::kill(self.child, Signal::SIGTERM);
    }

    pub async fn wait(&self, options: Option<WaitPidFlag>) -> io::Result<WaitStatus> {
        let pid = self.child;
        task::spawn_blocking(move || Ok(wait::waitpid(pid, options)?)).await?
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.kill();
        let _ = wait::waitpid(self.child, None);
    }
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
    use std::{collections::HashMap, sync::Arc};

    use super::Pty;
    use crate::tty::TtySize;

    async fn spawn<S: AsRef<str>>(command: &[S], extra_env: &HashMap<String, String>) -> Arc<Pty> {
        Arc::new(
            super::spawn(command, TtySize::default().into(), extra_env)
                .await
                .unwrap(),
        )
    }

    async fn read_output(pty: Arc<Pty>) -> Vec<String> {
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let mut output = Vec::new();

            while let Ok(n) = pty.read(&mut buf).await {
                if n == 0 {
                    break;
                }

                output.push(String::from_utf8_lossy(&buf[..n]).to_string());
            }

            output
        })
        .await
        .unwrap()
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
        let output = read_output(pty).await;

        assert!(!output.is_empty());
    }

    #[tokio::test]
    async fn spawn_extra_env() {
        let mut extra_env = HashMap::new();
        extra_env.insert("ASCIINEMA_TEST_FOO".to_owned(), "bar".to_owned());

        let pty = spawn(&["sh", "-c", "echo -n $ASCIINEMA_TEST_FOO"], &extra_env).await;
        let output = read_output(pty).await;

        assert_eq!(output, vec!["bar"]);
    }
}
