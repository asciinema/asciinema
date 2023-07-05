import json
from codecs import StreamReader
from json.decoder import JSONDecodeError
from typing import (
    Any,
    Dict,
    Generator,
    Iterable,
    List,
    Optional,
    TextIO,
    Union,
)

from .events import to_absolute_time


class LoadError(Exception):
    pass


class Asciicast:
    def __init__(self, attrs: Dict[str, Any]) -> None:
        self.version: int = 1
        self.__attrs = attrs
        self.idle_time_limit = None  # v1 doesn't store it

    @property
    def v2_header(self) -> Dict[str, Any]:
        keys = ["width", "height", "duration", "command", "title", "env"]
        header = {
            k: v
            for k, v in self.__attrs.items()
            if k in keys and v is not None
        }
        return header

    def events(self, type_: Optional[str] = None) -> Iterable[List[Any]]:
        if type_ in [None, "o"]:
            return to_absolute_time(self.__stdout_events())
        else:
            return []

    def __stdout_events(self) -> Generator[List[Any], None, None]:
        for time, data in self.__attrs["stdout"]:
            yield [time, "o", data]


class open_from_file:
    FORMAT_ERROR: str = "only asciicast v1 format can be opened"

    def __init__(
        self, first_line: str, file: Union[TextIO, StreamReader]
    ) -> None:
        self.first_line = first_line
        self.file = file

    def __enter__(self) -> Optional[Asciicast]:
        try:
            attrs = json.loads(self.first_line + self.file.read())

            if attrs.get("version") == 1:
                return Asciicast(attrs)
            raise LoadError(self.FORMAT_ERROR)
        except JSONDecodeError as e:
            raise LoadError(self.FORMAT_ERROR) from e

    def __exit__(
        self, exc_type: str, exc_value: str, exc_traceback: str
    ) -> None:
        self.file.close()
