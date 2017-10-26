import os
import sys
import time

from asciinema.term import raw, read_non_blocking


def compress_time(stdout, idle_time_limit):
    if idle_time_limit:
        return ([min(delay, idle_time_limit), text] for delay, text in stdout)
    else:
        return stdout


def adjust_speed(stdout, speed):
    return ([delay / speed, text] for delay, text in stdout)


class Player:

    def play(self, asciicast, idle_time_limit=None, speed=1.0):
        if os.isatty(sys.stdin.fileno()):
            with raw(sys.stdin.fileno()):
                self._play(asciicast, idle_time_limit, speed, True)
        else:
            self._play(asciicast, idle_time_limit, speed, False)

    def _play(self, asciicast, idle_time_limit, speed, raw):
        idle_time_limit = idle_time_limit or asciicast.idle_time_limit

        stdout = asciicast.stdout()
        stdout = compress_time(stdout, idle_time_limit)
        stdout = adjust_speed(stdout, speed)

        for delay, text in stdout:
            time.sleep(delay)
            sys.stdout.write(text)
            sys.stdout.flush()

            if raw:
                data = read_non_blocking(sys.stdin.fileno())
                if 0x03 in data:  # ctrl-c
                    break
