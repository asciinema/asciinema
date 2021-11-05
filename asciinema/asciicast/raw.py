from os import path, stat
from typing import IO, Any, Optional


class writer:
    def __init__(
        self,
        path_: str,
        metadata: Any = None,
        append: bool = False,
        buffering: int = 0,
    ) -> None:
        if (
            append and path.exists(path_) and stat(path_).st_size == 0
        ):  # true for pipes
            append = False

        self.path = path_
        self.buffering = buffering
        self.mode: str = "ab" if append else "wb"
        self.file: Optional[IO[Any]] = None
        self.metadata = metadata

    def __enter__(self) -> Any:
        self.file = open(self.path, mode=self.mode, buffering=self.buffering)
        return self

    def __exit__(
        self, exc_type: str, exc_value: str, exc_traceback: str
    ) -> None:
        assert self.file is not None
        self.file.close()

    def write_stdout(self, ts: float, data: Any) -> None:
        _ = ts
        assert self.file is not None
        self.file.write(data)

    # pylint: disable=no-self-use
    def write_stdin(self, ts: float, data: Any) -> None:
        pass
