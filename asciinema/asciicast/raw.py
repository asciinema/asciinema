import os
import sys
from os import path
from typing import Any, Callable, Optional

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
        self.mode: str = "ab" if append else "wb"
        self.metadata = metadata

    def write_stdout(self, _ts: float, data: Any) -> None:
        self._write(data)

    # pylint: disable=no-self-use
    def write_stdin(self, ts: float, data: Any) -> None:
        pass

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
