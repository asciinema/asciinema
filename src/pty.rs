use crate::io::set_non_blocking;
use crate::tty::{Tty, TtySize};
use anyhow::{bail, Result};
use nix::errno::Errno;
use nix::sys::select::{select, FdSet};
use nix::sys::signal;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{self, ForkResult};
use nix::{libc, pty};
use signal_hook::consts::{SIGALRM, SIGCHLD, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGWINCH};
use signal_hook::SigId;
use std::collections::HashMap;
use std::ffi::{CString, NulError};
use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::{AsFd, RawFd};
use std::os::fd::{BorrowedFd, OwnedFd};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::str::FromStr;
use std::{env, fs};

type ExtraEnv = HashMap<String, String>;

pub trait Recorder {
    fn start(&mut self, size: TtySize) -> io::Result<()>;
    fn output(&mut self, data: &[u8]);
    fn input(&mut self, data: &[u8]) -> bool;
    fn resize(&mut self, size: TtySize);
}

pub fn exec<S: AsRef<str>, T: Tty + ?Sized, R: Recorder>(
    command: &[S],
    extra_env: &ExtraEnv,
    tty: &mut T,
    winsize_override: Option<WinsizeOverride>,
    recorder: &mut R,
) -> Result<i32> {
    let winsize = get_winsize(&*tty, winsize_override.as_ref());
    recorder.start(winsize.into())?;
    let result = unsafe { pty::forkpty(Some(&winsize), None) }?;

    match result.fork_result {
        ForkResult::Parent { child } => handle_parent(
            result.master.as_raw_fd(),
            child,
            tty,
            winsize_override,
            recorder,
        ),

        ForkResult::Child => {
            handle_child(command, extra_env)?;
            unreachable!();
        }
    }
}

