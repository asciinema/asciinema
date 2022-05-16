from typing import Any, Optional

try:
    # Importing synchronize is to detect platforms where
    # multiprocessing does not work (python issue 3770)
    # and cause an ImportError. Otherwise it will happen
    # later when trying to use Queue().
    from multiprocessing import Process, Queue, synchronize

    # pylint: disable=pointless-statement
    lambda _=synchronize: None  # avoid pruning import
except ImportError:
    from queue import Queue  # type: ignore
    from threading import Thread as Process  # type: ignore


class async_worker:
    def __init__(self) -> None:
        self.queue: Queue[Any] = Queue()
        self.process: Optional[Process] = None

    def __enter__(self) -> Any:
        self.process = Process(target=self._run)
        self.process.start()
        return self

    def _run(self) -> None:
        try:
            self.run()
        except KeyboardInterrupt:
            pass

    def __exit__(
        self, exc_type: str, exc_value: str, exc_traceback: str
    ) -> None:
        self.queue.put(None)
        assert isinstance(self.process, Process)
        self.process.join()

        if self.process.exitcode != 0:
            raise RuntimeError(
                f"worker process exited with code {self.process.exitcode}"
            )

    def enqueue(self, payload: Any) -> None:
        self.queue.put(payload)

    def run(self) -> None:
        payload: Any
        for payload in iter(self.queue.get, None):
            # pylint: disable=no-member
            self.perform(payload)  # type: ignore[attr-defined]
