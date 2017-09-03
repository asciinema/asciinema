import os
import subprocess
import json
import time
import codecs
from multiprocessing import Process, Queue

from asciinema.pty_recorder import PtyRecorder


class Asciicast:

    def __init__(self, f, max_wait):
        self.version = 2
        self.__file = f
        self.max_wait = max_wait

    def stdout(self):
        prev_ts = 0

        for line in self.__file:
            ts, type, data = json.loads(line)

            if type == 'o':
                delay = ts - prev_ts
                prev_ts = ts
                yield [delay, data]


def load_from_file(f):
    header = json.loads(f.readline())
    max_wait = header.get('max_wait')
    return Asciicast(f, max_wait)


def write_json_lines_from_queue(path, queue):
    with open(path, 'w') as f:
        for json_value in iter(queue.get, None):
            line = json.dumps(json_value, ensure_ascii=False, indent=None, separators=(', ', ': '))
            f.write(line + '\n')


class incremental_writer():

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

        with incremental_writer(path, header) as w:
            command_env = os.environ.copy()
            command_env['ASCIINEMA_REC'] = '1'
            self.pty_recorder.record_command(['sh', '-c', command], w, command_env)
