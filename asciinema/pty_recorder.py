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

from .stdout import Stdout


class PtyRecorder(object):

    def record_command(self, command, output=None):
        command = shlex.split(command)
        output = output if output is not None else Stdout()
        master_fd = None

        def _set_pty_size():
            '''
            Sets the window size of the child pty based on the window size
            of our own controlling terminal.
            '''

            # Get the terminal size of the real terminal, set it on the pseudoterminal.
            buf = array.array('h', [0, 0, 0, 0])
            fcntl.ioctl(pty.STDOUT_FILENO, termios.TIOCGWINSZ, buf, True)
            fcntl.ioctl(master_fd, termios.TIOCSWINSZ, buf)

        def _signal_winch(signal, frame):
            '''Signal handler for SIGWINCH - window size has changed.'''

            _set_pty_size()

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

        def _copy():
            '''Main select loop.

            Passes control to _master_read() or _stdin_read()
            when new data arrives.
            '''

            while 1:
                try:
                    rfds, wfds, xfds = select.select([master_fd, pty.STDIN_FILENO], [], [])
                except select.error as e:
                    if e[0] == 4:   # Interrupted system call.
                        continue

                if master_fd in rfds:
                    data = os.read(master_fd, 1024)

                    if len(data) == 0:
                        break

                    _handle_master_read(data)

                if pty.STDIN_FILENO in rfds:
                    data = os.read(pty.STDIN_FILENO, 1024)
                    _handle_stdin_read(data)


        pid, master_fd = pty.fork()

        if pid == pty.CHILD:
            os.execlp(command[0], *command)

        old_handler = signal.signal(signal.SIGWINCH, _signal_winch)

        try:
            mode = tty.tcgetattr(pty.STDIN_FILENO)
            tty.setraw(pty.STDIN_FILENO)
            restore = 1
        except tty.error: # This is the same as termios.error
            restore = 0

        _set_pty_size()

        try:
            _copy()
        except (IOError, OSError):
            if restore:
                tty.tcsetattr(pty.STDIN_FILENO, tty.TCSAFLUSH, mode)

        os.close(master_fd)
        signal.signal(signal.SIGWINCH, old_handler)

        return output
