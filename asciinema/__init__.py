import sys

__author__ = "Marcin Kulik"
__version__ = "2.0.2"

if sys.version_info < (3, 6):
    raise ImportError("Python < 3.6 is unsupported.")

# pylint: disable=wrong-import-position
from typing import Any, Optional

from .recorder import record


def record_asciicast(  # pylint: disable=too-many-arguments
    path_: str,
    command: Any = None,
    append: bool = False,
    idle_time_limit: Optional[int] = None,
    rec_stdin: bool = False,
    title: Optional[str] = None,
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
