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
        except IOError as e:
            self.on_error("Write error, recording suspended")
            raise e


def noop(_: Any) -> None:
    return None
