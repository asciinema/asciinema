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

    def set_pty_size() -> None:
        cols, rows = get_tty_size()
        buf = array.array("h", [rows, cols, 0, 0])
        fcntl.ioctl(pty_fd, termios.TIOCSWINSZ, buf)

    def handle_master_read(data: Any) -> None:
        os.write(tty_stdout_fd, data)

        if not pause_time:
            assert start_time is not None
            writer.write_stdout(time.time() - start_time, data)

    def handle_stdin_read(data: Any) -> None:
        nonlocal pause_time
        nonlocal start_time
        nonlocal prefix_mode

        if not prefix_mode and prefix_key and data == prefix_key:
            prefix_mode = True
            return

        if prefix_mode or (not prefix_key and data in [pause_key]):
            prefix_mode = False

            if data == pause_key:
                if pause_time:
                    assert start_time is not None
                    start_time += time.time() - pause_time
                    pause_time = None
                    notify("Resumed recording")
                else:
                    pause_time = time.time()
                    notify("Paused recording")

            return

        remaining_data = data
        while remaining_data:
            n = os.write(pty_fd, remaining_data)
            remaining_data = remaining_data[n:]

        # save stdin unless paused or data is OSC response (e.g. \x1b]11;?\x07)
        if not pause_time and not (
            len(data) > 2
            and data[0] == 0x1B
            and data[1] == 0x5D
            and data[-1] == 0x07
        ):
            assert start_time is not None
            writer.write_stdin(time.time() - start_time, data)

    def copy(signal_fd: int) -> None:  # pylint: disable=too-many-branches
        fds = [pty_fd, tty_stdin_fd, signal_fd]

        while True:
            try:
                rfds, _, _ = select.select(fds, [], [])
            except KeyboardInterrupt:
                if tty_stdin_fd in fds:
                    fds.remove(tty_stdin_fd)

                break

            if pty_fd in rfds:
                try:
                    data = os.read(pty_fd, 1024)
                except OSError as e:
                    data = b""

                if not data:  # Reached EOF.
                    break
                else:
                    handle_master_read(data)

            if tty_stdin_fd in rfds:
                data = os.read(tty_stdin_fd, 1024)

                if not data:
                    if tty_stdin_fd in fds:
                        fds.remove(tty_stdin_fd)
                else:
                    handle_stdin_read(data)

            if signal_fd in rfds:
                data = os.read(signal_fd, 1024)

                if data:
                    signals = struct.unpack(f"{len(data)}B", data)

                    for sig in signals:
                        if sig in EXIT_SIGNALS:
                            fds.remove(signal_fd)
                        if sig == signal.SIGWINCH:
                            set_pty_size()

    pid, pty_fd = pty.fork()

    if pid == pty.CHILD:
        os.execvpe(command[0], command, env)

    start_time = time.time()
    set_pty_size()

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
