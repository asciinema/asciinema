import json


class Asciicast:

    def __init__(self, f, max_wait):
        self.version = 2
        self.__file = f
        self.max_wait = max_wait

    def stdout(self):
        prev_ts = 0

        for line in self.__file:
            ts, type, data = json.loads(line)

            if type == 'o':
                delay = ts - prev_ts
                prev_ts = ts
                yield [delay, data]


def load_from_file(f):
    header = json.loads(f.readline())
    max_wait = header.get('max_wait')
    return Asciicast(f, max_wait)
