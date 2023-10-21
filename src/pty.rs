use mio::unix::SourceFd;
use nix::{fcntl, libc, pty, unistd, unistd::ForkResult};
use std::fs;
use std::io::{self, Read, Write};
use std::ops::Deref;
use std::os::fd::RawFd;
use std::os::unix::io::{AsRawFd, FromRawFd};
use termion::raw::IntoRawMode;

pub fn exec<S: AsRef<str>>(args: &[S]) -> anyhow::Result<()> {
    let tty = open_tty()?;
    let winsize = get_tty_size(tty.as_raw_fd());
    let result = unsafe { pty::forkpty(Some(&winsize), None) }?;

    match result.fork_result {
        ForkResult::Parent { .. } => {
            handle_parent(result.master.as_raw_fd(), tty)?;
        }

        ForkResult::Child => {
            handle_child(args)?;
            // TODO wait for child pid
        }
    }

    Ok(())
}

fn open_tty() -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
}

fn get_tty_size(tty_fd: i32) -> pty::Winsize {
    let mut winsize = pty::Winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    unsafe { libc::ioctl(tty_fd, libc::TIOCGWINSZ, &mut winsize) };

    winsize
}

const MASTER: mio::Token = mio::Token(0);
const TTY: mio::Token = mio::Token(1);
const BUF_SIZE: usize = 128 * 1024;

fn handle_parent(master_fd: RawFd, tty: fs::File) -> anyhow::Result<()> {
    let mut master_file = unsafe { fs::File::from_raw_fd(master_fd) };
    let mut poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(128);
    let mut master_source = SourceFd(&master_fd);
    let mut tty = tty.into_raw_mode()?;
    let tty_fd = tty.as_raw_fd();
    let mut tty_source = SourceFd(&tty_fd);
    let mut buf = [0u8; BUF_SIZE];
    let mut input: Vec<u8> = Vec::with_capacity(BUF_SIZE);
    let mut output: Vec<u8> = Vec::with_capacity(BUF_SIZE);

    set_non_blocking(&master_fd)?;
    set_non_blocking(&tty_fd)?;

    poll.registry()
        .register(&mut master_source, MASTER, mio::Interest::READABLE)?;

    poll.registry()
        .register(&mut tty_source, TTY, mio::Interest::READABLE)?;

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                MASTER => {
                    if event.is_readable() {
                        let read = read_all(&mut master_file, &mut buf, &mut output)?;

                        if read > 0 {
                            poll.registry().reregister(
                                &mut tty_source,
                                TTY,
                                mio::Interest::READABLE | mio::Interest::WRITABLE,
                            )?;
                        }
                    }

                    if event.is_writable() {
                        master_file.write_all(&input).unwrap();
                        input.clear();

                        poll.registry().reregister(
                            &mut master_source,
                            MASTER,
                            mio::Interest::READABLE,
                        )?;
                    }

                    if event.is_read_closed() {
                        return Ok(());
                        // TODO don't return but deregister master_source and flush remaining output to tty
                    }
                }

                TTY => {
                    if event.is_writable() {
                        tty.write_all(&output)?;
                        output.clear();

                        poll.registry().reregister(
                            &mut tty_source,
                            TTY,
                            mio::Interest::READABLE,
                        )?;
                    }

                    if event.is_readable() {
                        let read = read_all(&mut tty.deref(), &mut buf, &mut input)?;

                        if read > 0 {
                            poll.registry().reregister(
                                &mut master_source,
                                MASTER,
                                mio::Interest::READABLE | mio::Interest::WRITABLE,
                            )?;
                        }
                    }

                    if event.is_read_closed() {
                        poll.registry().deregister(&mut tty_source).unwrap();
                        return Ok(());
                        // TODO don't return but deregister tty_source and flush remaining input to master
                    }
                }

                _ => (),
            }
        }
    }
}

fn handle_child<S: AsRef<str>>(args: &[S]) -> anyhow::Result<()> {
    use std::ffi::{CString, NulError};

    let args = args
        .iter()
        .map(|s| CString::new(s.as_ref()))
        .collect::<Result<Vec<CString>, NulError>>()?;

    unistd::execvp(&args[0], &args)?;
    unsafe { libc::_exit(1) }
}

fn set_non_blocking(fd: &i32) -> Result<(), io::Error> {
    use fcntl::{fcntl, FcntlArg::*, OFlag};

    let flags = fcntl(*fd, F_GETFL)?;
    let mut oflags = OFlag::from_bits_truncate(flags);
    oflags |= OFlag::O_NONBLOCK;
    fcntl(*fd, F_SETFL(oflags))?;

    Ok(())
}

fn read_all<R: Read>(source: &mut R, buf: &mut [u8], out: &mut Vec<u8>) -> io::Result<usize> {
    let mut read = 0;

    loop {
        match source.read(buf) {
            Ok(0) => (),

            Ok(n) => {
                out.extend_from_slice(&buf[0..n]);
                read += n;
            }

            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    break;
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(read)
}
