import time
import codecs


class Stdout:

    def __init__(self, timing=None):
        self.frames = []
        self.last_write_time = time.time()
        self.decoder = codecs.getincrementaldecoder('UTF-8')('replace')

    def write(self, data):
        now = time.time()
        delay = now - self.last_write_time
        # delay = int(delay * 1000000) / 1000000.0 # millisecond precission
        string = self.decoder.decode(data)
        self.frames.append([delay, string])
        self.last_write_time = now

    def close(self):
        pass
