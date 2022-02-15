import os
import time
from typing import Any, Callable, Dict, Optional, Tuple, Type

from . import pty_ as pty  # avoid collisions with standard library `pty`
from . import term
from .asciicast import v2
from .asciicast.v2 import writer as w2
from .async_worker import async_worker


def record(  # pylint: disable=too-many-arguments,too-many-locals
    path_: str,
    command: Any = None,
    append: bool = False,
    idle_time_limit: Optional[int] = None,
    rec_stdin: bool = False,
    title: Optional[str] = None,
    metadata: Any = None,
    command_env: Optional[Dict[Any, Any]] = None,
    capture_env: Any = None,
    writer: Type[w2] = v2.writer,
    record_: Callable[..., None] = pty.record,
    notifier: Any = None,
    key_bindings: Optional[Dict[str, Any]] = None,
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

    w, h = term.get_size()

    full_metadata: Dict[str, Any] = {
        "width": w,
        "height": h,
        "timestamp": int(time.time()),
    }

    full_metadata.update(metadata or {})

    if idle_time_limit is not None:
        full_metadata["idle_time_limit"] = idle_time_limit

    if capture_env:
        full_metadata["env"] = {
            var: command_env.get(var) for var in capture_env
        }

    if title:
        full_metadata["title"] = title

    time_offset: float = 0

    if append and os.stat(path_).st_size > 0:
        time_offset = v2.get_duration(path_)

    with async_notifier(notifier) as _notifier:
        with async_writer(
            writer,
            path_,
            full_metadata,
            append,
            _notifier.queue,
        ) as _writer:
            record_(
                ["sh", "-c", command],
                _writer,
                command_env,
                rec_stdin,
                time_offset,
                _notifier,
                key_bindings,
            )


class async_writer(async_worker):
    def __init__(
        self,
        writer: Type[w2],
        path_: str,
        metadata: Any,
        append: bool = False,
        notifier_q: Any = None,
    ) -> None:
        async_worker.__init__(self)
        self.writer = writer
        self.path = path_
        self.metadata = metadata
        self.append = append
        self.notifier_q = notifier_q

    def write_stdin(self, ts: float, data: Any) -> None:
        self.enqueue([ts, "i", data])

    def write_stdout(self, ts: float, data: Any) -> None:
        self.enqueue([ts, "o", data])

    def run(self) -> None:
        with self.writer(
            self.path,
            metadata=self.metadata,
            append=self.append,
            on_error=self.__on_error,
        ) as w:
            event: Tuple[float, str, Any]
            for event in iter(self.queue.get, None):
                assert event is not None
                ts, etype, data = event

                if etype == "o":
                    w.write_stdout(ts, data)
                elif etype == "i":
                    w.write_stdin(ts, data)

    def __on_error(self, reason: str) -> None:
        if self.notifier_q:
            self.notifier_q.put(reason)


class async_notifier(async_worker):
    def __init__(self, notifier: Any) -> None:
        async_worker.__init__(self)
        self.notifier = notifier

    def notify(self, text: str) -> None:
        self.enqueue(text)

    def perform(self, text: str) -> None:
        try:
            if self.notifier:
                self.notifier.notify(text)
        except:  # pylint: disable=bare-except # noqa: E722
            # we catch *ALL* exceptions here because we don't want failed
            # notification to crash the recording session
            pass
