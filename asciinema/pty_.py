import array
import errno
import fcntl
import os
import pty
import select
import signal
import struct
import termios
import time
from typing import Any, Callable, Dict, List, Optional, Tuple

from .term import raw


# pylint: disable=too-many-arguments,too-many-locals,too-many-statements
def record(
    command: Any,
    writer: Any,
    get_tty_size: Callable[[], Tuple[int, int]],
    env: Any = None,
    rec_stdin: bool = False,
    time_offset: float = 0,
    notifier: Any = None,
    key_bindings: Optional[Dict[str, Any]] = None,
    tty_stdin_fd: int = pty.STDIN_FILENO,
    tty_stdout_fd: int = pty.STDOUT_FILENO,
) -> None:
    if env is None:
        env = os.environ
    if key_bindings is None:
        key_bindings = {}
    master_fd: Any = None
    start_time: Optional[float] = None
    pause_time: Optional[float] = None
    prefix_mode: bool = False
    prefix_key = key_bindings.get("prefix")
    pause_key = key_bindings.get("pause")

    def _notify(text: str) -> None:
        if notifier:
            notifier.notify(text)

    def _set_pty_size() -> None:
        """
        Sets the window size of the child pty based on the window size
        of our own controlling terminal.
        """

        # 1. Get the terminal size of the real terminal.
        # 2. Set the same size on the pseudoterminal.

        cols, rows = get_tty_size()
        buf = array.array("h", [rows, cols, 0, 0])
        fcntl.ioctl(master_fd, termios.TIOCSWINSZ, buf)

    def _write_stdout(data: Any) -> None:
        """Writes to stdout as if the child process had written the data."""

        os.write(tty_stdout_fd, data)

    def _handle_master_read(data: Any) -> None:
        """Handles new data on child process stdout."""

        if not pause_time:
            assert start_time is not None
            writer.write_stdout(time.time() - start_time, data)

        _write_stdout(data)

    def _write_master(data: Any) -> None:
        """Writes to the child process from its controlling terminal."""

        while data:
            n = os.write(master_fd, data)
            data = data[n:]

    def _handle_stdin_read(data: Any) -> None:
        """Handles new data on child process stdin."""

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
                    _notify("Resumed recording")
                else:
                    pause_time = time.time()
                    _notify("Paused recording")

            return

        _write_master(data)

        if rec_stdin and not pause_time:
            assert start_time is not None
            writer.write_stdin(time.time() - start_time, data)

    def _signals(signal_list: Any) -> List[Tuple[Any, Any]]:
        old_handlers = []
        for sig, handler in signal_list:
            old_handlers.append((sig, signal.signal(sig, handler)))
        return old_handlers

    def _copy(signal_fd: int) -> None:  # pylint: disable=too-many-branches
        """Main select loop.

        Passes control to _master_read() or _stdin_read()
        when new data arrives.
        """

        fds = [master_fd, tty_stdin_fd, signal_fd]

        while True:
            try:
                rfds, _, _ = select.select(fds, [], [])
            except OSError as e:  # Python >= 3.3
                if e.errno == errno.EINTR:
                    continue

            if master_fd in rfds:
                data = os.read(master_fd, 1024)
                if not data:  # Reached EOF.
                    fds.remove(master_fd)
                else:
                    _handle_master_read(data)

            if tty_stdin_fd in rfds:
                data = os.read(tty_stdin_fd, 1024)
                if not data:
                    fds.remove(tty_stdin_fd)
                else:
                    _handle_stdin_read(data)

            if signal_fd in rfds:
                data = os.read(signal_fd, 1024)
                if data:
                    signals = struct.unpack(f"{len(data)}B", data)
                    for sig in signals:
                        if sig in [
                            signal.SIGCHLD,
                            signal.SIGHUP,
                            signal.SIGTERM,
                            signal.SIGQUIT,
                        ]:
                            os.close(master_fd)
                            return
                        if sig == signal.SIGWINCH:
                            _set_pty_size()

    pid, master_fd = pty.fork()

    if pid == pty.CHILD:
        os.execvpe(command[0], command, env)

    pipe_r, pipe_w = os.pipe()
    flags = fcntl.fcntl(pipe_w, fcntl.F_GETFL, 0)
    flags = flags | os.O_NONBLOCK
    flags = fcntl.fcntl(pipe_w, fcntl.F_SETFL, flags)

    signal.set_wakeup_fd(pipe_w)

    old_handlers = _signals(
        map(
            lambda s: (s, lambda signal, frame: None),
            [
                signal.SIGWINCH,
                signal.SIGCHLD,
                signal.SIGHUP,
                signal.SIGTERM,
                signal.SIGQUIT,
            ],
        )
    )

    _set_pty_size()

    start_time = time.time() - time_offset

    with raw(tty_stdin_fd):
        try:
            _copy(pipe_r)
        except (IOError, OSError):
            pass

    _signals(old_handlers)

    os.waitpid(pid, 0)
