import os
import stat
from typing import IO, Any, Callable, Optional


class file_writer:
    def __init__(
        self,
        path: str,
        on_error: Optional[Callable[[str], None]] = None,
    ) -> None:
        self.path = path
        self.file: Optional[IO[Any]] = None
        self.on_error = on_error or noop

    def __enter__(self) -> Any:
        self._open_file()
        return self

    def __exit__(
        self, exc_type: str, exc_value: str, exc_traceback: str
    ) -> None:
        assert self.file is not None
        self.file.close()

    def _open_file(self) -> None:
        raise NotImplementedError

    def _write(self, data: Any) -> None:
        try:
            self.file.write(data)  # type: ignore
        except BrokenPipeError as e:
            if self.path != "-" and stat.S_ISFIFO(os.stat(self.path).st_mode):
                self.on_error("Broken pipe, reopening...")
                self._open_file()
                self.on_error("Output pipe reopened successfully")
                self.file.write(data)  # type: ignore
            else:
                self.on_error("Output pipe broken")
                raise e


def noop(_: Any) -> None:
    return None
