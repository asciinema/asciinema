import os
import sys
import time

from asciinema.term import raw, read_non_blocking


class Player:

    def play(self, asciicast, max_wait=None, speed=1.0):
        if os.isatty(sys.stdin.fileno()):
            with raw(sys.stdin.fileno()):
                self._play(asciicast, max_wait, speed, True)
        else:
            self._play(asciicast, max_wait, speed, False)

    def _play(self, asciicast, max_wait, speed, raw):
        for delay, text in asciicast.stdout:
            if max_wait and delay > max_wait:
                delay = max_wait
            time.sleep(delay / speed)
            sys.stdout.write(text)
            sys.stdout.flush()

            if raw:
                data = read_non_blocking(sys.stdin.fileno())
                if 0x03 in data:  # ctrl-c
                    break