fn handle_parent<T: Tty + ?Sized, R: Recorder>(
    master_fd: RawFd,
    child: unistd::Pid,
    tty: &mut T,
    winsize_override: Option<WinsizeOverride>,
    recorder: &mut R,
) -> Result<i32> {
    let wait_result = match copy(master_fd, child, tty, winsize_override, recorder) {
        Ok(Some(status)) => Ok(status),
        Ok(None) => wait::waitpid(child, None),

        Err(e) => {
            let _ = wait::waitpid(child, None);
            return Err(e);
        }
    };

    match wait_result {
        Ok(WaitStatus::Exited(_pid, status)) => Ok(status),
        Ok(WaitStatus::Signaled(_pid, signal, ..)) => Ok(128 + signal as i32),
        Ok(_) => Ok(1),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

const BUF_SIZE: usize = 128 * 1024;

fn copy<T: Tty + ?Sized, R: Recorder>(
    master_raw_fd: RawFd,
    child: unistd::Pid,
    tty: &mut T,
    winsize_override: Option<WinsizeOverride>,
    recorder: &mut R,
) -> Result<Option<WaitStatus>> {
    let mut master = unsafe { fs::File::from_raw_fd(master_raw_fd) };
    let mut buf = [0u8; BUF_SIZE];
    let mut input: Vec<u8> = Vec::with_capacity(BUF_SIZE);
    let mut output: Vec<u8> = Vec::with_capacity(BUF_SIZE);
    let mut master_closed = false;

    let sigwinch_fd = SignalFd::open(SIGWINCH)?;
    let sigint_fd = SignalFd::open(SIGINT)?;
    let sigterm_fd = SignalFd::open(SIGTERM)?;
    let sigquit_fd = SignalFd::open(SIGQUIT)?;
    let sighup_fd = SignalFd::open(SIGHUP)?;
    let sigalrm_fd = SignalFd::open(SIGALRM)?;
    let sigchld_fd = SignalFd::open(SIGCHLD)?;

    set_non_blocking(&master_raw_fd)?;

    loop {
        let master_fd = master.as_fd();
        let tty_fd = tty.as_fd();
        let mut rfds = FdSet::new();
        let mut wfds = FdSet::new();

        rfds.insert(&tty_fd);
        rfds.insert(&sigwinch_fd);
        rfds.insert(&sigint_fd);
        rfds.insert(&sigterm_fd);
        rfds.insert(&sigquit_fd);
        rfds.insert(&sighup_fd);
        rfds.insert(&sigalrm_fd);
        rfds.insert(&sigchld_fd);

        if !master_closed {
            rfds.insert(&master_fd);

            if !input.is_empty() {
                wfds.insert(&master_fd);
            }
        }

        if !output.is_empty() {
            wfds.insert(&tty_fd);
        }

        if let Err(e) = select(None, &mut rfds, &mut wfds, None, None) {
            if e == Errno::EINTR {
                continue;
            }

            bail!(e);
        }

        let master_read = rfds.contains(&master_fd);
        let master_write = wfds.contains(&master_fd);
        let tty_read = rfds.contains(&tty_fd);
        let tty_write = wfds.contains(&tty_fd);
        let sigwinch_read = rfds.contains(&sigwinch_fd);
        let sigint_read = rfds.contains(&sigint_fd);
        let sigterm_read = rfds.contains(&sigterm_fd);
        let sigquit_read = rfds.contains(&sigquit_fd);
        let sighup_read = rfds.contains(&sighup_fd);
        let sigalrm_read = rfds.contains(&sigalrm_fd);
        let sigchld_read = rfds.contains(&sigchld_fd);

        if master_read {
            while let Some(n) = read_non_blocking(&mut master, &mut buf)? {
                if n > 0 {
                    recorder.output(&buf[0..n]);
                    output.extend_from_slice(&buf[0..n]);
                } else if output.is_empty() {
                    return Ok(None);
                } else {
                    master_closed = true;
                    break;
                }
            }
        }

        if master_write {
            let mut buf: &[u8] = input.as_ref();

            while let Some(n) = write_non_blocking(&mut master, buf)? {
                buf = &buf[n..];

                if buf.is_empty() {
                    break;
                }
            }

            let left = buf.len();

            if left == 0 {
                input.clear();
            } else {
                let rot = input.len() - left;
                input.rotate_left(rot);
                input.truncate(left);
            }
        }

        if tty_write {
            let mut buf: &[u8] = output.as_ref();

            while let Some(n) = write_non_blocking(tty, buf)? {
                buf = &buf[n..];

                if buf.is_empty() {
                    break;
                }
            }

            let left = buf.len();

            if left == 0 {
                if master_closed {
                    return Ok(None);
                }

                output.clear();
            } else {
                let rot = output.len() - left;
                output.rotate_left(rot);
                output.truncate(left);
            }
        }

        if tty_read {
            while let Some(n) = read_non_blocking(tty, &mut buf)? {
                if n > 0 {
                    if recorder.input(&buf[0..n]) {
                        input.extend_from_slice(&buf[0..n]);
                    }
                } else {
                    return Ok(None);
                }
            }
        }

        if sigwinch_read {
            sigwinch_fd.flush();
            let winsize = get_winsize(&*tty, winsize_override.as_ref());
            set_pty_size(master_raw_fd, &winsize);
            recorder.resize(winsize.into());
        }

        if sigint_read {
            sigint_fd.flush();
        }

        let mut kill_the_child = false;

        if sigterm_read {
            sigterm_fd.flush();
            kill_the_child = true;
        }

        if sigquit_read {
            sigquit_fd.flush();
            kill_the_child = true;
        }

        if sighup_read {
            sighup_fd.flush();
            kill_the_child = true;
        }

        if sigalrm_read {
            sigalrm_fd.flush();
        }

        if sigchld_read {
            sigchld_fd.flush();

            if let Ok(status) = wait::waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                if status != WaitStatus::StillAlive {
                    return Ok(Some(status));
                }
            }
        }

        if kill_the_child {
            unsafe { libc::kill(child.as_raw(), SIGTERM) };
            return Ok(None);
        }
    }
}

fn handle_child<S: AsRef<str>>(command: &[S], extra_env: &ExtraEnv) -> Result<()> {
    use signal::{SigHandler, Signal};

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

#[derive(Clone, Debug)]
pub enum WinsizeOverride {
    Full(u16, u16),
    Cols(u16),
    Rows(u16),
}

impl FromStr for WinsizeOverride {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('x') {
            Some((cols, "")) => {
                let cols: u16 = cols.parse()?;

                Ok(WinsizeOverride::Cols(cols))
            }

            Some(("", rows)) => {
                let rows: u16 = rows.parse()?;

                Ok(WinsizeOverride::Rows(rows))
            }

            Some((cols, rows)) => {
                let cols: u16 = cols.parse()?;
                let rows: u16 = rows.parse()?;

                Ok(WinsizeOverride::Full(cols, rows))
            }

            None => {
                bail!("{s}")
            }
        }
    }
}

fn get_winsize<T: Tty + ?Sized>(
    tty: &T,
    winsize_override: Option<&WinsizeOverride>,
) -> pty::Winsize {
    let mut winsize = tty.get_size();

    match winsize_override {
        Some(WinsizeOverride::Full(cols, rows)) => {
            winsize.ws_col = *cols;
            winsize.ws_row = *rows;
        }

        Some(WinsizeOverride::Cols(cols)) => {
            winsize.ws_col = *cols;
        }

        Some(WinsizeOverride::Rows(rows)) => {
            winsize.ws_row = *rows;
        }

        None => (),
    }

    winsize
}

fn set_pty_size(pty_fd: i32, winsize: &pty::Winsize) {
    unsafe { libc::ioctl(pty_fd, libc::TIOCSWINSZ, winsize) };
}

fn read_non_blocking<R: Read + ?Sized>(
    source: &mut R,
    buf: &mut [u8],
) -> io::Result<Option<usize>> {
    match source.read(buf) {
        Ok(n) => Ok(Some(n)),

        Err(e) => {
            if e.kind() == ErrorKind::WouldBlock {
                Ok(None)
            } else if e.raw_os_error().is_some_and(|code| code == 5) {
                Ok(Some(0))
            } else {
                return Err(e);
            }
        }
    }
}

fn write_non_blocking<W: Write + ?Sized>(sink: &mut W, buf: &[u8]) -> io::Result<Option<usize>> {
    match sink.write(buf) {
        Ok(n) => Ok(Some(n)),

        Err(e) => {
            if e.kind() == ErrorKind::WouldBlock {
                Ok(None)
            } else if e.raw_os_error().is_some_and(|code| code == 5) {
                Ok(Some(0))
            } else {
                return Err(e);
            }
        }
    }
}

struct SignalFd {
    sigid: SigId,
    rx: OwnedFd,
}

impl SignalFd {
    fn open(signal: libc::c_int) -> Result<Self> {
        let (rx, tx) = unistd::pipe()?;
        set_non_blocking(&rx)?;
        set_non_blocking(&tx)?;
        let rx = unsafe { OwnedFd::from_raw_fd(rx) };
        let tx = unsafe { OwnedFd::from_raw_fd(tx) };

        let sigid = unsafe {
            signal_hook::low_level::register(signal, move || {
                let _ = unistd::write(tx.as_raw_fd(), &[0]);
            })
        }?;

        Ok(Self { sigid, rx })
    }

    fn flush(&self) {
        let mut buf = [0; 256];

        while let Ok(n) = unistd::read(self.rx.as_raw_fd(), &mut buf) {
            if n == 0 {
                break;
            };
        }
    }
}

impl AsFd for SignalFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.rx.as_fd()
    }
}

