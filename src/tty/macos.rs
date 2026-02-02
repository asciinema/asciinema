/// This is an alternative implementation of DevTty that we use on macOS due to a bug in macOS's
/// kqueue implementation when polling /dev/tty.
///
/// See below links for more about the problem:
///
/// https://code.saghul.net/2016/05/libuv-internals-the-osx-select2-trick/
/// https://nathancraddock.com/blog/macos-dev-tty-polling/
///
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::thread;

use bytes::{Buf, BytesMut};

use async_trait::async_trait;
use nix::errno::Errno;
use nix::pty::Winsize;
use nix::sys::select;
use nix::sys::termios::{self, SetArg, Termios};
use nix::{libc, unistd};
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};

use super::{RawTty, TtySize};
use crate::fd::FdExt;

const BUF_SIZE: usize = 128 * 1024;

pub struct DevTty {
    file: File,
    read_r_fd: AsyncFd<OwnedFd>,
    write_w_fd: AsyncFd<OwnedFd>,
    settings: libc::termios,
}

impl DevTty {
    pub async fn open() -> anyhow::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/tty")?;

        let settings = super::make_raw(&file)?;

        let (read_r_fd, read_w_fd) = unistd::pipe()?;
        read_r_fd.set_nonblocking()?;
        let read_r_fd = AsyncFd::new(read_r_fd)?;

        let (write_r_fd, write_w_fd) = unistd::pipe()?;
        write_w_fd.set_nonblocking()?;
        let write_w_fd = AsyncFd::new(write_w_fd)?;

        // Note about unsafe borrow below: This is on purpose. We can't move proper BorrowedFd to a
        // thread (does not live long enough), and we also don't want to use Arc because the
        // threads would prevent closing of the file when DevTty is dropped. Use of borrow_raw here
        // lets us rely on the fact that dropping of DevTty will close the file and cause EOF or
        // I/O error in the background threads, which is what lets us shut down those threads.

        let tty_fd = unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) };

        thread::spawn(move || {
            copy(tty_fd, read_w_fd);
        });

        let tty_fd = unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) };

        thread::spawn(move || {
            copy(write_r_fd, tty_fd);
        });

        Ok(Self {
            file,
            read_r_fd,
            write_w_fd,
            settings,
        })
    }

    pub async fn resize(&mut self, size: TtySize) -> io::Result<()> {
        let xtwinops_seq = format!("\x1b[8;{};{}t", size.1, size.0);
        self.write_all(xtwinops_seq.as_bytes()).await?;

        Ok(())
    }
}

impl Drop for DevTty {
    fn drop(&mut self) {
        let termios = Termios::from(self.settings);
        let _ = termios::tcsetattr(self.file.as_fd(), SetArg::TCSANOW, &termios);
    }
}

#[async_trait(?Send)]
impl RawTty for DevTty {
    fn get_size(&self) -> Winsize {
        let mut winsize = Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe { libc::ioctl(self.file.as_raw_fd(), libc::TIOCGWINSZ, &mut winsize) };

        winsize
    }

    async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_r_fd
            .async_io(Interest::READABLE, |fd| {
                unistd::read(fd, buf).map_err(|e| e.into())
            })
            .await
    }

    async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.write_w_fd
            .async_io(Interest::WRITABLE, |fd| {
                unistd::write(fd, buf).map_err(|e| e.into())
            })
            .await
    }
}

fn copy<F: AsFd, G: AsFd>(src_fd: F, dst_fd: G) {
    let src_fd = src_fd.as_fd();
    let dst_fd = dst_fd.as_fd();
    let mut buf = [0u8; BUF_SIZE];
    let mut data = BytesMut::with_capacity(BUF_SIZE);

    loop {
        let mut read_fds = select::FdSet::new();
        let mut write_fds = select::FdSet::new();
        read_fds.insert(src_fd);

        if !data.is_empty() {
            write_fds.insert(dst_fd);
        }

        match select::select(None, Some(&mut read_fds), Some(&mut write_fds), None, None) {
            Ok(0) | Err(Errno::EINTR) => {
                continue;
            }

            Ok(_) => {}

            Err(_) => {
                break;
            }
        }

        if read_fds.contains(src_fd) {
            match unistd::read(src_fd, &mut buf) {
                Ok(0) => break,

                Ok(n) => {
                    data.extend_from_slice(&buf[..n]);
                }

                Err(Errno::EWOULDBLOCK) => {}

                Err(_) => {
                    break;
                }
            }
        }

        if write_fds.contains(dst_fd) {
            match unistd::write(dst_fd, &data) {
                Ok(n) => {
                    data.advance(n);
                }

                Err(Errno::EWOULDBLOCK) => {}

                Err(_) => {
                    break;
                }
            }
        }
    }

    while !data.is_empty() {
        let mut write_fds = select::FdSet::new();
        write_fds.insert(dst_fd);

        match select::select(None, None, Some(&mut write_fds), None, None) {
            Ok(1) => {}

            Ok(0) | Err(Errno::EINTR) => {
                continue;
            }

            Ok(_) => {
                unreachable!();
            }

            Err(_) => {
                break;
            }
        }

        match unistd::write(dst_fd, &data) {
            Ok(n) => {
                data.advance(n);
            }

            Err(Errno::EWOULDBLOCK) => {}

            Err(_) => {
                break;
            }
        }
    }
}
