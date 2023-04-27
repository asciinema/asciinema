import pytest
import os
import pty
from typing import Any, List, Union

import asciinema.pty_

from .test_helper import Test


class Writer:
    def __init__(self) -> None:
        self.data: List[Union[float, str]] = []

    def write_stdout(self, _ts: float, data: Any) -> None:
        self.data.append(data)

    def write_stdin(self, ts: float, data: Any) -> None:
        raise NotImplementedError


class TestRecord(Test):
    def setUp(self) -> None:
        self.real_os_write = os.write
        os.write = self.os_write  # type: ignore

    def tearDown(self) -> None:
        os.write = self.real_os_write

    def os_write(self, fd: int, data: Any) -> None:
        if fd != pty.STDOUT_FILENO:
            self.real_os_write(fd, data)

    @staticmethod
    def test_record_command_writes_to_stdout() -> None:
        writer = Writer()

        command = [
            "python3",
            "-c",
            (
                "import sys"
                "; import time"
                "; sys.stdout.write('foo')"
                "; sys.stdout.flush()"
                "; time.sleep(0.01)"
                "; sys.stdout.write('bar')"
            ),
        ]

        asciinema.pty_.record(
            command, {}, writer, lambda: (80, 24), lambda s: None, {}
        )

        assert writer.data == [b"foo", b"bar"]

    @staticmethod
    def test_unsupressed_record_command_writes_to_fd() -> None:
        writer = Writer()

        command = [
            "python3",
            "-c",
            (
                "import sys"
                "; import time"
                "; sys.stdout.write('foo')"
                "; sys.stdout.flush()"
                "; time.sleep(0.01)"
                "; sys.stdout.write('bar')"
            ),
        ]

        rfd, wfd = os.pipe()
        asciinema.pty_.record(
            command, {}, writer, lambda: (80, 24), lambda s: None, {}, tty_stdout_fd=wfd,
        )

        assert writer.data == [b"foo", b"bar"]
        assert os.read(rfd, 100) == b"foobar"

    @staticmethod
    def test_supressed_record_command_does_not_write_to_fd() -> None:
        writer = Writer()

        command = [
            "python3",
            "-c",
            (
                "import sys"
                "; import time"
                "; sys.stdout.write('foo')"
                "; sys.stdout.flush()"
                "; time.sleep(0.01)"
                "; sys.stdout.write('bar')"
            ),
        ]

        rfd, wfd = os.pipe()
        asciinema.pty_.record(
            command, {}, writer, lambda: (80, 24), lambda s: None, {}, tty_stdout_fd=wfd,
            suppress_output=True,
        )

        assert writer.data == [b"foo", b"bar"]
        # As we pass `suppress_output=True`, we expect `tty_stdout_fd` to never be written to.
        # As the pipe is empty, calling `os.read` on it will block forever
        # We use the raise `BlockingIOError` as a signal that the pipe is empty,
        # which means `pty_.record` did not write anything to `tty_stdout_fd`.
        os.set_blocking(rfd, False)
        with pytest.raises(BlockingIOError):
            assert os.read(rfd, 100) == b""
