import sys
import time

class Player:

    def play(self, asciicast, max_wait=None):
        for delay, text in asciicast.stdout:
            if max_wait and delay > max_wait:
                delay = max_wait
            time.sleep(delay)
            sys.stdout.write(text)
            sys.stdout.flush()
