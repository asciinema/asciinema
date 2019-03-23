try:
    # Importing synchronize is to detect platforms where
    # multiprocessing does not work (python issue 3770)
    # and cause an ImportError. Otherwise it will happen
    # later when trying to use Queue().
    from multiprocessing import synchronize, Process, Queue
except ImportError:
    from threading import Thread as Process
    from queue import Queue


class async_worker():

    def __init__(self):
        self.queue = Queue()

    def __enter__(self):
        self.process = Process(target=self.run)
        self.process.start()
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.queue.put(None)
        self.process.join()

    def enqueue(self, payload):
        self.queue.put(payload)

    def run(self):
        for payload in iter(self.queue.get, None):
            self.perform(payload)
