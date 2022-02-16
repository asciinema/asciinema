import os
import select
import termios as tty  # avoid `Module "tty" has no attribute ...` errors
from time import sleep
from tty import setraw
from typing import IO, Any, List, Optional, Union


class raw:
    def __init__(self, fd: Union[IO[str], int]) -> None:
        self.fd = fd
        self.restore: bool = False
        self.mode: Optional[List[Any]] = None

    def __enter__(self) -> None:
        try:
            self.mode = tty.tcgetattr(self.fd)
            setraw(self.fd)
            self.restore = True
        except tty.error:  # this is `termios.error`
            pass

    def __exit__(self, type_: str, value: str, traceback: str) -> None:
        if self.restore:
            sleep(0.01)  # give the terminal time to send answerbacks
            assert isinstance(self.mode, list)
            tty.tcsetattr(self.fd, tty.TCSAFLUSH, self.mode)


def read_blocking(fd: int, timeout: Any) -> bytes:
    if fd in select.select([fd], [], [], timeout)[0]:
        return os.read(fd, 1024)

    return b""
