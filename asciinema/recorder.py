import os
import sys
import time
from typing import Any, Callable, Dict, List, Optional, TextIO, Tuple, Type

from . import pty_ as pty  # avoid collisions with standard library `pty`
from .asciicast import v2
from .asciicast.v2 import writer as w2
from .async_worker import async_worker


def record(  # pylint: disable=too-many-arguments,too-many-locals
    path_: str,
    command: Optional[str] = None,
    append: bool = False,
    idle_time_limit: Optional[float] = None,
    record_stdin: bool = False,
    title: Optional[str] = None,
    command_env: Optional[Dict[str, str]] = None,
    capture_env: Optional[List[str]] = None,
    writer: Type[w2] = v2.writer,
    record_: Callable[..., None] = pty.record,
    notify: Callable[[str], None] = lambda _: None,
    key_bindings: Optional[Dict[str, Any]] = None,
    cols_override: Optional[int] = None,
    rows_override: Optional[int] = None,
) -> None:
    if command is None:
        command = os.environ.get("SHELL", "sh")

    if command_env is None:
        command_env = os.environ.copy()

    if key_bindings is None:
        key_bindings = {}

    command_env["ASCIINEMA_REC"] = "1"

    if capture_env is None:
        capture_env = ["SHELL", "TERM"]

    time_offset: float = 0

    if append and os.stat(path_).st_size > 0:
        time_offset = v2.get_duration(path_)

    with tty_fds() as (tty_stdin_fd, tty_stdout_fd), async_notifier(
        notify
    ) as _notifier:
        get_tty_size = _get_tty_size(
            tty_stdout_fd, cols_override, rows_override
        )
        cols, rows = get_tty_size()
        metadata = build_metadata(
            cols, rows, idle_time_limit, capture_env, command_env, title
        )

        sync_writer = writer(
            path_, metadata, append, on_error=_notifier.queue.put
        )

        with async_writer(sync_writer, time_offset, record_stdin) as _writer:
            record_(
                ["sh", "-c", command],
                command_env,
                _writer,
                get_tty_size,
                _notifier.notify,
                key_bindings,
                tty_stdin_fd=tty_stdin_fd,
                tty_stdout_fd=tty_stdout_fd,
            )


class tty_fds:
    def __init__(self) -> None:
        self.stdin_file: Optional[TextIO] = None
        self.stdout_file: Optional[TextIO] = None

    def __enter__(self) -> Tuple[int, int]:
        try:
            self.stdout_file = open("/dev/tty", "wt", encoding="utf_8")
        except OSError:
            self.stdout_file = open("/dev/null", "wt", encoding="utf_8")

        return (sys.stdin.fileno(), self.stdout_file.fileno())

    def __exit__(self, type_: str, value: str, traceback: str) -> None:
        assert self.stdout_file is not None
        self.stdout_file.close()


def build_metadata(  # pylint: disable=too-many-arguments
    cols: int,
    rows: int,
    idle_time_limit: Optional[float],
    capture_env: List[str],
    env: Dict[str, str],
    title: Optional[str],
) -> Dict[str, Any]:
    metadata: Dict[str, Any] = {
        "width": cols,
        "height": rows,
        "timestamp": int(time.time()),
    }

    if idle_time_limit is not None:
        metadata["idle_time_limit"] = idle_time_limit

    metadata["env"] = {var: env.get(var) for var in capture_env}

    if title:
        metadata["title"] = title

    return metadata


class async_writer(async_worker):
    def __init__(
        self, writer: w2, time_offset: float, record_stdin: bool
    ) -> None:
        async_worker.__init__(self)
        self.writer = writer
        self.time_offset = time_offset
        self.record_stdin = record_stdin

    def write_stdin(self, ts: float, data: Any) -> None:
        if self.record_stdin:
            self.enqueue([ts, "i", data])

    def write_stdout(self, ts: float, data: Any) -> None:
        self.enqueue([ts, "o", data])

    def write_marker(self, ts: float) -> None:
        self.enqueue([ts, "m", None])

    def write_resize(self, ts: float, size: Tuple[int, int]) -> None:
        self.enqueue([ts, "r", size])

    def run(self) -> None:
        try:
            with self.writer as w:
                event: Tuple[float, str, Any]
                for event in iter(self.queue.get, None):
                    assert event is not None
                    ts, etype, data = event

                    if etype == "o":
                        w.write_stdout(self.time_offset + ts, data)
                    elif etype == "i":
                        w.write_stdin(self.time_offset + ts, data)
                    elif etype == "m":
                        w.write_marker(self.time_offset + ts)
                    elif etype == "r":
                        w.write_resize(self.time_offset + ts, data)
        except IOError:
            for event in iter(self.queue.get, None):
                pass


class async_notifier(async_worker):
    def __init__(self, notify: Callable[[str], None]) -> None:
        async_worker.__init__(self)
        self._notify = notify

    def notify(self, text: str) -> None:
        self.enqueue(text)

    def perform(self, text: str) -> None:
        try:
            self._notify(text)
        except:  # pylint: disable=bare-except # noqa: E722
            # we catch *ALL* exceptions here because we don't want failed
            # notification to crash the recording session
            pass


def _get_tty_size(
    fd: int, cols_override: Optional[int], rows_override: Optional[int]
) -> Callable[[], Tuple[int, int]]:
    if cols_override is not None and rows_override is not None:

        def fixed_size() -> Tuple[int, int]:
            return (cols_override, rows_override)  # type: ignore

        return fixed_size

    if not os.isatty(fd):

        def fallback_size() -> Tuple[int, int]:
            return (cols_override or 80, rows_override or 24)

        return fallback_size

    def size() -> Tuple[int, int]:
        cols, rows = os.get_terminal_size(fd)
        return (cols_override or cols, rows_override or rows)

    return size
