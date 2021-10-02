import os


class writer():

    def __init__(self, path, metadata=None, append=False, buffering=0):
        if append and os.path.exists(path) and os.stat(path).st_size == 0:  # true for pipes
            append = False

        self.path = path
        self.buffering = buffering
        self.mode = 'ab' if append else 'wb'

    def __enter__(self):
        self.file = open(self.path, mode=self.mode, buffering=self.buffering)
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()

    def write_stdout(self, ts, data):
        self.file.write(data)

    def write_stdin(self, ts, data):
        pass
