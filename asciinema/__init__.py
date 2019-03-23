import sys

__author__ = 'Marcin Kulik'
__version__ = '2.0.2'

if sys.version_info[0] < 3:
    raise ImportError('Python < 3 is unsupported.')

import asciinema.recorder


def record_asciicast(path, command=None, append=False, idle_time_limit=None,
                     rec_stdin=False, title=None, metadata=None,
                     command_env=None, capture_env=None):
    asciinema.recorder.record(
        path,
        command=command,
        append=append,
        idle_time_limit=idle_time_limit,
        rec_stdin=rec_stdin,
        title=title,
        metadata=metadata,
        command_env=command_env,
        capture_env=capture_env
    )
