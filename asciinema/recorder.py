import os
import time

try:
    # Importing synchronize is to detect platforms where
    # multiprocessing does not work (python issue 3770)
    # and cause an ImportError. Otherwise it will happen
    # later when trying to use Queue().
    from multiprocessing import synchronize, Process, Queue
except ImportError:
    from threading import Thread as Process
    from queue import Queue

import asciinema.asciicast.v2 as v2
import asciinema.pty as pty
import asciinema.term as term


def record(path, command=None, append=False, idle_time_limit=None,
           rec_stdin=False, title=None, metadata=None, command_env=None,
           capture_env=None, writer=v2.writer, record=pty.record):
    if command is None:
        command = os.environ.get('SHELL') or 'sh'

    if command_env is None:
        command_env = os.environ.copy()
        command_env['ASCIINEMA_REC'] = '1'

    if capture_env is None:
        capture_env = ['SHELL', 'TERM']

    w, h = term.get_size()

    full_metadata = {
        'width': w,
        'height': h,
        'timestamp': int(time.time())
    }

    full_metadata.update(metadata or {})

    if idle_time_limit is not None:
        full_metadata['idle_time_limit'] = idle_time_limit

    if capture_env:
        full_metadata['env'] = {var: command_env.get(var) for var in capture_env}

    if title:
        full_metadata['title'] = title

    time_offset = 0

    if append and os.stat(path).st_size > 0:
        time_offset = v2.get_duration(path)

    with async_writer(writer, path, full_metadata, append, time_offset) as w:
        record(['sh', '-c', command], w, command_env, rec_stdin)


def write_events_from_queue(writer, path, metadata, append, queue):
    with writer(path, metadata=metadata, append=append) as w:
        for event in iter(queue.get, None):
            ts, etype, data = event

            if etype == 'o':
                w.write_stdout(ts, data)
            elif etype == 'i':
                w.write_stdin(ts, data)


class async_writer():

    def __init__(self, writer, path, metadata, append=False, time_offset=0):
        if append:
            assert time_offset > 0

        self.writer = writer
        self.path = path
        self.metadata = metadata
        self.append = append
        self.time_offset = time_offset
        self.queue = Queue()

    def __enter__(self):
        self.process = Process(
            target=write_events_from_queue,
            args=(self.writer, self.path, self.metadata, self.append, self.queue)
        )
        self.process.start()
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.queue.put(None)
        self.process.join()

    def write_stdin(self, ts, data):
        ts = ts + self.time_offset
        self.queue.put([ts, 'i', data])

    def write_stdout(self, ts, data):
        ts = ts + self.time_offset
        self.queue.put([ts, 'o', data])
