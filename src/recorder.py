import os
import timer

from asciicast import Asciicast
from pty_recorder import PtyRecorder


class Recorder(object):

    def __init__(self, pty_recorder=PtyRecorder(), env=os.environ):
        self.pty_recorder = pty_recorder
        self.env = env

    def record(self, cmd, title):
        duration, stdout = timer.timeit(self.pty_recorder.record_command,
                                        cmd or self.env['SHELL'])

        asciicast = Asciicast()
        asciicast.title = title
        asciicast.command = cmd
        asciicast.stdout = stdout
        asciicast.duration = duration

        return asciicast