impl Drop for SignalFd {
    fn drop(&mut self) {
        signal_hook::low_level::unregister(self.sigid);
    }
}

#[cfg(test)]
mod tests {
    use super::Recorder;
    use crate::pty::{ExtraEnv, WinsizeOverride};
    use crate::tty::{NullTty, TtySize};

    #[derive(Default)]
    struct TestRecorder {
        tty_size: Option<TtySize>,
        output: Vec<Vec<u8>>,
    }

    impl Recorder for TestRecorder {
        fn start(&mut self, tty_size: TtySize) -> std::io::Result<()> {
            self.tty_size = Some(tty_size);
            Ok(())
        }

        fn output(&mut self, data: &[u8]) {
            self.output.push(data.into());
        }

        fn input(&mut self, _data: &[u8]) -> bool {
            true
        }

        fn resize(&mut self, _size: TtySize) {}
    }

    impl TestRecorder {
        fn output(&self) -> Vec<String> {
            self.output
                .iter()
                .map(|x| String::from_utf8_lossy(x).to_string())
                .collect::<Vec<_>>()
        }
    }

    #[test]
    fn exec_basic() {
        let mut recorder = TestRecorder::default();

        let code = r#"
import sys;
import time;
sys.stdout.write('foo');
sys.stdout.flush();
time.sleep(0.1);
sys.stdout.write('bar');
"#;

        super::exec(
            &["python3", "-c", code],
            &ExtraEnv::new(),
            &mut NullTty::open().unwrap(),
            None,
            &mut recorder,
        )
        .unwrap();

        assert_eq!(recorder.output(), vec!["foo", "bar"]);
        assert_eq!(recorder.tty_size, Some(TtySize(80, 24)));
    }

    #[test]
    fn exec_no_output() {
        let mut recorder = TestRecorder::default();

        super::exec(
            &["true"],
            &ExtraEnv::new(),
            &mut NullTty::open().unwrap(),
            None,
            &mut recorder,
        )
        .unwrap();

        assert!(recorder.output().is_empty());
    }

    #[test]
    fn exec_quick() {
        let mut recorder = TestRecorder::default();

        super::exec(
            &["w"],
            &ExtraEnv::new(),
            &mut NullTty::open().unwrap(),
            None,
            &mut recorder,
        )
        .unwrap();

        assert!(!recorder.output().is_empty());
    }

    #[test]
    fn exec_extra_env() {
        let mut recorder = TestRecorder::default();

        let mut env = ExtraEnv::new();
        env.insert("ASCIINEMA_TEST_FOO".to_owned(), "bar".to_owned());

        super::exec(
            &["sh", "-c", "echo -n $ASCIINEMA_TEST_FOO"],
            &env,
            &mut NullTty::open().unwrap(),
            None,
            &mut recorder,
        )
        .unwrap();

        assert_eq!(recorder.output(), vec!["bar"]);
    }

    #[test]
    fn exec_winsize_override() {
        let mut recorder = TestRecorder::default();

        super::exec(
            &["true"],
            &ExtraEnv::new(),
            &mut NullTty::open().unwrap(),
            Some(WinsizeOverride::Full(100, 50)),
            &mut recorder,
        )
        .unwrap();

        assert_eq!(recorder.tty_size, Some(TtySize(100, 50)));
    }
}
