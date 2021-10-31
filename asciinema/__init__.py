import sys

__author__ = "Marcin Kulik"
__version__ = "2.0.2"

if sys.version_info < (3, 7):
    raise ImportError("Python < 3.7 is unsupported.")

# pylint: disable=wrong-import-position
from typing import Any

from .recorder import record


def record_asciicast(
    path_,
    command=None,
    append=False,
    idle_time_limit=None,
    rec_stdin=False,
    title=None,
    metadata: Any = None,
    command_env: Any = None,
    capture_env: Any = None,
) -> None:
    record(
        path_,
        command=command,
        append=append,
        idle_time_limit=idle_time_limit,
        rec_stdin=rec_stdin,
        title=title,
        metadata=metadata,
        command_env=command_env,
        capture_env=capture_env,
    )
