import sys
import time
from typing import Any, Dict, Optional, TextIO, Union

from .asciicast import events as ev
from .asciicast.v1 import Asciicast as v1
from .asciicast.v2 import Asciicast as v2
from .tty_ import raw, read_blocking


class Player:  # pylint: disable=too-few-public-methods
    def play(
        self,
        asciicast: Union[v1, v2],
        idle_time_limit: Optional[int] = None,
        speed: float = 1.0,
        key_bindings: Optional[Dict[str, Any]] = None,
    ) -> None:
        if key_bindings is None:
            key_bindings = {}
        try:
            with open("/dev/tty", "rt", encoding="utf-8") as stdin:
                with raw(stdin.fileno()):
                    self._play(
                        asciicast, idle_time_limit, speed, stdin, key_bindings
                    )
        except Exception:  # pylint: disable=broad-except
            self._play(asciicast, idle_time_limit, speed, None, key_bindings)

    @staticmethod
    def _play(  # pylint: disable=too-many-locals
        asciicast: Union[v1, v2],
        idle_time_limit: Optional[int],
        speed: float,
        stdin: Optional[TextIO],
        key_bindings: Dict[str, Any],
    ) -> None:
        idle_time_limit = idle_time_limit or asciicast.idle_time_limit
        pause_key = key_bindings.get("pause")
        step_key = key_bindings.get("step")

        stdout = asciicast.stdout_events()
        stdout = ev.to_relative_time(stdout)
        stdout = ev.cap_relative_time(stdout, idle_time_limit)
        stdout = ev.to_absolute_time(stdout)
        stdout = ev.adjust_speed(stdout, speed)

        base_time = time.time()
        ctrl_c = False
        paused = False
        pause_time: Optional[float] = None

        for t, _type, text in stdout:
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
                break

            sys.stdout.write(text)
            sys.stdout.flush()
