import curses
import sys
import time

class CursesWrapper:
    def __init__(self):
        self._stdscr = curses.initscr()

    def __enter__(self):
        curses.noecho()
        curses.cbreak()
        self._stdscr.keypad(1)
        return self

    def __exit__(self, type, value, traceback):
        curses.flushinp()
        curses.endwin()

class Player:

    def play(self, asciicast, max_wait=None, speed=1.0):
        with CursesWrapper():
            for delay, text in asciicast.stdout:
                if max_wait and delay > max_wait:
                    delay = max_wait
                time.sleep(delay / speed)
                sys.stdout.write(text)
                sys.stdout.flush()
