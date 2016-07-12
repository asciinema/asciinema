import os
import subprocess

from .asciicast import Asciicast
from .pty_recorder import PtyRecorder
from .stdout import Stdout


class Recorder:

    def __init__(self, pty_recorder=None, env=None):
        self.pty_recorder = pty_recorder if pty_recorder is not None else PtyRecorder()
        self.env = env if env is not None else os.environ

    def record(self, path, user_command, title, max_wait):
        command = user_command or self.env.get('SHELL') or 'sh'
        stdout = Stdout(max_wait)
        env = os.environ.copy()
        env['ASCIINEMA_REC'] = '1'

        self.pty_recorder.record_command(['sh', '-c', command], stdout, env)

        width = int(subprocess.check_output(['tput', 'cols']))
        height = int(subprocess.check_output(['tput', 'lines']))

        asciicast = Asciicast(
            stdout,
            width,
            height,
            stdout.duration,
            command=user_command,
            title=title,
            term=self.env.get('TERM'),
            shell=self.env.get('SHELL')
        )

        asciicast.save(path)
