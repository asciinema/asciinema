import os
import time

import asciinema.asciicast.v2 as v2
import asciinema.pty as pty
import asciinema.term as term
from asciinema.async_worker import async_worker


def record(path, command=None, append=False, idle_time_limit=None,
           rec_stdin=False, title=None, metadata=None, command_env=None,
           capture_env=None, writer=v2.writer, record=pty.record, notifier=None):
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

    with async_writer(writer, path, full_metadata, append) as w:
        with async_notifier(notifier) as n:
            record(
                ['sh', '-c', command],
                w,
                command_env,
                rec_stdin,
                time_offset,
                n
            )


class async_writer(async_worker):
    def __init__(self, writer, path, metadata, append=False):
        async_worker.__init__(self)
        self.writer = writer
        self.path = path
        self.metadata = metadata
        self.append = append

    def write_stdin(self, ts, data):
        self.enqueue([ts, 'i', data])

    def write_stdout(self, ts, data):
        self.enqueue([ts, 'o', data])

    def run(self):
        with self.writer(self.path, metadata=self.metadata, append=self.append) as w:
            for event in iter(self.queue.get, None):
                ts, etype, data = event

                if etype == 'o':
                    w.write_stdout(ts, data)
                elif etype == 'i':
                    w.write_stdin(ts, data)


class async_notifier(async_worker):
    def __init__(self, notifier):
        async_worker.__init__(self)
        self.notifier = notifier

    def notify(self, text):
        self.enqueue(text)

    def perform(self, text):
        try:
            if self.notifier:
                self.notifier.notify(text)
        except:
            # we catch *ALL* exceptions here because we don't want failed
            # notification to crash the recording session
            pass
