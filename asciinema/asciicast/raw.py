import os
from multiprocessing import Process, Queue

from asciinema.pty_recorder import PtyRecorder


def write_bytes_from_queue(path, mode, queue):
    mode = mode + 'b'
    with open(path, mode=mode, buffering=0) as f:
        for data in iter(queue.get, None):
            f.write(data)


class writer():

    def __init__(self, path, append):
        self.path = path
        self.mode = 'a' if append else 'w'
        self.queue = Queue()

    def __enter__(self):
        self.process = Process(
            target=write_bytes_from_queue,
            args=(self.path, self.mode, self.queue)
        )
        self.process.start()
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.queue.put(None)
        self.process.join()

    def write_stdin(self, data):
        pass

    def write_stdout(self, data):
        self.queue.put(data)


class Recorder:

    def __init__(self, pty_recorder=None):
        self.pty_recorder = pty_recorder if pty_recorder is not None else PtyRecorder()

    def record(self, path, append, command, command_env, _captured_env, _rec_stdin, _title, _idle_time_limit):
        if os.path.exists(path) and os.stat(path).st_size == 0:  # true for pipes
            append = False

        with writer(path, append) as w:
            self.pty_recorder.record_command(['sh', '-c', command], w, command_env)
