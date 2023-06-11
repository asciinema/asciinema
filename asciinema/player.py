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
        pause_on_markers: bool = False,
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
                        pause_on_markers,
                    )
        except IOError:
            self._play(
                asciicast,
                idle_time_limit,
                speed,
                None,
                key_bindings,
                output,
                False,
            )

    @staticmethod
    def _play(  # pylint: disable=too-many-locals
        asciicast: Union[v1, v2],
        idle_time_limit: Optional[int],
        speed: float,
        stdin: Optional[TextIO],
        key_bindings: Dict[str, Any],
        output: Output,
        pause_on_markers: bool,
    ) -> None:
        idle_time_limit = idle_time_limit or asciicast.idle_time_limit
        pause_key = key_bindings.get("pause")
        step_key = key_bindings.get("step")
        next_marker_key = key_bindings.get("next_marker")

        events = asciicast.events()
        events = ev.to_relative_time(events)
        events = ev.cap_relative_time(events, idle_time_limit)
        events = ev.to_absolute_time(events)
        events = ev.adjust_speed(events, speed)

        output.start(asciicast.v2_header)

        ctrl_c = False
        pause_elapsed_time: Optional[float] = None
        events_iter = iter(events)
        start_time = time.perf_counter()

        def wait(timeout: int) -> bytes:
            if stdin is not None:
                return read_blocking(stdin.fileno(), timeout)

            return b""

        def next_event() -> Any:
            try:
                return events_iter.__next__()
            except StopIteration:
                return (None, None, None)

        time_, event_type, text = next_event()

        while time_ is not None and not ctrl_c:
            if pause_elapsed_time:
                while time_ is not None:
                    key = wait(1000)

                    if 0x03 in key:  # ctrl-c
                        ctrl_c = True
                        break

                    if key == pause_key:
                        assert pause_elapsed_time is not None
                        start_time = time.perf_counter() - pause_elapsed_time
                        pause_elapsed_time = None
                        break

                    if key == step_key:
                        pause_elapsed_time = time_
                        output.write(time_, event_type, text)
                        time_, event_type, text = next_event()

                    elif key == next_marker_key:
                        while time_ is not None and event_type != "m":
                            output.write(time_, event_type, text)
                            time_, event_type, text = next_event()

                        if time_ is not None:
                            output.write(time_, event_type, text)
                            pause_elapsed_time = time_
                            time_, event_type, text = next_event()
            else:
                while time_ is not None:
                    elapsed_wall_time = time.perf_counter() - start_time
                    delay = time_ - elapsed_wall_time
                    key = b""

                    if delay > 0:
                        key = wait(delay)

                    if 0x03 in key:  # ctrl-c
                        ctrl_c = True
                        break

                    elif key == pause_key:
                        pause_elapsed_time = time.perf_counter() - start_time
                        break

                    else:
                        output.write(time_, event_type, text)

                        if event_type == "m" and pause_on_markers:
                            pause_elapsed_time = time_
                            time_, event_type, text = next_event()
                            break

                        time_, event_type, text = next_event()

        if ctrl_c:
            raise KeyboardInterrupt()
