import time
import io


class StdoutTiming(object):

    def __init__(self):
        self._items = []

    def append(self, item):
        self._items.append(item)

    def __str__(self):
        lines = ["%f %d" % (item[0], item[1]) for item in self._items]
        return "\n".join(lines)


class Stdout(object):

    def __init__(self, timing=None):
        self._data = io.BytesIO()
        self._timing = timing if timing is not None else StdoutTiming()

        self._start_timing()

    @property
    def data(self):
        return self._data.getvalue()

    @property
    def timing(self):
        return str(self._timing).encode()

    def write(self, data):
        now = time.time()
        delta = now - self._prev_time
        self._prev_time = now

        self._data.write(data)
        self._timing.append([delta, len(data)])

    def close(self):
        self._data.close()

    def _start_timing(self):
        self._prev_time = time.time()
