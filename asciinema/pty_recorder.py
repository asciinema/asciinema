import errno
import os
import pty
import signal
import tty
import array
import fcntl
import termios
import select
import io
import shlex
import sys
import struct


class PtyRecorder:

    def record_command(self, command, output, env=os.environ):
        master_fd = None

        def _set_pty_size():
            '''
            Sets the window size of the child pty based on the window size
            of our own controlling terminal.
            '''

            # Get the terminal size of the real terminal, set it on the pseudoterminal.
            if os.isatty(pty.STDOUT_FILENO):
                buf = array.array('h', [0, 0, 0, 0])
                fcntl.ioctl(pty.STDOUT_FILENO, termios.TIOCGWINSZ, buf, True)
                fcntl.ioctl(master_fd, termios.TIOCSWINSZ, buf)
            else:
                buf = array.array('h', [24, 80, 0, 0])
                fcntl.ioctl(master_fd, termios.TIOCSWINSZ, buf)

        def _write_stdout(data):
            '''Writes to stdout as if the child process had written the data.'''

            os.write(pty.STDOUT_FILENO, data)

        def _handle_master_read(data):
            '''Handles new data on child process stdout.'''

            _write_stdout(data)
            output.write(data)

        def _write_master(data):
            '''Writes to the child process from its controlling terminal.'''

            while data:
                n = os.write(master_fd, data)
                data = data[n:]

        def _handle_stdin_read(data):
            '''Handles new data on child process stdin.'''

            _write_master(data)

        def _copy(signal_fd):
            '''Main select loop.

            Passes control to _master_read() or _stdin_read()
            when new data arrives.
            '''

            fds = [master_fd, pty.STDIN_FILENO, signal_fd]

            while True:
                try:
                    rfds, wfds, xfds = select.select(fds, [], [])
                except OSError as e: # Python >= 3.3
                    if e.errno == errno.EINTR:
                        continue
                except select.error as e: # Python < 3.3
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
                            if sig == signal.SIGCHLD:
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

        old_sigwinch_handler = signal.signal(signal.SIGWINCH, lambda signal, frame: None)
        old_sigchld_handler = signal.signal(signal.SIGCHLD, lambda signal, frame: None)

        try:
            mode = tty.tcgetattr(pty.STDIN_FILENO)
            tty.setraw(pty.STDIN_FILENO)
            restore = 1
        except tty.error: # This is the same as termios.error
            restore = 0

        _set_pty_size()

        try:
            _copy(pipe_r)
        except (IOError, OSError):
            pass
        finally:
            if restore:
                tty.tcsetattr(pty.STDIN_FILENO, tty.TCSAFLUSH, mode)

        signal.signal(signal.SIGWINCH, old_sigwinch_handler)
        signal.signal(signal.SIGCHLD, old_sigchld_handler)

        os.waitpid(pid, 0)
        output.close()
