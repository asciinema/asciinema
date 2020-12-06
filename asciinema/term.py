import os
import select
import subprocess
import time
import tty


class raw():
    def __init__(self, fd):
        self.fd = fd
        self.restore = False

    def __enter__(self):
        try:
            self.mode = tty.tcgetattr(self.fd)
            tty.setraw(self.fd)
            self.restore = True
        except tty.error:  # This is the same as termios.error
            pass

    def __exit__(self, type, value, traceback):
        if self.restore:
            # Give the terminal time to send answerbacks
            time.sleep(0.01)
            tty.tcsetattr(self.fd, tty.TCSAFLUSH, self.mode)


def read_blocking(fd, timeout):
    if fd in select.select([fd], [], [], timeout)[0]:
        return os.read(fd, 1024)

    return b''


def get_size():
    try:
        return os.get_terminal_size()
    except:
        return (
            int(subprocess.check_output(['tput', 'cols'])),
            int(subprocess.check_output(['tput', 'lines']))
        )
