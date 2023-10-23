import array
import fcntl
import os
import pty
import select
import signal
import struct
import termios
import time
from typing import Any, Callable, Dict, List, Optional, Tuple

from .tty_ import raw

EXIT_SIGNALS = [
    signal.SIGCHLD,
    signal.SIGHUP,
    signal.SIGTERM,
    signal.SIGQUIT,
]

READ_LEN = 256 * 1024


# pylint: disable=too-many-arguments,too-many-locals,too-many-statements
def record(
    command: Any,
    env: Dict[str, str],
    writer: Any,
    get_tty_size: Callable[[], Tuple[int, int]],
    notify: Callable[[str], None],
    key_bindings: Dict[str, Any],
    tty_stdin_fd: int = pty.STDIN_FILENO,
    tty_stdout_fd: int = pty.STDOUT_FILENO,
) -> None:
    pty_fd: Any = None
    start_time: Optional[float] = None
    pause_time: Optional[float] = None
    prefix_mode: bool = False
    prefix_key = key_bindings.get("prefix")
    pause_key = key_bindings.get("pause")
    add_marker_key = key_bindings.get("add_marker")
    input_data = bytes()

    def handle_resize() -> None:
        size = get_tty_size()
        set_pty_size(size)
        assert start_time is not None
        writer.write_resize(time.perf_counter() - start_time, size)

    def set_pty_size(size: Tuple[int, int]) -> None:
        cols, rows = size
        buf = array.array("h", [rows, cols, 0, 0])
        fcntl.ioctl(pty_fd, termios.TIOCSWINSZ, buf)

    def handle_master_read(data: Any) -> None:
        remaining_data = memoryview(data)
        while remaining_data:
            n = os.write(tty_stdout_fd, remaining_data)
            remaining_data = remaining_data[n:]

        if not pause_time:
            assert start_time is not None
            writer.write_stdout(time.perf_counter() - start_time, data)

    def handle_stdin_read(data: Any) -> None:
        nonlocal input_data
        nonlocal pause_time
        nonlocal start_time
        nonlocal prefix_mode

        if not prefix_mode and prefix_key and data == prefix_key:
            prefix_mode = True
            return

        if prefix_mode or (
            not prefix_key and data in [pause_key, add_marker_key]
        ):
            prefix_mode = False

            if data == pause_key:
                if pause_time:
                    assert start_time is not None
                    start_time += time.perf_counter() - pause_time
                    pause_time = None
                    notify("Resumed recording")
                else:
                    pause_time = time.perf_counter()
                    notify("Paused recording")

            elif data == add_marker_key:
                assert start_time is not None
                writer.write_marker(time.perf_counter() - start_time)
                notify("Marker added")

            return

        input_data += data

        # save stdin unless paused or data is OSC response (e.g. \x1b]11;?\x07)
        if not pause_time and not (
            len(data) > 2
            and data[0] == 0x1B
            and data[1] == 0x5D
            and data[-1] == 0x07
        ):
            assert start_time is not None
            writer.write_stdin(time.perf_counter() - start_time, data)

    def copy(signal_fd: int) -> None:  # pylint: disable=too-many-branches
        nonlocal input_data

        crfds = [pty_fd, tty_stdin_fd, signal_fd]

        while True:
            if len(input_data) > 0:
                cwfds = [pty_fd]
            else:
                cwfds = []

            try:
                rfds, wfds, _ = select.select(crfds, cwfds, [])
            except KeyboardInterrupt:
                if tty_stdin_fd in crfds:
                    crfds.remove(tty_stdin_fd)

                break

            if pty_fd in rfds:
                try:
                    data = os.read(pty_fd, READ_LEN)
                except OSError as e:
                    data = b""

                if not data:  # Reached EOF.
                    break
                else:
                    handle_master_read(data)

            if tty_stdin_fd in rfds:
                data = os.read(tty_stdin_fd, READ_LEN)

                if not data:
                    if tty_stdin_fd in crfds:
                        crfds.remove(tty_stdin_fd)
                else:
                    handle_stdin_read(data)

            if signal_fd in rfds:
                data = os.read(signal_fd, READ_LEN)

                if data:
                    signals = struct.unpack(f"{len(data)}B", data)

                    for sig in signals:
                        if sig in EXIT_SIGNALS:
                            crfds.remove(signal_fd)
                        if sig == signal.SIGWINCH:
                            handle_resize()

            if pty_fd in wfds:
                try:
                    n = os.write(pty_fd, input_data)
                    input_data = input_data[n:]
                except BlockingIOError:
                    pass

    pid, pty_fd = pty.fork()

    if pid == pty.CHILD:
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
        os.execvpe(command[0], command, env)

    flags = fcntl.fcntl(pty_fd, fcntl.F_GETFL, 0) | os.O_NONBLOCK
    fcntl.fcntl(pty_fd, fcntl.F_SETFL, flags)

    start_time = time.perf_counter()
    set_pty_size(get_tty_size())

    with SignalFD(EXIT_SIGNALS + [signal.SIGWINCH]) as sig_fd:
        with raw(tty_stdin_fd):
            try:
                copy(sig_fd)
                os.close(pty_fd)
            except (IOError, OSError):
                pass

    os.waitpid(pid, 0)


class SignalFD:
    def __init__(self, signals: List[signal.Signals]) -> None:
        self.signals = signals
        self.orig_handlers: List[Tuple[signal.Signals, Any]] = []
        self.orig_wakeup_fd: Optional[int] = None

    def __enter__(self) -> int:
        r, w = os.pipe()
        flags = fcntl.fcntl(w, fcntl.F_GETFL, 0) | os.O_NONBLOCK
        fcntl.fcntl(w, fcntl.F_SETFL, flags)
        self.orig_wakeup_fd = signal.set_wakeup_fd(w)

        for sig, handler in self._noop_handlers(self.signals):
            self.orig_handlers.append((sig, signal.signal(sig, handler)))

        return r

    def __exit__(self, type_: str, value: str, traceback: str) -> None:
        assert self.orig_wakeup_fd is not None
        signal.set_wakeup_fd(self.orig_wakeup_fd)

        for sig, handler in self.orig_handlers:
            signal.signal(sig, handler)

    @staticmethod
    def _noop_handlers(
        signals: List[signal.Signals],
    ) -> List[Tuple[signal.Signals, Any]]:
        return list(map(lambda s: (s, lambda signal, frame: None), signals))
