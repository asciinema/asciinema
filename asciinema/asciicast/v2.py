import codecs
import json
import os
import sys
from codecs import StreamReader
from json.decoder import JSONDecodeError
from typing import (
    Any,
    Callable,
    Dict,
    Generator,
    List,
    Optional,
    TextIO,
    Tuple,
    Union,
)

from ..file_writer import file_writer


class LoadError(Exception):
    pass


class Asciicast:
    def __init__(
        self, f: Union[TextIO, StreamReader], header: Dict[str, Any]
    ) -> None:
        self.version: int = 2
        self.__file = f
        self.v2_header = header
        self.idle_time_limit = header.get("idle_time_limit")

    def events(
        self, type_: Optional[str] = None
    ) -> Generator[List[Any], None, None]:
        if type_ is None:
            for line in self.__file:
                if line == "\n":
                    break
                yield json.loads(line)
        else:
            for line in self.__file:
                if line == "\n":
                    break
                event = json.loads(line)
                if event[1] == type_:
                    yield event


def build_from_header_and_file(
    header: Dict[str, Any], f: Union[StreamReader, TextIO]
) -> Asciicast:
    return Asciicast(f, header)


class open_from_file:
    FORMAT_ERROR = "only asciicast v2 format can be opened"

    def __init__(
        self, first_line: str, file: Union[StreamReader, TextIO]
    ) -> None:
        self.first_line = first_line
        self.file = file

    def __enter__(self) -> Asciicast:
        try:
            v2_header: Dict[str, Any] = json.loads(self.first_line)
            if v2_header.get("version") == 2:
                return build_from_header_and_file(v2_header, self.file)
            raise LoadError(self.FORMAT_ERROR)
        except JSONDecodeError as e:
            raise LoadError(self.FORMAT_ERROR) from e

    def __exit__(
        self, exc_type: str, exc_value: str, exc_traceback: str
    ) -> None:
        self.file.close()


def get_duration(path_: str) -> Any:
    with open(path_, mode="rt", encoding="utf-8") as f:
        first_line = f.readline()
        with open_from_file(first_line, f) as a:
            last_frame = None
            for last_frame in a.events("o"):
                pass
            return last_frame[0]


def build_header(
    width: Optional[int], height: Optional[int], metadata: Any
) -> Dict[str, Any]:
    header = {"version": 2, "width": width, "height": height}
    header.update(metadata)

    assert "width" in header, "width missing in metadata"
    assert "height" in header, "height missing in metadata"
    assert isinstance(header["width"], int)
    assert isinstance(header["height"], int)

    if "timestamp" in header:
        assert isinstance(header["timestamp"], (int, float))

    return header


class writer(file_writer):
    def __init__(  # pylint: disable=too-many-arguments
        self,
        path_: str,
        metadata: Any = None,
        append: bool = False,
        buffering: int = 1,
        width: Optional[int] = None,
        height: Optional[int] = None,
        on_error: Optional[Callable[[str], None]] = None,
    ) -> None:
        super().__init__(path_, on_error)

        self.buffering = buffering
        self.stdin_decoder = codecs.getincrementaldecoder("UTF-8")("replace")
        self.stdout_decoder = codecs.getincrementaldecoder("UTF-8")("replace")

        if append:
            self.mode = "a"
            self.header = None
        else:
            self.mode = "w"
            self.header = build_header(width, height, metadata or {})

    def __enter__(self) -> Any:
        self._open_file()

        if self.header:
            self.__write_line(self.header)

        return self

    def write_stdout(self, ts: float, data: Union[str, bytes]) -> None:
        if isinstance(data, str):
            data = data.encode(encoding="utf-8", errors="strict")
        data = self.stdout_decoder.decode(data)
        self.__write_event(ts, "o", data)

    def write_stdin(self, ts: float, data: Union[str, bytes]) -> None:
        if isinstance(data, str):
            data = data.encode(encoding="utf-8", errors="strict")
        data = self.stdin_decoder.decode(data)
        self.__write_event(ts, "i", data)

    def write_marker(self, ts: float) -> None:
        self.__write_event(ts, "m", "")

    def write_resize(self, ts: float, size: Tuple[int, int]) -> None:
        cols, rows = size
        self.__write_event(ts, "r", f"{cols}x{rows}")

    # pylint: disable=consider-using-with
    def _open_file(self) -> None:
        if self.path == "-":
            self.file = os.fdopen(
                sys.stdout.fileno(),
                mode=self.mode,
                buffering=self.buffering,
                encoding="utf-8",
                closefd=False,
            )
        else:
            self.file = open(
                self.path,
                mode=self.mode,
                buffering=self.buffering,
                encoding="utf-8",
            )

    def __write_event(self, ts: float, etype: str, data: str) -> None:
        self.__write_line([round(ts, 6), etype, data])

    def __write_line(self, obj: Any) -> None:
        line = json.dumps(
            obj, ensure_ascii=False, indent=None, separators=(", ", ": ")
        )

        self._write(f"{line}\n")
