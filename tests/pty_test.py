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
