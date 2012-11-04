import time
import StringIO
import bz2


class TimedFile(object):
    '''File wrapper that records write times in separate file.'''

    def __init__(self, filename):
        self.data_filename = filename
        self.timing_filename = filename + '.time'

        self.mem_data_file = None
        self.mem_timing_file = None

        self.start_timing()

    def start_timing(self):
        self.prev_time = time.time()

    def write(self, data):
        if not self.mem_data_file:
            self.mem_data_file = StringIO.StringIO()
            self.mem_timing_file = StringIO.StringIO()

        now = time.time()
        delta = now - self.prev_time
        self.prev_time = now

        self.mem_data_file.write(data)
        self.mem_timing_file.write("%f %d\n" % (delta, len(data)))

    def close(self):
        if not self.mem_data_file:
            return

        bz2_data_file = bz2.BZ2File(self.data_filename, 'w')
        bz2_data_file.write(self.mem_data_file.getvalue())
        bz2_data_file.close()
        self.mem_data_file.close()

        bz2_timing_file = bz2.BZ2File(self.timing_filename, 'w')
        bz2_timing_file.write(self.mem_timing_file.getvalue())
        bz2_timing_file.close()
        self.mem_timing_file.close()
