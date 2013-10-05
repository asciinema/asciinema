import os
from . import timer

from .asciicast import Asciicast
from .pty_recorder import PtyRecorder


class Recorder(object):

    def __init__(self, pty_recorder=None, env=None):
        self.pty_recorder = pty_recorder if pty_recorder is not None else PtyRecorder()
        self.env = env if env is not None else os.environ

    def record(self, cmd, title):
        duration, stdout = timer.timeit(self.pty_recorder.record_command,
                                        cmd or self.env['SHELL'])

        asciicast = Asciicast()
        asciicast.title = title
        asciicast.command = cmd
        asciicast.stdout = stdout
        asciicast.duration = duration

        return asciicast
