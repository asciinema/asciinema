import os
import subprocess
import json
import time
import codecs
from multiprocessing import Process, Queue

from .asciicast import Asciicast
from .pty_recorder import PtyRecorder
from .stdout import Stdout


def write_json_lines_from_queue(path, queue):
    with open(path, 'w') as f:
        for json_value in iter(queue.get, None):
            f.write(json.dumps(json_value, ensure_ascii=False) + "\n")


class v2_writer():

    def __init__(self, path, header):
        self.tmp_path = path + '.tmp'
        self.final_path = path
        self.header = header
        self.queue = Queue()
        self.decoder = codecs.getincrementaldecoder('UTF-8')('replace')

    def __enter__(self):
        self.process = Process(
            target=write_json_lines_from_queue,
            args=(self.tmp_path, self.queue)
        )
        self.process.start()
        self.queue.put(self.header)
        self.start_time = time.time()
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.queue.put(None)
        self.process.join()
        os.rename(self.tmp_path, self.final_path)

    def write(self, data):
        text = self.decoder.decode(data)

        if text:
            ts = round(time.time() - self.start_time, 6)
            self.queue.put([ts, 'o', text])

        return len(data)


class Recorder:

    def __init__(self, pty_recorder=None, env=None):
        self.pty_recorder = pty_recorder if pty_recorder is not None else PtyRecorder()
        self.env = env if env is not None else os.environ

    def record(self, path, user_command, title, max_wait):
        cols = int(subprocess.check_output(['tput', 'cols']))
        lines = int(subprocess.check_output(['tput', 'lines']))

        saved_env = {
            'TERM': self.env.get('TERM'),
            'SHELL': self.env.get('SHELL')
        }

        header = {
            'version': 2,
            'width': cols,
            'height': lines,
            'env': saved_env,
            # TODO save max_wait here
        }

        if title:
            header['title'] = title

        if user_command:
            header['command'] = user_command

        command = user_command or self.env.get('SHELL') or 'sh'

        with v2_writer(path, header) as w:
            command_env = os.environ.copy()
            command_env['ASCIINEMA_REC'] = '1'
            self.pty_recorder.record_command(['sh', '-c', command], w, command_env)
