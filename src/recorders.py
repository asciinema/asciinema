import array
import fcntl
import os
import pty
import select
import select
import signal
import subprocess
import sys
import termios
import time
import tty


def record_process(command, is_shell, stdout_file, stdin_file=None):
    recorder = ProcessRecorder(command, is_shell, stdout_file, stdin_file)
    return record(recorder)


def record(recorder):
    start_time = time.time()
    recorder.run()
    end_time = time.time()
    return end_time - start_time


class ProcessRecorder(object):
    '''Pseudo-terminal recorder.

    Creates new pseudo-terminal for spawned process
    and saves stdin/stderr (and timing) to files.
    '''

    def __init__(self, command, is_shell, stdout_file, stdin_file=None):
        self.command = command.split(' ')
        self.is_shell = is_shell
        self.stdout_file = stdout_file
        self.stdin_file = stdin_file
        self.master_fd = None

    def run(self):
        self.reset_terminal()

        if self.is_shell:
            self._write_stdout('~ Asciicast recording started. Hit ^D (that\'s Ctrl+D) or type "exit" to finish.\n\n')

        success = self._spawn()
        self.reset_terminal()
        self._write_stdout('~ Asciicast recording finished.\n')

        return success

    def reset_terminal(self):
        pass
        # subprocess.call(["reset"])

    def _spawn(self):
        '''Create a spawned process.

        Based on pty.spawn() from standard library.
        '''

        assert self.master_fd is None

        pid, self.master_fd = pty.fork()

        if pid == pty.CHILD:
            os.execlp(self.command[0], *self.command)

        old_handler = signal.signal(signal.SIGWINCH, self._signal_winch)

        try:
            mode = tty.tcgetattr(pty.STDIN_FILENO)
            tty.setraw(pty.STDIN_FILENO)
            restore = 1
        except tty.error: # This is the same as termios.error
            restore = 0

        self._set_pty_size()

        try:
            self._copy()
        except (IOError, OSError):
            if restore:
                tty.tcsetattr(pty.STDIN_FILENO, tty.TCSAFLUSH, mode)

        os.close(self.master_fd)
        self.master_fd = None
        signal.signal(signal.SIGWINCH, old_handler)

        return True

    def _signal_winch(self, signal, frame):
        '''Signal handler for SIGWINCH - window size has changed.'''

        self._set_pty_size()

    def _set_pty_size(self):
        '''
        Sets the window size of the child pty based on the window size
        of our own controlling terminal.
        '''

        assert self.master_fd is not None

        # Get the terminal size of the real terminal, set it on the pseudoterminal.
        buf = array.array('h', [0, 0, 0, 0])
        fcntl.ioctl(pty.STDOUT_FILENO, termios.TIOCGWINSZ, buf, True)
        fcntl.ioctl(self.master_fd, termios.TIOCSWINSZ, buf)

    def _copy(self):
        '''Main select loop.

        Passes control to self._master_read() or self._stdin_read()
        when new data arrives.
        '''

        assert self.master_fd is not None

        while 1:
            try:
                rfds, wfds, xfds = select.select([self.master_fd, pty.STDIN_FILENO], [], [])
            except select.error, e:
                if e[0] == 4:   # Interrupted system call.
                    continue

            if self.master_fd in rfds:
                data = os.read(self.master_fd, 1024)

                if len(data) == 0:
                  break

                self._handle_master_read(data)

            if pty.STDIN_FILENO in rfds:
                data = os.read(pty.STDIN_FILENO, 1024)
                self._handle_stdin_read(data)

    def _handle_master_read(self, data):
        '''Handles new data on child process stdout.'''

        self._write_stdout(data)
        self.stdout_file.write(data)

    def _handle_stdin_read(self, data):
        '''Handles new data on child process stdin.'''

        self._write_master(data)
        if self.stdin_file:
            self.stdin_file.write(data)

    def _write_stdout(self, data):
        '''Writes to stdout as if the child process had written the data.'''

        os.write(pty.STDOUT_FILENO, data)

    def _write_master(self, data):
        '''Writes to the child process from its controlling terminal.'''

        assert self.master_fd is not None

        while data != '':
            n = os.write(self.master_fd, data)
            data = data[n:]
