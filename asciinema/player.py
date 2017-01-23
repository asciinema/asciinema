import sys
import time


class Player:

    def play(self, asciicast, max_wait=None, speed=1.0):
        for delay, text in asciicast.stdout:
            if max_wait and delay > max_wait:
                delay = max_wait
            time.sleep(delay / speed)
            sys.stdout.write(text)
            sys.stdout.flush()
