import os
import sys
import time

import asciinema.asciicast.frames as frames
from asciinema.term import raw, read_non_blocking


class Player:

    def play(self, asciicast, idle_time_limit=None, speed=1.0):
        try:
            stdin = open('/dev/tty')
            with raw(stdin.fileno()):
                self._play(asciicast, idle_time_limit, speed, stdin)
        except:
            self._play(asciicast, idle_time_limit, speed, None)

    def _play(self, asciicast, idle_time_limit, speed, stdin):
        idle_time_limit = idle_time_limit or asciicast.idle_time_limit

        stdout = asciicast.stdout()
        stdout = frames.to_relative_time(stdout)
        stdout = frames.cap_relative_time(stdout, idle_time_limit)
        stdout = frames.to_absolute_time(stdout)
        stdout = frames.adjust_speed(stdout, speed)

        base_time = time.time()

        for t, text in stdout:
            delay = t - (time.time() - base_time)

            if delay > 0:
                time.sleep(delay)

            sys.stdout.write(text)
            sys.stdout.flush()

            if stdin:
                data = read_non_blocking(stdin.fileno())
                if 0x03 in data:  # ctrl-c
                    break
