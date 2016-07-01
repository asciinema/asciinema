import sys
import time

class Player:

    def play(self, asciicast):
        for delay, text in asciicast.stdout:
            time.sleep(delay)
            sys.stdout.write(text)
            sys.stdout.flush()
