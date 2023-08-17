import os
import sys
from os import path
from typing import Any, Callable, Optional, Tuple

from ..file_writer import file_writer


class writer(file_writer):
    def __init__(  # pylint: disable=too-many-arguments
        self,
        path_: str,
        metadata: Any = None,
        append: bool = False,
        buffering: int = 0,
        on_error: Optional[Callable[[str], None]] = None,
    ) -> None:
        super().__init__(path_, on_error)

        if (
            append and path.exists(path_) and os.stat(path_).st_size == 0
        ):  # true for pipes
            append = False

        self.buffering = buffering

        if append:
            self.mode = "ab"
            self.header = None
        else:
            self.mode = "wb"
            width = metadata["width"]
            height = metadata["height"]
            self.header = f"\x1b[8;{height};{width}t".encode("utf-8")

    def __enter__(self) -> Any:
        super().__enter__()

        if self.header:
            self._write(self.header)

        return self

    def write_stdout(self, _ts: float, data: Any) -> None:
        self._write(data)

    def write_stdin(self, ts: float, data: Any) -> None:
        pass

    def write_marker(self, ts: float) -> None:
        pass

    def write_resize(self, ts: float, size: Tuple[int, int]) -> None:
        cols, rows = size
        self._write(f"\x1b[8;{rows};{cols}t".encode("utf-8"))

    # pylint: disable=consider-using-with
    def _open_file(self) -> None:
        if self.path == "-":
            self.file = os.fdopen(
                sys.stdout.fileno(),
                mode=self.mode,
                buffering=self.buffering,
                closefd=False,
            )
        else:
            self.file = open(
                self.path, mode=self.mode, buffering=self.buffering
            )
