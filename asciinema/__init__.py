import sys

__author__ = 'Marcin Kulik'
__version__ = '2.0.2'

if sys.version_info[0] < 3:
    raise ImportError('Python < 3 is unsupported.')


import os
import subprocess
import time

import asciinema.asciicast.v2 as v2
import asciinema.pty as pty
import asciinema.term as term


def record_asciicast(path, command=None, append=False, idle_time_limit=None,
                     rec_stdin=False, title=None, metadata=None,
                     command_env=None, capture_env=None, writer=v2.async_writer,
                     record=pty.record):
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

    with writer(path, full_metadata, append, time_offset) as w:
        record(['sh', '-c', command], w, command_env, rec_stdin)
