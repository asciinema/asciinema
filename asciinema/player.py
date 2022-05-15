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
                        stream,
                        output,
                    )
        except Exception:  # pylint: disable=broad-except
            self._play(
                asciicast,
                idle_time_limit,
                speed,
                None,
                key_bindings,
                stream,
                output,
            )

    @staticmethod
    def _play(  # pylint: disable=too-many-locals
        asciicast: Union[v1, v2],
        idle_time_limit: Optional[int],
        speed: float,
        stdin: Optional[TextIO],
        key_bindings: Dict[str, Any],
        stream: Optional[str],
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

        base_time = time.time()
        ctrl_c = False
        paused = False
        pause_time: Optional[float] = None

        for t, event_type, text in events:
            delay = t - (time.time() - base_time)

            while stdin and not ctrl_c and delay > 0:
                if paused:
                    while True:
                        data = read_blocking(stdin.fileno(), 1000)

                        if 0x03 in data:  # ctrl-c
                            ctrl_c = True
                            break

                        if data == pause_key:
                            paused = False
                            assert pause_time is not None
                            base_time += time.time() - pause_time
                            break

                        if data == step_key:
                            delay = 0
                            pause_time = time.time()
                            base_time = pause_time - t
                            break
                else:
                    data = read_blocking(stdin.fileno(), delay)

                    if not data:
                        break

                    if 0x03 in data:  # ctrl-c
                        ctrl_c = True
                        break

                    if data == pause_key:
                        paused = True
                        pause_time = time.time()
                        slept = t - (pause_time - base_time)
                        delay = delay - slept

            if ctrl_c:
                raise KeyboardInterrupt()

            output.write(t, event_type, text)
