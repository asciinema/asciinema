import sys
import time

class Player:

    def play(self, asciicast, max_wait=None, start_at=0):
        start = 0
        for delay, text in asciicast.stdout:
            start = start + delay
            if start < start_at:
                continue

            if max_wait and delay > max_wait:
                delay = max_wait
            time.sleep(delay)
            sys.stdout.write(text)
            sys.stdout.flush()
