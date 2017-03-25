import tty
import select
import os


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
            tty.tcsetattr(self.fd, tty.TCSAFLUSH, self.mode)


def read_non_blocking(fd):
    data = b''

    while fd in select.select([fd], [], [], 0)[0]:
        data += os.read(fd, 1024)

    return data
