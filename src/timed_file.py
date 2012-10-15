import time
import StringIO
import bz2

class TimedFile(object):
    '''File wrapper that records write times in separate file.'''

    def __init__(self, filename):
        self.filename = filename

        self.data_file = StringIO.StringIO()
        self.time_file = StringIO.StringIO()

        self.old_time = time.time()

    def write(self, data):
        self.data_file.write(data)
        now = time.time()
        delta = now - self.old_time
        self.time_file.write("%f %d\n" % (delta, len(data)))
        self.old_time = now

    def close(self):
        mode = 'w'

        bz2_data_file = bz2.BZ2File(self.filename, mode)
        bz2_data_file.write(self.data_file.getvalue())
        bz2_data_file.close()

        bz2_time_file = bz2.BZ2File(self.filename + '.time', mode)
        bz2_time_file.write(self.time_file.getvalue())
        bz2_time_file.close()

        self.data_file.close()
        self.time_file.close()
