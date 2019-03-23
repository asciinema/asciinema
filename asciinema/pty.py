import array
import errno
import fcntl
import io
import os
import pty
import select
import shlex
import signal
import struct
import sys
import termios
import time

from asciinema.term import raw


def record(command, writer, env=os.environ, rec_stdin=False, time_offset=0, notifier=None):
    master_fd = None
    start_time = None
    pause_time = None

    def _notify(text):
        if notifier:
            notifier.notify(text)

    def _set_pty_size():
        '''
        Sets the window size of the child pty based on the window size
        of our own controlling terminal.
        '''

        # Get the terminal size of the real terminal, set it on the pseudoterminal.
        if os.isatty(pty.STDOUT_FILENO):
            buf = array.array('h', [0, 0, 0, 0])
            fcntl.ioctl(pty.STDOUT_FILENO, termios.TIOCGWINSZ, buf, True)
        else:
            buf = array.array('h', [24, 80, 0, 0])

        fcntl.ioctl(master_fd, termios.TIOCSWINSZ, buf)

    def _write_stdout(data):
        '''Writes to stdout as if the child process had written the data.'''

        os.write(pty.STDOUT_FILENO, data)

    def _handle_master_read(data):
        '''Handles new data on child process stdout.'''

        if not pause_time:
            writer.write_stdout(time.time() - start_time, data)

        _write_stdout(data)

    def _write_master(data):
        '''Writes to the child process from its controlling terminal.'''

        while data:
            n = os.write(master_fd, data)
            data = data[n:]

    def _handle_stdin_read(data):
        '''Handles new data on child process stdin.'''

        nonlocal pause_time
        nonlocal start_time

        if data == b'\x10':  # ctrl+p
            if pause_time:
                start_time = start_time + (time.time() - pause_time)
                pause_time = None
                _notify('Resumed recording')
            else:
                pause_time = time.time()
                _notify('Paused recording')
        else:
            _write_master(data)

            if rec_stdin and not pause_time:
                writer.write_stdin(time.time() - start_time, data)

    def _signals(signal_list):
        old_handlers = []
        for sig, handler in signal_list:
            old_handlers.append((sig, signal.signal(sig, handler)))
        return old_handlers

    def _copy(signal_fd):
        '''Main select loop.

        Passes control to _master_read() or _stdin_read()
        when new data arrives.
        '''

        fds = [master_fd, pty.STDIN_FILENO, signal_fd]

        while True:
            try:
                rfds, wfds, xfds = select.select(fds, [], [])
            except OSError as e:  # Python >= 3.3
                if e.errno == errno.EINTR:
                    continue
            except select.error as e:  # Python < 3.3
                if e.args[0] == 4:
                    continue

            if master_fd in rfds:
                data = os.read(master_fd, 1024)
                if not data:  # Reached EOF.
                    fds.remove(master_fd)
                else:
                    _handle_master_read(data)

            if pty.STDIN_FILENO in rfds:
                data = os.read(pty.STDIN_FILENO, 1024)
                if not data:
                    fds.remove(pty.STDIN_FILENO)
                else:
                    _handle_stdin_read(data)

            if signal_fd in rfds:
                data = os.read(signal_fd, 1024)
                if data:
                    signals = struct.unpack('%uB' % len(data), data)
                    for sig in signals:
                        if sig in [signal.SIGCHLD, signal.SIGHUP, signal.SIGTERM, signal.SIGQUIT]:
                            os.close(master_fd)
                            return
                        elif sig == signal.SIGWINCH:
                            _set_pty_size()

    pid, master_fd = pty.fork()

    if pid == pty.CHILD:
        os.execvpe(command[0], command, env)

    pipe_r, pipe_w = os.pipe()
    flags = fcntl.fcntl(pipe_w, fcntl.F_GETFL, 0)
    flags = flags | os.O_NONBLOCK
    flags = fcntl.fcntl(pipe_w, fcntl.F_SETFL, flags)

    signal.set_wakeup_fd(pipe_w)

    old_handlers = _signals(map(lambda s: (s, lambda signal, frame: None),
                                [signal.SIGWINCH,
                                    signal.SIGCHLD,
                                    signal.SIGHUP,
                                    signal.SIGTERM,
                                    signal.SIGQUIT]))

    _set_pty_size()

    start_time = time.time() - time_offset

    with raw(pty.STDIN_FILENO):
        try:
            _copy(pipe_r)
        except (IOError, OSError):
            pass

    _signals(old_handlers)

    os.waitpid(pid, 0)
