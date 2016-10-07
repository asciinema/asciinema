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
import tempfile
import random


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

    def record_script(self, script, output, env=os.environ):

        def random_sleep(min_secs, max_secs):
            val = random.random()
            while(val < min_secs or val > max_secs):
                val = random.random()
            return val

        def escape_char(char):
            if char in ['$', '"']:
                char = '\\%s' % char
            return char

        def type_line(line, fh, min_secs, max_secs):
            # type specified line of text, like a human would
            # TAB: tab completion until next space
            # COPYPASTE: complete command until next space
            i, n = 0, len(line)
            while i < n:
                if line[i:].startswith('TAB') or line[i:].startswith('COPYPASTE'):
                    tab = False
                    if line[i:].startswith('TAB'):
                        i += len('TAB')
                        tab = True
                    else:
                        i += len('COPYPASTE')

                    # complete until next space or end of line
                    word = ''
                    while i < n and line[i] != ' ' and not (tab and line[i] == '/'):
                        word += escape_char(line[i])
                        i += 1
                else:
                    word = escape_char(line[i])
                    i += 1

                fh.write('echo -n "%s"; sleep %f\n' % (word, random_sleep(min_secs, max_secs)))

            fh.write('sleep %f; echo\n' % random_sleep(min_secs, max_secs))

        def verified_command(cmd, fh):
            cmd = cmd.replace('TAB', '').replace('COPYPASTE', '')
            fh.write(cmd + '\n')
            fh.write('if [ $? -ne 0 ]; then echo "ERROR: last command failed" >&2; exit 1; fi\n')

        fd, scriptfile = tempfile.mkstemp(suffix='.sh')
        os.close(fd)
        fh = open(scriptfile, 'w')
        fh.write('#!/bin/bash\n')

        for line in open(script).read().strip().split('\n'):

            if line.startswith('###'):
                # typed comment
                type_line(line[2:], fh, 0.01, 0.05)

            elif line.startswith('##') or line == '':
                # regular comment or empty line; just echo, but make sure bash doesn't interpret its contents
                fh.write('echo "%s"\n' % line[1:].replace("$", "\\$"))

            elif line.startswith('# '):
                # silent command (just execute it, don't show it
                verified_command(line[2:], fh)

            elif line == 'CLEAR':
                # wipe screen using 'clear' command
                fh.write('clear  # CLEAR\n')

            elif line.startswith('PAUSE '):
                # pause for specified amount of time
                secs = float(line[len('PAUSE '):])
                fh.write('sleep %f  # PAUSE\n' % secs)

            else:
                fh.write('echo; echo -n "$ "; sleep %f\n' % random_sleep(0.1, 0.3))
                type_line(line, fh, 0.01, 0.1)
                verified_command(line, fh)
        fh.close()

        self.record_command(['/bin/bash', '-i', scriptfile], output, env)
        os.remove(scriptfile)
