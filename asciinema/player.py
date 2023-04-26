import json
import sys
import time
from typing import Any, Dict, Optional, TextIO, Union

from .asciicast import events as ev
from .asciicast.v1 import Asciicast as v1
from .asciicast.v2 import Asciicast as v2
from .tty_ import raw, read_blocking

Header = Dict[str, Any]


class RawOutput:
    def __init__(self, stream: Optional[str]) -> None:
        self.stream = stream or "o"

    def start(self, _header: Header) -> None:
        pass

    def write(self, _time: float, event_type: str, data: str) -> None:
        if event_type == self.stream:
            sys.stdout.write(data)
            sys.stdout.flush()


class AsciicastOutput:
    def __init__(self, stream: Optional[str]) -> None:
        self.stream = stream

    def start(self, header: Header) -> None:
        self.__write_line(header)

    def write(self, time: float, event_type: str, data: str) -> None:
        if self.stream in [None, event_type]:
            self.__write_line([time, event_type, data])

    def __write_line(self, obj: Any) -> None:
        line = json.dumps(
            obj, ensure_ascii=False, indent=None, separators=(", ", ": ")
        )

        sys.stdout.write(f"{line}\r\n")
        sys.stdout.flush()


Output = Union[RawOutput, AsciicastOutput]


class Player:  # pylint: disable=too-few-public-methods
    def play(
        self,
        asciicast: Union[v1, v2],
        idle_time_limit: Optional[int] = None,
        speed: float = 1.0,
        key_bindings: Optional[Dict[str, Any]] = None,
        out_fmt: str = "raw",
        stream: Optional[str] = None,
    ) -> None:
        if key_bindings is None:
            key_bindings = {}

        output: Output = (
            RawOutput(stream) if out_fmt == "raw" else AsciicastOutput(stream)
        )

        try:
            with open("/dev/tty", "rt", encoding="utf-8") as stdin:
                with raw(stdin.fileno()):
                    self._play(
                        asciicast,
                        idle_time_limit,
                        speed,
                        stdin,
                        key_bindings,
                        output,
                    )
        except IOError:
            self._play(
                asciicast,
                idle_time_limit,
                speed,
                None,
                key_bindings,
                output,
            )

    @staticmethod
    def _play(  # pylint: disable=too-many-locals
        asciicast: Union[v1, v2],
        idle_time_limit: Optional[int],
        speed: float,
        stdin: Optional[TextIO],
        key_bindings: Dict[str, Any],
        output: Output,
    ) -> None:
        idle_time_limit = idle_time_limit or asciicast.idle_time_limit
        pause_key = key_bindings.get("pause")
        step_key = key_bindings.get("step")

        events = asciicast.events()
        events = ev.to_relative_time(events)
        events = ev.cap_relative_time(events, idle_time_limit)
        events = ev.to_absolute_time(events)
        events = ev.adjust_speed(events, speed)

        output.start(asciicast.v2_header)

        start_time = time.time()
        ctrl_c = False
        pause_elapsed_time: Optional[float] = None

        for time_, event_type, text in events:
            elapsed_wall_time = time.time() - start_time
            delay = time_ - elapsed_wall_time
            sleep = delay > 0

            while stdin and sleep and not ctrl_c:
                if pause_elapsed_time:
                    while True:
                        data = read_blocking(stdin.fileno(), 1000)

                        if 0x03 in data:  # ctrl-c
                            ctrl_c = True
                            break

                        if data == pause_key:
                            assert pause_elapsed_time is not None
                            start_time = time.time() - pause_elapsed_time
                            pause_elapsed_time = None
                            break

                        if data == step_key:
                            pause_elapsed_time = time_
                            sleep = False
                            break
                else:
                    data = read_blocking(stdin.fileno(), delay)

                    if not data:
                        break

                    if 0x03 in data:  # ctrl-c
                        ctrl_c = True
                        break

                    if data == pause_key:
                        pause_elapsed_time = time.time() - start_time

            if ctrl_c:
                raise KeyboardInterrupt()

            output.write(time_, event_type, text)
