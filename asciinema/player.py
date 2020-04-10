import os
import sys
import time

import asciinema.asciicast.events as ev
from asciinema.term import raw, read_blocking


class Player:

    def play(self, asciicast, idle_time_limit=None, speed=1.0, show_timer=False):
        try:
            stdin = open('/dev/tty')
            with raw(stdin.fileno()):
                self._play(asciicast, idle_time_limit, speed, stdin, show_timer)
        except Exception:
            self._play(asciicast, idle_time_limit, speed, None, show_timer)

    def _show_timer(self, asciicast, time):
        if hasattr(asciicast, 'v2_header'):
            y = 1
            x = asciicast.v2_header['width'] - 20
            padded = '{:4.6f}'.format(time)
            sys.stdout.write("\u001b[s\u001b[{y};{x}H\u001b[7m{padded:>20s}\u001b[m\u001b[u".format(**locals()))
            sys.stdout.flush()

    def _play(self, asciicast, idle_time_limit, speed, stdin, show_timer):
        idle_time_limit = idle_time_limit or asciicast.idle_time_limit

        stdout = asciicast.stdout_events()
        stdout = ev.to_relative_time(stdout)
        stdout = ev.cap_relative_time(stdout, idle_time_limit)
        stdout = ev.to_absolute_time(stdout)
        stdout = ev.adjust_speed(stdout, speed)

        base_time = time.time()
        ctrl_c = False
        paused = False
        pause_time = None

        for t, _type, text in stdout:
            delay = t - (time.time() - base_time)

            while stdin and not ctrl_c and delay > 0:
                if show_timer:
                    self._show_timer(asciicast, t)

                if paused:
                    while True:
                        data = read_blocking(stdin.fileno(), 1000)

                        if 0x03 in data:  # ctrl-c
                            ctrl_c = True
                            break

                        if 0x20 in data:  # space
                            paused = False
                            base_time = base_time + (time.time() - pause_time)
                            break

                        if 0x2e in data:  # period (dot)
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

                    if 0x20 in data:  # space
                        paused = True
                        pause_time = time.time()
                        slept = t - (pause_time - base_time)
                        delay = delay - slept

            if ctrl_c:
                break

            sys.stdout.write(text)
            sys.stdout.flush()
