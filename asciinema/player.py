import sys
import time

from asciinema.term import raw, read_non_blocking


class Player:

    def play(self, asciicast, max_wait=None, speed=1.0):
        with raw(sys.stdin.fileno()):
            for delay, text in asciicast.stdout:
                if max_wait and delay > max_wait:
                    delay = max_wait
                time.sleep(delay / speed)
                sys.stdout.write(text)
                sys.stdout.flush()

                data = read_non_blocking(sys.stdin.fileno())
                if 0x03 in data:  # ctrl-c
                    break
