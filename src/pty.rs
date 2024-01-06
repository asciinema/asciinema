use crate::io::set_non_blocking;
use crate::tty::Tty;
use anyhow::bail;
use nix::errno::Errno;
use nix::sys::select::{pselect, FdSet};
use nix::{libc, pty, sys::signal, sys::wait, unistd, unistd::ForkResult};
use std::collections::HashMap;
use std::ffi::{CString, NulError};
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, RawFd};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::{env, fs};

type ExtraEnv = HashMap<String, String>;

pub trait Recorder {
    fn start(&mut self, size: (u16, u16)) -> io::Result<()>;
    fn output(&mut self, data: &[u8]);
    fn input(&mut self, data: &[u8]);
    fn resize(&mut self, size: (u16, u16));
}

pub fn exec<S: AsRef<str>, R: Recorder>(
    args: &[S],
    extra_env: &ExtraEnv,
    tty: Box<dyn Tty>,
    winsize_override: (Option<u16>, Option<u16>),
    recorder: &mut R,
) -> anyhow::Result<i32> {
    let winsize = get_tty_size(&*tty, winsize_override);
    recorder.start((winsize.ws_col, winsize.ws_row))?;
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
            handle_child(args, extra_env)?;
            unreachable!();
        }
    }
}

fn handle_parent<R: Recorder>(
    master_fd: RawFd,
    child: unistd::Pid,
    tty: Box<dyn Tty>,
    winsize_override: (Option<u16>, Option<u16>),
    recorder: &mut R,
) -> anyhow::Result<i32> {
    let copy_result = copy(master_fd, child, tty, winsize_override, recorder);
    let wait_result = wait::waitpid(child, None);
    copy_result?;

    match wait_result {
        Ok(wait::WaitStatus::Exited(_pid, status)) => Ok(status),
        Ok(wait::WaitStatus::Signaled(_pid, signal, ..)) => Ok(128 + signal as i32),
        Ok(_) => Ok(1),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

const BUF_SIZE: usize = 128 * 1024;

fn copy<R: Recorder>(
    master_raw_fd: RawFd,
    child: unistd::Pid,
    mut tty: Box<dyn Tty>,
    winsize_override: (Option<u16>, Option<u16>),
    recorder: &mut R,
) -> anyhow::Result<()> {
    let mut master = unsafe { fs::File::from_raw_fd(master_raw_fd) };
    let mut buf = [0u8; BUF_SIZE];
    let mut input: Vec<u8> = Vec::with_capacity(BUF_SIZE);
    let mut output: Vec<u8> = Vec::with_capacity(BUF_SIZE);
    let mut flush = false;

    set_non_blocking(&master_raw_fd)?;

    loop {
        let master_fd = master.as_fd();
        let tty_fd = tty.as_fd();
        let mut rfds = FdSet::new();
        let mut wfds = FdSet::new();

        rfds.insert(&tty_fd);

        if !flush {
            rfds.insert(&master_fd);
        }

        if !input.is_empty() {
            wfds.insert(&master_fd);
        }

        if !output.is_empty() {
            wfds.insert(&tty_fd);
        }

        if let Err(e) = pselect(None, &mut rfds, &mut wfds, None, None, None) {
            if e == Errno::EINTR {
                continue;
            } else {
                bail!(e);
            }
        }

        let master_read = rfds.contains(&master_fd);
        let master_write = wfds.contains(&master_fd);
        let tty_read = rfds.contains(&tty_fd);
        let tty_write = wfds.contains(&tty_fd);

        if master_read {
            let offset = output.len();
            let read = read_all(&mut master, &mut buf, &mut output)?;

            if read > 0 {
                recorder.output(&output[offset..]);
            } else if output.is_empty() {
                return Ok(());
            } else {
                flush = true;
            }
        }

        if master_write {
            write_all(&mut master, &mut input)?;
        }

        if tty_write {
            let left = write_all(&mut tty, &mut output)?;

            if left == 0 && flush {
                return Ok(());
            }
        }

        if tty_read {
            let offset = input.len();
            let read = read_all(&mut tty, &mut buf, &mut input)?;

            if read > 0 {
                recorder.input(&input[offset..]);
            }
        }
    }
}

fn handle_child<S: AsRef<str>>(args: &[S], extra_env: &ExtraEnv) -> anyhow::Result<()> {
    use signal::{SigHandler, Signal};

    let args = args
        .iter()
        .map(|s| CString::new(s.as_ref()))
        .collect::<Result<Vec<CString>, NulError>>()?;

    for (k, v) in extra_env {
        env::set_var(k, v);
    }

    unsafe { signal::signal(Signal::SIGPIPE, SigHandler::SigDfl) }?;
    unistd::execvp(&args[0], &args)?;
    unsafe { libc::_exit(1) }
}

fn get_tty_size<T: Tty + ?Sized>(
    tty: &T,
    winsize_override: (Option<u16>, Option<u16>),
) -> pty::Winsize {
    let mut winsize = tty.get_size();

    if let Some(cols) = winsize_override.0 {
        winsize.ws_col = cols;
    }

    if let Some(rows) = winsize_override.1 {
        winsize.ws_row = rows;
    }

    winsize
}

fn set_pty_size(pty_fd: i32, winsize: &pty::Winsize) {
    unsafe { libc::ioctl(pty_fd, libc::TIOCSWINSZ, winsize) };
}

fn read_all<R: Read>(source: &mut R, buf: &mut [u8], out: &mut Vec<u8>) -> io::Result<usize> {
    let mut read = 0;

    loop {
        match source.read(buf) {
            Ok(0) => break,

            Ok(n) => {
                out.extend_from_slice(&buf[0..n]);
                read += n;
            }

            Err(_) => break,
        }
    }

    Ok(read)
}

fn write_all<W: Write>(sink: &mut W, data: &mut Vec<u8>) -> io::Result<usize> {
    let mut buf: &[u8] = data.as_ref();

    loop {
        match sink.write(buf) {
            Ok(0) => break,

            Ok(n) => {
                buf = &buf[n..];

                if buf.is_empty() {
                    break;
                }
            }

            Err(_) => break,
        }
    }

    let left = buf.len();

    if left == 0 {
        data.clear();
    } else {
        let rot = data.len() - left;
        data.rotate_left(rot);
        data.truncate(left);
    }

    Ok(left)
}

#[cfg(test)]
mod tests {
    use crate::pty::ExtraEnv;
    use crate::tty::DevNull;

    #[derive(Default)]
    struct TestRecorder {
        size: Option<(u16, u16)>,
        output: Vec<Vec<u8>>,
    }

    impl super::Recorder for TestRecorder {
        fn start(&mut self, size: (u16, u16)) -> std::io::Result<()> {
            self.size = Some(size);
            Ok(())
        }

        fn output(&mut self, data: &[u8]) {
            self.output.push(data.into());
        }

        fn input(&mut self, _data: &[u8]) {}
        fn resize(&mut self, _size: (u16, u16)) {}
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
    fn exec() {
        let mut recorder = TestRecorder::default();

        let code = r#"
import sys;
import time;
sys.stdout.write('foo');
sys.stdout.flush();
time.sleep(0.01);
sys.stdout.write('bar');
"#;

        let result = super::exec(
            &["python3", "-c", code],
            &ExtraEnv::new(),
            Box::new(DevNull::open().unwrap()),
            (None, None),
            &mut recorder,
        );

        assert!(result.is_ok());
        assert_eq!(recorder.output(), vec!["foo", "bar"]);
        assert!(recorder.size.is_some());
    }
}
